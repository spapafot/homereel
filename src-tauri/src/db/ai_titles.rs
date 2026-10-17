use std::collections::HashMap;

use rusqlite::{Connection, params, params_from_iter};
use serde::{Deserialize, Serialize};

use super::{as_params, placeholders, sql_now};

#[derive(Debug, Clone, PartialEq)]
pub struct TitleRecord {
    pub status: String,
    pub md5_checksum: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleCounts {
    pub done: i64,
    pub failed: i64,
}

pub struct GeneratedTitle<'a> {
    pub title: &'a str,
    pub confidence: f64,
    pub model: &'a str,
    pub md5_checksum: Option<&'a str>,
    pub prompt_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

/// Which videos a run of the title queue covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TitleScope {
    Missing,
    MissingAndFailed,
    /// Every video again, for example after adding people. The queue skips videos the user
    /// titled themselves, since their AI title is not shown.
    All,
}

/// Generate once: a video needs a title when it has none, or when its content changed since the
/// title was generated. Failed videos are only retried on request, and good titles only
/// replaced when everything is regenerated.
pub fn needs_title(
    record: Option<&TitleRecord>,
    md5_checksum: Option<&str>,
    scope: TitleScope,
) -> bool {
    let Some(record) = record else {
        return true;
    };
    if scope == TitleScope::All {
        return true;
    }
    match record.status.as_str() {
        "pending" => true,
        "failed" => scope == TitleScope::MissingAndFailed,
        _ => match (record.md5_checksum.as_deref(), md5_checksum) {
            (Some(stored), Some(current)) => stored != current,
            _ => false,
        },
    }
}

pub fn records(
    connection: &Connection,
    ids: &[String],
) -> rusqlite::Result<HashMap<String, TitleRecord>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT drive_id, status, md5_checksum FROM video_ai_titles WHERE drive_id IN ({})",
        placeholders(ids.len())
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(as_params(ids)), |row| {
        Ok((
            row.get::<_, String>(0)?,
            TitleRecord {
                status: row.get(1)?,
                md5_checksum: row.get(2)?,
            },
        ))
    })?;
    rows.collect()
}

pub fn save_done(
    connection: &Connection,
    drive_id: &str,
    generated: &GeneratedTitle<'_>,
) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "INSERT INTO video_ai_titles (drive_id, title, confidence, model, md5_checksum, status,
               error, attempts, prompt_tokens, output_tokens, generated_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'done', NULL, 1, ?6, ?7, ",
            sql_now!(),
            ", ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET title = excluded.title,
               confidence = excluded.confidence, model = excluded.model,
               md5_checksum = excluded.md5_checksum, status = 'done', error = NULL,
               attempts = video_ai_titles.attempts + 1, prompt_tokens = excluded.prompt_tokens,
               output_tokens = excluded.output_tokens, generated_at = excluded.generated_at,
               updated_at = excluded.updated_at"
        ),
        params![
            drive_id,
            generated.title,
            generated.confidence,
            generated.model,
            generated.md5_checksum,
            generated.prompt_tokens,
            generated.output_tokens
        ],
    )?;
    Ok(())
}

/// Records a failure. A previously generated title is kept and stays on screen.
pub fn save_failed(connection: &Connection, drive_id: &str, error: &str) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "INSERT INTO video_ai_titles (drive_id, status, error, attempts, updated_at)
             VALUES (?1, 'failed', ?2, 1, ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET
               status = CASE WHEN video_ai_titles.title IS NULL THEN 'failed' ELSE video_ai_titles.status END,
               error = excluded.error, attempts = video_ai_titles.attempts + 1,
               updated_at = excluded.updated_at"
        ),
        params![drive_id, error],
    )?;
    Ok(())
}

pub fn counts(connection: &Connection) -> rusqlite::Result<TitleCounts> {
    connection.query_row(
        "SELECT COALESCE(SUM(status = 'done'), 0), COALESCE(SUM(status = 'failed'), 0)
         FROM video_ai_titles",
        [],
        |row| {
            Ok(TitleCounts {
                done: row.get(0)?,
                failed: row.get(1)?,
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn record(status: &str, md5: Option<&str>) -> TitleRecord {
        TitleRecord {
            status: status.into(),
            md5_checksum: md5.map(Into::into),
        }
    }

    fn generated<'a>(title: &'a str, md5: Option<&'a str>) -> GeneratedTitle<'a> {
        GeneratedTitle {
            title,
            confidence: 0.8,
            model: "gemini-3.8-flash",
            md5_checksum: md5,
            prompt_tokens: Some(1_400),
            output_tokens: Some(30),
        }
    }

    #[test]
    fn generates_once_and_again_only_when_content_changes() {
        let scope = TitleScope::Missing;

        assert!(needs_title(None, Some("x"), scope));
        assert!(needs_title(Some(&record("pending", None)), None, scope));
        assert!(!needs_title(
            Some(&record("done", Some("x"))),
            Some("x"),
            scope
        ));
        assert!(needs_title(
            Some(&record("done", Some("x"))),
            Some("y"),
            scope
        ));
        assert!(!needs_title(Some(&record("done", None)), Some("y"), scope));
    }

    #[test]
    fn retries_failed_videos_only_on_request() {
        let failed = record("failed", None);

        assert!(!needs_title(Some(&failed), Some("x"), TitleScope::Missing));
        assert!(needs_title(
            Some(&failed),
            Some("x"),
            TitleScope::MissingAndFailed
        ));
    }

    #[test]
    fn regenerating_everything_includes_good_titles() {
        let done = record("done", Some("x"));

        assert!(!needs_title(
            Some(&done),
            Some("x"),
            TitleScope::MissingAndFailed
        ));
        assert!(needs_title(Some(&done), Some("x"), TitleScope::All));
    }

    #[test]
    fn a_failed_regeneration_keeps_the_existing_title() {
        let database = Database::open_in_memory();
        database
            .with(|c| {
                save_done(c, "a", &generated("Kids at the beach", Some("x")))?;
                save_failed(c, "a", "rate limited")?;
                save_failed(c, "b", "no frames")
            })
            .unwrap();

        let records = database
            .with(|c| records(c, &["a".to_string(), "b".to_string()]))
            .unwrap();
        assert_eq!(records["a"].status, "done");
        assert_eq!(records["b"].status, "failed");
        assert_eq!(
            database.with(|c| counts(c)).unwrap(),
            TitleCounts { done: 1, failed: 1 }
        );
    }
}
