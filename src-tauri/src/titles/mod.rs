//! AI display titles: frames sampled with ffmpeg through the local media proxy, named by
//! Gemini, stored locally. Drive files are never renamed.

pub mod gemini;
pub mod prompt;
pub mod queue;

use std::time::Duration;

use crate::db::ai_titles::{self, GeneratedTitle};
use crate::db::{people, snapshots};
use crate::error::AppError;
use crate::ffmpeg;
use crate::library::video_summary::VideoFile;
use crate::recorded_date::recorded_date;
use crate::state::AppState;
use gemini::{GeminiError, Image};
use prompt::PromptContext;

/// Fewer frames than this and the thumbnail is added so the model still sees the subject.
const MIN_FRAMES: usize = 3;
const MAX_ATTEMPTS: u32 = 5;
const BASE_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const MAX_JITTER_MS: u32 = 1_000;

#[derive(Debug, thiserror::Error)]
pub enum TitleError {
    #[error(transparent)]
    Gemini(#[from] GeminiError),
    #[error("No frames could be read from this video.")]
    NoFrames,
    #[error("The ffmpeg sidecar is missing. Reinstall Homereel or run scripts/fetch-ffmpeg.ps1.")]
    FfmpegMissing,
    #[error("Gemini returned an empty title.")]
    EmptyTitle,
    #[error(transparent)]
    App(#[from] AppError),
}

impl TitleError {
    pub fn stops_queue(&self) -> bool {
        match self {
            TitleError::Gemini(error) => error.stops_queue(),
            TitleError::FfmpegMissing => true,
            _ => false,
        }
    }
}

impl From<TitleError> for AppError {
    fn from(error: TitleError) -> Self {
        match error {
            TitleError::App(error) => error,
            TitleError::Gemini(GeminiError::RateLimited { .. }) => AppError::RateLimited,
            TitleError::Gemini(
                error @ (GeminiError::InvalidKey | GeminiError::ModelNotFound(_)),
            ) => AppError::Forbidden(error.to_string()),
            other => AppError::internal(other.to_string()),
        }
    }
}

/// Exponential backoff with jitter, preferring the delay the API asked for.
pub fn backoff_delay(attempt: u32, retry_after: Option<Duration>) -> Duration {
    let exponential = BASE_BACKOFF.saturating_mul(2u32.saturating_pow(attempt));
    let base = retry_after.unwrap_or(exponential).min(MAX_BACKOFF);
    let jitter = getrandom::u32().unwrap_or(0) % MAX_JITTER_MS;
    base + Duration::from_millis(u64::from(jitter))
}

/// Generates and stores a title for one video, naming the people and pets from Settings that
/// Gemini recognizes. The video must be known to the media proxy.
pub async fn generate_title(
    state: &AppState,
    video: &VideoFile,
    api_key: &str,
    model: &str,
) -> Result<String, TitleError> {
    let sample = sample_video(state, video).await?;
    let instructions = state.gemini_instructions()?;
    let reference_people = state
        .db
        .with(|connection| people::with_photos(connection))?;
    let names: Vec<String> = reference_people
        .iter()
        .map(|entry| entry.person.name.clone())
        .collect();
    let context = PromptContext {
        video,
        recorded_date: sample.recorded_date.as_deref(),
        instructions: instructions.as_deref(),
    };
    let parts = prompt::build_parts(&context, &reference_people, sample.frames);

    let mut attempt = 0;
    let suggestion = loop {
        match state
            .gemini
            .suggest_title(api_key, model, &parts, &names)
            .await
        {
            Ok(suggestion) => break suggestion,
            Err(error) if error.is_transient() && attempt + 1 < MAX_ATTEMPTS => {
                let retry_after = match &error {
                    GeminiError::RateLimited { retry_after } => *retry_after,
                    _ => None,
                };
                tokio::time::sleep(backoff_delay(attempt, retry_after)).await;
                attempt += 1;
            }
            Err(error) => return Err(error.into()),
        }
    };

    let title = prompt::sanitize_title(&suggestion.title).ok_or(TitleError::EmptyTitle)?;
    let recognized: Vec<i64> = reference_people
        .iter()
        .filter(|entry| suggestion.people.contains(&entry.person.name))
        .map(|entry| entry.person.id)
        .collect();
    let generated = GeneratedTitle {
        title: &title,
        confidence: suggestion.confidence,
        model,
        md5_checksum: video.md5_checksum.as_deref(),
        prompt_tokens: suggestion.prompt_tokens,
        output_tokens: suggestion.output_tokens,
    };
    state.db.with(|connection| {
        ai_titles::save_done(connection, &video.id, &generated)?;
        people::set_recognized(connection, &video.id, &recognized)?;
        snapshots::upsert(connection, std::slice::from_ref(video))
    })?;
    Ok(title)
}

/// What ffmpeg reads from a video for its title.
struct VideoSample {
    frames: Vec<Image>,
    recorded_date: Option<String>,
}

async fn sample_video(state: &AppState, video: &VideoFile) -> Result<VideoSample, TitleError> {
    let ffmpeg = ffmpeg::locate_ffmpeg().ok_or(TitleError::FfmpegMissing)?;
    let url = state.media_proxy.ffmpeg_input(&video.id).await?;
    // Both read the file, over HTTP for Drive; running them together hides the probe's round
    // trips.
    let (info, frames) = tokio::join!(
        ffmpeg::probe(&ffmpeg, &url),
        ffmpeg::extract_frames(&ffmpeg, &url, video.duration_ms),
    );
    let mut collected: Vec<Image> = frames
        .into_iter()
        .map(|bytes| Image {
            mime_type: "image/jpeg",
            bytes,
        })
        .collect();

    if collected.len() < MIN_FRAMES
        && video.has_thumbnail
        && let Ok(thumbnail) = state.thumbnails.load(&video.id).await
    {
        collected.push(Image {
            mime_type: thumbnail.content_type,
            bytes: thumbnail.bytes,
        });
    }
    if collected.is_empty() {
        return Err(TitleError::NoFrames);
    }
    Ok(VideoSample {
        frames: collected,
        recorded_date: recorded_date(&video.file_name, info.recorded_date),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_exponentially_and_is_capped() {
        let first = backoff_delay(0, None);
        let third = backoff_delay(2, None);
        let huge = backoff_delay(20, None);

        assert!(first >= Duration::from_secs(2) && first < Duration::from_secs(3));
        assert!(third >= Duration::from_secs(8) && third < Duration::from_secs(9));
        assert!(huge >= MAX_BACKOFF && huge < MAX_BACKOFF + Duration::from_secs(1));
    }

    #[test]
    fn backoff_prefers_the_delay_the_api_asked_for() {
        let delay = backoff_delay(0, Some(Duration::from_secs(13)));

        assert!(delay >= Duration::from_secs(13) && delay < Duration::from_secs(14));
    }

    /// Live check with the account signed in to the app: streams real Drive videos (including
    /// ones Drive has no metadata for) through the media proxy into ffmpeg. Run with
    /// `cargo test live_ -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "uses the signed-in Google account and the network"]
    async fn live_extracts_frames_from_drive_through_the_proxy() {
        use crate::auth::token_manager::TokenManager;
        use crate::auth::token_store::{KeyringSecretStore, init_platform_store};
        use crate::config::{self, DRIVE_API_BASE, KEYRING_SERVICE};
        use crate::drive::client::DriveClient;
        use crate::library::KnownFiles;
        use crate::protocol::media_proxy::MediaProxy;
        use std::sync::Arc;
        use std::time::Instant;

        init_platform_store().unwrap();
        let http = reqwest::Client::new();
        let tokens = TokenManager::new(
            http.clone(),
            config::google_client(),
            Arc::new(KeyringSecretStore::new(KEYRING_SERVICE)),
        );
        let media_http = reqwest::Client::builder().http1_only().build().unwrap();
        let drive = Arc::new(DriveClient::new(
            http,
            media_http,
            DRIVE_API_BASE,
            Arc::new(tokens),
        ));
        let mut files = drive.list_all_videos(&[]).await.unwrap();
        files.truncate(4);
        let known_files = KnownFiles::default();
        known_files.remember(&files);
        let proxy = MediaProxy::start(drive.clone(), known_files, Arc::default())
            .await
            .unwrap();
        let ffmpeg = ffmpeg::locate_ffmpeg().unwrap();

        for file in &files {
            let duration_ms = file
                .video_media_metadata
                .as_ref()
                .and_then(|metadata| metadata.duration_millis);
            let started = Instant::now();
            let frames =
                ffmpeg::extract_frames(&ffmpeg, &proxy.media_url(&file.id), duration_ms).await;
            eprintln!(
                "{} ({}, duration {:?}): {} frames in {:.1?}",
                file.name,
                file.mime_type,
                duration_ms,
                frames.len(),
                started.elapsed()
            );
            assert!(!frames.is_empty(), "no frames from {}", file.name);
        }
    }

    /// Live check that Gemini accepts labelled reference photos and the `people` enum. Sends
    /// only generated test patterns. Run with `cargo test live_ -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "uses GEMINI_API_KEY from .env and the network"]
    async fn live_titles_with_reference_people() {
        use crate::db::people::{Person, PersonKind, PersonWithPhotos};
        use crate::db::test_support::video;
        use gemini::GeminiClient;

        let api_key = crate::config::dev_gemini_key().expect("add GEMINI_API_KEY to .env");
        let ffmpeg = ffmpeg::locate_ffmpeg().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let clip = dir.path().join("clip.mp4");
        let status = std::process::Command::new(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi"])
            .args(["-i", "testsrc2=size=640x360:rate=10", "-t", "6"])
            .args(["-pix_fmt", "yuv420p"])
            .arg(&clip)
            .status()
            .unwrap();
        assert!(status.success());
        let input = clip.to_string_lossy();
        let frames: Vec<Image> = ffmpeg::extract_frames(&ffmpeg, &input, Some(6_000))
            .await
            .into_iter()
            .map(|bytes| Image {
                mime_type: "image/jpeg",
                bytes,
            })
            .collect();
        let reference = ffmpeg::extract_frame(&ffmpeg, &input, 1.0, 256)
            .await
            .unwrap();
        let people = [PersonWithPhotos {
            person: Person {
                id: 1,
                name: "Testy".into(),
                kind: PersonKind::Animal,
                description: Some("a colourful test pattern".into()),
                photo_ids: Vec::new(),
            },
            photos: vec![reference],
        }];
        let file = video("a", "VID_0001.mp4");
        let context = PromptContext {
            video: &file,
            recorded_date: None,
            instructions: None,
        };
        let parts = prompt::build_parts(&context, &people, frames);

        let suggestion = GeminiClient::new(reqwest::Client::new(), gemini::API_BASE)
            .suggest_title(
                &api_key,
                gemini::DEFAULT_MODEL,
                &parts,
                &["Testy".to_string()],
            )
            .await
            .unwrap();

        eprintln!("{suggestion:?}");
        assert!(!suggestion.title.trim().is_empty());
        assert!(suggestion.people.iter().all(|name| name == "Testy"));
    }

    #[test]
    fn only_account_wide_problems_stop_the_queue() {
        assert!(TitleError::Gemini(GeminiError::InvalidKey).stops_queue());
        assert!(TitleError::FfmpegMissing.stops_queue());
        assert!(TitleError::Gemini(GeminiError::NoCredits).stops_queue());
        assert!(!TitleError::NoFrames.stops_queue());
        assert!(!TitleError::Gemini(GeminiError::Unavailable("503".into())).stops_queue());
    }
}
