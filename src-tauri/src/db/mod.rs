//! Local SQLite store for what Homereel adds on top of Drive: favorites, watch later,
//! playlists, history and resume points, edited and AI-generated titles, people and pets for
//! titles, recording dates and media details read from the files, and settings.

pub mod ai_titles;
pub mod media_info;
pub mod people;
pub mod playlists;
pub mod recorded_dates;
pub mod settings;
pub mod snapshots;
pub mod user_state;

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{Connection, ToSql};
use rusqlite_migration::{M, Migrations};

use crate::error::{AppError, AppResult};

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("../../migrations/001_init.sql")),
        M::up(include_str!("../../migrations/002_people.sql")),
        M::up(include_str!("../../migrations/003_recorded_dates.sql")),
        M::up(include_str!("../../migrations/004_media_info.sql")),
    ])
}

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Current UTC time as RFC 3339 with milliseconds, for SQL statements.
macro_rules! sql_now {
    () => {
        "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')"
    };
}
pub(crate) use sql_now;

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::initialize(Connection::open(path).map_err(db_error)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Self {
        Self::initialize(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn initialize(mut connection: Connection) -> AppResult<Self> {
        connection
            .query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
            .map_err(db_error)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(db_error)?;
        connection.busy_timeout(BUSY_TIMEOUT).map_err(db_error)?;
        migrations()
            .to_latest(&mut connection)
            .map_err(|error| AppError::internal(format!("Database migration failed: {error}")))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Runs `operation` with exclusive access to the connection.
    pub fn with<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> AppResult<T> {
        let mut connection = self
            .connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        operation(&mut connection).map_err(db_error)
    }
}

pub fn db_error(error: rusqlite::Error) -> AppError {
    AppError::internal(format!("Database error: {error}"))
}

/// `?, ?, ?` for an `IN (...)` clause with `count` parameters.
pub(crate) fn placeholders(count: usize) -> String {
    vec!["?"; count].join(", ")
}

pub(crate) fn as_params(ids: &[String]) -> Vec<&dyn ToSql> {
    ids.iter().map(|id| id as &dyn ToSql).collect()
}

#[cfg(test)]
pub mod test_support {
    use crate::library::video_summary::{VideoFile, VideoSource};

    pub fn video(id: &str, name: &str) -> VideoFile {
        VideoFile {
            id: id.into(),
            file_name: name.into(),
            mime_type: "video/mp4".into(),
            duration_ms: Some(60_000),
            width: Some(1920),
            height: Some(1080),
            size_bytes: Some(1_000),
            modified_time: Some("2026-10-01T10:00:00Z".into()),
            created_time: None,
            folder_id: Some("fold1".into()),
            folder_name: "Holidays".into(),
            has_thumbnail: true,
            md5_checksum: Some("md5".into()),
            source: VideoSource::Drive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid_and_apply_cleanly() {
        migrations().validate().unwrap();

        let database = Database::open_in_memory();

        let tables: i64 = database
            .with(|connection| {
                connection.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN \
                     ('video_snapshots', 'user_video_state', 'playlists', 'playlist_items', \
                      'video_ai_titles', 'app_settings', 'people', 'person_photos', \
                      'video_ai_people', 'video_recorded_dates', 'video_media_info')",
                    [],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert_eq!(tables, 11);
    }

    #[test]
    fn opens_a_database_file_and_reopens_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("homereel.sqlite3");

        drop(Database::open(&path).unwrap());
        let reopened = Database::open(&path);

        assert!(reopened.is_ok());
    }
}
