use std::collections::HashMap;

use rusqlite::{Connection, params, params_from_iter};

use super::{as_params, placeholders, sql_now};

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedDateRecord {
    /// `YYYY-MM-DD`, or `None` when the file holds no date.
    pub recorded_date: Option<String>,
    pub md5_checksum: Option<String>,
}

/// A video is read once, and again only if its content changed since.
pub fn needs_check(record: Option<&RecordedDateRecord>, md5_checksum: Option<&str>) -> bool {
    let Some(record) = record else {
        return true;
    };
    match (record.md5_checksum.as_deref(), md5_checksum) {
        (Some(stored), Some(current)) => stored != current,
        _ => false,
    }
}

pub fn records(
    connection: &Connection,
    ids: &[String],
) -> rusqlite::Result<HashMap<String, RecordedDateRecord>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT drive_id, recorded_date, md5_checksum FROM video_recorded_dates \
         WHERE drive_id IN ({})",
        placeholders(ids.len())
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(as_params(ids)), |row| {
        Ok((
            row.get::<_, String>(0)?,
            RecordedDateRecord {
                recorded_date: row.get(1)?,
                md5_checksum: row.get(2)?,
            },
        ))
    })?;
    rows.collect()
}

pub fn save(
    connection: &Connection,
    drive_id: &str,
    recorded_date: Option<&str>,
    md5_checksum: Option<&str>,
) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "INSERT INTO video_recorded_dates (drive_id, recorded_date, md5_checksum, checked_at) \
             VALUES (?1, ?2, ?3, ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET recorded_date = excluded.recorded_date, \
             md5_checksum = excluded.md5_checksum, checked_at = excluded.checked_at"
        ),
        params![drive_id, recorded_date, md5_checksum],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn record(md5: Option<&str>) -> RecordedDateRecord {
        RecordedDateRecord {
            recorded_date: Some("2024-06-12".into()),
            md5_checksum: md5.map(str::to_owned),
        }
    }

    #[test]
    fn checks_unread_and_changed_videos_only() {
        assert!(needs_check(None, Some("a")));
        assert!(needs_check(Some(&record(Some("a"))), Some("b")));
        assert!(!needs_check(Some(&record(Some("a"))), Some("a")));
        assert!(!needs_check(Some(&record(None)), Some("a")));
    }

    #[test]
    fn saves_and_replaces_dates_including_files_without_one() {
        let database = Database::open_in_memory();
        let ids = vec!["v1".to_owned(), "v2".to_owned(), "missing".to_owned()];

        let stored = database
            .with(|connection| {
                save(connection, "v1", Some("2019-08-13"), Some("old"))?;
                save(connection, "v1", Some("2019-08-14"), Some("new"))?;
                save(connection, "v2", None, None)?;
                records(connection, &ids)
            })
            .unwrap();

        assert_eq!(stored.len(), 2);
        assert_eq!(stored["v1"].recorded_date.as_deref(), Some("2019-08-14"));
        assert_eq!(stored["v1"].md5_checksum.as_deref(), Some("new"));
        assert_eq!(stored["v2"].recorded_date, None);
    }
}
