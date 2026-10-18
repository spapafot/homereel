use tauri::State;

use crate::error::{AppError, AppResult};
use crate::ffmpeg;
use crate::state::AppState;
use crate::titles::TitleError;

/// Replaces a video's thumbnail with a sharp frame from the video: the frame at `position_ms`,
/// or else the most detailed of several. Takes a few seconds per frame over Drive.
#[tauri::command]
pub async fn thumbnail_regenerate(
    state: State<'_, AppState>,
    drive_id: String,
    position_ms: Option<u64>,
) -> AppResult<()> {
    let ffmpeg = ffmpeg::locate_ffmpeg()
        .ok_or_else(|| AppError::internal(TitleError::FfmpegMissing.to_string()))?;
    let video = state.video_file(&drive_id).await?;
    state
        .thumbnails
        .regenerate(&ffmpeg, &video.id, video.duration_ms, position_ms)
        .await
}
