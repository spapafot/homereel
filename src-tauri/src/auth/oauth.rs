use serde::Deserialize;
use url::Url;

use crate::config::DRIVE_READONLY_SCOPE;
use crate::error::{AppError, AppResult};

/// A Google "Desktop app" OAuth client and the endpoints it talks to.
#[derive(Clone)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub auth_endpoint: String,
    pub token_endpoint: String,
    pub revoke_endpoint: String,
}

/// Token endpoint response. Deliberately not `Debug` so tokens cannot end up in logs.
#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: String,
}

#[derive(Deserialize)]
struct TokenErrorResponse {
    error: String,
}

pub fn build_auth_url(
    client: &OAuthClient,
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
) -> AppResult<Url> {
    let mut url = Url::parse(&client.auth_endpoint)
        .map_err(|error| AppError::internal(format!("Invalid auth endpoint: {error}")))?;
    url.query_pairs_mut()
        .append_pair("client_id", &client.client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", DRIVE_READONLY_SCOPE)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        // Offline access plus forced consent guarantees a refresh token, even on reconnect.
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    Ok(url)
}

pub async fn exchange_code(
    http: &reqwest::Client,
    client: &OAuthClient,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> AppResult<TokenResponse> {
    post_token_request(
        http,
        &client.token_endpoint,
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", redirect_uri),
            ("client_id", &client.client_id),
            ("client_secret", &client.client_secret),
        ],
    )
    .await
}

pub async fn refresh_access_token(
    http: &reqwest::Client,
    client: &OAuthClient,
    refresh_token: &str,
) -> AppResult<TokenResponse> {
    post_token_request(
        http,
        &client.token_endpoint,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &client.client_id),
            ("client_secret", &client.client_secret),
        ],
    )
    .await
}

pub async fn revoke_token(
    http: &reqwest::Client,
    client: &OAuthClient,
    token: &str,
) -> AppResult<()> {
    let response = http
        .post(&client.revoke_endpoint)
        .form(&[("token", token)])
        .send()
        .await?;
    // 400 means the token was already invalid, which is the outcome we wanted.
    if response.status().is_success() || response.status() == reqwest::StatusCode::BAD_REQUEST {
        Ok(())
    } else {
        Err(AppError::internal(format!(
            "Google rejected the revoke request ({})",
            response.status()
        )))
    }
}

pub fn has_drive_scope(granted_scopes: &str) -> bool {
    granted_scopes
        .split_whitespace()
        .any(|scope| scope == DRIVE_READONLY_SCOPE)
}

async fn post_token_request(
    http: &reqwest::Client,
    endpoint: &str,
    form: &[(&str, &str)],
) -> AppResult<TokenResponse> {
    let response = http.post(endpoint).form(form).send().await?;
    let status = response.status();
    if status.is_success() {
        return response.json::<TokenResponse>().await.map_err(|error| {
            AppError::internal(format!(
                "Unexpected token response: {}",
                error.without_url()
            ))
        });
    }

    let error_body = response.json::<TokenErrorResponse>().await.ok();
    match (status, error_body) {
        (_, Some(body)) if body.error == "invalid_grant" => Err(AppError::AuthRevoked),
        (reqwest::StatusCode::TOO_MANY_REQUESTS, _) => Err(AppError::RateLimited),
        (_, Some(body)) => Err(AppError::internal(format!(
            "Google rejected the token request ({status}: {})",
            body.error
        ))),
        (_, None) => Err(AppError::internal(format!(
            "Google rejected the token request ({status})"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_client(server_uri: &str) -> OAuthClient {
        OAuthClient {
            client_id: "client-123.apps.googleusercontent.com".into(),
            client_secret: "secret-xyz".into(),
            auth_endpoint: "https://accounts.example.com/o/oauth2/v2/auth".into(),
            token_endpoint: format!("{server_uri}/token"),
            revoke_endpoint: format!("{server_uri}/revoke"),
        }
    }

    #[test]
    fn auth_url_contains_pkce_offline_access_and_readonly_scope() {
        let client = test_client("http://unused");

        let url = build_auth_url(&client, "http://127.0.0.1:5555", "challenge", "state-1").unwrap();

        let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
        assert_eq!(params["client_id"], "client-123.apps.googleusercontent.com");
        assert_eq!(params["redirect_uri"], "http://127.0.0.1:5555");
        assert_eq!(params["response_type"], "code");
        assert_eq!(params["scope"], DRIVE_READONLY_SCOPE);
        assert_eq!(params["code_challenge"], "challenge");
        assert_eq!(params["code_challenge_method"], "S256");
        assert_eq!(params["state"], "state-1");
        assert_eq!(params["access_type"], "offline");
        assert_eq!(params["prompt"], "consent");
        assert!(!params.contains_key("client_secret"));
    }

    #[test]
    fn detects_drive_scope_among_granted_scopes() {
        assert!(has_drive_scope(&format!("openid {DRIVE_READONLY_SCOPE}")));
        assert!(!has_drive_scope(
            "https://www.googleapis.com/auth/drive.metadata.readonly"
        ));
        assert!(!has_drive_scope(""));
    }

    #[tokio::test]
    async fn exchange_sends_verifier_and_secret_and_parses_tokens() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code_verifier=verifier-1"))
            .and(body_string_contains("client_secret=secret-xyz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "access-1",
                "expires_in": 3599,
                "refresh_token": "refresh-1",
                "scope": DRIVE_READONLY_SCOPE,
                "token_type": "Bearer"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let client = test_client(&server.uri());

        let tokens = exchange_code(
            &reqwest::Client::new(),
            &client,
            "code-1",
            "verifier-1",
            "http://127.0.0.1:5555",
        )
        .await
        .unwrap();

        assert_eq!(tokens.access_token, "access-1");
        assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-1"));
        assert_eq!(tokens.expires_in, 3599);
    }

    #[tokio::test]
    async fn invalid_grant_maps_to_auth_revoked() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": "Token has been expired or revoked."
            })))
            .mount(&server)
            .await;
        let client = test_client(&server.uri());

        let result = refresh_access_token(&reqwest::Client::new(), &client, "refresh-1").await;

        assert!(matches!(result, Err(AppError::AuthRevoked)));
    }
}
