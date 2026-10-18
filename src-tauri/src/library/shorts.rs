//! Which videos Homereel shows as Shorts. It follows YouTube's rule: a vertical or square video
//! up to three minutes long.

use super::video_summary::VideoFile;

pub const MAX_SHORT_DURATION_MS: u64 = 3 * 60 * 1000;

/// Videos Drive has not measured yet (no size or duration) are never Shorts.
pub fn is_short(video: &VideoFile) -> bool {
    match (video.width, video.height, video.duration_ms) {
        (Some(width), Some(height), Some(duration_ms)) => {
            width > 0 && height >= width && duration_ms > 0 && duration_ms <= MAX_SHORT_DURATION_MS
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::video;

    fn shaped(width: Option<u32>, height: Option<u32>, duration_ms: Option<u64>) -> VideoFile {
        let mut file = video("v", "IMG_0001.MOV");
        file.width = width;
        file.height = height;
        file.duration_ms = duration_ms;
        file
    }

    #[test]
    fn vertical_and_square_videos_up_to_three_minutes_are_shorts() {
        assert!(is_short(&shaped(Some(1080), Some(1920), Some(15_000))));
        assert!(is_short(&shaped(Some(1080), Some(1080), Some(60_000))));
        assert!(is_short(&shaped(
            Some(1080),
            Some(1920),
            Some(MAX_SHORT_DURATION_MS)
        )));
    }

    #[test]
    fn landscape_long_and_unmeasured_videos_are_not_shorts() {
        assert!(!is_short(&shaped(Some(1920), Some(1080), Some(15_000))));
        assert!(!is_short(&shaped(
            Some(1080),
            Some(1920),
            Some(MAX_SHORT_DURATION_MS + 1)
        )));
        assert!(!is_short(&shaped(None, None, Some(15_000))));
        assert!(!is_short(&shaped(Some(1080), Some(1920), None)));
        assert!(!is_short(&shaped(Some(1080), Some(1920), Some(0))));
    }
}
