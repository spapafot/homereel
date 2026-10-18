use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::future::join_all;
use reqwest::StatusCode;
use tokio::sync::Semaphore;

use crate::drive::client::DriveClient;
use crate::drive::is_valid_drive_id;
use crate::error::{AppError, AppResult};
use crate::ffmpeg;
use crate::library::KnownFiles;
use crate::library::local_files::is_local_id;
use crate::protocol::media_proxy::MediaProxy;

/// Long edge in pixels. Large enough for grid cards on HiDPI screens.
pub const THUMBNAIL_SIZE: u32 = 800;
/// Where to take a frame-based thumbnail: a tenth in, past black intro frames.
const FRAME_THUMBNAIL_FRACTION: f64 = 0.1;
const FRAME_THUMBNAIL_MAX_SECONDS: f64 = 30.0;
const FRAME_THUMBNAIL_DEFAULT_SECONDS: f64 = 2.0;
const DOWNLOAD_CONCURRENCY: usize = 6;

/// Numbers temporary files, so two writes of the same thumbnail never share one.
static NEXT_WRITE: AtomicU64 = AtomicU64::new(0);

pub struct Thumbnail {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
}

impl Thumbnail {
    fn new(bytes: Vec<u8>) -> Self {
        let content_type = sniff_image_type(&bytes);
        Self {
            bytes,
            content_type,
        }
    }
}

/// Serves thumbnails for the `thumb` URI scheme. `thumbnailLink` needs a credentialed request and
/// expires within hours, so images are fetched in Rust and cached on disk. Videos on this computer,
/// and those Drive could not process (often iPhone HEVC `.MOV`), get an extracted frame instead.
pub struct ThumbnailCache {
    dir: PathBuf,
    drive: Arc<DriveClient>,
    known_files: KnownFiles,
    media: MediaProxy,
    permits: Semaphore,
}

impl ThumbnailCache {
    pub fn new(
        dir: PathBuf,
        drive: Arc<DriveClient>,
        known_files: KnownFiles,
        media: MediaProxy,
    ) -> Self {
        Self {
            dir,
            drive,
            known_files,
            media,
            permits: Semaphore::new(DOWNLOAD_CONCURRENCY),
        }
    }

    pub async fn load(&self, file_id: &str) -> AppResult<Thumbnail> {
        if !is_valid_drive_id(file_id) {
            return Err(AppError::NotFound);
        }
        let path = self.dir.join(format!("{file_id}.img"));
        if let Ok(bytes) = tokio::fs::read(&path).await {
            return Ok(Thumbnail::new(bytes));
        }

        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| AppError::internal("Thumbnail downloads are shutting down."))?;
        // Another request for the same image may have filled the cache while this one waited.
        if let Ok(bytes) = tokio::fs::read(&path).await {
            return Ok(Thumbnail::new(bytes));
        }

        let bytes = self.download(file_id).await?;
        self.store(file_id, &bytes).await?;
        Ok(Thumbnail::new(bytes))
    }

    /// Replaces the thumbnail with a sharp frame from the video itself: the frame at `at_ms`, or
    /// else the most detailed of frames spread over the video. For Drive thumbnails that are too
    /// small, and frames that came out dark.
    pub async fn regenerate(
        &self,
        ffmpeg: &Path,
        file_id: &str,
        duration_ms: Option<u64>,
        at_ms: Option<u64>,
    ) -> AppResult<()> {
        if !is_valid_drive_id(file_id) {
            return Err(AppError::NotFound);
        }
        let url = self.media.ffmpeg_input(file_id).await?;
        let info = ffmpeg::probe(ffmpeg, &url).await;
        let duration_ms = duration_ms.or(info
            .duration_seconds
            .map(|seconds| (seconds * 1000.0) as u64));
        let bytes = match at_ms {
            Some(at_ms) => {
                let at_seconds = ffmpeg::capture_position(at_ms, duration_ms) as f64 / 1000.0;
                ffmpeg::extract_preview(ffmpeg, &url, at_seconds, info.is_hdr)
                    .await
                    .ok()
            }
            None => most_detailed_frame(ffmpeg, &url, duration_ms, info.is_hdr).await,
        };
        let bytes = bytes.ok_or_else(|| {
            AppError::internal("No frame could be read from this video. Try another moment.")
        })?;
        self.store(file_id, &bytes).await
    }

    /// Writes through a temporary file, so the protocol never serves half an image.
    async fn store(&self, file_id: &str, bytes: &[u8]) -> AppResult<()> {
        tokio::fs::create_dir_all(&self.dir).await?;
        let write = NEXT_WRITE.fetch_add(1, Ordering::Relaxed);
        let partial = self.dir.join(format!("{file_id}.{write}.part"));
        tokio::fs::write(&partial, bytes).await?;
        tokio::fs::rename(&partial, self.dir.join(format!("{file_id}.img"))).await?;
        Ok(())
    }

    async fn download(&self, file_id: &str) -> AppResult<Vec<u8>> {
        if is_local_id(file_id) {
            let duration_ms = self
                .known_files
                .video(file_id)
                .and_then(|video| video.duration_ms);
            return self.extract_thumbnail(file_id, duration_ms).await;
        }
        if let Some(link) = self.known_files.thumbnail_link(file_id)
            && let Some(bytes) = self.fetch(&link).await?
        {
            return Ok(bytes);
        }
        // The cached link was missing or expired, so ask Drive for a fresh one.
        let file = self.drive.file(file_id).await?;
        self.known_files.remember(std::slice::from_ref(&file));
        if let Some(link) = file.thumbnail_link.as_deref()
            && let Some(bytes) = self.fetch(link).await?
        {
            return Ok(bytes);
        }
        let duration_ms = file
            .video_media_metadata
            .as_ref()
            .and_then(|metadata| metadata.duration_millis);
        self.extract_thumbnail(file_id, duration_ms).await
    }

    /// Takes a frame from the video, for videos without a Drive thumbnail.
    async fn extract_thumbnail(
        &self,
        file_id: &str,
        duration_ms: Option<u64>,
    ) -> AppResult<Vec<u8>> {
        let ffmpeg = ffmpeg::locate_ffmpeg().ok_or(AppError::NotFound)?;
        let url = self.media.ffmpeg_input(file_id).await?;
        let info = ffmpeg::probe(&ffmpeg, &url).await;
        let hdr = info.is_hdr;
        let at_seconds = duration_ms
            .map(|ms| ms as f64 / 1000.0)
            .or(info.duration_seconds)
            .map(|seconds| (seconds * FRAME_THUMBNAIL_FRACTION).min(FRAME_THUMBNAIL_MAX_SECONDS))
            .unwrap_or(FRAME_THUMBNAIL_DEFAULT_SECONDS);
        if let Ok(frame) = ffmpeg::extract_preview(&ffmpeg, &url, at_seconds, hdr).await {
            return Ok(frame);
        }
        // Very short clips can end before the chosen moment; the first frame always exists.
        ffmpeg::extract_preview(&ffmpeg, &url, 0.0, hdr)
            .await
            .map_err(|_| AppError::NotFound)
    }

    /// Returns `None` when Google rejects an expired link.
    async fn fetch(&self, link: &str) -> AppResult<Option<Vec<u8>>> {
        let response = self
            .drive
            .fetch_authorized(&sized_thumbnail_link(link, THUMBNAIL_SIZE))
            .await?;
        match response.status() {
            status if status.is_success() => Ok(Some(response.bytes().await?.to_vec())),
            StatusCode::FORBIDDEN | StatusCode::NOT_FOUND | StatusCode::GONE => Ok(None),
            status => Err(AppError::internal(format!(
                "Thumbnail request failed ({status})"
            ))),
        }
    }
}

/// Takes previews spread over the video and keeps the largest JPEG. Size stands in for detail:
/// black, blank and motion-blurred frames compress small.
async fn most_detailed_frame(
    ffmpeg: &Path,
    url: &str,
    duration_ms: Option<u64>,
    hdr: bool,
) -> Option<Vec<u8>> {
    let timestamps: Vec<f64> = match duration_ms {
        Some(ms) if ms > 0 => ffmpeg::FRAME_FRACTIONS
            .iter()
            .map(|fraction| ms as f64 / 1000.0 * fraction)
            .collect(),
        _ => vec![FRAME_THUMBNAIL_DEFAULT_SECONDS, 0.0],
    };
    join_all(
        timestamps
            .into_iter()
            .map(|at_seconds| ffmpeg::extract_preview(ffmpeg, url, at_seconds, hdr)),
    )
    .await
    .into_iter()
    .filter_map(Result::ok)
    .max_by_key(Vec::len)
}

/// Builds the `thumb` URI scheme response for one Drive file.
pub async fn respond(cache: &ThumbnailCache, file_id: &str) -> tauri::http::Response<Vec<u8>> {
    let builder = tauri::http::Response::builder();
    let response = match cache.load(file_id).await {
        Ok(thumbnail) => builder
            .header(tauri::http::header::CONTENT_TYPE, thumbnail.content_type)
            // A regenerated thumbnail keeps its URL, so the webview must not keep the old one.
            .header(tauri::http::header::CACHE_CONTROL, "no-cache")
            .body(thumbnail.bytes),
        Err(AppError::NotFound) => builder.status(404).body(Vec::new()),
        Err(error) => {
            log::warn!("Thumbnail for {file_id} failed: {}", error.code());
            builder.status(502).body(Vec::new())
        }
    };
    response.unwrap_or_else(|_| unavailable())
}

pub fn unavailable() -> tauri::http::Response<Vec<u8>> {
    let mut response = tauri::http::Response::new(Vec::new());
    *response.status_mut() = tauri::http::StatusCode::SERVICE_UNAVAILABLE;
    response
}

/// `thumbnailLink` ends with a size suffix such as `=s220`. Replacing it asks for another size.
pub fn sized_thumbnail_link(link: &str, size: u32) -> String {
    match link.rfind("=s") {
        Some(index) if link[index + 2..].chars().all(|c| c.is_ascii_digit()) => {
            format!("{}=s{size}", &link[..index])
        }
        _ => link.to_owned(),
    }
}

fn sniff_image_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/jpeg"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::oauth::OAuthClient;
    use crate::auth::token_manager::{REFRESH_TOKEN_KEY, TokenManager};
    use crate::auth::token_store::testing::MemorySecretStore;
    use crate::drive::models::DriveFile;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn cache_for(
        server: &MockServer,
        dir: PathBuf,
        known_files: KnownFiles,
    ) -> ThumbnailCache {
        let tokens = TokenManager::new(
            reqwest::Client::new(),
            Some(OAuthClient {
                client_id: "client".into(),
                client_secret: "secret".into(),
                auth_endpoint: format!("{}/auth", server.uri()),
                token_endpoint: format!("{}/token", server.uri()),
                revoke_endpoint: format!("{}/revoke", server.uri()),
            }),
            Arc::new(MemorySecretStore::with(REFRESH_TOKEN_KEY, "refresh-1")),
        );
        let drive = DriveClient::new(
            reqwest::Client::new(),
            reqwest::Client::new(),
            &format!("{}/drive/v3", server.uri()),
            Arc::new(tokens),
        );
        let drive = Arc::new(drive);
        let media = MediaProxy::start(drive.clone(), known_files.clone(), Arc::default())
            .await
            .unwrap();
        ThumbnailCache::new(dir, drive, known_files, media)
    }

    async fn mount_token(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "access-1",
                "expires_in": 3599
            })))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn refreshes_an_expired_link_then_serves_from_disk() {
        let server = MockServer::start().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/thumbs/old=s800"))
            .respond_with(ResponseTemplate::new(403))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files/vid1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vid1",
                "name": "a.mp4",
                "mimeType": "video/mp4",
                "thumbnailLink": format!("{}/thumbs/new=s220", server.uri())
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/thumbs/new=s800"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"\x89PNG-image".to_vec()))
            .expect(1)
            .mount(&server)
            .await;
        let known_files = KnownFiles::default();
        let stale: DriveFile = serde_json::from_value(serde_json::json!({
            "id": "vid1",
            "name": "a.mp4",
            "mimeType": "video/mp4",
            "thumbnailLink": format!("{}/thumbs/old=s220", server.uri())
        }))
        .unwrap();
        known_files.remember(&[stale]);
        let dir = tempfile::tempdir().unwrap();
        let cache = cache_for(&server, dir.path().to_path_buf(), known_files).await;

        let first = cache.load("vid1").await.unwrap();
        let second = cache.load("vid1").await.unwrap();

        assert_eq!(first.bytes, b"\x89PNG-image");
        assert_eq!(first.content_type, "image/png");
        assert_eq!(second.bytes, first.bytes);
    }

    #[tokio::test]
    async fn rejects_invalid_ids() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let cache = cache_for(&server, dir.path().to_path_buf(), KnownFiles::default()).await;

        let result = cache.load("../secrets").await;

        assert!(matches!(result, Err(AppError::NotFound)));
    }

    /// Drive often has no thumbnail for iPhone HEVC `.MOV` files; a frame is extracted instead.
    #[tokio::test]
    async fn extracts_a_frame_when_drive_has_no_thumbnail() {
        let ffmpeg = ffmpeg::locate_ffmpeg().expect("run scripts/fetch-ffmpeg.ps1 first");
        let clip_dir = tempfile::tempdir().unwrap();
        let clip = clip_dir.path().join("clip.mp4");
        let status = std::process::Command::new(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi"])
            .args([
                "-i",
                "testsrc2=size=320x240:rate=10",
                "-t",
                "4",
                "-pix_fmt",
                "yuv420p",
            ])
            .args(["-movflags", "+faststart"])
            .arg(&clip)
            .status()
            .unwrap();
        assert!(status.success());

        let server = MockServer::start().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files/mov1"))
            .and(query_param("alt", "media"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "video/quicktime")
                    .set_body_bytes(std::fs::read(&clip).unwrap()),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files/mov1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "mov1", "name": "IMG_0635.MOV", "mimeType": "video/quicktime", "hasThumbnail": false
            })))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let cache = cache_for(&server, dir.path().to_path_buf(), KnownFiles::default()).await;

        let thumbnail = cache.load("mov1").await.unwrap();

        assert_eq!(thumbnail.content_type, "image/jpeg");
        assert!(thumbnail.bytes.starts_with(&[0xFF, 0xD8]));
        assert!(dir.path().join("mov1.img").exists());
    }

    /// A clip that opens on two seconds of black, like many phone videos, then shows a pattern.
    fn black_then_pattern_clip(ffmpeg: &Path, path: &Path) {
        let status = std::process::Command::new(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error"])
            .args(["-f", "lavfi", "-i", "color=c=black:s=320x240:r=10:d=2"])
            .args(["-f", "lavfi", "-i", "testsrc2=s=320x240:r=10:d=4"])
            .args([
                "-filter_complex",
                "[0][1]concat=n=2:v=1",
                "-pix_fmt",
                "yuv420p",
            ])
            .args(["-movflags", "+faststart"])
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn regenerates_from_the_most_detailed_frame_or_the_chosen_moment() {
        let ffmpeg = ffmpeg::locate_ffmpeg().expect("run scripts/fetch-ffmpeg.ps1 first");
        let clip_dir = tempfile::tempdir().unwrap();
        let clip = clip_dir.path().join("clip.mp4");
        black_then_pattern_clip(&ffmpeg, &clip);
        let server = MockServer::start().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files/clip1"))
            .and(query_param("alt", "media"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "video/mp4")
                    .set_body_bytes(std::fs::read(&clip).unwrap()),
            )
            .mount(&server)
            .await;
        // The media proxy only streams files a listing has shown.
        let known_files = KnownFiles::default();
        let listed: DriveFile = serde_json::from_value(serde_json::json!({
            "id": "clip1", "name": "clip.mp4", "mimeType": "video/mp4"
        }))
        .unwrap();
        known_files.remember(&[listed]);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clip1.img"),
            b"\x89PNG-small-drive-thumbnail",
        )
        .unwrap();
        let cache = cache_for(&server, dir.path().to_path_buf(), known_files).await;

        cache
            .regenerate(&ffmpeg, "clip1", Some(6_000), None)
            .await
            .unwrap();
        let best = cache.load("clip1").await.unwrap();
        cache
            .regenerate(&ffmpeg, "clip1", Some(6_000), Some(1_000))
            .await
            .unwrap();
        let chosen = cache.load("clip1").await.unwrap();

        assert_eq!(best.content_type, "image/jpeg");
        assert_eq!(chosen.content_type, "image/jpeg");
        // The pattern is far more detailed than the black opening, where the chosen moment is.
        assert!(best.bytes.len() > chosen.bytes.len() * 2);
        let leftovers = std::fs::read_dir(dir.path())
            .unwrap()
            .filter(|entry| entry.as_ref().unwrap().path().extension() == Some("part".as_ref()))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[tokio::test]
    async fn refuses_to_regenerate_invalid_ids() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let cache = cache_for(&server, dir.path().to_path_buf(), KnownFiles::default()).await;

        let result = cache
            .regenerate(Path::new("ffmpeg"), "../secrets", None, None)
            .await;

        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[test]
    fn resizes_thumbnail_size_suffix() {
        assert_eq!(
            sized_thumbnail_link("https://lh3.googleusercontent.com/d/AJQWt=s220", 800),
            "https://lh3.googleusercontent.com/d/AJQWt=s800"
        );
        assert_eq!(
            sized_thumbnail_link("https://example.com/thumb", 800),
            "https://example.com/thumb"
        );
    }

    #[test]
    fn sniffs_png_webp_and_defaults_to_jpeg() {
        assert_eq!(sniff_image_type(b"\x89PNG\r\n"), "image/png");
        assert_eq!(sniff_image_type(b"RIFF\0\0\0\0WEBPVP8 "), "image/webp");
        assert_eq!(sniff_image_type(b"\xff\xd8\xff"), "image/jpeg");
    }
}
