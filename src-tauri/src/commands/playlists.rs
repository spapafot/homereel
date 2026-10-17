use serde::Serialize;
use tauri::State;

use crate::db::playlists::{self, PlaylistMembership, PlaylistSummary};
use crate::drive::is_valid_drive_id;
use crate::error::{AppError, AppResult};
use crate::library::video_summary::VideoSummary;
use crate::state::AppState;

const MAX_NAME_CHARS: usize = 150;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    playlist: PlaylistSummary,
    videos: Vec<VideoSummary>,
}

fn clean_name(name: &str) -> AppResult<String> {
    let name: String = name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_NAME_CHARS)
        .collect();
    if name.is_empty() {
        Err(AppError::internal("Give the playlist a name."))
    } else {
        Ok(name)
    }
}

#[tauri::command]
pub fn playlists_list(state: State<'_, AppState>) -> AppResult<Vec<PlaylistSummary>> {
    state.db.with(|connection| playlists::list(connection))
}

#[tauri::command]
pub fn playlist_get(state: State<'_, AppState>, playlist_id: i64) -> AppResult<PlaylistDetail> {
    let (playlist, ids) = state.db.with(|connection| {
        Ok((
            playlists::get(connection, playlist_id)?,
            playlists::item_ids(connection, playlist_id)?,
        ))
    })?;
    Ok(PlaylistDetail {
        playlist: playlist.ok_or(AppError::NotFound)?,
        videos: state.summaries_for_ids(&ids)?,
    })
}

/// Creates a playlist, optionally starting it with one video.
#[tauri::command]
pub async fn playlist_create(
    state: State<'_, AppState>,
    name: String,
    drive_id: Option<String>,
) -> AppResult<PlaylistSummary> {
    let name = clean_name(&name)?;
    if let Some(drive_id) = &drive_id {
        state.ensure_snapshot(drive_id).await?;
    }
    state.db.with(|connection| {
        let id = playlists::create(connection, &name)?;
        if let Some(drive_id) = &drive_id {
            playlists::add_video(connection, id, drive_id)?;
        }
        playlists::get(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
    })
}

#[tauri::command]
pub fn playlist_rename(
    state: State<'_, AppState>,
    playlist_id: i64,
    name: String,
) -> AppResult<()> {
    let name = clean_name(&name)?;
    if state
        .db
        .with(|connection| playlists::rename(connection, playlist_id, &name))?
    {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

/// Deletes the local playlist only; the videos stay in Google Drive.
#[tauri::command]
pub fn playlist_delete(state: State<'_, AppState>, playlist_id: i64) -> AppResult<()> {
    state
        .db
        .with(|connection| playlists::delete(connection, playlist_id))
}

#[tauri::command]
pub async fn playlist_add_video(
    state: State<'_, AppState>,
    playlist_id: i64,
    drive_id: String,
) -> AppResult<()> {
    if !is_valid_drive_id(&drive_id) {
        return Err(AppError::NotFound);
    }
    state.ensure_snapshot(&drive_id).await?;
    state
        .db
        .with(|connection| playlists::add_video(connection, playlist_id, &drive_id))
}

#[tauri::command]
pub fn playlist_remove_video(
    state: State<'_, AppState>,
    playlist_id: i64,
    drive_id: String,
) -> AppResult<()> {
    state
        .db
        .with(|connection| playlists::remove_video(connection, playlist_id, &drive_id))
}

#[tauri::command]
pub fn playlist_reorder(
    state: State<'_, AppState>,
    playlist_id: i64,
    drive_ids: Vec<String>,
) -> AppResult<()> {
    state
        .db
        .with(|connection| playlists::reorder(connection, playlist_id, &drive_ids))
}

#[tauri::command]
pub fn playlists_for_video(
    state: State<'_, AppState>,
    drive_id: String,
) -> AppResult<Vec<PlaylistMembership>> {
    state
        .db
        .with(|connection| playlists::memberships(connection, &drive_id))
}
