use std::path::Path;

const CLIENT_FILE: &str = "google-oauth-client.json";
const ID_KEY: &str = "HOMEREEL_GOOGLE_CLIENT_ID";
const SECRET_KEY: &str = "HOMEREEL_GOOGLE_CLIENT_SECRET";

fn main() {
    embed_google_client_credentials();
    embed_target_triple();
    tauri_build::build()
}

/// `ffmpeg.rs` finds the downloaded sidecar by the target-triple name Tauri's externalBin uses.
fn embed_target_triple() {
    let target = std::env::var("TARGET").expect("Cargo sets TARGET for build scripts");
    println!("cargo:rustc-env=HOMEREEL_TARGET_TRIPLE={target}");
}

/// Google Desktop OAuth clients are public clients: their secret is not confidential, but it
/// still must not be committed. The JSON downloaded from Google Cloud (gitignored) or the build
/// environment provides it, and `config.rs` reads it back with `option_env!`.
fn embed_google_client_credentials() {
    println!("cargo:rerun-if-changed={CLIENT_FILE}");
    println!("cargo:rerun-if-env-changed={ID_KEY}");
    println!("cargo:rerun-if-env-changed={SECRET_KEY}");

    let (file_id, file_secret) = read_client_file(Path::new(CLIENT_FILE)).unwrap_or_default();
    let client_id = std::env::var(ID_KEY).ok().or(file_id);
    let client_secret = std::env::var(SECRET_KEY).ok().or(file_secret);

    if let Some(id) = client_id.filter(|v| !v.trim().is_empty()) {
        println!("cargo:rustc-env={ID_KEY}={}", id.trim());
    }
    if let Some(secret) = client_secret.filter(|v| !v.trim().is_empty()) {
        println!("cargo:rustc-env={SECRET_KEY}={}", secret.trim());
    }
}

/// Reads the `{"installed": {"client_id", "client_secret"}}` file Google Cloud offers for
/// Desktop app clients.
fn read_client_file(path: &Path) -> Option<(Option<String>, Option<String>)> {
    let text = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = match serde_json::from_str(&text) {
        Ok(json) => json,
        Err(error) => {
            println!("cargo:warning={CLIENT_FILE} is not valid JSON: {error}");
            return None;
        }
    };
    let installed = json.get("installed")?;
    let field = |name: &str| {
        installed
            .get(name)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    };
    Some((field("client_id"), field("client_secret")))
}
