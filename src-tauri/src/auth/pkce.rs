use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Returns `byte_len` bytes from the OS random generator, base64url-encoded without padding.
pub fn random_token(byte_len: usize) -> AppResult<String> {
    let mut bytes = vec![0u8; byte_len];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::internal(format!("Random number generator failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// PKCE (RFC 7636) verifier and its S256 challenge.
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn generate() -> AppResult<Self> {
        // 32 random bytes encode to a 43-character verifier, the minimum RFC 7636 allows.
        let verifier = random_token(32)?;
        let challenge = challenge_for(&verifier);
        Ok(Self {
            verifier,
            challenge,
        })
    }
}

pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_matches_rfc_7636_example() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

        let challenge = challenge_for(verifier);

        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn generated_verifier_is_url_safe_and_long_enough() {
        let pkce = Pkce::generate().unwrap();

        assert_eq!(pkce.verifier.len(), 43);
        assert!(
            pkce.verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        assert_eq!(pkce.challenge, challenge_for(&pkce.verifier));
    }

    #[test]
    fn random_tokens_differ() {
        assert_ne!(random_token(16).unwrap(), random_token(16).unwrap());
    }
}
