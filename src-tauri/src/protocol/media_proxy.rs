use std::io::SeekFrom;
use std::net::Ipv4Addr;
use std::path::Path as FilePath;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::net::TcpListener;
use tokio_util::io::ReaderStream;

use crate::auth::pkce::random_token;
use crate::drive::client::DriveClient;
use crate::error::{AppError, AppResult};
use crate::library::KnownFiles;
use crate::library::local_files::{LocalLibrary, is_local_id};

/// MOV and M4V use the same container format as MP4. Labelling them `video/mp4` lets WebView2
/// play them instead of rejecting the QuickTime type.
const MP4_COMPATIBLE_TYPES: [&str; 2] = ["video/quicktime", "video/x-m4v"];

const RELAYED_HEADERS: [header::HeaderName; 6] = [
    header::CONTENT_TYPE,
    header::CONTENT_LENGTH,
    header::CONTENT_RANGE,
    header::ACCEPT_RANGES,
    header::ETAG,
    header::LAST_MODIFIED,
];

/// Serves videos to `<video>` on 127.0.0.1: an authenticated pass-through to Drive
/// `files.get?alt=media` that keeps the OAuth token inside the Rust process, and the files in the
/// folders on this computer. Both honour `Range`, so the player and ffmpeg can seek.
#[derive(Clone)]
pub struct MediaProxy {
    port: u16,
    secret: String,
    local: Arc<LocalLibrary>,
}

struct ProxyState {
    drive: Arc<DriveClient>,
    known_files: KnownFiles,
    local: Arc<LocalLibrary>,
    secret: String,
    expected_host: String,
}

impl MediaProxy {
    pub async fn start(
        drive: Arc<DriveClient>,
        known_files: KnownFiles,
        local: Arc<LocalLibrary>,
    ) -> AppResult<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();
        let secret = random_token(32)?;
        let state = Arc::new(ProxyState {
            drive,
            known_files,
            local: local.clone(),
            secret: secret.clone(),
            expected_host: format!("127.0.0.1:{port}"),
        });
        let router = Router::new()
            .route("/media/{secret}/{file_id}", get(serve_media))
            .with_state(state);
        tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, router).await {
                log::error!("Media proxy stopped: {error}");
            }
        });
        Ok(Self {
            port,
            secret,
            local,
        })
    }

    pub fn media_url(&self, file_id: &str) -> String {
        format!(
            "http://127.0.0.1:{}/media/{}/{file_id}",
            self.port, self.secret
        )
    }

    /// What ffmpeg reads for a video: a file on this computer directly, which seeks far faster
    /// than over HTTP, or else the proxy URL, so the OAuth token never appears in ffmpeg's
    /// arguments.
    pub async fn ffmpeg_input(&self, file_id: &str) -> AppResult<String> {
        if !is_local_id(file_id) {
            return Ok(self.media_url(file_id));
        }
        let video = self.local.find(file_id).await.ok_or(AppError::NotFound)?;
        // `file:` keeps ffmpeg from reading a name such as `concat:x` as a protocol.
        Ok(format!("file:{}", video.path.to_string_lossy()))
    }
}

async fn serve_media(
    State(state): State<Arc<ProxyState>>,
    Path((secret, file_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !constant_time_eq(secret.as_bytes(), state.secret.as_bytes()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    // Rejecting other Host values blocks DNS-rebinding pages from reaching the proxy.
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok());
    if host != Some(state.expected_host.as_str()) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    // Only videos in the chosen folders resolve, so the proxy never serves other files.
    if is_local_id(&file_id) {
        return match state.local.find(&file_id).await {
            Some(video) => serve_file(&video.path, video.mime_type, range).await,
            None => StatusCode::NOT_FOUND.into_response(),
        };
    }
    // Only files Homereel has listed are playable, so other local software cannot use the
    // proxy to read arbitrary Drive files.
    if !state.known_files.contains(&file_id) {
        return StatusCode::NOT_FOUND.into_response();
    }

    match state.drive.open_media(&file_id, range).await {
        Ok(upstream) => relay(upstream),
        Err(error) => error_response(&error),
    }
}

/// Serves a file on this computer, or the byte range of it that `<video>` asks for.
async fn serve_file(path: &FilePath, mime_type: &'static str, range: Option<&str>) -> Response {
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(length) = file.metadata().await.map(|metadata| metadata.len()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (status, start, end) = match requested_range(range, length) {
        Ok(None) => (StatusCode::OK, 0, length),
        Ok(Some((start, end))) => (StatusCode::PARTIAL_CONTENT, start, end),
        Err(Unsatisfiable) => {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [(header::CONTENT_RANGE, format!("bytes */{length}"))],
            )
                .into_response();
        }
    };
    if start > 0 && file.seek(SeekFrom::Start(start)).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, playable_type(mime_type))
        .header(header::CONTENT_LENGTH, end - start)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "no-store");
    if status == StatusCode::PARTIAL_CONTENT {
        builder = builder.header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{}/{length}", end - 1),
        );
    }
    builder
        .body(Body::from_stream(ReaderStream::new(file.take(end - start))))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[derive(Debug, PartialEq)]
struct Unsatisfiable;

/// The `start..end` bytes a `Range: bytes=` header asks for, or `None` for the whole file. A
/// header that can't be parsed is ignored, as HTTP allows; only the first of several ranges is
/// served, which is all `<video>` asks for.
fn requested_range(range: Option<&str>, length: u64) -> Result<Option<(u64, u64)>, Unsatisfiable> {
    let Some(spec) = range.and_then(|value| value.trim().strip_prefix("bytes=")) else {
        return Ok(None);
    };
    let first = spec.split(',').next().unwrap_or_default();
    let Some((start, end)) = first.trim().split_once('-') else {
        return Ok(None);
    };
    let (start, end) = match (start.trim(), end.trim()) {
        ("", suffix) => match suffix.parse::<u64>() {
            Ok(0) => return Err(Unsatisfiable),
            Ok(suffix) => (length.saturating_sub(suffix), length),
            Err(_) => return Ok(None),
        },
        (start, end) => {
            let Ok(start) = start.parse::<u64>() else {
                return Ok(None);
            };
            let end = match end {
                "" => length,
                end => match end.parse::<u64>() {
                    Ok(end) if end >= start => end.saturating_add(1).min(length),
                    _ => return Ok(None),
                },
            };
            (start, end)
        }
    };
    if start >= length {
        return Err(Unsatisfiable);
    }
    Ok(Some((start, end)))
}

fn playable_type(mime_type: &str) -> &str {
    if MP4_COMPATIBLE_TYPES.contains(&mime_type) {
        "video/mp4"
    } else {
        mime_type
    }
}

fn relay(upstream: reqwest::Response) -> Response {
    let mut builder = Response::builder().status(upstream.status());
    for name in RELAYED_HEADERS {
        if let Some(value) = upstream.headers().get(&name) {
            let is_mp4_compatible = name == header::CONTENT_TYPE
                && value
                    .to_str()
                    .is_ok_and(|kind| MP4_COMPATIBLE_TYPES.contains(&kind));
            if is_mp4_compatible {
                builder = builder.header(&name, HeaderValue::from_static("video/mp4"));
            } else {
                builder = builder.header(&name, value);
            }
        }
    }
    // Drive honours byte ranges for binary files; advertising it lets WebView2 seek.
    if upstream.status().is_success() && !upstream.headers().contains_key(header::ACCEPT_RANGES) {
        builder = builder.header(header::ACCEPT_RANGES, "bytes");
    }
    builder
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

fn error_response(error: &AppError) -> Response {
    let status = match error {
        AppError::NotConnected | AppError::AuthRevoked | AppError::NotConfigured => {
            StatusCode::UNAUTHORIZED
        }
        AppError::NotFound => StatusCode::NOT_FOUND,
        AppError::Forbidden(_) => StatusCode::FORBIDDEN,
        AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::BAD_GATEWAY,
    };
    (status, error.code()).into_response()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::oauth::OAuthClient;
    use crate::auth::token_manager::{REFRESH_TOKEN_KEY, TokenManager};
    use crate::auth::token_store::testing::MemorySecretStore;
    use crate::drive::models::DriveFile;
    use wiremock::matchers::{header as header_is, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn start_proxy(server: &MockServer, known: &[&str]) -> MediaProxy {
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "access-1",
                "expires_in": 3599
            })))
            .mount(server)
            .await;
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
        let known_files = KnownFiles::default();
        let files: Vec<DriveFile> = known
            .iter()
            .map(|id| {
                serde_json::from_value(serde_json::json!({
                    "id": id, "name": "video", "mimeType": "video/mp4"
                }))
                .unwrap()
            })
            .collect();
        known_files.remember(&files);
        MediaProxy::start(Arc::new(drive), known_files, Arc::default())
            .await
            .unwrap()
    }

    #[test]
    fn reads_byte_ranges() {
        assert_eq!(requested_range(None, 1000), Ok(None));
        assert_eq!(requested_range(Some("bytes=0-"), 1000), Ok(Some((0, 1000))));
        assert_eq!(
            requested_range(Some("bytes=100-199"), 1000),
            Ok(Some((100, 200)))
        );
        assert_eq!(
            requested_range(Some("bytes=900-5000"), 1000),
            Ok(Some((900, 1000)))
        );
        assert_eq!(
            requested_range(Some("bytes=-100"), 1000),
            Ok(Some((900, 1000)))
        );
        assert_eq!(
            requested_range(Some("bytes=0-9, 20-29"), 1000),
            Ok(Some((0, 10)))
        );
        assert_eq!(requested_range(Some("bytes=abc"), 1000), Ok(None));
        assert_eq!(requested_range(Some("items=0-9"), 1000), Ok(None));
        assert_eq!(
            requested_range(Some("bytes=1000-"), 1000),
            Err(Unsatisfiable)
        );
        assert_eq!(requested_range(Some("bytes=-0"), 1000), Err(Unsatisfiable));
        assert_eq!(requested_range(Some("bytes=0-"), 0), Err(Unsatisfiable));
    }

    #[tokio::test]
    async fn serves_ranges_of_local_files_in_the_chosen_folders_only() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let content: Vec<u8> = (0..=255).collect();
        std::fs::write(dir.path().join("clip.MOV"), &content).unwrap();
        let local = Arc::new(LocalLibrary::default());
        local.set_roots(vec![dir.path().to_owned()]);
        let id = local.scan().await[0].id.clone();
        let drive = DriveClient::new(
            reqwest::Client::new(),
            reqwest::Client::new(),
            &server.uri(),
            Arc::new(TokenManager::new(
                reqwest::Client::new(),
                None,
                Arc::new(MemorySecretStore::default()),
            )),
        );
        let proxy = MediaProxy::start(Arc::new(drive), KnownFiles::default(), local.clone())
            .await
            .unwrap();
        let http = reqwest::Client::new();

        let partial = http
            .get(proxy.media_url(&id))
            .header("range", "bytes=10-19")
            .send()
            .await
            .unwrap();
        assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(partial.headers()["content-range"], "bytes 10-19/256");
        assert_eq!(partial.headers()["content-type"], "video/mp4");
        assert_eq!(partial.bytes().await.unwrap().as_ref(), &content[10..20]);

        let whole = http.get(proxy.media_url(&id)).send().await.unwrap();
        assert_eq!(whole.status(), StatusCode::OK);
        assert_eq!(whole.bytes().await.unwrap().len(), 256);

        local.set_roots(Vec::new());
        let removed = http.get(proxy.media_url(&id)).send().await.unwrap();
        assert_eq!(removed.status(), StatusCode::NOT_FOUND);
        assert!(
            proxy
                .ffmpeg_input("local-0123")
                .await
                .is_err_and(|error| matches!(error, AppError::NotFound))
        );
    }

    #[tokio::test]
    async fn labels_quicktime_as_mp4_so_webview2_plays_it() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files/mov1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "video/quicktime")
                    .set_body_bytes(vec![1u8; 10]),
            )
            .mount(&server)
            .await;
        let proxy = start_proxy(&server, &["mov1"]).await;

        let response = reqwest::get(proxy.media_url("mov1")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "video/mp4");
    }

    #[tokio::test]
    async fn forwards_range_and_relays_partial_content() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files/vid1"))
            .and(query_param("alt", "media"))
            .and(header_is("range", "bytes=100-199"))
            .and(header_is("authorization", "Bearer access-1"))
            .respond_with(
                ResponseTemplate::new(206)
                    .insert_header("content-type", "video/mp4")
                    .insert_header("content-range", "bytes 100-199/5000")
                    .set_body_bytes(vec![7u8; 100]),
            )
            .expect(1)
            .mount(&server)
            .await;
        let proxy = start_proxy(&server, &["vid1"]).await;

        let response = reqwest::Client::new()
            .get(proxy.media_url("vid1"))
            .header("range", "bytes=100-199")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["content-range"], "bytes 100-199/5000");
        assert_eq!(response.headers()["content-type"], "video/mp4");
        assert_eq!(response.headers()["accept-ranges"], "bytes");
        assert_eq!(response.bytes().await.unwrap().len(), 100);
    }

    #[tokio::test]
    async fn rejects_wrong_secret_unknown_files_and_foreign_hosts() {
        let server = MockServer::start().await;
        let proxy = start_proxy(&server, &["vid1"]).await;
        let http = reqwest::Client::new();
        let good_url = proxy.media_url("vid1");
        let wrong_secret = good_url.replace(&proxy.secret, "not-the-secret");

        let wrong_secret_status = http.get(&wrong_secret).send().await.unwrap().status();
        let unknown_status = http
            .get(proxy.media_url("other"))
            .send()
            .await
            .unwrap()
            .status();
        let foreign_host_status = http
            .get(&good_url)
            .header("host", "evil.example:80")
            .send()
            .await
            .unwrap()
            .status();

        assert_eq!(wrong_secret_status, StatusCode::NOT_FOUND);
        assert_eq!(unknown_status, StatusCode::NOT_FOUND);
        assert_eq!(foreign_host_status, StatusCode::BAD_REQUEST);
        let drive_calls = server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|request| request.url.path().starts_with("/drive"))
            .count();
        assert_eq!(drive_calls, 0);
    }
}
