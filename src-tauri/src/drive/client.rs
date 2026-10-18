use std::sync::Arc;
use std::time::Duration;

use reqwest::{RequestBuilder, Response, StatusCode, header};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::is_valid_drive_id;
use super::models::{About, DriveFile, DriveFolder, DriveUser, FolderEntry, Page};
use crate::auth::token_manager::TokenManager;
use crate::error::{AppError, AppResult};

pub const VIDEO_FIELDS: &str = "id,name,mimeType,parents,modifiedTime,createdTime,size,\
md5Checksum,videoMediaMetadata(width,height,durationMillis),hasThumbnail,thumbnailLink";
const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";
/// My Drive's top folder, which Drive accepts in place of its ID.
pub const MY_DRIVE: &str = "root";

const JSON_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PAGE_SIZE: u32 = 1000;
/// Stops a runaway page loop; 100,000 videos is far beyond a personal Drive.
const MAX_LIST_PAGES: usize = 100;
/// Folders matched by one query. Each adds about 50 characters to the URL, which Google caps at
/// a few thousand.
const MAX_PARENTS_PER_QUERY: usize = 40;

/// Builds the Drive `q` expression for non-trashed videos, optionally in some folders and/or
/// with a name match.
pub fn build_video_filter(search: Option<&str>, folder_ids: &[String]) -> AppResult<String> {
    let mut clauses = vec![
        "mimeType contains 'video/'".to_owned(),
        "trashed = false".to_owned(),
    ];
    if !folder_ids.is_empty() {
        clauses.push(parents_clause(folder_ids)?);
    }
    if let Some(text) = search.map(str::trim).filter(|text| !text.is_empty()) {
        clauses.push(format!("name contains '{}'", escape_query_literal(text)));
    }
    Ok(clauses.join(" and "))
}

/// The `q` expression for the non-trashed folders directly inside the given folders.
pub fn build_folder_filter(parent_ids: &[String]) -> AppResult<String> {
    Ok(format!(
        "mimeType = '{FOLDER_MIME_TYPE}' and trashed = false and {}",
        parents_clause(parent_ids)?
    ))
}

/// `'a' in parents`, or `('a' in parents or 'b' in parents)` for several folders.
fn parents_clause(folder_ids: &[String]) -> AppResult<String> {
    let clauses = folder_ids
        .iter()
        .map(|id| {
            ensure_valid_id(id)?;
            Ok(format!("'{id}' in parents"))
        })
        .collect::<AppResult<Vec<String>>>()?;
    Ok(match clauses.as_slice() {
        [single] => single.clone(),
        _ => format!("({})", clauses.join(" or ")),
    })
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

    /// Every video the account can see, or every video directly inside the given folders, in no
    /// particular order. Takes one request per 1,000 videos (and per 40 folders).
    pub async fn list_all_videos(&self, folder_ids: &[String]) -> AppResult<Vec<DriveFile>> {
        let fields = format!("nextPageToken,files({VIDEO_FIELDS})");
        if folder_ids.is_empty() {
            return self
                .list_every_page(&build_video_filter(None, &[])?, &fields, None)
                .await;
        }
        let mut files = Vec::new();
        for chunk in folder_ids.chunks(MAX_PARENTS_PER_QUERY) {
            let filter = build_video_filter(None, chunk)?;
            files.extend(self.list_every_page(&filter, &fields, None).await?);
        }
        Ok(files)
    }

    /// The folders directly inside the given folders, by name.
    pub async fn list_folders(&self, parent_ids: &[String]) -> AppResult<Vec<FolderEntry>> {
        let fields = "nextPageToken,files(id,name)";
        let mut folders = Vec::new();
        for chunk in parent_ids.chunks(MAX_PARENTS_PER_QUERY) {
            let filter = build_folder_filter(chunk)?;
            folders.extend(
                self.list_every_page(&filter, fields, Some("name_natural"))
                    .await?,
            );
        }
        Ok(folders)
    }

    async fn list_every_page<T: DeserializeOwned>(
        &self,
        filter: &str,
        fields: &str,
        order_by: Option<&str>,
    ) -> AppResult<Vec<T>> {
        let url = format!("{}/files", self.api_base);
        let page_size = MAX_PAGE_SIZE.to_string();
        let mut items = Vec::new();
        let mut page_token: Option<String> = None;
        for _ in 0..MAX_LIST_PAGES {
            let page: Page<T> = self
                .get_json(|http| {
                    let mut request = http.get(&url).query(&[
                        ("q", filter),
                        ("pageSize", page_size.as_str()),
                        ("fields", fields),
                        ("supportsAllDrives", "true"),
                        ("includeItemsFromAllDrives", "true"),
                    ]);
                    if let Some(order_by) = order_by {
                        request = request.query(&[("orderBy", order_by)]);
                    }
                    match page_token.as_deref() {
                        Some(token) => request.query(&[("pageToken", token)]),
                        None => request,
                    }
                })
                .await?;
            items.extend(page.files);
            match page.next_page_token {
                Some(token) => page_token = Some(token),
                None => break,
            }
        }
        Ok(items)
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
        let filter = build_video_filter(Some(" Tom's \\ trip "), &["folder_1".into()]).unwrap();

        assert_eq!(
            filter,
            "mimeType contains 'video/' and trashed = false and 'folder_1' in parents \
             and name contains 'Tom\\'s \\\\ trip'"
        );
    }

    #[test]
    fn video_filter_ignores_blank_search_and_rejects_bad_folder_ids() {
        assert_eq!(
            build_video_filter(Some("   "), &[]).unwrap(),
            "mimeType contains 'video/' and trashed = false"
        );
        assert!(matches!(
            build_video_filter(None, &["x' or name contains '".into()]),
            Err(AppError::NotFound)
        ));
    }

    #[test]
    fn folder_filter_matches_any_of_several_parents() {
        assert_eq!(
            build_folder_filter(&["a".into(), "b".into()]).unwrap(),
            "mimeType = 'application/vnd.google-apps.folder' and trashed = false \
             and ('a' in parents or 'b' in parents)"
        );
    }

    #[tokio::test]
    async fn lists_videos_in_folders_across_every_page() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server).await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param(
                "q",
                "mimeType contains 'video/' and trashed = false \
                 and ('fold1' in parents or 'fold2' in parents)",
            ))
            .and(query_param("pageSize", "1000"))
            .and(query_param_is_missing("pageToken"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "nextPageToken": "page-2",
                "files": [{ "id": "f1", "name": "a.mp4", "mimeType": "video/mp4", "parents": ["fold1"] }]
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param("pageToken", "page-2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [{ "id": "f2", "name": "b.mp4", "mimeType": "video/mp4", "parents": ["fold2"] }]
            })))
            .expect(1)
            .mount(&server)
            .await;
        let client = drive_client(&server);

        let files = client
            .list_all_videos(&["fold1".into(), "fold2".into()])
            .await
            .unwrap();

        let ids: Vec<&str> = files.iter().map(|file| file.id.as_str()).collect();
        assert_eq!(ids, ["f1", "f2"]);
        assert_eq!(files[1].parents, vec!["fold2".to_string()]);
    }

    #[tokio::test]
    async fn lists_child_folders_by_name() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server).await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param(
                "q",
                "mimeType = 'application/vnd.google-apps.folder' and trashed = false \
                 and 'root' in parents",
            ))
            .and(query_param("orderBy", "name_natural"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [{ "id": "vids", "name": "VIDS" }]
            })))
            .expect(1)
            .mount(&server)
            .await;
        let client = drive_client(&server);

        let folders = client.list_folders(&[MY_DRIVE.into()]).await.unwrap();

        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "VIDS");
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
