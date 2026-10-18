use std::collections::HashMap;

use rusqlite::{Connection, params, params_from_iter};

use super::{as_params, placeholders, sql_now};

/// What ffmpeg read from a video on this computer; see `video_media_info`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MediaInfoRecord {
    pub duration_ms: Option<u64>,
    /// Upright, as the video plays.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub fn records(
    connection: &Connection,
    ids: &[String],
) -> rusqlite::Result<HashMap<String, MediaInfoRecord>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT drive_id, duration_ms, width, height FROM video_media_info \
         WHERE drive_id IN ({})",
        placeholders(ids.len())
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(as_params(ids)), |row| {
        Ok((
            row.get::<_, String>(0)?,
            MediaInfoRecord {
                duration_ms: row
                    .get::<_, Option<i64>>(1)?
                    .map(|duration| duration.max(0) as u64),
                width: row.get(2)?,
                height: row.get(3)?,
            },
        ))
    })?;
    rows.collect()
}

pub fn save(
    connection: &Connection,
    drive_id: &str,
    record: &MediaInfoRecord,
) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "INSERT INTO video_media_info (drive_id, duration_ms, width, height, checked_at) \
             VALUES (?1, ?2, ?3, ?4, ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET duration_ms = excluded.duration_ms, \
             width = excluded.width, height = excluded.height, checked_at = excluded.checked_at"
        ),
        params![
            drive_id,
            record
                .duration_ms
                .map(|duration| duration.min(i64::MAX as u64) as i64),
            record.width,
            record.height
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn saves_and_replaces_media_info() {
        let database = Database::open_in_memory();
        let portrait = MediaInfoRecord {
            duration_ms: Some(15_000),
            width: Some(1080),
            height: Some(1920),
        };
        let ids = vec!["v1".to_owned(), "v2".to_owned(), "missing".to_owned()];

        let stored = database
            .with(|connection| {
                save(connection, "v1", &MediaInfoRecord::default())?;
                save(connection, "v1", &portrait)?;
                save(connection, "v2", &MediaInfoRecord::default())?;
                records(connection, &ids)
            })
            .unwrap();

        assert_eq!(stored.len(), 2);
        assert_eq!(stored["v1"], portrait);
        assert_eq!(stored["v2"], MediaInfoRecord::default());
    }
}
