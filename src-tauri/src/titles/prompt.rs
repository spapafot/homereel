use super::gemini::{Image, PromptPart};
use crate::db::people::{Person, PersonWithPhotos};
use crate::library::video_summary::VideoFile;

const MAX_TITLE_CHARS: usize = 100;
/// Person photos are always stored as JPEG; see `commands::people`.
const PHOTO_MIME_TYPE: &str = "image/jpeg";

/// What the prompt says about a video besides its frames.
pub struct PromptContext<'a> {
    pub video: &'a VideoFile,
    /// From the file itself; Drive only knows the upload date. See [`crate::recorded_date`].
    pub recorded_date: Option<&'a str>,
    /// The owner's notes from Settings, such as who usually appears in which videos.
    pub instructions: Option<&'a str>,
}

/// The whole request in order: instructions, each reference person's photos under a label with
/// their name, then the frames.
pub fn build_parts(
    context: &PromptContext,
    people: &[PersonWithPhotos],
    frames: Vec<Image>,
) -> Vec<PromptPart> {
    let mut parts = vec![PromptPart::Text(build_prompt(
        context,
        frames.len(),
        people,
    ))];
    for entry in people {
        parts.push(PromptPart::Text(reference_label(
            &entry.person,
            entry.photos.len(),
        )));
        parts.extend(entry.photos.iter().map(|bytes| {
            PromptPart::Image(Image {
                mime_type: PHOTO_MIME_TYPE,
                bytes: bytes.clone(),
            })
        }));
    }
    if !people.is_empty() {
        parts.push(PromptPart::Text(format!(
            "The {} frames from the video, in chronological order:",
            frames.len()
        )));
    }
    parts.extend(frames.into_iter().map(PromptPart::Image));
    parts
}

fn reference_label(person: &Person, photo_count: usize) -> String {
    let noun = if photo_count == 1 { "photo" } else { "photos" };
    let kind = person.kind.as_str();
    match person.description.as_deref() {
        Some(description) => format!(
            "Reference {noun} of {} ({kind}, {description}):",
            person.name
        ),
        None => format!("Reference {noun} of {} ({kind}):", person.name),
    }
}

/// The instructions. Metadata helps with places and occasions; the frames carry the subject.
fn build_prompt(
    context: &PromptContext,
    frame_count: usize,
    people: &[PersonWithPhotos],
) -> String {
    let video = context.video;
    let mut details = vec![
        format!("File name: {}", video.file_name),
        format!("Folder: {}", video.folder_name),
    ];
    if let Some(duration_ms) = video.duration_ms {
        details.push(format!("Duration: {}", format_duration(duration_ms)));
    }
    if let (Some(width), Some(height)) = (video.width, video.height) {
        let orientation = if height > width {
            "portrait"
        } else {
            "landscape"
        };
        details.push(format!("Resolution: {width}x{height} ({orientation})"));
    }
    // Drive's dates are upload and copy dates, so a date is only given when the file has one.
    if let Some(date) = context.recorded_date {
        details.push(format!("Recorded: {date}"));
    }

    let people_instructions = if people.is_empty() {
        ""
    } else {
        "\n\nAfter these instructions come reference photos of the owner's own family, friends \
         and pets, each labelled by the owner with a name, and then the frames. If one of them \
         clearly appears in the frames, use their name in the title where it reads naturally, in \
         place of a generic word such as \"a girl\" or \"the dog\". Compare faces, build, hair, \
         fur and markings. Only use a name when you are confident it is the same individual; \
         when unsure, describe them generically. In people, list every name you recognised in \
         the frames, whether or not it is in the title, or leave it empty."
    };

    let owner_notes = match context.instructions {
        Some(notes) => format!(
            "\n\nNotes from the owner of these videos. Follow them, and let them override the \
             rules above where they conflict. If a note depends on when the video was recorded \
             and no recording date is listed below, go by the frames.\n{}",
            notes.trim()
        ),
        None => String::new(),
    };

    format!(
        "You name personal videos for a private video library. Write a short, specific, natural \
         title of 3 to 8 words that says what happens in this video, the way a person would \
         name it. Base it on what is visible in the {frame_count} frames, which are in \
         chronological order, and use the details below only as context. Do not include the \
         file name, file extension, camera codes, dates or the word \"video\". Write the title in \
         English with normal capitalisation and no quotes. Set confidence between 0 and 1 to \
         reflect how clearly the frames show what the video is about.{people_instructions}\
         {owner_notes}\n\n{}",
        details.join("\n")
    )
}

/// Cleans up a model title: trims, removes wrapping quotes and a trailing full stop, collapses
/// whitespace and caps the length. Returns `None` when nothing usable is left.
pub fn sanitize_title(raw: &str) -> Option<String> {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let unquoted = collapsed
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '“' | '”' | '‘' | '’' | '`'))
        .trim()
        .trim_end_matches('.')
        .trim();
    if unquoted.is_empty() {
        return None;
    }
    let title: String = unquoted.chars().take(MAX_TITLE_CHARS).collect();
    Some(title.trim_end().to_owned())
}

fn format_duration(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    let (hours, minutes, seconds) = (
        total_seconds / 3600,
        (total_seconds % 3600) / 60,
        total_seconds % 60,
    );
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::people::PersonKind;
    use crate::db::test_support::video;

    fn context(video: &VideoFile) -> PromptContext<'_> {
        PromptContext {
            video,
            recorded_date: None,
            instructions: None,
        }
    }

    #[test]
    fn prompt_includes_metadata_and_frame_count() {
        let mut file = video("a", "VID_20240612_181455.mp4");
        file.width = Some(1080);
        file.height = Some(1920);
        file.duration_ms = Some(754_000);
        let context = PromptContext {
            recorded_date: Some("2024-06-12"),
            ..context(&file)
        };

        let prompt = build_prompt(&context, 5, &[]);

        assert!(prompt.contains("File name: VID_20240612_181455.mp4"));
        assert!(prompt.contains("Folder: Holidays"));
        assert!(prompt.contains("Duration: 12:34"));
        assert!(prompt.contains("Resolution: 1080x1920 (portrait)"));
        assert!(prompt.contains("Recorded: 2024-06-12"));
        assert!(prompt.contains("5 frames"));
        assert!(!prompt.contains("reference photos"));
        assert!(!prompt.contains("Notes from the owner"));
    }

    #[test]
    fn leaves_out_drive_dates_that_would_mislead_date_rules() {
        let mut file = video("a", "IMG_0525.m4v");
        file.created_time = Some("2026-06-17T10:00:00Z".into());
        file.modified_time = Some("2026-06-16T09:44:34Z".into());

        let prompt = build_prompt(&context(&file), 5, &[]);

        assert!(!prompt.contains("2026"));
        assert!(!prompt.contains("Recorded:"));
    }

    #[test]
    fn adds_the_owners_notes_before_the_details() {
        let file = video("a", "IMG_3344.MOV");
        let context = PromptContext {
            recorded_date: Some("2025-03-14"),
            instructions: Some("  The baby after 24 August 2024 is almost always Anna.\n"),
            ..context(&file)
        };

        let prompt = build_prompt(&context, 5, &[]);
        let notes = prompt.find("The baby after 24 August 2024 is almost always Anna.\n\n");

        assert!(prompt.contains("let them override the rules above"));
        assert!(notes.is_some_and(|at| at < prompt.find("File name:").unwrap()));
    }

    fn person(
        name: &str,
        kind: PersonKind,
        description: Option<&str>,
        photos: usize,
    ) -> PersonWithPhotos {
        PersonWithPhotos {
            person: Person {
                id: 1,
                name: name.into(),
                kind,
                description: description.map(Into::into),
                photo_ids: Vec::new(),
            },
            photos: (0..photos).map(|index| vec![index as u8]).collect(),
        }
    }

    fn frame(byte: u8) -> Image {
        Image {
            mime_type: "image/jpeg",
            bytes: vec![byte],
        }
    }

    fn describe(parts: &[PromptPart]) -> Vec<String> {
        parts
            .iter()
            .map(|part| match part {
                PromptPart::Text(text) => text.clone(),
                PromptPart::Image(image) => format!("[image {:?}]", image.bytes),
            })
            .collect()
    }

    #[test]
    fn labels_each_persons_photos_before_the_frames() {
        let people = [
            person("Maria", PersonKind::Person, Some("my daughter"), 2),
            person("Rex", PersonKind::Animal, None, 1),
        ];

        let file = video("a", "IMG_1.mp4");
        let parts = describe(&build_parts(&context(&file), &people, vec![frame(9)]));

        assert!(parts[0].contains("reference photos of the owner's own family"));
        assert_eq!(
            parts[1..].to_vec(),
            vec![
                "Reference photos of Maria (person, my daughter):",
                "[image [0]]",
                "[image [1]]",
                "Reference photo of Rex (animal):",
                "[image [0]]",
                "The 1 frames from the video, in chronological order:",
                "[image [9]]",
            ]
        );
    }

    #[test]
    fn sends_only_the_prompt_and_frames_without_people() {
        let file = video("a", "IMG_1.mp4");
        let parts = describe(&build_parts(&context(&file), &[], vec![frame(9)]));

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], "[image [9]]");
    }

    #[test]
    fn sanitizes_titles() {
        assert_eq!(
            sanitize_title("  \"Kids  building a\n sandcastle.\" "),
            Some("Kids building a sandcastle".to_string())
        );
        assert_eq!(
            sanitize_title("“Sunset over Santorini”"),
            Some("Sunset over Santorini".to_string())
        );
        assert_eq!(sanitize_title("  ''  "), None);
        assert_eq!(
            sanitize_title(&"word ".repeat(40)).unwrap().chars().count(),
            99
        );
    }
}
