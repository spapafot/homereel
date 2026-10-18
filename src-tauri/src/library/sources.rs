//! Where the library's videos come from, as chosen in Settings: the connected Google Drive (all
//! of it, or one folder with everything inside it) and folders on this computer.

use std::path::PathBuf;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::settings;

/// The Drive folder the library is limited to. The name is kept for Settings, so showing it
/// needs no request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFolderChoice {
    pub id: String,
    pub name: String,
}

fn to_json(value: &impl Serialize) -> rusqlite::Result<String> {
    serde_json::to_string(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

/// `None` means all of Drive.
pub fn drive_folder(connection: &Connection) -> rusqlite::Result<Option<DriveFolderChoice>> {
    Ok(settings::get(connection, settings::DRIVE_FOLDER)?
        .and_then(|json| serde_json::from_str(&json).ok()))
}

pub fn set_drive_folder(
    connection: &Connection,
    folder: Option<&DriveFolderChoice>,
) -> rusqlite::Result<()> {
    match folder {
        Some(folder) => settings::set(connection, settings::DRIVE_FOLDER, &to_json(folder)?),
        None => settings::remove(connection, settings::DRIVE_FOLDER),
    }
}

/// Folders on this computer, in the order they were added.
pub fn local_folders(connection: &Connection) -> rusqlite::Result<Vec<PathBuf>> {
    Ok(settings::get(connection, settings::LOCAL_FOLDERS)?
        .and_then(|json| serde_json::from_str::<Vec<PathBuf>>(&json).ok())
        .unwrap_or_default())
}

pub fn set_local_folders(connection: &Connection, folders: &[PathBuf]) -> rusqlite::Result<()> {
    settings::set(connection, settings::LOCAL_FOLDERS, &to_json(&folders)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn stores_and_clears_the_drive_folder() {
        let database = Database::open_in_memory();
        let vids = DriveFolderChoice {
            id: "vids".into(),
            name: "VIDS".into(),
        };

        database.with(|c| set_drive_folder(c, Some(&vids))).unwrap();
        assert_eq!(database.with(|c| drive_folder(c)).unwrap(), Some(vids));

        database.with(|c| set_drive_folder(c, None)).unwrap();
        assert_eq!(database.with(|c| drive_folder(c)).unwrap(), None);
    }

    #[test]
    fn stores_local_folders_in_order() {
        let database = Database::open_in_memory();
        let folders = vec![
            PathBuf::from(r"D:\Clips"),
            PathBuf::from(r"C:\Users\Sam\Videos"),
        ];

        assert!(database.with(|c| local_folders(c)).unwrap().is_empty());
        database.with(|c| set_local_folders(c, &folders)).unwrap();

        assert_eq!(database.with(|c| local_folders(c)).unwrap(), folders);
    }
}
