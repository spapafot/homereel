use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::auth::token_manager::TokenManager;
use crate::auth::token_store::{KeyringSecretStore, SecretStore, init_platform_store};
use crate::config::{self, DATABASE_FILE, DRIVE_API_BASE, GEMINI_KEY_NAME, KEYRING_SERVICE};
use crate::db::{Database, media_info, recorded_dates, settings, snapshots, user_state};
use crate::drive::client::DriveClient;
use crate::drive::models::{DriveFile, DriveUser};
use crate::error::{AppError, AppResult};
use crate::library::KnownFiles;
use crate::library::date_scan::DateScan;
use crate::library::drive_tree::DriveTreeCache;
use crate::library::folder_names::FolderNames;
use crate::library::local_files::{LocalLibrary, LocalVideo, is_local_folder_id, is_local_id};
use crate::library::local_listings::LocalListings;
use crate::library::sources::{self, DriveFolderChoice};
use crate::library::video_summary::{VideoFile, VideoSummary};
use crate::protocol::media_proxy::MediaProxy;
use crate::protocol::thumbs::ThumbnailCache;
use crate::titles::gemini::{self, GeminiClient};
use crate::titles::queue::TitleQueue;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

pub struct AppState {
    pub tokens: Arc<TokenManager>,
    pub drive: Arc<DriveClient>,
    pub local: Arc<LocalLibrary>,
    pub media_proxy: MediaProxy,
    pub thumbnails: ThumbnailCache,
    pub known_files: KnownFiles,
    pub db: Database,
    pub secrets: Arc<dyn SecretStore>,
    pub gemini: GeminiClient,
    pub titles: TitleQueue,
    pub date_scan: DateScan,
    pub local_listings: LocalListings,
    folder_names: FolderNames,
    drive_tree: DriveTreeCache,
    account: tokio::sync::Mutex<Option<DriveUser>>,
}

impl AppState {
    pub async fn initialize(cache_dir: &Path, data_dir: &Path) -> AppResult<Self> {
        init_platform_store()?;
        // No overall timeout here: media streams stay open while a video plays. JSON calls set
        // their own per-request timeout.
        let http = reqwest::Client::builder()
            .user_agent(concat!("Homereel/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .build()?;
        let media_http = reqwest::Client::builder()
            .user_agent(concat!("Homereel/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .http1_only()
            .build()?;
        let secrets: Arc<dyn SecretStore> = Arc::new(KeyringSecretStore::new(KEYRING_SERVICE));
        let tokens = Arc::new(TokenManager::new(
            http.clone(),
            config::google_client(),
            secrets.clone(),
        ));
        let drive = Arc::new(DriveClient::new(
            http.clone(),
            media_http,
            DRIVE_API_BASE,
            tokens.clone(),
        ));
        let db = Database::open(&data_dir.join(DATABASE_FILE))?;
        let local = Arc::new(LocalLibrary::default());
        local.set_roots(db.with(|connection| sources::local_folders(connection))?);
        let known_files = KnownFiles::default();
        let media_proxy =
            MediaProxy::start(drive.clone(), known_files.clone(), local.clone()).await?;
        let thumbnails = ThumbnailCache::new(
            cache_dir.join("thumbnails"),
            drive.clone(),
            known_files.clone(),
            media_proxy.clone(),
        );
        let state = Self {
            tokens,
            drive,
            local,
            media_proxy,
            thumbnails,
            known_files,
            db,
            secrets,
            gemini: GeminiClient::new(http, gemini::API_BASE),
            titles: TitleQueue::default(),
            date_scan: DateScan::default(),
            local_listings: LocalListings::default(),
            folder_names: FolderNames::default(),
            drive_tree: DriveTreeCache::default(),
            account: tokio::sync::Mutex::new(None),
        };
        #[cfg(debug_assertions)]
        state.import_dev_gemini_key()?;
        Ok(state)
    }

    /// Copies `GEMINI_API_KEY` from `.env` into Credential Manager once, in development builds.
    #[cfg(debug_assertions)]
    fn import_dev_gemini_key(&self) -> AppResult<()> {
        if self.secrets.get(GEMINI_KEY_NAME)?.is_none()
            && let Some(key) = config::dev_gemini_key()
        {
            self.secrets.set(GEMINI_KEY_NAME, &key)?;
            log::info!("Imported GEMINI_API_KEY from .env into Windows Credential Manager");
        }
        Ok(())
    }

    /// The connected Google account, fetched once per connection.
    pub async fn account(&self) -> AppResult<DriveUser> {
        let mut account = self.account.lock().await;
        if let Some(user) = account.as_ref() {
            return Ok(user.clone());
        }
        let user = self.drive.about().await?;
        *account = Some(user.clone());
        Ok(user)
    }

    pub async fn forget_account(&self) {
        self.account.lock().await.take();
        self.known_files.clear();
        self.folder_names.clear().await;
        self.drive_tree.clear().await;
    }

    /// Whether the library has anywhere to list videos from.
    pub fn has_sources(&self) -> AppResult<bool> {
        Ok(self.local.has_roots() || self.tokens.is_connected()?)
    }

    pub fn drive_folder(&self) -> AppResult<Option<DriveFolderChoice>> {
        self.db.with(|connection| sources::drive_folder(connection))
    }

    pub async fn set_drive_folder(&self, folder: Option<DriveFolderChoice>) -> AppResult<()> {
        self.db
            .with(|connection| sources::set_drive_folder(connection, folder.as_ref()))?;
        self.drive_tree.clear().await;
        Ok(())
    }

    pub fn set_local_folders(&self, folders: Vec<PathBuf>) -> AppResult<()> {
        self.db
            .with(|connection| sources::set_local_folders(connection, &folders))?;
        self.local.set_roots(folders);
        Ok(())
    }

    /// Every video in the library, or in one of its folders: those in Drive (inside the folder
    /// chosen in Settings, if any) and those in the folders on this computer. Registers them for
    /// playback and thumbnails.
    pub async fn library_videos(&self, folder_id: Option<&str>) -> AppResult<Vec<VideoFile>> {
        let local_folder = folder_id.filter(|id| is_local_folder_id(id));
        let drive_folder = folder_id.filter(|id| !is_local_folder_id(id));
        let (drive, local) = tokio::join!(
            async {
                match local_folder {
                    Some(_) => Ok(Vec::new()),
                    None => self.drive_videos(drive_folder).await,
                }
            },
            async {
                match drive_folder {
                    Some(_) => Ok(Vec::new()),
                    None => self.local_videos(local_folder).await,
                }
            },
        );
        let mut videos = match drive {
            Ok(videos) => videos,
            // With folders on this computer the library still works while Drive is unreachable.
            Err(error) if folder_id.is_none() && self.local.has_roots() => {
                log::warn!("Listing Google Drive failed: {error}");
                Vec::new()
            }
            Err(error) => return Err(error),
        };
        videos.extend(local?);
        self.register(&videos)?;
        Ok(videos)
    }

    async fn drive_videos(&self, folder_id: Option<&str>) -> AppResult<Vec<VideoFile>> {
        if !self.tokens.is_connected()? {
            return if folder_id.is_none() && self.local.has_roots() {
                Ok(Vec::new())
            } else {
                Err(AppError::NotConnected)
            };
        }
        let files = match (folder_id, self.drive_folder()?) {
            (Some(folder_id), _) => self.drive.list_all_videos(&[folder_id.to_owned()]).await?,
            (None, None) => self.drive.list_all_videos(&[]).await?,
            (None, Some(root)) => {
                let tree = self.drive_tree.get(&self.drive, &root).await?;
                self.folder_names.insert(&tree.folders).await;
                self.drive.list_all_videos(&tree.folder_ids()).await?
            }
        };
        Ok(self.drive_video_files(files).await)
    }

    async fn local_videos(&self, folder_id: Option<&str>) -> AppResult<Vec<VideoFile>> {
        let mut found = self.local.scan().await;
        if let Some(folder_id) = folder_id {
            found.retain(|video| video.folder_id == folder_id);
        }
        self.local_video_files(found)
    }

    /// Converts listed Drive files, remembering their thumbnail links and looking up the names
    /// of their folders.
    async fn drive_video_files(&self, files: Vec<DriveFile>) -> Vec<VideoFile> {
        self.known_files.remember(&files);
        let folder_ids: Vec<String> = files
            .iter()
            .filter_map(|file| file.parents.first().cloned())
            .collect();
        let names = self.folder_names.resolve(&self.drive, &folder_ids).await;
        files
            .into_iter()
            .map(|file| VideoFile::from_drive(file, &names))
            .collect()
    }

    /// Adds what ffmpeg has read from each file so far.
    fn local_video_files(&self, found: Vec<LocalVideo>) -> AppResult<Vec<VideoFile>> {
        let ids: Vec<String> = found.iter().map(|video| video.id.clone()).collect();
        let records = self
            .db
            .with(|connection| media_info::records(connection, &ids))?;
        Ok(found
            .into_iter()
            .map(|video| {
                let record = records.get(&video.id).copied();
                VideoFile::from_local(video, record.as_ref())
            })
            .collect())
    }

    /// Remembers listed videos for this session and refreshes their stored snapshots.
    fn register(&self, videos: &[VideoFile]) -> AppResult<()> {
        self.known_files.remember_videos(videos);
        self.db
            .with(|connection| snapshots::refresh_existing(connection, videos))
    }

    /// Registers Drive files for playback and thumbnails, attaches folder names and merges in
    /// Homereel's local state.
    pub async fn summarize(&self, files: Vec<DriveFile>) -> AppResult<Vec<VideoSummary>> {
        let videos = self.drive_video_files(files).await;
        self.register(&videos)?;
        self.with_local_state(videos)
    }

    pub fn with_local_state(&self, videos: Vec<VideoFile>) -> AppResult<Vec<VideoSummary>> {
        let ids: Vec<String> = videos.iter().map(|video| video.id.clone()).collect();
        let (states, mut dates) = self.db.with(|connection| {
            Ok((
                user_state::load(connection, &ids)?,
                recorded_dates::records(connection, &ids)?,
            ))
        })?;
        Ok(videos
            .into_iter()
            .map(|video| {
                let state = states.get(&video.id);
                let recorded_date = dates
                    .remove(&video.id)
                    .and_then(|record| record.recorded_date);
                VideoSummary::new(video, state, recorded_date)
            })
            .collect())
    }

    /// Metadata for one video, from this session's listings or else from its source.
    pub async fn video_file(&self, drive_id: &str) -> AppResult<VideoFile> {
        if let Some(video) = self.known_files.video(drive_id) {
            return Ok(video);
        }
        if is_local_id(drive_id) {
            return self.local_video(drive_id).await;
        }
        let file = self.drive.file(drive_id).await?;
        let mut summaries = self.summarize(vec![file]).await?;
        Ok(summaries.remove(0).file)
    }

    /// A video on this computer, checked against its file so a deleted or edited one shows as
    /// unavailable.
    pub async fn local_video(&self, id: &str) -> AppResult<VideoFile> {
        let found = self.local.current(id).await.ok_or(AppError::NotFound)?;
        let mut videos = self.local_video_files(vec![found])?;
        self.register(&videos)?;
        Ok(videos.remove(0))
    }

    pub async fn video_summary(&self, drive_id: &str) -> AppResult<VideoSummary> {
        let video = self.video_file(drive_id).await?;
        let mut summaries = self.with_local_state(vec![video])?;
        Ok(summaries.remove(0))
    }

    /// Stores a display snapshot before the video gets local state, so lists such as
    /// Favorites can show it without asking Drive.
    pub async fn ensure_snapshot(&self, drive_id: &str) -> AppResult<()> {
        if self
            .db
            .with(|connection| snapshots::exists(connection, drive_id))?
        {
            return Ok(());
        }
        let video = self.video_file(drive_id).await?;
        self.db
            .with(|connection| snapshots::upsert(connection, std::slice::from_ref(&video)))
    }

    /// Summaries for stored IDs in the given order, using snapshots.
    pub fn summaries_for_ids(&self, ids: &[String]) -> AppResult<Vec<VideoSummary>> {
        let mut snapshots = self
            .db
            .with(|connection| snapshots::load(connection, ids))?;
        let videos: Vec<VideoFile> = ids.iter().filter_map(|id| snapshots.remove(id)).collect();
        self.known_files.remember_videos(&videos);
        self.with_local_state(videos)
    }

    pub fn gemini_key(&self) -> AppResult<Option<String>> {
        self.secrets.get(GEMINI_KEY_NAME)
    }

    pub fn gemini_model(&self) -> AppResult<String> {
        Ok(self
            .db
            .with(|connection| settings::get(connection, settings::GEMINI_MODEL))?
            .unwrap_or_else(|| gemini::DEFAULT_MODEL.to_owned()))
    }

    /// The owner's notes for Gemini from Settings, if any.
    pub fn gemini_instructions(&self) -> AppResult<Option<String>> {
        Ok(self
            .db
            .with(|connection| settings::get(connection, settings::GEMINI_INSTRUCTIONS))?
            .filter(|instructions| !instructions.trim().is_empty()))
    }

    pub fn require_gemini_key(&self) -> AppResult<String> {
        self.gemini_key()?
            .ok_or_else(|| AppError::Forbidden("Add a Gemini API key in Settings first.".into()))
    }
}
