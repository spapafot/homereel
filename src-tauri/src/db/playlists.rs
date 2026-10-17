use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::sql_now;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub video_count: i64,
    pub first_video_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistMembership {
    pub id: i64,
    pub name: String,
    pub contains: bool,
}

const SUMMARY_SELECT: &str = "
    SELECT p.id, p.name,
      (SELECT count(*) FROM playlist_items i WHERE i.playlist_id = p.id),
      (SELECT i.drive_id FROM playlist_items i WHERE i.playlist_id = p.id
         ORDER BY i.position LIMIT 1),
      p.updated_at
    FROM playlists p";

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlaylistSummary> {
    Ok(PlaylistSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        video_count: row.get(2)?,
        first_video_id: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn touch(connection: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "UPDATE playlists SET updated_at = ",
            sql_now!(),
            " WHERE id = ?1"
        ),
        params![playlist_id],
    )?;
    Ok(())
}

pub fn create(connection: &Connection, name: &str) -> rusqlite::Result<i64> {
    connection.execute(
        concat!(
            "INSERT INTO playlists (name, created_at, updated_at) VALUES (?1, ",
            sql_now!(),
            ", ",
            sql_now!(),
            ")"
        ),
        params![name],
    )?;
    Ok(connection.last_insert_rowid())
}

pub fn rename(connection: &Connection, playlist_id: i64, name: &str) -> rusqlite::Result<bool> {
    let changed = connection.execute(
        concat!(
            "UPDATE playlists SET name = ?2, updated_at = ",
            sql_now!(),
            " WHERE id = ?1"
        ),
        params![playlist_id, name],
    )?;
    Ok(changed > 0)
}

/// Deletes the playlist and its items. The videos themselves are untouched.
pub fn delete(connection: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])?;
    Ok(())
}

pub fn list(connection: &Connection) -> rusqlite::Result<Vec<PlaylistSummary>> {
    let mut statement =
        connection.prepare(&format!("{SUMMARY_SELECT} ORDER BY p.updated_at DESC"))?;
    statement.query_map([], summary_from_row)?.collect()
}

pub fn get(connection: &Connection, playlist_id: i64) -> rusqlite::Result<Option<PlaylistSummary>> {
    connection
        .query_row(
            &format!("{SUMMARY_SELECT} WHERE p.id = ?1"),
            params![playlist_id],
            summary_from_row,
        )
        .optional()
}

pub fn item_ids(connection: &Connection, playlist_id: i64) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT drive_id FROM playlist_items WHERE playlist_id = ?1 ORDER BY position, added_at",
    )?;
    statement
        .query_map(params![playlist_id], |row| row.get(0))?
        .collect()
}

/// Appends the video at the end. Adding a video that is already there changes nothing.
pub fn add_video(
    connection: &Connection,
    playlist_id: i64,
    drive_id: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        concat!(
            "INSERT OR IGNORE INTO playlist_items (playlist_id, drive_id, position, added_at)
             SELECT ?1, ?2, COALESCE(MAX(position) + 1, 0), ",
            sql_now!(),
            " FROM playlist_items WHERE playlist_id = ?1"
        ),
        params![playlist_id, drive_id],
    )?;
    touch(connection, playlist_id)
}

pub fn remove_video(
    connection: &mut Connection,
    playlist_id: i64,
    drive_id: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "DELETE FROM playlist_items WHERE playlist_id = ?1 AND drive_id = ?2",
        params![playlist_id, drive_id],
    )?;
    renumber(&transaction, playlist_id)?;
    touch(&transaction, playlist_id)?;
    transaction.commit()
}

/// Sets the order to `drive_ids`. Items missing from `drive_ids` keep their relative order after
/// the listed ones, so a stale list from the UI can never drop videos.
pub fn reorder(
    connection: &mut Connection,
    playlist_id: i64,
    drive_ids: &[String],
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    {
        let mut statement = transaction.prepare(
            "UPDATE playlist_items SET position = ?3 WHERE playlist_id = ?1 AND drive_id = ?2",
        )?;
        for (index, drive_id) in drive_ids.iter().enumerate() {
            statement.execute(params![
                playlist_id,
                drive_id,
                -(drive_ids.len() as i64) + index as i64
            ])?;
        }
    }
    renumber(&transaction, playlist_id)?;
    touch(&transaction, playlist_id)?;
    transaction.commit()
}

/// Rewrites positions as 0, 1, 2, … in the current order.
fn renumber(connection: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
    let ordered = item_ids(connection, playlist_id)?;
    let mut statement = connection.prepare(
        "UPDATE playlist_items SET position = ?3 WHERE playlist_id = ?1 AND drive_id = ?2",
    )?;
    for (position, drive_id) in ordered.iter().enumerate() {
        statement.execute(params![playlist_id, drive_id, position as i64])?;
    }
    Ok(())
}

pub fn memberships(
    connection: &Connection,
    drive_id: &str,
) -> rusqlite::Result<Vec<PlaylistMembership>> {
    let mut statement = connection.prepare(
        "SELECT p.id, p.name, EXISTS(
           SELECT 1 FROM playlist_items i WHERE i.playlist_id = p.id AND i.drive_id = ?1)
         FROM playlists p ORDER BY p.updated_at DESC",
    )?;
    statement
        .query_map(params![drive_id], |row| {
            Ok(PlaylistMembership {
                id: row.get(0)?,
                name: row.get(1)?,
                contains: row.get(2)?,
            })
        })?
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn playlist_with(database: &Database, videos: &[&str]) -> i64 {
        database
            .with(|c| {
                let id = create(c, "Summer")?;
                for video in videos {
                    add_video(c, id, video)?;
                }
                Ok(id)
            })
            .unwrap()
    }

    #[test]
    fn adds_videos_in_order_without_duplicates() {
        let database = Database::open_in_memory();
        let id = playlist_with(&database, &["a", "b", "a", "c"]);

        let items = database.with(|c| item_ids(c, id)).unwrap();
        let summary = database.with(|c| get(c, id)).unwrap().unwrap();

        assert_eq!(items, ids(&["a", "b", "c"]));
        assert_eq!(summary.video_count, 3);
        assert_eq!(summary.first_video_id.as_deref(), Some("a"));
    }

    #[test]
    fn reorders_and_keeps_items_missing_from_the_new_order() {
        let database = Database::open_in_memory();
        let id = playlist_with(&database, &["a", "b", "c", "d"]);

        database
            .with(|c| reorder(c, id, &ids(&["c", "a"])))
            .unwrap();

        assert_eq!(
            database.with(|c| item_ids(c, id)).unwrap(),
            ids(&["c", "a", "b", "d"])
        );
    }

    #[test]
    fn removing_a_video_closes_the_gap() {
        let database = Database::open_in_memory();
        let id = playlist_with(&database, &["a", "b", "c"]);

        database.with(|c| remove_video(c, id, "b")).unwrap();
        database.with(|c| add_video(c, id, "d")).unwrap();

        assert_eq!(
            database.with(|c| item_ids(c, id)).unwrap(),
            ids(&["a", "c", "d"])
        );
    }

    #[test]
    fn deleting_a_playlist_removes_its_items() {
        let database = Database::open_in_memory();
        let id = playlist_with(&database, &["a"]);

        database.with(|c| delete(c, id)).unwrap();

        assert!(database.with(|c| get(c, id)).unwrap().is_none());
        let orphans: i64 = database
            .with(|c| c.query_row("SELECT count(*) FROM playlist_items", [], |row| row.get(0)))
            .unwrap();
        assert_eq!(orphans, 0);
    }

    #[test]
    fn reports_which_playlists_contain_a_video() {
        let database = Database::open_in_memory();
        let summer = playlist_with(&database, &["a"]);
        let winter = database.with(|c| create(c, "Winter")).unwrap();

        let memberships = database.with(|c| memberships(c, "a")).unwrap();

        let contains = |id: i64| memberships.iter().find(|m| m.id == id).unwrap().contains;
        assert!(contains(summer));
        assert!(!contains(winter));
    }

    #[test]
    fn renames_and_rejects_blank_names() {
        let database = Database::open_in_memory();
        let id = playlist_with(&database, &[]);

        assert!(database.with(|c| rename(c, id, "Holidays")).unwrap());
        assert!(database.with(|c| rename(c, id, "   ")).is_err());
        assert_eq!(
            database.with(|c| get(c, id)).unwrap().unwrap().name,
            "Holidays"
        );
    }
}
