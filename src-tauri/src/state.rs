use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::auth::token_manager::TokenManager;
use crate::auth::token_store::{KeyringSecretStore, SecretStore, init_platform_store};
use crate::config::{self, DATABASE_FILE, DRIVE_API_BASE, GEMINI_KEY_NAME, KEYRING_SERVICE};
use crate::db::{Database, recorded_dates, settings, snapshots, user_state};
use crate::drive::client::DriveClient;
use crate::drive::models::{DriveFile, DriveUser};
use crate::error::{AppError, AppResult};
use crate::library::KnownFiles;
use crate::library::date_scan::DateScan;
use crate::library::folder_names::FolderNames;
use crate::library::local_listings::LocalListings;
use crate::library::video_summary::{VideoFile, VideoSummary};
use crate::protocol::media_proxy::MediaProxy;
use crate::protocol::thumbs::ThumbnailCache;
use crate::titles::gemini::{self, GeminiClient};
use crate::titles::queue::TitleQueue;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

pub struct AppState {
    pub tokens: Arc<TokenManager>,
    pub drive: Arc<DriveClient>,
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
    account: tokio::sync::Mutex<Option<DriveUser>>,
}

impl AppState {
    pub async fn initialize(cache_dir: &Path, data_dir: &Path) -> AppResult<Self> {
        init_platform_store()?;
        // No overall timeout here: media streams stay open while a video plays. JSON calls set
        // their own per-request timeout.
        let http = reqwest::Client::builder()
            .user_agent(concat!("DriveTube/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .build()?;
        let media_http = reqwest::Client::builder()
            .user_agent(concat!("DriveTube/", env!("CARGO_PKG_VERSION")))
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
        let known_files = KnownFiles::default();
        let media_proxy = MediaProxy::start(drive.clone(), known_files.clone()).await?;
        let thumbnails = ThumbnailCache::new(
            cache_dir.join("thumbnails"),
            drive.clone(),
            known_files.clone(),
            media_proxy.clone(),
        );
        let state = Self {
            tokens,
            drive,
            media_proxy,
            thumbnails,
            known_files,
            db: Database::open(&data_dir.join(DATABASE_FILE))?,
            secrets,
            gemini: GeminiClient::new(http, gemini::API_BASE),
            titles: TitleQueue::default(),
            date_scan: DateScan::default(),
            local_listings: LocalListings::default(),
            folder_names: FolderNames::default(),
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
    }

    /// Registers listed files for playback and thumbnails, attaches folder names and merges in
    /// DriveTube's local state.
    pub async fn summarize(&self, files: Vec<DriveFile>) -> AppResult<Vec<VideoSummary>> {
        self.known_files.remember(&files);
        let folder_ids: Vec<String> = files
            .iter()
            .filter_map(|file| file.parents.first().cloned())
            .collect();
        let names = self.folder_names.resolve(&self.drive, &folder_ids).await;
        let videos: Vec<VideoFile> = files
            .into_iter()
            .map(|file| VideoFile::from_drive(file, &names))
            .collect();
        self.known_files.remember_videos(&videos);
        self.db
            .with(|connection| snapshots::refresh_existing(connection, &videos))?;
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

    /// Metadata for one video, from this session's listings or else from Drive.
    pub async fn video_file(&self, drive_id: &str) -> AppResult<VideoFile> {
        if let Some(video) = self.known_files.video(drive_id) {
            return Ok(video);
        }
        let file = self.drive.file(drive_id).await?;
        let mut summaries = self.summarize(vec![file]).await?;
        Ok(summaries.remove(0).file)
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

    pub async fn folder_names(&self, folder_ids: &[String]) -> HashMap<String, String> {
        self.folder_names.resolve(&self.drive, folder_ids).await
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
