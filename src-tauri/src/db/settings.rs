use rusqlite::{Connection, OptionalExtension, params};

pub const GEMINI_MODEL: &str = "gemini_model";
/// The owner's notes that every title request includes.
pub const GEMINI_INSTRUCTIONS: &str = "gemini_instructions";

pub fn get(connection: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
}

pub fn set(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn stores_and_overwrites_values() {
        let database = Database::open_in_memory();

        database
            .with(|c| {
                set(c, GEMINI_MODEL, "gemini-3.5-flash")?;
                set(c, GEMINI_MODEL, "gemini-3.8-flash")
            })
            .unwrap();

        let value = database.with(|c| get(c, GEMINI_MODEL)).unwrap();
        assert_eq!(value.as_deref(), Some("gemini-3.8-flash"));
        assert_eq!(database.with(|c| get(c, "missing")).unwrap(), None);
    }
}
