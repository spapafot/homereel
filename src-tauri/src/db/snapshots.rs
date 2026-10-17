use std::collections::HashMap;

use rusqlite::{Connection, params, params_from_iter};

use super::{as_params, placeholders, sql_now};
use crate::library::video_summary::VideoFile;

fn to_json(file: &VideoFile) -> rusqlite::Result<String> {
    serde_json::to_string(file)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

/// Stores or replaces the display snapshot of each video.
pub fn upsert(connection: &mut Connection, files: &[VideoFile]) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    {
        let mut statement = transaction.prepare(concat!(
            "INSERT INTO video_snapshots (drive_id, data, updated_at) VALUES (?1, ?2, ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at"
        ))?;
        for file in files {
            statement.execute(params![file.id, to_json(file)?])?;
        }
    }
    transaction.commit()
}

/// Updates snapshots that already exist, so renamed or moved files show current details,
/// without snapshotting every listed video.
pub fn refresh_existing(connection: &mut Connection, files: &[VideoFile]) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    {
        let mut statement = transaction.prepare(concat!(
            "UPDATE video_snapshots SET data = ?2, updated_at = ",
            sql_now!(),
            " WHERE drive_id = ?1 AND data <> ?2"
        ))?;
        for file in files {
            statement.execute(params![file.id, to_json(file)?])?;
        }
    }
    transaction.commit()
}

pub fn exists(connection: &Connection, id: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM video_snapshots WHERE drive_id = ?1)",
        params![id],
        |row| row.get(0),
    )
}

pub fn load(
    connection: &Connection,
    ids: &[String],
) -> rusqlite::Result<HashMap<String, VideoFile>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT drive_id, data FROM video_snapshots WHERE drive_id IN ({})",
        placeholders(ids.len())
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(as_params(ids)), |row| {
        let id: String = row.get(0)?;
        let data: String = row.get(1)?;
        let file: VideoFile = serde_json::from_str(&data).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        Ok((id, file))
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::db::test_support::video;

    #[test]
    fn round_trips_snapshots() {
        let database = Database::open_in_memory();
        database
            .with(|c| upsert(c, &[video("a", "a.mp4")]))
            .unwrap();

        let loaded = database
            .with(|c| load(c, &["a".to_string(), "missing".to_string()]))
            .unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded["a"], video("a", "a.mp4"));
        assert!(database.with(|c| exists(c, "a")).unwrap());
    }

    #[test]
    fn refresh_only_touches_existing_snapshots() {
        let database = Database::open_in_memory();
        database
            .with(|c| upsert(c, &[video("a", "old.mp4")]))
            .unwrap();

        database
            .with(|c| refresh_existing(c, &[video("a", "renamed.mp4"), video("b", "b.mp4")]))
            .unwrap();

        let loaded = database
            .with(|c| load(c, &["a".to_string(), "b".to_string()]))
            .unwrap();
        assert_eq!(loaded["a"].file_name, "renamed.mp4");
        assert!(!loaded.contains_key("b"));
    }
}
