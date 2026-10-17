use std::net::Ipv4Addr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::net::TcpListener;

use crate::auth::pkce::random_token;
use crate::drive::client::DriveClient;
use crate::error::{AppError, AppResult};
use crate::library::KnownFiles;

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

/// Authenticated pass-through from 127.0.0.1 to Drive `files.get?alt=media`. It forwards `Range`
/// so `<video>` and ffmpeg can seek, and keeps the OAuth token inside the Rust process.
#[derive(Clone)]
pub struct MediaProxy {
    port: u16,
    secret: String,
}

struct ProxyState {
    drive: Arc<DriveClient>,
    known_files: KnownFiles,
    secret: String,
    expected_host: String,
}

impl MediaProxy {
    pub async fn start(drive: Arc<DriveClient>, known_files: KnownFiles) -> AppResult<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();
        let secret = random_token(32)?;
        let state = Arc::new(ProxyState {
            drive,
            known_files,
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
        Ok(Self { port, secret })
    }

    pub fn media_url(&self, file_id: &str) -> String {
        format!(
            "http://127.0.0.1:{}/media/{}/{file_id}",
            self.port, self.secret
        )
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
    // Only files DriveTube has listed are playable, so other local software cannot use the
    // proxy to read arbitrary Drive files.
    if !state.known_files.contains(&file_id) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    match state.drive.open_media(&file_id, range).await {
        Ok(upstream) => relay(upstream),
        Err(error) => error_response(&error),
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
        MediaProxy::start(Arc::new(drive), known_files)
            .await
            .unwrap()
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
