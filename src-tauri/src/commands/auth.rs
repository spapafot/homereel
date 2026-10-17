use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    configured: bool,
    connected: bool,
    email: Option<String>,
    display_name: Option<String>,
}

impl AuthStatus {
    fn disconnected(configured: bool) -> Self {
        Self {
            configured,
            connected: false,
            email: None,
            display_name: None,
        }
    }
}

#[tauri::command]
pub async fn auth_status(state: State<'_, AppState>) -> AppResult<AuthStatus> {
    current_status(&state).await
}

#[tauri::command]
pub async fn auth_connect(app: AppHandle, state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state
        .tokens
        .connect(|url| {
            app.opener()
                .open_url(url, None::<&str>)
                .map_err(|error| AppError::internal(format!("Could not open the browser: {error}")))
        })
        .await?;
    state.forget_account().await;
    state.date_scan.start(&app);
    current_status(&state).await
}

#[tauri::command]
pub fn auth_cancel(state: State<'_, AppState>) {
    state.tokens.cancel_connect();
}

#[tauri::command]
pub async fn auth_disconnect(state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.tokens.disconnect().await?;
    state.forget_account().await;
    Ok(AuthStatus::disconnected(state.tokens.is_configured()))
}

async fn current_status(state: &AppState) -> AppResult<AuthStatus> {
    let configured = state.tokens.is_configured();
    if !configured || !state.tokens.is_connected()? {
        return Ok(AuthStatus::disconnected(configured));
    }
    match state.account().await {
        Ok(user) => Ok(AuthStatus {
            configured,
            connected: true,
            email: user.email_address,
            display_name: user.display_name,
        }),
        Err(AppError::AuthRevoked | AppError::NotConnected) => {
            Ok(AuthStatus::disconnected(configured))
        }
        // Offline launches still count as connected; the account details load later.
        Err(AppError::Offline(_)) => Ok(AuthStatus {
            configured,
            connected: true,
            email: None,
            display_name: None,
        }),
        Err(error) => Err(error),
    }
}
