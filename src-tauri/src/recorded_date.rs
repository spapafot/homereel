//! The day a video was recorded. Drive only knows when a file was uploaded, and its modified
//! time is often the day a copy or export was made, so the date comes from the file itself.

/// Before this, a date is a camera's unset clock (1904 or 1970), not a real one.
const FIRST_PLAUSIBLE_YEAR: u32 = 1990;
const LAST_PLAUSIBLE_YEAR: u32 = 2100;

/// The recording date, `YYYY-MM-DD`: a date in the file name first, as cameras write it there
/// when recording, else the file's metadata.
pub fn recorded_date(file_name: &str, metadata_date: Option<String>) -> Option<String> {
    from_file_name(file_name).or(metadata_date)
}

/// Finds a date a camera wrote into a file name, such as `VID_20240612_181455.mp4`,
/// `08152019_194329.m4v` or `Screen Recording 2024-06-12 at 18.14.55.mov`.
pub fn from_file_name(file_name: &str) -> Option<String> {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    stem.split(|c: char| !c.is_ascii_digit() && c != '-')
        .map(|token| token.trim_matches('-'))
        .find_map(|token| dashed_date(token).or_else(|| token.split('-').find_map(compact_date)))
}

/// Reads the recording date from ffmpeg's input summary. iPhones store the local date in
/// `com.apple.quicktime.creationdate`; Apple's exports keep the original in `date` while
/// `creation_time` becomes the export date; other cameras write only `creation_time`.
pub fn from_ffmpeg_metadata(stderr: &str) -> Option<String> {
    let metadata = |wanted: &str| {
        stderr.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == wanted).then(|| value.trim())
        })
    };
    ["com.apple.quicktime.creationdate", "date", "creation_time"]
        .into_iter()
        .find_map(|key| metadata(key).and_then(|value| dashed_date(value.get(..10)?)))
}

/// `YYYY-MM-DD`.
fn dashed_date(text: &str) -> Option<String> {
    let mut parts = text.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    valid_date(number(year)?, number(month)?, number(day)?)
}

/// Eight digits: `YYYYMMDD` as Android and Pixel phones write it, else `MMDDYYYY`.
fn compact_date(digits: &str) -> Option<String> {
    if digits.len() != 8 {
        return None;
    }
    let part = |range: std::ops::Range<usize>| digits.get(range).and_then(number);
    valid_date(part(0..4)?, part(4..6)?, part(6..8)?)
        .or_else(|| valid_date(part(4..8)?, part(0..2)?, part(2..4)?))
}

fn number(text: &str) -> Option<u32> {
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

fn valid_date(year: u32, month: u32, day: u32) -> Option<String> {
    let plausible = (FIRST_PLAUSIBLE_YEAR..=LAST_PLAUSIBLE_YEAR).contains(&year)
        && (1..=12).contains(&month)
        && (1..=31).contains(&day);
    plausible.then(|| format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_dates_that_cameras_write_into_file_names() {
        assert_eq!(
            from_file_name("VID_20240612_181455.mp4").as_deref(),
            Some("2024-06-12")
        );
        assert_eq!(
            from_file_name("08152019_194329.m4v").as_deref(),
            Some("2019-08-15")
        );
        assert_eq!(
            from_file_name("IMG-20230101-WA0004.mp4").as_deref(),
            Some("2023-01-01")
        );
        assert_eq!(
            from_file_name("Screen Recording 2024-06-12 at 18.14.55.mov").as_deref(),
            Some("2024-06-12")
        );
    }

    #[test]
    fn finds_no_date_in_plain_camera_names() {
        assert_eq!(from_file_name("IMG_3344.MOV"), None);
        assert_eq!(from_file_name("DSCN1580.mov"), None);
        assert_eq!(from_file_name("12345678.mp4"), None);
    }

    #[test]
    fn prefers_the_iphone_local_date_in_metadata() {
        let stderr = "  Metadata:\n    creation_time   : 2025-03-14T22:30:00.000000Z\n    \
                      com.apple.quicktime.creationdate: 2025-03-15T00:30:00+0200\n";

        assert_eq!(from_ffmpeg_metadata(stderr).as_deref(), Some("2025-03-15"));
    }

    #[test]
    fn prefers_the_original_date_an_apple_export_keeps() {
        let stderr = "  Metadata:\n    major_brand     : M4V \n    \
                      creation_time   : 2026-06-16T09:44:34.000000Z\n    \
                      date            : 2017-07-09T11:13:01+0300\n";

        assert_eq!(from_ffmpeg_metadata(stderr).as_deref(), Some("2017-07-09"));
    }

    #[test]
    fn falls_back_to_creation_time_and_ignores_unset_clocks() {
        let camera = "    creation_time   : 2019-08-15T19:43:29.000000Z\n";
        let unset = "    creation_time   : 1970-01-01T00:00:00.000000Z\n";
        let year_only = "    date            : 2017\n";

        assert_eq!(from_ffmpeg_metadata(camera).as_deref(), Some("2019-08-15"));
        assert_eq!(from_ffmpeg_metadata(unset), None);
        assert_eq!(from_ffmpeg_metadata(year_only), None);
    }

    #[test]
    fn trusts_the_file_name_over_an_export_date_in_metadata() {
        let date = recorded_date("08132019_224238.m4v", Some("2026-06-16".into()));

        assert_eq!(date.as_deref(), Some("2019-08-13"));
        assert_eq!(
            recorded_date("IMG_3344.MOV", Some("2025-03-14".into())).as_deref(),
            Some("2025-03-14")
        );
    }
}
