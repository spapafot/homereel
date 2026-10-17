pub mod client;
pub mod models;

/// Drive IDs are URL-safe base64-like strings. Validating them keeps arbitrary input out of
/// request paths built from IDs.
pub fn is_valid_drive_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_drive_ids_and_rejects_path_tricks() {
        assert!(is_valid_drive_id("1AbC-def_GHI2"));
        assert!(!is_valid_drive_id(""));
        assert!(!is_valid_drive_id("../files"));
        assert!(!is_valid_drive_id("abc?alt=media"));
    }
}
