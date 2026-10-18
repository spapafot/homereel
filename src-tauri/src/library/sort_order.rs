//! The orders a listing can be sorted in. Videos come from Drive and from folders on this
//! computer, so Homereel lists every match and sorts them itself (see `local_listings` for the
//! paging).

use std::cmp::Ordering;

use super::date_order;
use super::video_summary::VideoSummary;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoOrder {
    /// By the day the video was recorded ("date created" in the UI), newest first.
    #[default]
    CreatedNewest,
    CreatedOldest,
    /// By the day the file was uploaded to Drive (Drive's `createdTime`), or added to this
    /// computer.
    UploadedNewest,
    UploadedOldest,
    /// By file name, with numbers in numeric order as Drive's `name_natural` sorts them.
    Name,
}

pub fn sort(videos: &mut [VideoSummary], order: VideoOrder) {
    match order {
        VideoOrder::CreatedNewest => date_order::sort_by_date_created(videos, true),
        VideoOrder::CreatedOldest => date_order::sort_by_date_created(videos, false),
        VideoOrder::UploadedNewest => sort_by_uploaded(videos, true),
        VideoOrder::UploadedOldest => sort_by_uploaded(videos, false),
        VideoOrder::Name => {
            videos.sort_by(|a, b| natural_cmp(&a.file.file_name, &b.file.file_name));
        }
    }
}

/// Videos without an upload time go last in both directions.
fn sort_by_uploaded(videos: &mut [VideoSummary], newest_first: bool) {
    videos.sort_by(|a, b| {
        match (
            a.file.created_time.as_deref(),
            b.file.created_time.as_deref(),
        ) {
            (Some(a_time), Some(b_time)) => {
                // Drive and Homereel both write RFC 3339 UTC with milliseconds, which sorts as
                // text.
                let ordering = a_time
                    .cmp(b_time)
                    .then_with(|| natural_cmp(&a.file.file_name, &b.file.file_name));
                if newest_first {
                    ordering.reverse()
                } else {
                    ordering
                }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => natural_cmp(&a.file.file_name, &b.file.file_name),
        }
    });
}

/// Compares names ignoring case, with runs of digits compared as numbers: `IMG_2` before
/// `IMG_10`.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut left, mut right) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(l), Some(r)) if l.is_ascii_digit() && r.is_ascii_digit() => {
                let left_number = take_digits(&mut left);
                let right_number = take_digits(&mut right);
                let ordering = left_number
                    .len()
                    .cmp(&right_number.len())
                    .then_with(|| left_number.cmp(&right_number));
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(l), Some(r)) => {
                let ordering = l.to_lowercase().cmp(r.to_lowercase());
                if ordering != Ordering::Equal {
                    return ordering;
                }
                left.next();
                right.next();
            }
        }
    }
}

/// The next run of digits, without leading zeros.
fn take_digits(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut digits = String::new();
    while let Some(c) = chars.peek().copied().filter(char::is_ascii_digit) {
        if !(digits.is_empty() && c == '0') {
            digits.push(c);
        }
        chars.next();
    }
    digits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::video;

    fn summary(name: &str, uploaded: Option<&str>) -> VideoSummary {
        let mut file = video(name, name);
        file.created_time = uploaded.map(str::to_owned);
        VideoSummary::new(file, None, None)
    }

    fn names(videos: &[VideoSummary]) -> Vec<&str> {
        videos
            .iter()
            .map(|video| video.file.file_name.as_str())
            .collect()
    }

    #[test]
    fn compares_numbers_in_names_as_numbers() {
        let mut list = vec![
            "IMG_10.MOV",
            "img_2.mov",
            "IMG_0002.MOV",
            "Beach.mp4",
            "IMG_1.MOV",
        ];

        list.sort_by(|a, b| natural_cmp(a, b));

        assert_eq!(
            list,
            [
                "Beach.mp4",
                "IMG_1.MOV",
                "IMG_0002.MOV",
                "img_2.mov",
                "IMG_10.MOV"
            ]
        );
    }

    #[test]
    fn sorts_by_upload_time_with_unknown_times_last() {
        let mut videos = vec![
            summary("none.mp4", None),
            summary("old.mp4", Some("2020-01-01T00:00:00.000Z")),
            summary("new.mp4", Some("2026-01-01T00:00:00.000Z")),
        ];

        sort(&mut videos, VideoOrder::UploadedNewest);
        assert_eq!(names(&videos), ["new.mp4", "old.mp4", "none.mp4"]);

        sort(&mut videos, VideoOrder::UploadedOldest);
        assert_eq!(names(&videos), ["old.mp4", "new.mp4", "none.mp4"]);

        sort(&mut videos, VideoOrder::Name);
        assert_eq!(names(&videos), ["new.mp4", "none.mp4", "old.mp4"]);
    }
}
