use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use tauri::State;

use crate::db::people::{self, Person, PersonChanges, PersonKind, SaveResult};
use crate::error::{AppError, AppResult};
use crate::ffmpeg;
use crate::state::AppState;
use crate::titles::TitleError;

/// More photos help across ages and angles, but every photo is sent with every title request.
const MAX_PHOTOS: usize = 3;
const MAX_NAME_CHARS: usize = 60;
const MAX_DESCRIPTION_CHARS: usize = 120;
/// The app scales crops down to 512 px, far below this; it only stops runaway payloads.
const MAX_PHOTO_BYTES: usize = 1_000_000;
/// Wide enough to crop a face out of a group shot.
const CAPTURE_WIDTH: u32 = 1280;

/// Everything the person dialog edits, saved at once.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonInput {
    id: Option<i64>,
    name: String,
    kind: PersonKind,
    description: Option<String>,
    keep_photo_ids: Vec<i64>,
    /// Base64 JPEGs cropped in the app.
    new_photos: Vec<String>,
}

fn clean_text(text: &str, max_chars: usize) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn decode_photo(encoded: &str) -> AppResult<Vec<u8>> {
    let bytes = STANDARD
        .decode(encoded.trim())
        .map_err(|_| AppError::internal("A photo could not be read."))?;
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(AppError::internal("Photos must be JPEG images."));
    }
    if bytes.len() > MAX_PHOTO_BYTES {
        return Err(AppError::internal("A photo is too large."));
    }
    Ok(bytes)
}

#[tauri::command]
pub fn people_list(state: State<'_, AppState>) -> AppResult<Vec<Person>> {
    state.db.with(|connection| people::list(connection))
}

/// Creates or updates a person or pet. Titles already generated are not changed.
#[tauri::command]
pub fn person_save(state: State<'_, AppState>, input: PersonInput) -> AppResult<Person> {
    let name = clean_text(&input.name, MAX_NAME_CHARS);
    if name.is_empty() {
        return Err(AppError::internal("Give them a name."));
    }
    let description = input
        .description
        .as_deref()
        .map(|text| clean_text(text, MAX_DESCRIPTION_CHARS))
        .filter(|text| !text.is_empty());
    let photo_count = input.keep_photo_ids.len() + input.new_photos.len();
    if photo_count == 0 {
        return Err(AppError::internal(
            "Add a photo so Gemini can recognize them.",
        ));
    }
    if photo_count > MAX_PHOTOS {
        return Err(AppError::internal(format!(
            "Use up to {MAX_PHOTOS} photos."
        )));
    }
    let new_photos = input
        .new_photos
        .iter()
        .map(|photo| decode_photo(photo))
        .collect::<AppResult<Vec<_>>>()?;

    let changes = PersonChanges {
        name: &name,
        kind: input.kind,
        description: description.as_deref(),
        keep_photo_ids: &input.keep_photo_ids,
        new_photos: &new_photos,
    };
    match state
        .db
        .with(|connection| people::save(connection, input.id, &changes))?
    {
        SaveResult::Saved(person) => Ok(person),
        SaveResult::NameTaken => Err(AppError::internal(format!(
            "You already have someone called {name}."
        ))),
        SaveResult::NotFound => Err(AppError::NotFound),
    }
}

#[tauri::command]
pub fn person_delete(state: State<'_, AppState>, person_id: i64) -> AppResult<()> {
    state
        .db
        .with(|connection| people::delete(connection, person_id))?;
    Ok(())
}

/// Who Gemini recognized the last time it titled this video.
#[tauri::command]
pub fn video_people(state: State<'_, AppState>, drive_id: String) -> AppResult<Vec<String>> {
    state
        .db
        .with(|connection| people::recognized_names(connection, &drive_id))
}

/// One frame as a JPEG data URL, to crop a person or pet out of. The `<video>` element cannot
/// be drawn to a canvas because the media proxy is another origin, so ffmpeg reads the frame.
#[tauri::command]
pub async fn video_frame_capture(
    state: State<'_, AppState>,
    drive_id: String,
    position_ms: u64,
) -> AppResult<String> {
    let ffmpeg = ffmpeg::locate_ffmpeg()
        .ok_or_else(|| AppError::internal(TitleError::FfmpegMissing.to_string()))?;
    let video = state.video_file(&drive_id).await?;
    let url = state.media_proxy.ffmpeg_input(&video.id).await?;
    let duration_ms = match video.duration_ms {
        Some(duration_ms) => Some(duration_ms),
        None => ffmpeg::probe_duration(&ffmpeg, &url)
            .await
            .map(|seconds| (seconds * 1000.0) as u64),
    };
    let at_ms = ffmpeg::capture_position(position_ms, duration_ms);
    let bytes = ffmpeg::extract_frame(&ffmpeg, &url, at_ms as f64 / 1000.0, CAPTURE_WIDTH)
        .await
        .map_err(AppError::internal)?;
    Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_jpeg_photos() {
        let jpeg = STANDARD.encode([0xFF, 0xD8, 0xFF, 0xE0]);
        let png = STANDARD.encode(b"\x89PNG\r\n");

        assert_eq!(decode_photo(&jpeg).unwrap(), vec![0xFF, 0xD8, 0xFF, 0xE0]);
        assert!(decode_photo(&png).is_err());
        assert!(decode_photo("not base64!").is_err());
    }

    #[test]
    fn rejects_oversized_photos() {
        let mut bytes = vec![0xFF, 0xD8];
        bytes.resize(MAX_PHOTO_BYTES + 1, 0);

        assert!(decode_photo(&STANDARD.encode(bytes)).is_err());
    }

    #[test]
    fn cleans_names_and_descriptions() {
        assert_eq!(
            clean_text("  Maria \n Papadopoulou ", 60),
            "Maria Papadopoulou"
        );
        assert_eq!(clean_text("abcdef", 3), "abc");
        assert_eq!(clean_text("   ", 60), "");
    }
}
