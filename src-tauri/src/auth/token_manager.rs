use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;

use super::loopback::LoopbackListener;
use super::oauth::{
    OAuthClient, build_auth_url, exchange_code, has_drive_scope, refresh_access_token, revoke_token,
};
use super::pkce::{Pkce, random_token};
use super::token_store::SecretStore;
use crate::error::{AppError, AppResult};

pub const REFRESH_TOKEN_KEY: &str = "google-refresh-token";

/// Refresh slightly early so a token never expires mid-request.
const EXPIRY_MARGIN: Duration = Duration::from_secs(60);
const AUTHORIZATION_TIMEOUT: Duration = Duration::from_secs(5 * 60);

struct CachedToken {
    value: String,
    expires_at: Instant,
}

impl CachedToken {
    fn new(value: String, expires_in_secs: u64) -> Self {
        Self {
            value,
            expires_at: Instant::now() + Duration::from_secs(expires_in_secs),
        }
    }

    fn is_fresh(&self) -> bool {
        Instant::now() + EXPIRY_MARGIN < self.expires_at
    }
}

/// Owns the Google OAuth grant: the refresh token in secure storage and a cached access token.
/// Access tokens never leave the Rust layer.
pub struct TokenManager {
    http: reqwest::Client,
    client: Option<OAuthClient>,
    store: Arc<dyn SecretStore>,
    cached: tokio::sync::Mutex<Option<CachedToken>>,
    pending_authorization: Mutex<Option<CancellationToken>>,
}

impl TokenManager {
    pub fn new(
        http: reqwest::Client,
        client: Option<OAuthClient>,
        store: Arc<dyn SecretStore>,
    ) -> Self {
        Self {
            http,
            client,
            store,
            cached: tokio::sync::Mutex::new(None),
            pending_authorization: Mutex::new(None),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.client.is_some()
    }

    pub fn is_connected(&self) -> AppResult<bool> {
        Ok(self.store.get(REFRESH_TOKEN_KEY)?.is_some())
    }

    /// Runs the loopback + PKCE authorization flow. `open_browser` receives the consent URL.
    pub async fn connect<F>(&self, open_browser: F) -> AppResult<()>
    where
        F: FnOnce(&str) -> AppResult<()>,
    {
        let client = self.client()?;
        let listener = LoopbackListener::bind().await?;
        let redirect_uri = listener.redirect_uri();
        let pkce = Pkce::generate()?;
        let state = random_token(16)?;
        let auth_url = build_auth_url(client, &redirect_uri, &pkce.challenge, &state)?;

        let cancel = CancellationToken::new();
        if let Some(previous) = self.pending().replace(cancel.clone()) {
            previous.cancel();
        }
        open_browser(auth_url.as_str())?;

        let outcome = tokio::select! {
            result = listener.wait_for_code(&state) => result,
            _ = cancel.cancelled() => Err(AppError::Cancelled),
            _ = tokio::time::sleep(AUTHORIZATION_TIMEOUT) => Err(AppError::internal(
                "Timed out waiting for Google authorization.",
            )),
        };
        drop(listener);
        self.pending().take();
        let code = outcome?;

        let tokens =
            exchange_code(&self.http, client, &code, &pkce.verifier, &redirect_uri).await?;
        if !has_drive_scope(&tokens.scope) {
            return Err(AppError::Forbidden(
                "Homereel needs permission to view your Google Drive files. Connect again and allow it."
                    .into(),
            ));
        }
        let refresh_token = tokens
            .refresh_token
            .ok_or_else(|| AppError::internal("Google did not return a refresh token."))?;
        self.store.set(REFRESH_TOKEN_KEY, &refresh_token)?;
        *self.cached.lock().await = Some(CachedToken::new(tokens.access_token, tokens.expires_in));
        Ok(())
    }

    pub fn cancel_connect(&self) {
        if let Some(pending) = self.pending().take() {
            pending.cancel();
        }
    }

    /// Returns a valid access token, refreshing it when needed. A revoked grant is removed from
    /// storage and reported as `AuthRevoked`.
    pub async fn access_token(&self) -> AppResult<String> {
        let mut cached = self.cached.lock().await;
        if let Some(token) = cached.as_ref().filter(|token| token.is_fresh()) {
            return Ok(token.value.clone());
        }

        let client = self.client()?;
        let refresh_token = self
            .store
            .get(REFRESH_TOKEN_KEY)?
            .ok_or(AppError::NotConnected)?;
        match refresh_access_token(&self.http, client, &refresh_token).await {
            Ok(tokens) => {
                if let Some(rotated) = tokens.refresh_token.as_deref() {
                    self.store.set(REFRESH_TOKEN_KEY, rotated)?;
                }
                let value = tokens.access_token.clone();
                *cached = Some(CachedToken::new(tokens.access_token, tokens.expires_in));
                Ok(value)
            }
            Err(AppError::AuthRevoked) => {
                *cached = None;
                self.store.delete(REFRESH_TOKEN_KEY)?;
                Err(AppError::AuthRevoked)
            }
            Err(error) => Err(error),
        }
    }

    /// Drops the cached access token after Google rejected it. Comparing by value avoids
    /// discarding a token that a concurrent request has just refreshed.
    pub async fn invalidate(&self, rejected_token: &str) {
        let mut cached = self.cached.lock().await;
        if cached
            .as_ref()
            .is_some_and(|token| token.value == rejected_token)
        {
            *cached = None;
        }
    }

    /// Revokes the grant at Google (best effort) and deletes the stored refresh token.
    pub async fn disconnect(&self) -> AppResult<()> {
        self.cached.lock().await.take();
        let Some(refresh_token) = self.store.get(REFRESH_TOKEN_KEY)? else {
            return Ok(());
        };
        if let Ok(client) = self.client()
            && let Err(error) = revoke_token(&self.http, client, &refresh_token).await
        {
            log::warn!("Could not revoke the Google grant: {}", error.code());
        }
        self.store.delete(REFRESH_TOKEN_KEY)
    }

    fn client(&self) -> AppResult<&OAuthClient> {
        self.client.as_ref().ok_or(AppError::NotConfigured)
    }

    fn pending(&self) -> MutexGuard<'_, Option<CancellationToken>> {
        self.pending_authorization
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::token_store::testing::MemorySecretStore;
    use crate::config::DRIVE_READONLY_SCOPE;
    use std::collections::HashMap;
    use url::Url;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_for(server: &MockServer) -> OAuthClient {
        OAuthClient {
            client_id: "client-1".into(),
            client_secret: "secret-1".into(),
            auth_endpoint: format!("{}/auth", server.uri()),
            token_endpoint: format!("{}/token", server.uri()),
            revoke_endpoint: format!("{}/revoke", server.uri()),
        }
    }

    fn token_json(access_token: &str, refresh_token: Option<&str>) -> serde_json::Value {
        let mut body = serde_json::json!({
            "access_token": access_token,
            "expires_in": 3599,
            "scope": DRIVE_READONLY_SCOPE,
            "token_type": "Bearer"
        });
        if let Some(refresh_token) = refresh_token {
            body["refresh_token"] = refresh_token.into();
        }
        body
    }

    #[tokio::test]
    async fn connect_completes_loopback_flow_and_stores_refresh_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=auth-code"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_json("access-1", Some("refresh-1"))),
            )
            .mount(&server)
            .await;
        let store = Arc::new(MemorySecretStore::default());
        let manager = TokenManager::new(
            reqwest::Client::new(),
            Some(client_for(&server)),
            store.clone(),
        );

        // Simulate the browser: follow the consent URL's redirect_uri with the same state.
        manager
            .connect(|auth_url| {
                let params: HashMap<String, String> = Url::parse(auth_url)
                    .unwrap()
                    .query_pairs()
                    .into_owned()
                    .collect();
                let callback = format!(
                    "{}/?state={}&code=auth-code",
                    params["redirect_uri"], params["state"]
                );
                tokio::spawn(async move { reqwest::get(callback).await.unwrap() });
                Ok(())
            })
            .await
            .unwrap();

        assert_eq!(
            store.get(REFRESH_TOKEN_KEY).unwrap().as_deref(),
            Some("refresh-1")
        );
        assert_eq!(manager.access_token().await.unwrap(), "access-1");
    }

    #[tokio::test]
    async fn access_token_is_refreshed_once_and_cached() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_json("access-2", None)))
            .expect(1)
            .mount(&server)
            .await;
        let store = Arc::new(MemorySecretStore::with(REFRESH_TOKEN_KEY, "refresh-1"));
        let manager = TokenManager::new(reqwest::Client::new(), Some(client_for(&server)), store);

        let first = manager.access_token().await.unwrap();
        let second = manager.access_token().await.unwrap();

        assert_eq!(first, "access-2");
        assert_eq!(second, "access-2");
    }

    #[tokio::test]
    async fn revoked_grant_is_deleted_and_reported() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({"error": "invalid_grant"})),
            )
            .mount(&server)
            .await;
        let store = Arc::new(MemorySecretStore::with(REFRESH_TOKEN_KEY, "refresh-1"));
        let manager = TokenManager::new(
            reqwest::Client::new(),
            Some(client_for(&server)),
            store.clone(),
        );

        let result = manager.access_token().await;

        assert!(matches!(result, Err(AppError::AuthRevoked)));
        assert_eq!(store.get(REFRESH_TOKEN_KEY).unwrap(), None);
    }

    #[tokio::test]
    async fn missing_refresh_token_reports_not_connected() {
        let server = MockServer::start().await;
        let manager = TokenManager::new(
            reqwest::Client::new(),
            Some(client_for(&server)),
            Arc::new(MemorySecretStore::default()),
        );

        let result = manager.access_token().await;

        assert!(matches!(result, Err(AppError::NotConnected)));
    }

    #[tokio::test]
    async fn disconnect_revokes_and_deletes_refresh_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/revoke"))
            .and(body_string_contains("token=refresh-1"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
        let store = Arc::new(MemorySecretStore::with(REFRESH_TOKEN_KEY, "refresh-1"));
        let manager = TokenManager::new(
            reqwest::Client::new(),
            Some(client_for(&server)),
            store.clone(),
        );

        manager.disconnect().await.unwrap();

        assert_eq!(store.get(REFRESH_TOKEN_KEY).unwrap(), None);
    }

    #[tokio::test]
    async fn unconfigured_client_reports_not_configured() {
        let manager = TokenManager::new(
            reqwest::Client::new(),
            None,
            Arc::new(MemorySecretStore::with(REFRESH_TOKEN_KEY, "refresh-1")),
        );

        let result = manager.access_token().await;

        assert!(matches!(result, Err(AppError::NotConfigured)));
    }
}
