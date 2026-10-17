//! The "date created" orders. Drive cannot sort by the day a video was recorded, so DriveTube
//! lists every matching video and sorts them here (see `local_listings` for the paging).

use std::cmp::Ordering;

use super::video_summary::VideoSummary;

/// Sorts by date created, then by file name in the same direction so clips from one day follow
/// the camera's numbering. Videos without any date go last.
pub fn sort_by_date_created(videos: &mut [VideoSummary], newest_first: bool) {
    videos.sort_by(|a, b| {
        let ordering = match (a.date_created(), b.date_created()) {
            (Some(a_date), Some(b_date)) => a_date
                .cmp(b_date)
                .then_with(|| a.file.file_name.cmp(&b.file.file_name)),
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (None, None) => a.file.file_name.cmp(&b.file.file_name),
        };
        if newest_first {
            ordering.reverse()
        } else {
            ordering
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::video;

    fn summary(name: &str, recorded: Option<&str>, modified: Option<&str>) -> VideoSummary {
        let mut file = video(name, name);
        file.modified_time = modified.map(str::to_owned);
        VideoSummary::new(file, None, recorded.map(str::to_owned))
    }

    fn names(videos: &[VideoSummary]) -> Vec<&str> {
        videos
            .iter()
            .map(|video| video.file.file_name.as_str())
            .collect()
    }

    #[test]
    fn sorts_by_recording_date_with_drive_modified_as_fallback_and_undated_last() {
        let mut videos = vec![
            summary("undated.mov", None, None),
            summary(
                "IMG_0002.MOV",
                Some("2022-05-15"),
                Some("2024-09-03T10:00:00Z"),
            ),
            summary("IMG_0001.MOV", Some("2022-05-15"), None),
            summary("export.m4v", None, Some("2026-06-16T10:00:00Z")),
            summary("old.mov", Some("2017-07-09"), Some("2026-06-16T10:00:00Z")),
        ];

        sort_by_date_created(&mut videos, true);
        assert_eq!(
            names(&videos),
            [
                "export.m4v",
                "IMG_0002.MOV",
                "IMG_0001.MOV",
                "old.mov",
                "undated.mov"
            ]
        );

        sort_by_date_created(&mut videos, false);
        assert_eq!(
            names(&videos),
            [
                "old.mov",
                "IMG_0001.MOV",
                "IMG_0002.MOV",
                "export.m4v",
                "undated.mov"
            ]
        );
    }
}
