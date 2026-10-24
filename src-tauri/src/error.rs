use serde::ser::{Serialize, SerializeStruct, Serializer};

pub type AppResult<T> = Result<T, AppError>;

/// Error returned to the frontend as `{ code, message }`. Messages are user-facing and must
/// never contain tokens, API keys, or request URLs with credentials.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The build has no Google OAuth client (see `build.rs`), so users can't connect Drive.
    #[error("Drive connect is not available. Contact Homereel.")]
    NotConfigured,
    #[error("Google Drive is not connected.")]
    NotConnected,
    #[error("Google Drive access was revoked or has expired. Reconnect Google Drive.")]
    AuthRevoked,
    #[error("Authorization was cancelled.")]
    Cancelled,
    #[error("Could not reach Google: {0}")]
    Offline(String),
    #[error("Google is limiting requests right now. Try again shortly.")]
    RateLimited,
    #[error("The file was not found or is no longer accessible.")]
    NotFound,
    #[error("Access denied: {0}")]
    Forbidden(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotConfigured => "not_configured",
            AppError::NotConnected => "not_connected",
            AppError::AuthRevoked => "auth_revoked",
            AppError::Cancelled => "cancelled",
            AppError::Offline(_) => "offline",
            AppError::RateLimited => "rate_limited",
            AppError::NotFound => "not_found",
            AppError::Forbidden(_) => "forbidden",
            AppError::Internal(_) => "internal",
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        AppError::Internal(message.into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        // Strip the URL so query strings never reach logs or the UI.
        let error = error.without_url();
        if error.is_connect() || error.is_timeout() {
            AppError::Offline(error.to_string())
        } else {
            AppError::Internal(format!("Network request failed: {error}"))
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Internal(format!("I/O error: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_code_and_message() {
        let json = serde_json::to_value(AppError::AuthRevoked).unwrap();

        assert_eq!(json["code"], "auth_revoked");
        assert_eq!(
            json["message"],
            "Google Drive access was revoked or has expired. Reconnect Google Drive."
        );
    }
}
