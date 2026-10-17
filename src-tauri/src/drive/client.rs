use std::sync::Arc;
use std::time::Duration;

use reqwest::{RequestBuilder, Response, StatusCode, header};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::is_valid_drive_id;
use super::models::{About, DriveFile, DriveFolder, DriveUser, FileList, FileParents, ParentList};
use crate::auth::token_manager::TokenManager;
use crate::error::{AppError, AppResult};

pub const VIDEO_FIELDS: &str = "id,name,mimeType,parents,modifiedTime,createdTime,size,\
md5Checksum,videoMediaMetadata(width,height,durationMillis),hasThumbnail,thumbnailLink";

const JSON_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PAGE_SIZE: u32 = 1000;
/// Stops a runaway page loop; 100,000 videos is far beyond a personal Drive.
const MAX_LIST_PAGES: usize = 100;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoOrder {
    /// By the day the video was recorded ("date created" in the UI), newest first.
    #[default]
    CreatedNewest,
    CreatedOldest,
    /// By the day the file was uploaded to Drive, which Drive calls `createdTime`.
    UploadedNewest,
    UploadedOldest,
    Name,
}

impl VideoOrder {
    /// Drive's `orderBy`, or `None` for the recording-date orders: Drive does not know that
    /// date, so DriveTube lists every match and sorts them itself.
    pub fn as_drive_order_by(self) -> Option<&'static str> {
        match self {
            VideoOrder::CreatedNewest | VideoOrder::CreatedOldest => None,
            VideoOrder::UploadedNewest => Some("createdTime desc"),
            VideoOrder::UploadedOldest => Some("createdTime"),
            VideoOrder::Name => Some("name_natural"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VideoQuery {
    pub search: Option<String>,
    pub folder_id: Option<String>,
    pub order: VideoOrder,
    pub page_token: Option<String>,
    pub page_size: u32,
}

/// Builds the Drive `q` expression for non-trashed videos, optionally in one folder and/or
/// with a name match.
pub fn build_video_filter(search: Option<&str>, folder_id: Option<&str>) -> AppResult<String> {
    let mut clauses = vec![
        "mimeType contains 'video/'".to_owned(),
        "trashed = false".to_owned(),
    ];
    if let Some(folder_id) = folder_id {
        ensure_valid_id(folder_id)?;
        clauses.push(format!("'{folder_id}' in parents"));
    }
    if let Some(text) = search.map(str::trim).filter(|text| !text.is_empty()) {
        clauses.push(format!("name contains '{}'", escape_query_literal(text)));
    }
    Ok(clauses.join(" and "))
}

/// Drive query string literals escape backslashes and single quotes with a backslash.
fn escape_query_literal(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Read-only Google Drive API v3 client. Every request carries the OAuth bearer token and is
/// retried once with a refreshed token after a 401.
pub struct DriveClient {
    http: reqwest::Client,
    media_http: reqwest::Client,
    api_base: String,
    tokens: Arc<TokenManager>,
}

impl DriveClient {
    /// `media_http` serves file content and should be HTTP/1.1 only: over a shared HTTP/2
    /// connection, one partly read video stream holds the flow-control window and stalls every
    /// other stream (the player, thumbnails and title frames run at the same time).
    pub fn new(
        http: reqwest::Client,
        media_http: reqwest::Client,
        api_base: &str,
        tokens: Arc<TokenManager>,
    ) -> Self {
        Self {
            http,
            media_http,
            api_base: api_base.trim_end_matches('/').to_owned(),
            tokens,
        }
    }

    pub async fn about(&self) -> AppResult<DriveUser> {
        let url = format!("{}/about", self.api_base);
        let about: About = self
            .get_json(|http| {
                http.get(&url)
                    .query(&[("fields", "user(displayName,emailAddress)")])
            })
            .await?;
        Ok(about.user)
    }

    /// One page of videos the account can see, across all folders unless a folder is given.
    pub async fn list_videos(&self, query: &VideoQuery) -> AppResult<FileList> {
        let url = format!("{}/files", self.api_base);
        let filter = build_video_filter(query.search.as_deref(), query.folder_id.as_deref())?;
        let fields = format!("nextPageToken,files({VIDEO_FIELDS})");
        let page_size = query.page_size.clamp(1, MAX_PAGE_SIZE).to_string();
        self.get_json(|http| {
            let mut request = http.get(&url).query(&[
                ("q", filter.as_str()),
                ("pageSize", page_size.as_str()),
                ("fields", fields.as_str()),
                ("supportsAllDrives", "true"),
                ("includeItemsFromAllDrives", "true"),
            ]);
            if let Some(order_by) = query.order.as_drive_order_by() {
                request = request.query(&[("orderBy", order_by)]);
            }
            match query.page_token.as_deref() {
                Some(token) => request.query(&[("pageToken", token)]),
                None => request,
            }
        })
        .await
    }

    /// Every video, or every video in one folder, in no particular order. Takes one request per
    /// 1,000 videos.
    pub async fn list_all_videos(&self, folder_id: Option<&str>) -> AppResult<Vec<DriveFile>> {
        let mut query = VideoQuery {
            search: None,
            folder_id: folder_id.map(str::to_owned),
            order: VideoOrder::CreatedNewest,
            page_token: None,
            page_size: MAX_PAGE_SIZE,
        };
        let mut files = Vec::new();
        for _ in 0..MAX_LIST_PAGES {
            let page = self.list_videos(&query).await?;
            files.extend(page.files);
            match page.next_page_token {
                Some(token) => query.page_token = Some(token),
                None => return Ok(files),
            }
        }
        Ok(files)
    }

    /// Parent folder IDs of every video the account can see, used to list the folders that hold
    /// videos. Takes one request per 1,000 videos.
    pub async fn list_video_parents(&self) -> AppResult<Vec<FileParents>> {
        let url = format!("{}/files", self.api_base);
        let filter = build_video_filter(None, None)?;
        let page_size = MAX_PAGE_SIZE.to_string();
        let mut files = Vec::new();
        let mut page_token: Option<String> = None;
        for _ in 0..MAX_LIST_PAGES {
            let page: ParentList = self
                .get_json(|http| {
                    let request = http.get(&url).query(&[
                        ("q", filter.as_str()),
                        ("pageSize", page_size.as_str()),
                        ("fields", "nextPageToken,files(parents)"),
                        ("supportsAllDrives", "true"),
                        ("includeItemsFromAllDrives", "true"),
                    ]);
                    match page_token.as_deref() {
                        Some(token) => request.query(&[("pageToken", token)]),
                        None => request,
                    }
                })
                .await?;
            files.extend(page.files);
            match page.next_page_token {
                Some(token) => page_token = Some(token),
                None => break,
            }
        }
        Ok(files)
    }

    pub async fn folder(&self, folder_id: &str) -> AppResult<DriveFolder> {
        ensure_valid_id(folder_id)?;
        let url = format!("{}/files/{folder_id}", self.api_base);
        self.get_json(|http| {
            http.get(&url)
                .query(&[("fields", "name"), ("supportsAllDrives", "true")])
        })
        .await
    }

    pub async fn file(&self, file_id: &str) -> AppResult<DriveFile> {
        ensure_valid_id(file_id)?;
        let url = format!("{}/files/{file_id}", self.api_base);
        self.get_json(|http| {
            http.get(&url)
                .query(&[("fields", VIDEO_FIELDS), ("supportsAllDrives", "true")])
        })
        .await
    }

    /// Opens the file's content, forwarding an optional HTTP `Range` header. The raw response is
    /// returned so callers can relay partial-content statuses and headers unchanged.
    pub async fn open_media(&self, file_id: &str, range: Option<&str>) -> AppResult<Response> {
        ensure_valid_id(file_id)?;
        let url = format!("{}/files/{file_id}", self.api_base);
        self.send_authorized(&self.media_http, |http| {
            let request = http
                .get(&url)
                .query(&[("alt", "media"), ("supportsAllDrives", "true")]);
            match range {
                Some(range) => request.header(header::RANGE, range),
                None => request,
            }
        })
        .await
    }

    /// Fetches a `thumbnailLink`, which needs a credentialed request for private files.
    pub async fn fetch_authorized(&self, url: &str) -> AppResult<Response> {
        self.send_authorized(&self.http, |http| {
            http.get(url).timeout(JSON_REQUEST_TIMEOUT)
        })
        .await
    }

    async fn get_json<T, F>(&self, build: F) -> AppResult<T>
    where
        T: DeserializeOwned,
        F: Fn(&reqwest::Client) -> RequestBuilder,
    {
        let response = self
            .send_authorized(&self.http, |http| build(http).timeout(JSON_REQUEST_TIMEOUT))
            .await?;
        let response = error_for_status(response).await?;
        response.json::<T>().await.map_err(|error| {
            AppError::internal(format!(
                "Unexpected Drive response: {}",
                error.without_url()
            ))
        })
    }

    async fn send_authorized<F>(&self, client: &reqwest::Client, build: F) -> AppResult<Response>
    where
        F: Fn(&reqwest::Client) -> RequestBuilder,
    {
        let token = self.tokens.access_token().await?;
        let response = build(client).bearer_auth(&token).send().await?;
        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(response);
        }
        self.tokens.invalidate(&token).await;
        let token = self.tokens.access_token().await?;
        Ok(build(client).bearer_auth(&token).send().await?)
    }
}

fn ensure_valid_id(file_id: &str) -> AppResult<()> {
    if is_valid_drive_id(file_id) {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

#[derive(Deserialize)]
struct DriveErrorBody {
    error: DriveErrorDetail,
}

#[derive(Deserialize)]
struct DriveErrorDetail {
    #[serde(default)]
    message: String,
    #[serde(default)]
    errors: Vec<DriveErrorReason>,
}

#[derive(Deserialize)]
struct DriveErrorReason {
    #[serde(default)]
    reason: String,
}

/// Maps a Drive error response to an `AppError`, using Drive's `reason` codes where they
/// distinguish rate limits from permission problems.
pub async fn error_for_status(response: Response) -> AppResult<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let detail = response
        .json::<DriveErrorBody>()
        .await
        .ok()
        .map(|body| body.error);
    let reason = detail
        .as_ref()
        .and_then(|d| d.errors.first())
        .map(|r| r.reason.as_str())
        .unwrap_or_default();
    let message = detail
        .as_ref()
        .map(|d| d.message.clone())
        .unwrap_or_default();

    Err(match status {
        StatusCode::UNAUTHORIZED => AppError::AuthRevoked,
        StatusCode::TOO_MANY_REQUESTS => AppError::RateLimited,
        StatusCode::FORBIDDEN
            if matches!(reason, "userRateLimitExceeded" | "rateLimitExceeded") =>
        {
            AppError::RateLimited
        }
        StatusCode::FORBIDDEN => AppError::Forbidden(if message.is_empty() {
            "Google Drive denied the request.".into()
        } else {
            message
        }),
        StatusCode::NOT_FOUND => AppError::NotFound,
        _ => AppError::internal(format!("Google Drive returned {status}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::oauth::OAuthClient;
    use crate::auth::token_manager::REFRESH_TOKEN_KEY;
    use crate::auth::token_store::testing::MemorySecretStore;
    use crate::config::DRIVE_READONLY_SCOPE;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::matchers::{
        header as header_is, method, path, query_param, query_param_is_missing,
    };
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    /// Token endpoint that hands out "access-1", "access-2", ... on each refresh.
    async fn mount_token_endpoint(server: &MockServer) {
        let counter = Arc::new(AtomicUsize::new(0));
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(move |_: &Request| {
                let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "access_token": format!("access-{n}"),
                    "expires_in": 3599,
                    "scope": DRIVE_READONLY_SCOPE
                }))
            })
            .mount(server)
            .await;
    }

    fn drive_client(server: &MockServer) -> DriveClient {
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
        DriveClient::new(
            reqwest::Client::new(),
            reqwest::Client::new(),
            &format!("{}/drive/v3", server.uri()),
            Arc::new(tokens),
        )
    }

    #[tokio::test]
    async fn retries_once_with_a_refreshed_token_after_401() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server).await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/about"))
            .and(header_is("authorization", "Bearer access-1"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/about"))
            .and(header_is("authorization", "Bearer access-2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user": { "displayName": "Sam", "emailAddress": "sam@example.com" }
            })))
            .expect(1)
            .mount(&server)
            .await;
        let client = drive_client(&server);

        let user = client.about().await.unwrap();

        assert_eq!(user.email_address.as_deref(), Some("sam@example.com"));
    }

    #[test]
    fn video_filter_combines_folder_and_escaped_search() {
        let filter = build_video_filter(Some(" Tom's \\ trip "), Some("folder_1")).unwrap();

        assert_eq!(
            filter,
            "mimeType contains 'video/' and trashed = false and 'folder_1' in parents \
             and name contains 'Tom\\'s \\\\ trip'"
        );
    }

    #[test]
    fn video_filter_ignores_blank_search_and_rejects_bad_folder_ids() {
        assert_eq!(
            build_video_filter(Some("   "), None).unwrap(),
            "mimeType contains 'video/' and trashed = false"
        );
        assert!(matches!(
            build_video_filter(None, Some("x' or name contains '")),
            Err(AppError::NotFound)
        ));
    }

    #[test]
    fn upload_orders_use_drive_created_time_and_recording_orders_sort_locally() {
        assert_eq!(
            VideoOrder::UploadedNewest.as_drive_order_by(),
            Some("createdTime desc")
        );
        assert_eq!(
            VideoOrder::UploadedOldest.as_drive_order_by(),
            Some("createdTime")
        );
        assert_eq!(VideoOrder::CreatedNewest.as_drive_order_by(), None);
        assert_eq!(VideoOrder::CreatedOldest.as_drive_order_by(), None);
    }

    #[tokio::test]
    async fn lists_a_page_of_videos_with_filter_order_and_token() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server).await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param(
                "q",
                "mimeType contains 'video/' and trashed = false and 'fold1' in parents",
            ))
            .and(query_param("orderBy", "name_natural"))
            .and(query_param("pageSize", "48"))
            .and(query_param("pageToken", "page-2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "nextPageToken": "page-3",
                "files": [{ "id": "f1", "name": "a.mp4", "mimeType": "video/mp4", "parents": ["fold1"] }]
            })))
            .expect(1)
            .mount(&server)
            .await;
        let client = drive_client(&server);
        let query = VideoQuery {
            folder_id: Some("fold1".into()),
            order: VideoOrder::Name,
            page_token: Some("page-2".into()),
            page_size: 48,
            ..VideoQuery::default()
        };

        let page = client.list_videos(&query).await.unwrap();

        assert_eq!(page.files.len(), 1);
        assert_eq!(page.files[0].parents, vec!["fold1".to_string()]);
        assert_eq!(page.next_page_token.as_deref(), Some("page-3"));
    }

    #[tokio::test]
    async fn lists_video_parents_across_every_page() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server).await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param("fields", "nextPageToken,files(parents)"))
            .and(query_param_is_missing("pageToken"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "nextPageToken": "page-2",
                "files": [{ "parents": ["fold1"] }, { "parents": ["fold1"] }]
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param("pageToken", "page-2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [{ "parents": ["fold2"] }]
            })))
            .expect(1)
            .mount(&server)
            .await;
        let client = drive_client(&server);

        let files = client.list_video_parents().await.unwrap();

        let parents: Vec<&str> = files.iter().map(|file| file.parents[0].as_str()).collect();
        assert_eq!(parents, ["fold1", "fold1", "fold2"]);
    }

    #[tokio::test]
    async fn maps_rate_limit_reason_on_403() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server).await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files/f1"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "error": {
                    "code": 403,
                    "message": "User Rate Limit Exceeded",
                    "errors": [{ "reason": "userRateLimitExceeded" }]
                }
            })))
            .mount(&server)
            .await;
        let client = drive_client(&server);

        let result = client.file("f1").await;

        assert!(matches!(result, Err(AppError::RateLimited)));
    }

    #[tokio::test]
    async fn rejects_invalid_ids_without_calling_drive() {
        let server = MockServer::start().await;
        let client = drive_client(&server);

        let result = client.open_media("../about", None).await;

        assert!(matches!(result, Err(AppError::NotFound)));
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
