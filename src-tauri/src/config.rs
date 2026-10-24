use crate::auth::oauth::OAuthClient;

pub const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";
pub const DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";

/// `drive.readonly` is required because frame extraction for AI titles reads file contents.
pub const DRIVE_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";

/// Service name for entries in Windows Credential Manager and the macOS keychain.
pub const KEYRING_SERVICE: &str = "com.homereel.desktop";
pub const GEMINI_KEY_NAME: &str = "gemini-api-key";

pub const DATABASE_FILE: &str = "homereel.sqlite3";

/// The OAuth client baked in at build time by `build.rs`, if one was provided.
pub fn google_client() -> Option<OAuthClient> {
    let client_id = option_env!("HOMEREEL_GOOGLE_CLIENT_ID")?;
    let client_secret = option_env!("HOMEREEL_GOOGLE_CLIENT_SECRET")?;
    Some(OAuthClient {
        client_id: client_id.to_owned(),
        client_secret: client_secret.to_owned(),
        auth_endpoint: GOOGLE_AUTH_ENDPOINT.to_owned(),
        token_endpoint: GOOGLE_TOKEN_ENDPOINT.to_owned(),
        revoke_endpoint: GOOGLE_REVOKE_ENDPOINT.to_owned(),
    })
}

/// Development convenience: `GEMINI_API_KEY` from the project's `.env`. Release builds never
/// read `.env`; there the key is entered in Settings and kept in the system credential store.
#[cfg(debug_assertions)]
pub fn dev_gemini_key() -> Option<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(".env");
    let text = std::fs::read_to_string(path).ok()?;
    env_file_value(&text, "GEMINI_API_KEY")
}

/// Reads `KEY=value` from `.env` text, allowing quotes, `export` and comments.
#[cfg(any(debug_assertions, test))]
pub fn env_file_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        let (name, value) = line.split_once('=')?;
        if name.trim() != key || line.starts_with('#') {
            return None;
        }
        let value = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_env_file_values() {
        let text = "# keys\nOTHER=1\nexport GEMINI_API_KEY = \"abc-123\"\n";

        assert_eq!(
            env_file_value(text, "GEMINI_API_KEY").as_deref(),
            Some("abc-123")
        );
        assert_eq!(env_file_value(text, "OTHER").as_deref(), Some("1"));
        assert_eq!(
            env_file_value("#GEMINI_API_KEY=x\nGEMINI_API_KEY=", "GEMINI_API_KEY"),
            None
        );
    }
}
