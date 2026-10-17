use tauri::State;

use crate::db::user_state;
use crate::drive::is_valid_drive_id;
use crate::error::{AppError, AppResult};
use crate::library::video_summary::VideoSummary;
use crate::state::AppState;

const MAX_TITLE_CHARS: usize = 200;

fn ensure_valid_id(drive_id: &str) -> AppResult<()> {
    if is_valid_drive_id(drive_id) {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

#[tauri::command]
pub async fn video_set_favorite(
    state: State<'_, AppState>,
    drive_id: String,
    value: bool,
) -> AppResult<VideoSummary> {
    ensure_valid_id(&drive_id)?;
    state.ensure_snapshot(&drive_id).await?;
    state
        .db
        .with(|connection| user_state::set_favorite(connection, &drive_id, value))?;
    state.video_summary(&drive_id).await
}

#[tauri::command]
pub async fn video_set_watch_later(
    state: State<'_, AppState>,
    drive_id: String,
    value: bool,
) -> AppResult<VideoSummary> {
    ensure_valid_id(&drive_id)?;
    state.ensure_snapshot(&drive_id).await?;
    state
        .db
        .with(|connection| user_state::set_watch_later(connection, &drive_id, value))?;
    state.video_summary(&drive_id).await
}

/// Sets the user's display title. `None` or a blank title goes back to the AI title or filename.
/// The Drive file keeps its name.
#[tauri::command]
pub async fn video_set_custom_title(
    state: State<'_, AppState>,
    drive_id: String,
    title: Option<String>,
) -> AppResult<VideoSummary> {
    ensure_valid_id(&drive_id)?;
    let title = title
        .as_deref()
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|text| !text.is_empty())
        .map(|text| text.chars().take(MAX_TITLE_CHARS).collect::<String>());
    state.ensure_snapshot(&drive_id).await?;
    state
        .db
        .with(|connection| user_state::set_custom_title(connection, &drive_id, title.as_deref()))?;
    state.video_summary(&drive_id).await
}

/// Saves the resume point; called every few seconds while a video plays.
#[tauri::command]
pub async fn video_record_progress(
    state: State<'_, AppState>,
    drive_id: String,
    position_ms: u64,
    duration_ms: u64,
) -> AppResult<()> {
    ensure_valid_id(&drive_id)?;
    state.ensure_snapshot(&drive_id).await?;
    state.db.with(|connection| {
        user_state::record_progress(
            connection,
            &drive_id,
            position_ms.min(duration_ms),
            duration_ms,
        )
    })
}

#[tauri::command]
pub fn history_remove(state: State<'_, AppState>, drive_id: String) -> AppResult<()> {
    state
        .db
        .with(|connection| user_state::remove_from_history(connection, &drive_id))
}

#[tauri::command]
pub fn history_clear(state: State<'_, AppState>) -> AppResult<()> {
    state
        .db
        .with(|connection| user_state::clear_history(connection))
}
