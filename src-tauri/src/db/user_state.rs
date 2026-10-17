use std::collections::HashMap;

use rusqlite::{Connection, params, params_from_iter};
use serde::Deserialize;

use super::{as_params, placeholders, sql_now};

/// Videos must be watched this far to count as watched, as on YouTube's red progress bar.
const WATCHED_FRACTION: f64 = 0.9;
/// Resume points shorter than this are not worth a Continue watching entry.
const MIN_RESUME_MS: i64 = 5_000;
const CONTINUE_WATCHING_LIMIT: u32 = 12;

/// DriveTube's own data about one video, merged into `VideoSummary`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocalState {
    pub custom_title: Option<String>,
    pub is_favorite: bool,
    pub is_watch_later: bool,
    pub last_position_ms: Option<u64>,
    pub watched: bool,
    pub last_watched_at: Option<String>,
    pub ai_title: Option<String>,
    pub ai_status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoList {
    Favorites,
    WatchLater,
    History,
    ContinueWatching,
}

/// SQLite integers are signed; millisecond values never come close to the limit.
fn as_sql_int(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub fn load(
    connection: &Connection,
    ids: &[String],
) -> rusqlite::Result<HashMap<String, LocalState>> {
    let mut states: HashMap<String, LocalState> = HashMap::new();
    if ids.is_empty() {
        return Ok(states);
    }

    let sql = format!(
        "SELECT drive_id, custom_title, is_favorite, is_watch_later, last_position_ms, watched,
                last_watched_at
         FROM user_video_state WHERE drive_id IN ({})",
        placeholders(ids.len())
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(as_params(ids)), |row| {
        Ok((
            row.get::<_, String>(0)?,
            LocalState {
                custom_title: row.get(1)?,
                is_favorite: row.get(2)?,
                is_watch_later: row.get(3)?,
                last_position_ms: row
                    .get::<_, Option<i64>>(4)?
                    .map(|position| position.max(0) as u64),
                watched: row.get(5)?,
                last_watched_at: row.get(6)?,
                ..LocalState::default()
            },
        ))
    })?;
    for row in rows {
        let (id, state) = row?;
        states.insert(id, state);
    }

    let sql = format!(
        "SELECT drive_id, title, status FROM video_ai_titles WHERE drive_id IN ({})",
        placeholders(ids.len())
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(as_params(ids)), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (id, title, status) = row?;
        let state = states.entry(id).or_default();
        state.ai_title = title;
        state.ai_status = Some(status);
    }
    Ok(states)
}

pub fn set_favorite(connection: &Connection, id: &str, value: bool) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "INSERT INTO user_video_state (drive_id, is_favorite, favorited_at, updated_at)
             VALUES (?1, ?2, CASE WHEN ?2 THEN ",
            sql_now!(),
            " END, ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET is_favorite = excluded.is_favorite,
               favorited_at = excluded.favorited_at, updated_at = excluded.updated_at"
        ),
        params![id, value],
    )?;
    Ok(())
}

pub fn set_watch_later(connection: &Connection, id: &str, value: bool) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "INSERT INTO user_video_state (drive_id, is_watch_later, watch_later_at, updated_at)
             VALUES (?1, ?2, CASE WHEN ?2 THEN ",
            sql_now!(),
            " END, ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET is_watch_later = excluded.is_watch_later,
               watch_later_at = excluded.watch_later_at, updated_at = excluded.updated_at"
        ),
        params![id, value],
    )?;
    Ok(())
}

pub fn set_custom_title(
    connection: &Connection,
    id: &str,
    title: Option<&str>,
) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "INSERT INTO user_video_state (drive_id, custom_title, updated_at) VALUES (?1, ?2, ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET custom_title = excluded.custom_title,
               updated_at = excluded.updated_at"
        ),
        params![id, title],
    )?;
    Ok(())
}

/// Saves the resume point and adds the video to history. Once watched, a video stays watched.
pub fn record_progress(
    connection: &Connection,
    id: &str,
    position_ms: u64,
    duration_ms: u64,
) -> rusqlite::Result<()> {
    let watched = duration_ms > 0 && position_ms as f64 >= duration_ms as f64 * WATCHED_FRACTION;
    connection.execute(
        concat!(
            "INSERT INTO user_video_state
               (drive_id, last_position_ms, duration_ms, last_watched_at, watched, updated_at)
             VALUES (?1, ?2, ?3, ",
            sql_now!(),
            ", ?4, ",
            sql_now!(),
            ") ON CONFLICT(drive_id) DO UPDATE SET last_position_ms = excluded.last_position_ms,
               duration_ms = excluded.duration_ms, last_watched_at = excluded.last_watched_at,
               watched = MAX(user_video_state.watched, excluded.watched),
               updated_at = excluded.updated_at"
        ),
        params![
            id,
            as_sql_int(position_ms),
            as_sql_int(duration_ms),
            watched
        ],
    )?;
    Ok(())
}

/// Removes one video from history, including its resume point.
pub fn remove_from_history(connection: &Connection, id: &str) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "UPDATE user_video_state SET last_watched_at = NULL, last_position_ms = NULL,
               watched = 0, updated_at = ",
            sql_now!(),
            " WHERE drive_id = ?1"
        ),
        params![id],
    )?;
    Ok(())
}

pub fn clear_history(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "UPDATE user_video_state SET last_watched_at = NULL, last_position_ms = NULL,
               watched = 0, updated_at = ",
            sql_now!(),
            " WHERE last_watched_at IS NOT NULL OR last_position_ms IS NOT NULL OR watched = 1"
        ),
        [],
    )?;
    Ok(())
}

/// `limit` IDs of a list, starting `offset` from the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListPage {
    pub offset: u32,
    pub limit: u32,
}

/// Drive IDs in a list, in the order the list shows them: all of them, or one page.
pub fn list_ids(
    connection: &Connection,
    list: VideoList,
    page: Option<ListPage>,
) -> rusqlite::Result<Vec<String>> {
    let (sql, full_length) = match list {
        VideoList::Favorites => (
            "SELECT drive_id FROM user_video_state WHERE is_favorite = 1
             ORDER BY favorited_at DESC LIMIT ?1 OFFSET ?2",
            u32::MAX,
        ),
        VideoList::WatchLater => (
            "SELECT drive_id FROM user_video_state WHERE is_watch_later = 1
             ORDER BY watch_later_at DESC LIMIT ?1 OFFSET ?2",
            u32::MAX,
        ),
        VideoList::History => (
            "SELECT drive_id FROM user_video_state WHERE last_watched_at IS NOT NULL
             ORDER BY last_watched_at DESC LIMIT ?1 OFFSET ?2",
            u32::MAX,
        ),
        VideoList::ContinueWatching => (
            "SELECT drive_id FROM user_video_state
             WHERE last_watched_at IS NOT NULL AND watched = 0 AND last_position_ms >= ?3
             ORDER BY last_watched_at DESC LIMIT ?1 OFFSET ?2",
            CONTINUE_WATCHING_LIMIT,
        ),
    };
    let page = page.unwrap_or(ListPage {
        offset: 0,
        limit: full_length,
    });
    let mut statement = connection.prepare(sql)?;
    let ids = if list == VideoList::ContinueWatching {
        statement
            .query_map(params![page.limit, page.offset, MIN_RESUME_MS], |row| {
                row.get(0)
            })?
            .collect::<rusqlite::Result<Vec<String>>>()?
    } else {
        statement
            .query_map(params![page.limit, page.offset], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?
    };
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn favorites_and_watch_later_toggle_independently() {
        let database = Database::open_in_memory();

        database
            .with(|c| {
                set_favorite(c, "a", true)?;
                set_watch_later(c, "a", true)?;
                set_favorite(c, "a", false)
            })
            .unwrap();

        let state = &database.with(|c| load(c, &ids(&["a"]))).unwrap()["a"];
        assert!(!state.is_favorite);
        assert!(state.is_watch_later);
        assert!(
            database
                .with(|c| list_ids(c, VideoList::Favorites, None))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            database
                .with(|c| list_ids(c, VideoList::WatchLater, None))
                .unwrap(),
            ids(&["a"])
        );
    }

    #[test]
    fn progress_marks_watched_at_ninety_percent_and_feeds_continue_watching() {
        let database = Database::open_in_memory();

        database
            .with(|c| {
                record_progress(c, "half", 30_000, 60_000)?;
                record_progress(c, "done", 55_000, 60_000)?;
                record_progress(c, "barely", 2_000, 60_000)
            })
            .unwrap();

        let states = database.with(|c| load(c, &ids(&["half", "done"]))).unwrap();
        assert!(!states["half"].watched);
        assert!(states["done"].watched);
        assert_eq!(states["half"].last_position_ms, Some(30_000));
        assert_eq!(
            database
                .with(|c| list_ids(c, VideoList::ContinueWatching, None))
                .unwrap(),
            ids(&["half"])
        );
        assert_eq!(
            database
                .with(|c| list_ids(c, VideoList::History, None))
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn watched_stays_watched_when_rewatching_from_the_start() {
        let database = Database::open_in_memory();

        database
            .with(|c| {
                record_progress(c, "a", 59_000, 60_000)?;
                record_progress(c, "a", 1_000, 60_000)
            })
            .unwrap();

        assert!(database.with(|c| load(c, &ids(&["a"]))).unwrap()["a"].watched);
    }

    #[test]
    fn removing_history_keeps_favorites() {
        let database = Database::open_in_memory();
        database
            .with(|c| {
                set_favorite(c, "a", true)?;
                record_progress(c, "a", 10_000, 60_000)?;
                record_progress(c, "b", 10_000, 60_000)?;
                remove_from_history(c, "a")
            })
            .unwrap();

        assert_eq!(
            database
                .with(|c| list_ids(c, VideoList::History, None))
                .unwrap(),
            ids(&["b"])
        );
        let state = &database.with(|c| load(c, &ids(&["a"]))).unwrap()["a"];
        assert!(state.is_favorite);
        assert_eq!(state.last_position_ms, None);

        database.with(|c| clear_history(c)).unwrap();
        assert!(
            database
                .with(|c| list_ids(c, VideoList::History, None))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn history_pages_through_every_watched_video() {
        let database = Database::open_in_memory();
        database
            .with(|c| {
                for id in ["a", "b", "c"] {
                    record_progress(c, id, 10_000, 60_000)?;
                    // Watch times are stored to the millisecond; keep them distinct.
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                Ok(())
            })
            .unwrap();
        let page = |offset| {
            database
                .with(|c| list_ids(c, VideoList::History, Some(ListPage { offset, limit: 2 })))
                .unwrap()
        };

        assert_eq!(page(0), ids(&["c", "b"]));
        assert_eq!(page(2), ids(&["a"]));
        assert!(page(4).is_empty());
    }

    #[test]
    fn load_merges_ai_titles_for_videos_without_user_state() {
        let database = Database::open_in_memory();
        database
            .with(|c| {
                c.execute(
                    "INSERT INTO video_ai_titles (drive_id, title, status, updated_at)
                     VALUES ('a', 'Kids at the beach', 'done', 'now')",
                    [],
                )
            })
            .unwrap();

        let state = &database.with(|c| load(c, &ids(&["a"]))).unwrap()["a"];

        assert_eq!(state.ai_title.as_deref(), Some("Kids at the beach"));
        assert_eq!(state.ai_status.as_deref(), Some("done"));
        assert!(!state.is_favorite);
    }
}
