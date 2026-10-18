//! Homereel was first released as DriveTube (0.1.0). Its data folders, database file and
//! Credential Manager entries were named after the old identifier. They move to the new names on
//! the first start after the rename, so favorites, playlists, history, titles, people and the
//! Google connection carry over.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::auth::token_store::SecretStore;
use crate::error::AppResult;

const FORMER_IDENTIFIER: &str = "com.drivetube.desktop";
/// Credential Manager entries were filed under the identifier too.
pub const FORMER_KEYRING_SERVICE: &str = FORMER_IDENTIFIER;
const FORMER_DATABASE_FILE: &str = "drivetube.sqlite3";
/// SQLite keeps recent commits in `-wal` until they reach the database file, so both companions
/// must move with it.
const SQLITE_COMPANIONS: [&str; 2] = ["-wal", "-shm"];

/// Moves the former roaming folder (database, window state) and local folder (WebView2 profile,
/// thumbnails, logs) to the ones Tauri derives from `identifier`, then renames the database.
/// Must run before Tauri starts: Tauri creates the new folders right away.
pub fn move_data(identifier: &str, database_file: &str) -> io::Result<()> {
    let (Some(roaming), Some(local)) = (dirs::data_dir(), dirs::data_local_dir()) else {
        return Ok(());
    };
    move_data_in(&roaming, &local, identifier, database_file)
}

fn move_data_in(
    roaming: &Path,
    local: &Path,
    identifier: &str,
    database_file: &str,
) -> io::Result<()> {
    let data_dir = roaming.join(identifier);
    move_dir(&roaming.join(FORMER_IDENTIFIER), &data_dir)?;
    move_dir(&local.join(FORMER_IDENTIFIER), &local.join(identifier))?;
    rename_database(&data_dir, database_file)
}

/// Renames `former` to `current` in one step. Windows refuses while DriveTube still has files
/// open there, so nothing moves halfway. If `current` already exists, moves only the entries it
/// lacks; whatever is left stays in `former` rather than being deleted.
fn move_dir(former: &Path, current: &Path) -> io::Result<()> {
    if !former.is_dir() {
        return Ok(());
    }
    if !current.exists() {
        return fs::rename(former, current);
    }
    for entry in fs::read_dir(former)? {
        let entry = entry?;
        let target = current.join(entry.file_name());
        if !target.exists() {
            fs::rename(entry.path(), target)?;
        }
    }
    // Fails while entries are left, which keeps them.
    let _ = fs::remove_dir(former);
    Ok(())
}

/// Renames the companions before the database: if this stops halfway, the next start still finds
/// the former database and finishes, and SQLite never opens the database without its WAL.
fn rename_database(dir: &Path, database_file: &str) -> io::Result<()> {
    let former = dir.join(FORMER_DATABASE_FILE);
    let current = dir.join(database_file);
    if !former.exists() || current.exists() {
        return Ok(());
    }
    for suffix in SQLITE_COMPANIONS {
        let companion = with_suffix(&former, suffix);
        if companion.exists() {
            fs::rename(companion, with_suffix(&current, suffix))?;
        }
    }
    fs::rename(former, current)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Explains why Homereel stops. Tauri is not running yet, so this is a plain message box.
pub fn report_move_failure(error: &io::Error) {
    let message = format!(
        "Homereel could not move your data from DriveTube, its former name: {error}\n\n\
         Close DriveTube if it is open, then start Homereel again."
    );
    #[cfg(windows)]
    {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Homereel")
            .set_description(message)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
    #[cfg(not(windows))]
    eprintln!("{message}");
}

/// Moves each secret from the former Credential Manager service to the current one. A secret the
/// current service already has wins; the former copy is deleted either way.
pub fn move_secrets(
    former: &dyn SecretStore,
    current: &dyn SecretStore,
    keys: &[&str],
) -> AppResult<()> {
    for key in keys {
        let Some(value) = former.get(key)? else {
            continue;
        };
        if current.get(key)?.is_none() {
            current.set(key, &value)?;
        }
        former.delete(key)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::token_store::testing::MemorySecretStore;

    const IDENTIFIER: &str = "com.homereel.desktop";
    const DATABASE_FILE: &str = "homereel.sqlite3";

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn moves_the_former_folders_and_renames_the_database_with_its_wal() {
        let roaming = tempfile::tempdir().unwrap();
        let local = tempfile::tempdir().unwrap();
        let former_data = roaming.path().join(FORMER_IDENTIFIER);
        let former_local = local.path().join(FORMER_IDENTIFIER);
        write(&former_data.join("drivetube.sqlite3"), "database");
        write(&former_data.join("drivetube.sqlite3-wal"), "wal");
        write(&former_data.join(".window-state.json"), "{}");
        write(&former_local.join("thumbnails").join("a.img"), "thumbnail");

        move_data_in(roaming.path(), local.path(), IDENTIFIER, DATABASE_FILE).unwrap();

        let data = roaming.path().join(IDENTIFIER);
        assert_eq!(read(&data.join("homereel.sqlite3")), "database");
        assert_eq!(read(&data.join("homereel.sqlite3-wal")), "wal");
        assert_eq!(read(&data.join(".window-state.json")), "{}");
        let thumbnail = local
            .path()
            .join(IDENTIFIER)
            .join("thumbnails")
            .join("a.img");
        assert_eq!(read(&thumbnail), "thumbnail");
        assert!(!former_data.exists());
        assert!(!former_local.exists());
    }

    #[test]
    fn creates_nothing_when_there_is_no_former_data() {
        let roaming = tempfile::tempdir().unwrap();
        let local = tempfile::tempdir().unwrap();

        move_data_in(roaming.path(), local.path(), IDENTIFIER, DATABASE_FILE).unwrap();

        assert!(!roaming.path().join(IDENTIFIER).exists());
        assert!(!local.path().join(IDENTIFIER).exists());
    }

    #[test]
    fn merges_into_an_existing_folder_without_replacing_its_entries() {
        let base = tempfile::tempdir().unwrap();
        let former = base.path().join("former");
        let current = base.path().join("current");
        write(&former.join("thumbnails").join("a.img"), "thumbnail");
        write(&former.join("logs").join("former.log"), "former log");
        write(&current.join("logs").join("current.log"), "current log");

        move_dir(&former, &current).unwrap();

        assert_eq!(read(&current.join("thumbnails").join("a.img")), "thumbnail");
        assert_eq!(
            read(&current.join("logs").join("current.log")),
            "current log"
        );
        assert!(!current.join("logs").join("former.log").exists());
        assert_eq!(read(&former.join("logs").join("former.log")), "former log");
    }

    #[test]
    fn finishes_a_database_rename_that_stopped_after_the_wal() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("drivetube.sqlite3"), "database");
        write(&dir.path().join("homereel.sqlite3-wal"), "wal");

        rename_database(dir.path(), DATABASE_FILE).unwrap();

        assert_eq!(read(&dir.path().join("homereel.sqlite3")), "database");
        assert_eq!(read(&dir.path().join("homereel.sqlite3-wal")), "wal");
        assert!(!dir.path().join("drivetube.sqlite3").exists());
    }

    #[test]
    fn keeps_an_existing_database_and_leaves_the_former_one_alone() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("drivetube.sqlite3"), "former");
        write(&dir.path().join("homereel.sqlite3"), "current");

        rename_database(dir.path(), DATABASE_FILE).unwrap();

        assert_eq!(read(&dir.path().join("homereel.sqlite3")), "current");
        assert_eq!(read(&dir.path().join("drivetube.sqlite3")), "former");
    }

    #[test]
    fn moves_secrets_to_the_current_service_and_deletes_the_former_ones() {
        let former = MemorySecretStore::with("google-refresh-token", "refresh");
        former.set("gemini-api-key", "former key").unwrap();
        let current = MemorySecretStore::with("gemini-api-key", "current key");

        move_secrets(
            &former,
            &current,
            &["google-refresh-token", "gemini-api-key"],
        )
        .unwrap();

        let get = |store: &MemorySecretStore, key| store.get(key).unwrap();
        assert_eq!(
            get(&current, "google-refresh-token").as_deref(),
            Some("refresh")
        );
        assert_eq!(
            get(&current, "gemini-api-key").as_deref(),
            Some("current key")
        );
        assert_eq!(get(&former, "google-refresh-token"), None);
        assert_eq!(get(&former, "gemini-api-key"), None);
    }
}
