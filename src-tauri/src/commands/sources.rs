use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::drive::client::MY_DRIVE;
use crate::drive::is_valid_drive_id;
use crate::error::{AppError, AppResult};
use crate::library::local_files::folder_display_name;
use crate::library::sources::DriveFolderChoice;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySources {
    /// `None` when the whole of Drive is in the library.
    drive_folder: Option<DriveFolderChoice>,
    local_folders: Vec<LocalFolder>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFolder {
    path: String,
    name: String,
    /// False while the folder is missing, for example on an unplugged drive.
    available: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFolderItem {
    id: String,
    name: String,
}

#[tauri::command]
pub fn library_sources_get(state: State<'_, AppState>) -> AppResult<LibrarySources> {
    current_sources(&state)
}

/// Asks for a folder with the system folder picker and adds it to the library. Cancelling the
/// picker changes nothing.
#[tauri::command]
pub async fn library_local_folder_add(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<LibrarySources> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose a folder of videos")
        .set_parent(&window)
        .pick_folder(move |folder| {
            let _ = sender.send(folder);
        });
    let Some(picked) = receiver.await.ok().flatten() else {
        return current_sources(&state);
    };
    let path = picked
        .into_path()
        .map_err(|_| AppError::internal("Homereel can’t use that folder."))?;
    let mut folders = state.local.roots();
    if !folders.iter().any(|folder| same_folder(folder, &path)) {
        folders.push(path);
        state.set_local_folders(folders)?;
        state.date_scan.start(&app);
    }
    current_sources(&state)
}

#[tauri::command]
pub fn library_local_folder_remove(
    state: State<'_, AppState>,
    path: String,
) -> AppResult<LibrarySources> {
    let mut folders = state.local.roots();
    folders.retain(|folder| !same_folder(folder, Path::new(&path)));
    state.set_local_folders(folders)?;
    current_sources(&state)
}

/// Limits the Drive part of the library to one folder and everything inside it, or with `None`
/// opens it to all of Drive again.
#[tauri::command]
pub async fn library_drive_folder_set(
    app: AppHandle,
    state: State<'_, AppState>,
    folder: Option<DriveFolderChoice>,
) -> AppResult<LibrarySources> {
    if let Some(folder) = &folder
        && (!is_valid_drive_id(&folder.id) || folder.id == MY_DRIVE)
    {
        return Err(AppError::NotFound);
    }
    state.set_drive_folder(folder).await?;
    state.date_scan.start(&app);
    current_sources(&state)
}

/// The folders directly inside a Drive folder, or at the top of My Drive, by name.
#[tauri::command]
pub async fn drive_folders_list(
    state: State<'_, AppState>,
    parent_id: Option<String>,
) -> AppResult<Vec<DriveFolderItem>> {
    let parent_id = parent_id.unwrap_or_else(|| MY_DRIVE.to_owned());
    let folders = state.drive.list_folders(&[parent_id]).await?;
    Ok(folders
        .into_iter()
        .map(|folder| DriveFolderItem {
            id: folder.id,
            name: folder.name,
        })
        .collect())
}

fn current_sources(state: &AppState) -> AppResult<LibrarySources> {
    Ok(LibrarySources {
        drive_folder: state.drive_folder()?,
        local_folders: state
            .local
            .roots()
            .into_iter()
            .map(|path| LocalFolder {
                name: folder_display_name(&path),
                available: path.is_dir(),
                path: path.to_string_lossy().into_owned(),
            })
            .collect(),
    })
}

/// Windows paths ignore case and a trailing separator.
fn same_folder(a: &Path, b: &Path) -> bool {
    fn normalized(path: &Path) -> String {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    }
    normalized(a) == normalized(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn compares_folders_like_windows_does() {
        assert!(same_folder(
            &PathBuf::from(r"C:\Users\Sam\Videos"),
            Path::new("c:/users/sam/videos/")
        ));
        assert!(!same_folder(
            &PathBuf::from(r"C:\Users\Sam\Videos"),
            Path::new(r"C:\Users\Sam\Videos\Trip")
        ));
    }
}
