use serde::Serialize;
use tauri::{AppHandle, State};

use crate::config::GEMINI_KEY_NAME;
use crate::db::ai_titles::{self, TitleCounts, TitleScope};
use crate::db::settings;
use crate::error::{AppError, AppResult};
use crate::library::video_summary::VideoSummary;
use crate::state::AppState;
use crate::titles::gemini::GeminiError;
use crate::titles::generate_title;
use crate::titles::queue::QueueStatus;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSettings {
    key_configured: bool,
    model: String,
    /// The owner's notes for every title request; empty when there are none.
    instructions: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitlesOverview {
    queue: QueueStatus,
    counts: TitleCounts,
}

fn gemini_settings(state: &AppState) -> AppResult<GeminiSettings> {
    Ok(GeminiSettings {
        key_configured: state.gemini_key()?.is_some(),
        model: state.gemini_model()?,
        instructions: state.gemini_instructions()?.unwrap_or_default(),
    })
}

/// Every title request carries the notes, so a long text costs tokens on each video.
const MAX_INSTRUCTIONS_CHARS: usize = 2_000;

fn key_error(error: GeminiError) -> AppError {
    match error {
        GeminiError::InvalidKey => {
            AppError::Forbidden("That Gemini API key was not accepted.".into())
        }
        GeminiError::RateLimited { .. } => AppError::RateLimited,
        GeminiError::Unavailable(detail) => AppError::Offline(detail),
        other => AppError::internal(other.to_string()),
    }
}

/// Whether a key is stored and which model titles use. The key itself never leaves Rust.
#[tauri::command]
pub fn gemini_settings_get(state: State<'_, AppState>) -> AppResult<GeminiSettings> {
    gemini_settings(&state)
}

/// Validates the key with Gemini, then stores it in Windows Credential Manager.
#[tauri::command]
pub async fn gemini_key_set(
    state: State<'_, AppState>,
    api_key: String,
) -> AppResult<GeminiSettings> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(AppError::Forbidden("Paste a Gemini API key.".into()));
    }
    state
        .gemini
        .list_title_models(api_key)
        .await
        .map_err(key_error)?;
    state.secrets.set(GEMINI_KEY_NAME, api_key)?;
    gemini_settings(&state)
}

#[tauri::command]
pub fn gemini_key_remove(state: State<'_, AppState>) -> AppResult<GeminiSettings> {
    state.secrets.delete(GEMINI_KEY_NAME)?;
    gemini_settings(&state)
}

#[tauri::command]
pub async fn gemini_models_list(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    let api_key = state.require_gemini_key()?;
    state
        .gemini
        .list_title_models(&api_key)
        .await
        .map_err(key_error)
}

#[tauri::command]
pub fn gemini_model_set(state: State<'_, AppState>, model: String) -> AppResult<GeminiSettings> {
    let model = model.trim();
    if model.is_empty() {
        return Err(AppError::internal("Choose a model."));
    }
    state
        .db
        .with(|connection| settings::set(connection, settings::GEMINI_MODEL, model))?;
    gemini_settings(&state)
}

/// Saves the owner's notes for Gemini, such as who usually appears in which videos. Empty
/// text removes them.
#[tauri::command]
pub fn gemini_instructions_set(
    state: State<'_, AppState>,
    instructions: String,
) -> AppResult<GeminiSettings> {
    let instructions = instructions.trim();
    if instructions.chars().count() > MAX_INSTRUCTIONS_CHARS {
        return Err(AppError::internal(format!(
            "Keep the instructions under {MAX_INSTRUCTIONS_CHARS} characters."
        )));
    }
    state.db.with(|connection| {
        settings::set(connection, settings::GEMINI_INSTRUCTIONS, instructions)
    })?;
    gemini_settings(&state)
}

#[tauri::command]
pub fn titles_status(state: State<'_, AppState>) -> AppResult<TitlesOverview> {
    Ok(TitlesOverview {
        queue: state.titles.status(),
        counts: state.db.with(|connection| ai_titles::counts(connection))?,
    })
}

#[tauri::command]
pub fn titles_start(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: TitleScope,
) -> AppResult<QueueStatus> {
    state.require_gemini_key()?;
    Ok(state.titles.start(&app, scope))
}

#[tauri::command]
pub fn titles_pause(app: AppHandle, state: State<'_, AppState>) -> QueueStatus {
    state.titles.pause(&app)
}

/// Generates a new AI title for one video right away, replacing the previous one.
#[tauri::command]
pub async fn title_regenerate(
    state: State<'_, AppState>,
    drive_id: String,
) -> AppResult<VideoSummary> {
    let api_key = state.require_gemini_key()?;
    let model = state.gemini_model()?;
    let video = state.video_file(&drive_id).await?;
    generate_title(&state, &video, &api_key, &model).await?;
    state.video_summary(&drive_id).await
}
