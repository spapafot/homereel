//! Videos in folders on this computer. Like Drive's, they are listed live: each listing walks the
//! chosen folders again, and nothing is indexed ahead of time.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// Local video and folder IDs start with these, so they never clash with Drive IDs and stay
/// valid wherever a Drive ID is (see `drive::is_valid_drive_id`).
const VIDEO_ID_PREFIX: &str = "local-";
const FOLDER_ID_PREFIX: &str = "localdir-";
/// Hex digits of the SHA-256 kept in an ID.
const ID_HASH_LENGTH: usize = 24;
/// Stops loops through odd links and runaway walks of a whole disk.
const MAX_DEPTH: usize = 32;
const MAX_ENTRIES: usize = 200_000;
/// A video not found by the last scan triggers a new one at most this often, so a list of
/// deleted videos does not walk the folders once per thumbnail.
const MISSING_RESCAN_INTERVAL: Duration = Duration::from_secs(10);

/// Extensions listed as videos, with the type the media proxy serves them as. `.ts` is left out
/// because it is far more often TypeScript than MPEG transport stream.
const VIDEO_TYPES: [(&str, &str); 14] = [
    ("mp4", "video/mp4"),
    ("m4v", "video/x-m4v"),
    ("mov", "video/quicktime"),
    ("webm", "video/webm"),
    ("mkv", "video/x-matroska"),
    ("avi", "video/x-msvideo"),
    ("wmv", "video/x-ms-wmv"),
    ("mpg", "video/mpeg"),
    ("mpeg", "video/mpeg"),
    ("3gp", "video/3gpp"),
    ("mts", "video/mp2t"),
    ("m2ts", "video/mp2t"),
    ("ogv", "video/ogg"),
    ("flv", "video/x-flv"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct LocalVideo {
    pub id: String,
    pub path: PathBuf,
    pub file_name: String,
    pub mime_type: &'static str,
    pub size_bytes: u64,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub folder_id: String,
    pub folder_name: String,
}

pub fn is_local_id(id: &str) -> bool {
    id.starts_with(VIDEO_ID_PREFIX)
}

pub fn is_local_folder_id(id: &str) -> bool {
    id.starts_with(FOLDER_ID_PREFIX)
}

/// The type a video file is served as, or `None` for files that are not videos.
pub fn mime_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    VIDEO_TYPES
        .iter()
        .find(|(known, _)| *known == extension)
        .map(|(_, mime_type)| *mime_type)
}

/// The name a folder is shown with: its own name, or the drive for a drive's root (`D:`).
pub fn folder_display_name(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            dir.to_string_lossy()
                .trim_end_matches(['\\', '/'])
                .to_owned()
        })
}

/// Every video under `roots`, files before subfolders and in name order. A file found twice (an
/// exact copy, or a folder chosen inside another) is listed once.
pub fn scan(roots: &[PathBuf]) -> Vec<LocalVideo> {
    let mut videos = Vec::new();
    let mut seen = HashSet::new();
    let mut budget = MAX_ENTRIES;
    for root in roots {
        walk(root, 0, &mut budget, &mut |video| {
            if seen.insert(video.id.clone()) {
                videos.push(video);
            }
        });
    }
    videos
}

fn walk(dir: &Path, depth: usize, budget: &mut usize, found: &mut impl FnMut(LocalVideo)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<std::fs::DirEntry> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.file_name().to_ascii_lowercase());
    let mut subfolders = Vec::new();
    for entry in entries {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        // On Windows both come from the directory listing itself, without opening the file.
        let (Ok(file_type), Ok(metadata)) = (entry.file_type(), entry.metadata()) else {
            continue;
        };
        if file_type.is_symlink() || is_hidden(&entry.file_name(), &metadata) {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            subfolders.push(path);
        } else if let Some(video) = video_at(path, &metadata) {
            found(video);
        }
    }
    if depth < MAX_DEPTH {
        for subfolder in subfolders {
            walk(&subfolder, depth + 1, budget, found);
        }
    }
}

/// Dot files (including macOS `._` copies) and Windows hidden or system entries such as
/// `$RECYCLE.BIN`.
fn is_hidden(name: &OsStr, metadata: &Metadata) -> bool {
    if name.to_string_lossy().starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
        metadata.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

/// Reads one file again, for checking that a known video is still there unchanged.
pub fn read(path: &Path) -> Option<LocalVideo> {
    let metadata = std::fs::metadata(path).ok()?;
    metadata
        .is_file()
        .then(|| video_at(path.to_owned(), &metadata))
        .flatten()
}

fn video_at(path: PathBuf, metadata: &Metadata) -> Option<LocalVideo> {
    let mime_type = mime_type(&path)?;
    let folder = path.parent()?.to_owned();
    let modified = metadata.modified().ok();
    Some(LocalVideo {
        id: video_id(metadata.len(), modified, &path),
        file_name: path.file_name()?.to_string_lossy().into_owned(),
        mime_type,
        size_bytes: metadata.len(),
        modified,
        created: metadata.created().ok(),
        folder_id: folder_id(&folder),
        folder_name: folder_display_name(&folder),
        path,
    })
}

/// Comes from the file's size and modification time rather than its path, so favorites,
/// history and titles follow a video that is renamed or moved, and copies of one file count once.
/// Editing a video makes it a new one, as re-uploading does on YouTube.
fn video_id(size: u64, modified: Option<SystemTime>, path: &Path) -> String {
    let key = match modified.and_then(|time| time.duration_since(UNIX_EPOCH).ok()) {
        // Whole seconds, because copying to FAT or exFAT drives rounds finer timestamps.
        Some(since_epoch) => format!("{size}:{}", since_epoch.as_secs()),
        None => format!("path:{}", normalized_path(path)),
    };
    format!("{VIDEO_ID_PREFIX}{}", short_hash(&key))
}

fn folder_id(dir: &Path) -> String {
    format!("{FOLDER_ID_PREFIX}{}", short_hash(&normalized_path(dir)))
}

/// Windows paths ignore case, so `C:\Videos` and `c:\videos` are one folder.
fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

fn short_hash(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    hex.truncate(ID_HASH_LENGTH);
    hex
}

/// UTC time as RFC 3339 with milliseconds, the shape Drive uses for its times.
pub fn rfc3339(time: SystemTime) -> Option<String> {
    let since_epoch = time.duration_since(UNIX_EPOCH).ok()?;
    let seconds = since_epoch.as_secs();
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let second_of_day = seconds % 86_400;
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        second_of_day / 3600,
        second_of_day % 3600 / 60,
        second_of_day % 60,
        since_epoch.subsec_millis()
    ))
}

/// Year, month and day of a day number counted from 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The chosen folders and what the last scan found in them. Playback, thumbnails and ffmpeg find
/// a video's file here.
#[derive(Default)]
pub struct LocalLibrary {
    roots: RwLock<Vec<PathBuf>>,
    videos: RwLock<HashMap<String, LocalVideo>>,
    /// One scan at a time for videos that are missing, and when the last one finished.
    missing_scan: tokio::sync::Mutex<Option<Instant>>,
}

impl LocalLibrary {
    pub fn set_roots(&self, roots: Vec<PathBuf>) {
        *self.roots.write().unwrap_or_else(|p| p.into_inner()) = roots;
        self.videos
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
    }

    pub fn roots(&self) -> Vec<PathBuf> {
        self.roots.read().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn has_roots(&self) -> bool {
        !self
            .roots
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .is_empty()
    }

    /// Walks every chosen folder, off the async runtime.
    pub async fn scan(&self) -> Vec<LocalVideo> {
        let roots = self.roots();
        if roots.is_empty() {
            return Vec::new();
        }
        let videos = tokio::task::spawn_blocking(move || scan(&roots))
            .await
            .unwrap_or_default();
        *self.videos.write().unwrap_or_else(|p| p.into_inner()) = videos
            .iter()
            .map(|video| (video.id.clone(), video.clone()))
            .collect();
        videos
    }

    /// The video as last scanned, or after a new scan when it was not found, for example when a
    /// video in History is opened before anything was listed.
    pub async fn find(&self, id: &str) -> Option<LocalVideo> {
        if let Some(video) = self.known(id) {
            return Some(video);
        }
        let mut last_scan = self.missing_scan.lock().await;
        if let Some(video) = self.known(id) {
            return Some(video);
        }
        if last_scan.is_some_and(|at| at.elapsed() < MISSING_RESCAN_INTERVAL) {
            return None;
        }
        self.scan().await;
        *last_scan = Some(Instant::now());
        self.known(id)
    }

    /// The video only if its file is still there unchanged, scanning again when it moved.
    pub async fn current(&self, id: &str) -> Option<LocalVideo> {
        let is_current = |video: &LocalVideo| read(&video.path).is_some_and(|file| file.id == id);
        if let Some(video) = self.find(id).await
            && is_current(&video)
        {
            return Some(video);
        }
        self.scan().await;
        self.known(id).filter(is_current)
    }

    fn known(&self, id: &str) -> Option<LocalVideo> {
        self.videos
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, bytes: usize) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![0u8; bytes]).unwrap();
    }

    fn names(videos: &[LocalVideo]) -> Vec<&str> {
        videos
            .iter()
            .map(|video| video.file_name.as_str())
            .collect()
    }

    #[test]
    fn finds_videos_in_subfolders_and_skips_other_and_hidden_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("VIDS");
        write(&root.join("b.MOV"), 10);
        write(&root.join("a.mp4"), 11);
        write(&root.join("notes.txt"), 12);
        write(&root.join("code.ts"), 13);
        write(&root.join("._a.mp4"), 14);
        write(&root.join(".cache").join("c.mp4"), 15);
        write(&root.join("Trip").join("d.mkv"), 16);

        let videos = scan(std::slice::from_ref(&root));

        assert_eq!(names(&videos), ["a.mp4", "b.MOV", "d.mkv"]);
        assert_eq!(videos[1].mime_type, "video/quicktime");
        assert_eq!(videos[0].folder_name, "VIDS");
        assert_eq!(videos[2].folder_name, "Trip");
        assert_ne!(videos[0].folder_id, videos[2].folder_id);
        assert!(videos.iter().all(|video| is_local_id(&video.id)));
        assert!(is_local_folder_id(&videos[0].folder_id));
        assert!(!is_local_id(&videos[0].folder_id));
    }

    #[test]
    fn ids_follow_renamed_files_and_copies_count_once() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("IMG_0001.MOV");
        write(&first, 100);
        let before = scan(&[dir.path().to_owned()]);

        let copy = dir.path().join("Backup").join("IMG_0001.MOV");
        std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
        std::fs::copy(&first, &copy).unwrap();
        let modified = std::fs::metadata(&first).unwrap().modified().unwrap();
        std::fs::File::options()
            .write(true)
            .open(&copy)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        let renamed = dir.path().join("Beach.MOV");
        std::fs::rename(&first, &renamed).unwrap();
        let after = scan(&[dir.path().to_owned()]);

        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, before[0].id);
        assert_eq!(after[0].file_name, "Beach.MOV");
    }

    #[test]
    fn nested_roots_list_each_video_once() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("Trip").join("a.mp4"), 5);

        let videos = scan(&[dir.path().to_owned(), dir.path().join("Trip")]);

        assert_eq!(videos.len(), 1);
    }

    #[test]
    fn missing_roots_list_nothing() {
        assert!(scan(&[PathBuf::from(r"Z:\does\not\exist")]).is_empty());
    }

    #[test]
    fn folder_ids_ignore_case_and_slashes() {
        assert_eq!(
            folder_id(Path::new(r"C:\Videos\Trip")),
            folder_id(Path::new("c:/videos/TRIP"))
        );
        assert_eq!(folder_display_name(Path::new(r"D:\")), "D:");
        assert_eq!(folder_display_name(Path::new(r"D:\Clips")), "Clips");
    }

    #[test]
    fn formats_times_like_drive() {
        let time = UNIX_EPOCH + Duration::from_millis(1_718_216_095_123);

        assert_eq!(rfc3339(time).as_deref(), Some("2024-06-12T18:14:55.123Z"));
        assert_eq!(
            rfc3339(UNIX_EPOCH).as_deref(),
            Some("1970-01-01T00:00:00.000Z")
        );
        assert_eq!(
            rfc3339(UNIX_EPOCH + Duration::from_secs(951_782_400)).as_deref(),
            Some("2000-02-29T00:00:00.000Z")
        );
    }

    #[tokio::test]
    async fn finds_unlisted_videos_by_scanning_and_drops_changed_ones() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.mp4");
        write(&path, 10);
        let library = LocalLibrary::default();
        library.set_roots(vec![dir.path().to_owned()]);
        let id = scan(&library.roots())[0].id.clone();

        assert_eq!(library.find(&id).await.unwrap().path, path);
        assert!(library.current(&id).await.is_some());

        write(&path, 20);
        assert!(library.current(&id).await.is_none());
    }
}
