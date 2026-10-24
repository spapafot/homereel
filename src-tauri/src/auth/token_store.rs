use std::collections::HashMap;

use keyring_core::{Entry, Error as KeyringError};

use crate::error::{AppError, AppResult};

/// Secure storage for long-lived secrets (OAuth refresh token, Gemini API key).
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> AppResult<Option<String>>;
    fn set(&self, key: &str, value: &str) -> AppResult<()>;
    fn delete(&self, key: &str) -> AppResult<()>;
}

/// Registers the system credential store (Windows Credential Manager, or the login keychain on
/// macOS) as the process-wide keyring store.
pub fn init_platform_store() -> AppResult<()> {
    #[cfg(windows)]
    {
        let store = windows_native_keyring_store::Store::new().map_err(keyring_error)?;
        keyring_core::set_default_store(store);
    }
    #[cfg(target_os = "macos")]
    {
        let store = apple_native_keyring_store::keychain::Store::new().map_err(keyring_error)?;
        keyring_core::set_default_store(store);
    }
    Ok(())
}

pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
        }
    }

    fn entry(&self, key: &str) -> AppResult<Entry> {
        // "Local" keeps the credential on this machine instead of roaming with the profile. The
        // macOS keychain store rejects modifiers it doesn't know, so it gets none.
        #[cfg(windows)]
        let modifiers = HashMap::from([("persistence", "Local")]);
        #[cfg(not(windows))]
        let modifiers = HashMap::new();
        Entry::new_with_modifiers(&self.service, key, &modifiers).map_err(keyring_error)
    }
}

impl SecretStore for KeyringSecretStore {
    fn get(&self, key: &str) -> AppResult<Option<String>> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(keyring_error(error)),
        }
    }

    fn set(&self, key: &str, value: &str) -> AppResult<()> {
        self.entry(key)?.set_password(value).map_err(keyring_error)
    }

    fn delete(&self, key: &str) -> AppResult<()> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(keyring_error(error)),
        }
    }
}

/// Describes a keyring failure without echoing stored bytes, which some variants carry.
fn keyring_error(error: KeyringError) -> AppError {
    let detail = match &error {
        KeyringError::PlatformFailure(cause) => cause.to_string(),
        KeyringError::NoStorageAccess(cause) => {
            format!("credential store is not accessible: {cause}")
        }
        KeyringError::NoDefaultStore => "no credential store is available on this platform".into(),
        KeyringError::TooLong(name, limit) => format!("{name} exceeds the {limit}-byte limit"),
        _ => "unexpected credential store error".into(),
    };
    AppError::internal(format!("Credential store error: {detail}"))
}

#[cfg(test)]
pub mod testing {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::SecretStore;
    use crate::error::AppResult;

    #[derive(Default)]
    pub struct MemorySecretStore {
        values: Mutex<HashMap<String, String>>,
    }

    impl MemorySecretStore {
        pub fn with(key: &str, value: &str) -> Self {
            let store = Self::default();
            store.set(key, value).unwrap();
            store
        }
    }

    impl SecretStore for MemorySecretStore {
        fn get(&self, key: &str) -> AppResult<Option<String>> {
            Ok(self.values.lock().unwrap().get(key).cloned())
        }

        fn set(&self, key: &str, value: &str) -> AppResult<()> {
            self.values
                .lock()
                .unwrap()
                .insert(key.to_owned(), value.to_owned());
            Ok(())
        }

        fn delete(&self, key: &str) -> AppResult<()> {
            self.values.lock().unwrap().remove(key);
            Ok(())
        }
    }
}
