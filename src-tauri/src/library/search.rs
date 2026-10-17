//! Search over titles and file names. Drive only matches the start of words in file names and
//! knows nothing of edited or AI titles, so DriveTube lists every video and matches here.

use super::video_summary::{TitleSource, VideoSummary};

/// The videos in which every word of `query` appears, ignoring case, in the title shown or the
/// file name. Videos whose edited or AI title holds every word come first, then those that also
/// need the file name; each group keeps the order it was given.
pub fn matching(videos: Vec<VideoSummary>, query: &str) -> Vec<VideoSummary> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let contains_every_word = |text: &str| {
        let text = text.to_lowercase();
        words.iter().all(|word| text.contains(word.as_str()))
    };

    let mut title_matches = Vec::new();
    let mut name_matches = Vec::new();
    for video in videos {
        let has_title = video.title_source != TitleSource::FileName;
        if has_title && contains_every_word(&video.title) {
            title_matches.push(video);
        } else if contains_every_word(&format!("{} {}", video.title, video.file.file_name)) {
            name_matches.push(video);
        }
    }
    title_matches.extend(name_matches);
    title_matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::video;
    use crate::db::user_state::LocalState;

    fn titled(id: &str, file_name: &str, custom: Option<&str>, ai: Option<&str>) -> VideoSummary {
        let state = LocalState {
            custom_title: custom.map(str::to_owned),
            ai_title: ai.map(str::to_owned),
            ai_status: ai.map(|_| "done".to_owned()),
            ..LocalState::default()
        };
        VideoSummary::new(video(id, file_name), Some(&state), None)
    }

    fn ids(videos: &[VideoSummary]) -> Vec<&str> {
        videos.iter().map(|video| video.file.id.as_str()).collect()
    }

    #[test]
    fn returns_every_title_match_before_file_name_matches() {
        let mut videos: Vec<VideoSummary> = (0..30)
            .map(|n| {
                titled(
                    &format!("ai{n}"),
                    "IMG_1.MOV",
                    None,
                    Some("Ninja in the yard"),
                )
            })
            .collect();
        videos.insert(0, titled("name", "ninja_cam.mp4", None, None));
        videos.push(titled("other", "IMG_2.MOV", None, Some("Beach day")));

        let found = matching(videos, "NINJA");

        assert_eq!(found.len(), 31);
        assert_eq!(ids(&found)[0], "ai0");
        assert_eq!(ids(&found)[30], "name");
    }

    #[test]
    fn needs_every_word_in_any_title_or_the_file_name() {
        let videos = vec![
            titled(
                "custom",
                "a.mp4",
                Some("Stratos playing"),
                Some("Ignored AI words"),
            ),
            titled("split", "beach_2019.mp4", None, Some("Kids on the sand")),
            titled("partial", "b.mp4", None, Some("Kids at home")),
        ];

        assert_eq!(ids(&matching(videos.clone(), "stratos")), ["custom"]);
        // A replaced AI title is not shown, so it does not match either.
        assert!(matching(videos.clone(), "ignored").is_empty());
        assert_eq!(ids(&matching(videos.clone(), "kids beach")), ["split"]);
        assert!(matching(videos, "   ").is_empty());
    }
}
