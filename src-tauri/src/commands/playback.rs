use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::drive::is_valid_drive_id;
use crate::error::{AppError, AppResult};
use crate::library::local_files::is_local_id;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSource {
    url: String,
}

/// A localhost media-proxy URL the `<video>` element can play and seek. The proxy finds local
/// videos in the chosen folders itself.
#[tauri::command]
pub fn playback_source(state: State<'_, AppState>, drive_id: String) -> AppResult<PlaybackSource> {
    let is_playable = (is_local_id(&drive_id) && is_valid_drive_id(&drive_id))
        || state.known_files.contains(&drive_id);
    if !is_playable {
        return Err(AppError::NotFound);
    }
    Ok(PlaybackSource {
        url: state.media_proxy.media_url(&drive_id),
    })
}

/// Plays the video outside Homereel: a Drive file on its Drive page in the system browser,
/// where the user's own Google session handles playback, and a local file in the default player.
#[tauri::command]
pub async fn playback_open_external(
    app: AppHandle,
    state: State<'_, AppState>,
    drive_id: String,
) -> AppResult<()> {
    if !is_valid_drive_id(&drive_id) {
        return Err(AppError::NotFound);
    }
    if is_local_id(&drive_id) {
        let video = state
            .local
            .find(&drive_id)
            .await
            .ok_or(AppError::NotFound)?;
        return app
            .opener()
            .open_path(video.path.to_string_lossy(), None::<&str>)
            .map_err(|error| AppError::internal(format!("Could not open the video: {error}")));
    }
    let email = state
        .account()
        .await
        .ok()
        .and_then(|user| user.email_address);
    let url = drive_view_url(&drive_id, email.as_deref())?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|error| AppError::internal(format!("Could not open the browser: {error}")))
}

/// Shows a video on this computer in File Explorer, selected in its folder.
#[tauri::command]
pub async fn video_show_in_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    drive_id: String,
) -> AppResult<()> {
    if !is_local_id(&drive_id) {
        return Err(AppError::NotFound);
    }
    let video = state
        .local
        .find(&drive_id)
        .await
        .ok_or(AppError::NotFound)?;
    app.opener()
        .reveal_item_in_dir(&video.path)
        .map_err(|error| AppError::internal(format!("Could not open File Explorer: {error}")))
}

/// Microsoft's HEVC Video Extensions ($0.99). WebView2 decodes HEVC, which iPhones record in,
/// only when this codec is installed; without it those videos play sound over a black picture.
const HEVC_EXTENSIONS_STORE_URL: &str = "ms-windows-store://pdp/?ProductId=9NMZLZ57R3T7";

/// Opens the HEVC Video Extensions page in the Microsoft Store app.
#[tauri::command]
pub fn playback_open_hevc_store(app: AppHandle) -> AppResult<()> {
    app.opener()
        .open_url(HEVC_EXTENSIONS_STORE_URL, None::<&str>)
        .map_err(|error| AppError::internal(format!("Could not open the Microsoft Store: {error}")))
}

/// `authuser` picks the right account when the browser is signed in to several.
pub fn drive_view_url(drive_id: &str, email: Option<&str>) -> AppResult<Url> {
    let mut url = Url::parse(&format!("https://drive.google.com/file/d/{drive_id}/view"))
        .map_err(|error| AppError::internal(format!("Invalid Drive URL: {error}")))?;
    if let Some(email) = email {
        url.query_pairs_mut().append_pair("authuser", email);
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_url_selects_the_connected_account() {
        let url = drive_view_url("abc123", Some("sam+videos@gmail.com")).unwrap();

        assert_eq!(
            url.as_str(),
            "https://drive.google.com/file/d/abc123/view?authuser=sam%2Bvideos%40gmail.com"
        );
    }

    #[test]
    fn view_url_without_account_has_no_query() {
        let url = drive_view_url("abc123", None).unwrap();

        assert_eq!(url.as_str(), "https://drive.google.com/file/d/abc123/view");
    }
}
