use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::sql_now;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PersonKind {
    Person,
    Animal,
}

impl PersonKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PersonKind::Person => "person",
            PersonKind::Animal => "animal",
        }
    }

    fn from_db(text: &str) -> Self {
        if text == "animal" {
            PersonKind::Animal
        } else {
            PersonKind::Person
        }
    }
}

/// A person or pet as the Settings list shows it. Photos are served by ID.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub kind: PersonKind,
    pub description: Option<String>,
    pub photo_ids: Vec<i64>,
}

/// A person with photo bytes, as sent to Gemini.
pub struct PersonWithPhotos {
    pub person: Person,
    pub photos: Vec<Vec<u8>>,
}

/// The whole edit from the person dialog, applied at once.
pub struct PersonChanges<'a> {
    pub name: &'a str,
    pub kind: PersonKind,
    pub description: Option<&'a str>,
    pub keep_photo_ids: &'a [i64],
    pub new_photos: &'a [Vec<u8>],
}

#[derive(Debug, PartialEq)]
pub enum SaveResult {
    Saved(Person),
    NameTaken,
    NotFound,
}

pub fn list(connection: &Connection) -> rusqlite::Result<Vec<Person>> {
    let mut photo_ids = photo_ids_by_person(connection)?;
    let mut statement = connection.prepare(
        "SELECT id, name, kind, description FROM people ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(Person {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: PersonKind::from_db(&row.get::<_, String>(2)?),
            description: row.get(3)?,
            photo_ids: Vec::new(),
        })
    })?;
    rows.map(|row| {
        row.map(|mut person| {
            person.photo_ids = photo_ids.remove(&person.id).unwrap_or_default();
            person
        })
    })
    .collect()
}

fn photo_ids_by_person(connection: &Connection) -> rusqlite::Result<HashMap<i64, Vec<i64>>> {
    let mut statement =
        connection.prepare("SELECT person_id, id FROM person_photos ORDER BY person_id, id")?;
    let mut by_person: HashMap<i64, Vec<i64>> = HashMap::new();
    let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get(1)?)))?;
    for row in rows {
        let (person_id, photo_id) = row?;
        by_person.entry(person_id).or_default().push(photo_id);
    }
    Ok(by_person)
}

pub fn get(connection: &Connection, id: i64) -> rusqlite::Result<Option<Person>> {
    Ok(list(connection)?.into_iter().find(|person| person.id == id))
}

/// Creates a person (`id` is `None`) or updates one, replacing the photos not kept.
pub fn save(
    connection: &mut Connection,
    id: Option<i64>,
    changes: &PersonChanges<'_>,
) -> rusqlite::Result<SaveResult> {
    let transaction = connection.transaction()?;
    let name_taken = transaction
        .query_row(
            "SELECT 1 FROM people WHERE name = ?1 AND id IS NOT ?2",
            params![changes.name, id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if name_taken {
        return Ok(SaveResult::NameTaken);
    }

    let id = match id {
        Some(id) => {
            let updated = transaction.execute(
                concat!(
                    "UPDATE people SET name = ?2, kind = ?3, description = ?4, updated_at = ",
                    sql_now!(),
                    " WHERE id = ?1"
                ),
                params![id, changes.name, changes.kind.as_str(), changes.description],
            )?;
            if updated == 0 {
                return Ok(SaveResult::NotFound);
            }
            id
        }
        None => {
            transaction.execute(
                concat!(
                    "INSERT INTO people (name, kind, description, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ",
                    sql_now!(),
                    ", ",
                    sql_now!(),
                    ")"
                ),
                params![changes.name, changes.kind.as_str(), changes.description],
            )?;
            transaction.last_insert_rowid()
        }
    };

    let kept: Vec<String> = changes.keep_photo_ids.iter().map(i64::to_string).collect();
    transaction.execute(
        &format!(
            "DELETE FROM person_photos WHERE person_id = ?1 AND id NOT IN ({})",
            // IDs are integers, so they are safe to inline.
            kept.join(", ")
        ),
        [id],
    )?;
    for photo in changes.new_photos {
        transaction.execute(
            concat!(
                "INSERT INTO person_photos (person_id, data, created_at) VALUES (?1, ?2, ",
                sql_now!(),
                ")"
            ),
            params![id, photo],
        )?;
    }
    let saved = get(&transaction, id)?;
    transaction.commit()?;
    Ok(saved.map_or(SaveResult::NotFound, SaveResult::Saved))
}

pub fn delete(connection: &Connection, id: i64) -> rusqlite::Result<bool> {
    Ok(connection.execute("DELETE FROM people WHERE id = ?1", [id])? > 0)
}

pub fn photo(connection: &Connection, photo_id: i64) -> rusqlite::Result<Option<Vec<u8>>> {
    connection
        .query_row(
            "SELECT data FROM person_photos WHERE id = ?1",
            [photo_id],
            |row| row.get(0),
        )
        .optional()
}

/// Everyone who has at least one photo, with the photos, for a title request.
pub fn with_photos(connection: &Connection) -> rusqlite::Result<Vec<PersonWithPhotos>> {
    let mut statement =
        connection.prepare("SELECT data FROM person_photos WHERE person_id = ?1 ORDER BY id")?;
    let mut people = Vec::new();
    for person in list(connection)? {
        let photos = statement
            .query_map([person.id], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<Vec<u8>>>>()?;
        if !photos.is_empty() {
            people.push(PersonWithPhotos { person, photos });
        }
    }
    Ok(people)
}

/// Replaces who was recognized in a video. People deleted in the meantime are skipped.
pub fn set_recognized(
    connection: &Connection,
    drive_id: &str,
    person_ids: &[i64],
) -> rusqlite::Result<()> {
    connection.execute(
        "DELETE FROM video_ai_people WHERE drive_id = ?1",
        [drive_id],
    )?;
    for person_id in person_ids {
        connection.execute(
            "INSERT OR IGNORE INTO video_ai_people (drive_id, person_id)
             SELECT ?1, id FROM people WHERE id = ?2",
            params![drive_id, person_id],
        )?;
    }
    Ok(())
}

pub fn recognized_names(connection: &Connection, drive_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT people.name FROM video_ai_people
         JOIN people ON people.id = video_ai_people.person_id
         WHERE video_ai_people.drive_id = ?1
         ORDER BY people.name COLLATE NOCASE",
    )?;
    let rows = statement.query_map([drive_id], |row| row.get(0))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn changes<'a>(
        name: &'a str,
        keep_photo_ids: &'a [i64],
        new_photos: &'a [Vec<u8>],
    ) -> PersonChanges<'a> {
        PersonChanges {
            name,
            kind: PersonKind::Person,
            description: Some("my daughter"),
            keep_photo_ids,
            new_photos,
        }
    }

    fn saved(result: SaveResult) -> Person {
        match result {
            SaveResult::Saved(person) => person,
            other => panic!("expected a saved person, got {other:?}"),
        }
    }

    #[test]
    fn creates_a_person_with_photos_and_lists_by_name() {
        let database = Database::open_in_memory();

        let people = database
            .with(|c| {
                save(c, None, &changes("Maria", &[], &[vec![1], vec![2]]))?;
                save(
                    c,
                    None,
                    &PersonChanges {
                        kind: PersonKind::Animal,
                        description: None,
                        ..changes("Rex", &[], &[vec![3]])
                    },
                )?;
                save(c, None, &changes("alex", &[], &[vec![4]]))?;
                list(c)
            })
            .unwrap();

        let names: Vec<&str> = people.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["alex", "Maria", "Rex"]);
        assert_eq!(people[1].photo_ids.len(), 2);
        assert_eq!(people[1].description.as_deref(), Some("my daughter"));
        assert_eq!(people[2].kind, PersonKind::Animal);
    }

    #[test]
    fn names_are_unique_ignoring_case() {
        let database = Database::open_in_memory();

        let (duplicate, same_person) = database
            .with(|c| {
                let maria = saved(save(c, None, &changes("Maria", &[], &[vec![1]]))?);
                let duplicate = save(c, None, &changes("maria", &[], &[vec![2]]))?;
                let keep = maria.photo_ids.clone();
                let same_person = save(c, Some(maria.id), &changes("MARIA", &keep, &[]))?;
                Ok((duplicate, same_person))
            })
            .unwrap();

        assert_eq!(duplicate, SaveResult::NameTaken);
        assert_eq!(saved(same_person).name, "MARIA");
    }

    #[test]
    fn updating_keeps_only_the_listed_photos_and_adds_new_ones() {
        let database = Database::open_in_memory();

        let (person, photos) = database
            .with(|c| {
                let maria = saved(save(c, None, &changes("Maria", &[], &[vec![1], vec![2]]))?);
                let keep = [maria.photo_ids[1]];
                let updated = saved(save(
                    c,
                    Some(maria.id),
                    &changes("Maria", &keep, &[vec![3]]),
                )?);
                let photos = updated
                    .photo_ids
                    .iter()
                    .map(|id| photo(c, *id))
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok((updated, photos))
            })
            .unwrap();

        assert_eq!(person.photo_ids.len(), 2);
        assert_eq!(photos, vec![Some(vec![2]), Some(vec![3])]);
    }

    #[test]
    fn saving_an_unknown_person_reports_not_found() {
        let database = Database::open_in_memory();

        let result = database
            .with(|c| save(c, Some(99), &changes("Maria", &[], &[vec![1]])))
            .unwrap();

        assert_eq!(result, SaveResult::NotFound);
    }

    #[test]
    fn records_who_was_recognized_and_forgets_deleted_people() {
        let database = Database::open_in_memory();

        let (before, after) = database
            .with(|c| {
                let maria = saved(save(c, None, &changes("Maria", &[], &[vec![1]]))?);
                let rex = saved(save(c, None, &changes("Rex", &[], &[vec![2]]))?);
                set_recognized(c, "vid1", &[rex.id, maria.id, 404])?;
                let before = recognized_names(c, "vid1")?;
                delete(c, rex.id)?;
                Ok((before, recognized_names(c, "vid1")?))
            })
            .unwrap();

        assert_eq!(before, vec!["Maria", "Rex"]);
        assert_eq!(after, vec!["Maria"]);
    }

    #[test]
    fn deleting_a_person_removes_their_photos() {
        let database = Database::open_in_memory();

        let (photo_after_delete, remaining) = database
            .with(|c| {
                let maria = saved(save(c, None, &changes("Maria", &[], &[vec![1]]))?);
                delete(c, maria.id)?;
                Ok((photo(c, maria.photo_ids[0])?, with_photos(c)?.len()))
            })
            .unwrap();

        assert_eq!(photo_after_delete, None);
        assert_eq!(remaining, 0);
    }
}
