use std::sync::{Mutex, MutexGuard};

use futures_util::{StreamExt, stream};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use super::{TitleError, generate_title};
use crate::db::ai_titles::{self, TitleScope};
use crate::error::AppError;
use crate::library::video_summary::VideoFile;
use crate::state::AppState;

/// Two to three requests at a time stays well inside Gemini's rate limits.
const CONCURRENCY: usize = 3;

pub const PROGRESS_EVENT: &str = "titles://progress";
pub const ITEM_EVENT: &str = "titles://item";

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueStatus {
    pub running: bool,
    pub scanning: bool,
    pub stopping: bool,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub pause_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleItemEvent {
    pub drive_id: String,
    pub title: Option<String>,
    pub error: Option<String>,
}

/// Background generation of titles for every video that has none yet.
#[derive(Default)]
pub struct TitleQueue {
    status: Mutex<QueueStatus>,
    cancel: Mutex<Option<CancellationToken>>,
}

impl TitleQueue {
    pub fn status(&self) -> QueueStatus {
        self.lock_status().clone()
    }

    /// Starts a run over the videos in `scope` unless one is already going.
    pub fn start(&self, app: &AppHandle, scope: TitleScope) -> QueueStatus {
        {
            let mut status = self.lock_status();
            if status.running {
                return status.clone();
            }
            *status = QueueStatus {
                running: true,
                scanning: true,
                ..QueueStatus::default()
            };
        }
        let cancel = CancellationToken::new();
        *self.cancel.lock().unwrap_or_else(|p| p.into_inner()) = Some(cancel.clone());

        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            let pause_reason = run(&state, &app, scope, &cancel).await.err();
            let queue = &state.titles;
            {
                let mut status = queue.lock_status();
                status.running = false;
                status.scanning = false;
                status.stopping = false;
                status.pause_reason = pause_reason;
            }
            queue.emit_progress(&app);
        });
        self.status()
    }

    /// Stops after the requests in flight finish. Completed titles are kept; starting again
    /// continues with the videos that still have none.
    pub fn pause(&self, app: &AppHandle) -> QueueStatus {
        if let Some(cancel) = self
            .cancel
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
        {
            cancel.cancel();
        }
        {
            let mut status = self.lock_status();
            if status.running {
                status.stopping = true;
            }
        }
        self.emit_progress(app);
        self.status()
    }

    fn update(&self, app: &AppHandle, change: impl FnOnce(&mut QueueStatus)) {
        change(&mut self.lock_status());
        self.emit_progress(app);
    }

    fn emit_progress(&self, app: &AppHandle) {
        if let Err(error) = app.emit(PROGRESS_EVENT, self.status()) {
            log::warn!("Could not send title progress: {error}");
        }
    }

    fn lock_status(&self) -> MutexGuard<'_, QueueStatus> {
        self.status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Returns `Err(reason)` when the run had to stop early for a reason the user should see.
async fn run(
    state: &AppState,
    app: &AppHandle,
    scope: TitleScope,
    cancel: &CancellationToken,
) -> Result<(), String> {
    let api_key = state
        .gemini_key()
        .map_err(|error| error.to_string())?
        .ok_or("Add a Gemini API key to generate titles.")?;
    let model = state.gemini_model().map_err(|error| error.to_string())?;

    let pending = find_videos_needing_titles(state, scope, cancel)
        .await
        .map_err(|error| error.to_string())?;
    state.titles.update(app, |status| {
        status.scanning = false;
        status.total = pending.len();
    });

    let stop_reason: Mutex<Option<String>> = Mutex::new(None);
    stream::iter(pending)
        .for_each_concurrent(CONCURRENCY, |video| {
            let (api_key, model, stop_reason) = (&api_key, &model, &stop_reason);
            async move {
                if cancel.is_cancelled() {
                    return;
                }
                let result = generate_title(state, &video, api_key, model).await;
                let event = match result {
                    Ok(title) => {
                        state.titles.update(app, |status| status.completed += 1);
                        TitleItemEvent {
                            drive_id: video.id.clone(),
                            title: Some(title),
                            error: None,
                        }
                    }
                    Err(error) => record_failure(state, app, &video, &error, cancel, stop_reason),
                };
                if let Err(error) = app.emit(ITEM_EVENT, event) {
                    log::warn!("Could not send a title update: {error}");
                }
            }
        })
        .await;

    match stop_reason.into_inner().unwrap_or_else(|p| p.into_inner()) {
        Some(reason) => Err(reason),
        None => Ok(()),
    }
}

fn record_failure(
    state: &AppState,
    app: &AppHandle,
    video: &VideoFile,
    error: &TitleError,
    cancel: &CancellationToken,
    stop_reason: &Mutex<Option<String>>,
) -> TitleItemEvent {
    let message = error.to_string();
    if error.stops_queue() {
        // Every other request would fail the same way, so stop instead of failing each video.
        cancel.cancel();
        stop_reason
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_or_insert_with(|| message.clone());
    } else {
        if let Err(db_error) = state
            .db
            .with(|connection| ai_titles::save_failed(connection, &video.id, &message))
        {
            log::warn!("Could not record a failed title: {db_error}");
        }
        state.titles.update(app, |status| status.failed += 1);
    }
    TitleItemEvent {
        drive_id: video.id.clone(),
        title: None,
        error: Some(message),
    }
}

/// Lists every video in the library and keeps those that need a title.
async fn find_videos_needing_titles(
    state: &AppState,
    scope: TitleScope,
    cancel: &CancellationToken,
) -> Result<Vec<VideoFile>, AppError> {
    let listed = state.library_videos(None).await?;
    if cancel.is_cancelled() {
        return Ok(Vec::new());
    }
    let videos: Vec<VideoFile> = state
        .with_local_state(listed)?
        .into_iter()
        .filter(|summary| scope != TitleScope::All || summary.custom_title.is_none())
        .map(|summary| summary.file)
        .collect();

    let ids: Vec<String> = videos.iter().map(|video| video.id.clone()).collect();
    let records = state
        .db
        .with(|connection| ai_titles::records(connection, &ids))?;
    Ok(videos
        .into_iter()
        .filter(|video| {
            ai_titles::needs_title(records.get(&video.id), video.md5_checksum.as_deref(), scope)
        })
        .collect())
}
