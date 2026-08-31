use std::fmt;

use keyring::Entry;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const SERVICE_NAME: &str = "sanket-ipo-provider";
const TOKEN_KEY: &str = "upstox:analytics-token:v1";
const MAX_TOKEN_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderCredentialKey {
    UpstoxAnalyticsToken,
}

impl ProviderCredentialKey {
    pub const fn service_name(self) -> &'static str {
        SERVICE_NAME
    }

    pub const fn key_id(self) -> &'static str {
        match self {
            Self::UpstoxAnalyticsToken => TOKEN_KEY,
        }
    }
}

/// An opaque, non-serializable provider secret.
pub struct SecretValue(Zeroizing<String>);

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }

    fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretValue(REDACTED)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderConnectionState {
    NotConnected,
    Connected,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProviderConnectionStatusDto {
    pub provider: &'static str,
    pub state: ProviderConnectionState,
    pub safe_message: Option<&'static str>,
}

impl ProviderConnectionStatusDto {
    pub const fn not_connected() -> Self {
        Self {
            provider: "UPSTOX_IPO_DATA",
            state: ProviderConnectionState::NotConnected,
            safe_message: None,
        }
    }

    const fn connected() -> Self {
        Self {
            provider: "UPSTOX_IPO_DATA",
            state: ProviderConnectionState::Connected,
            safe_message: None,
        }
    }

    const fn failed(message: &'static str) -> Self {
        Self {
            provider: "UPSTOX_IPO_DATA",
            state: ProviderConnectionState::Failed,
            safe_message: Some(message),
        }
    }
}

#[derive(Deserialize)]
pub struct ConnectUpstoxAnalyticsTokenRequest {
    pub token: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialError {
    EmptyCredential,
    CredentialTooLong,
    NoDurableBackend,
    KeyringUnavailable,
    KeyringLocked,
    StorageFailure,
}

impl CredentialError {
    pub const fn safe_message(self) -> &'static str {
        match self {
            Self::EmptyCredential => "Enter an Analytics Token.",
            Self::CredentialTooLong => "The Analytics Token is too long.",
            Self::NoDurableBackend => "OS keyring is unavailable.",
            Self::KeyringUnavailable | Self::KeyringLocked | Self::StorageFailure => {
                "Could not access the OS keyring."
            }
        }
    }
}

/// Provider credentials are intentionally separate from PAN identity keys.
/// There is no token getter: callers can only store, delete, or check status.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsProviderCredentialStore;

pub trait ProviderCredentialStore: Send + Sync {
    fn store(&self, key: ProviderCredentialKey, value: SecretValue) -> Result<(), CredentialError>;
    fn delete(&self, key: ProviderCredentialKey) -> Result<(), CredentialError>;
    fn is_configured(&self, key: ProviderCredentialKey) -> Result<bool, CredentialError>;
}

impl OsProviderCredentialStore {
    fn entry(&self, key: ProviderCredentialKey) -> Result<Entry, CredentialError> {
        Entry::new(key.service_name(), key.key_id())
            .map_err(|_| CredentialError::KeyringUnavailable)
    }

    fn ensure_durable_backend(&self, key: ProviderCredentialKey) -> Result<Entry, CredentialError> {
        let entry = self.entry(key)?;
        if entry
            .get_credential()
            .downcast_ref::<keyring::mock::MockCredential>()
            .is_some()
        {
            return Err(CredentialError::NoDurableBackend);
        }
        Ok(entry)
    }
}

impl ProviderCredentialStore for OsProviderCredentialStore {
    fn store(&self, key: ProviderCredentialKey, value: SecretValue) -> Result<(), CredentialError> {
        if value.as_str().trim().is_empty() {
            return Err(CredentialError::EmptyCredential);
        }
        if value.as_str().len() > MAX_TOKEN_BYTES {
            return Err(CredentialError::CredentialTooLong);
        }
        let entry = self.ensure_durable_backend(key)?;
        entry
            .set_password(value.as_str())
            .map_err(|error| match error {
                keyring::Error::NoStorageAccess(_) => CredentialError::KeyringLocked,
                _ => CredentialError::StorageFailure,
            })
    }

    fn delete(&self, key: ProviderCredentialKey) -> Result<(), CredentialError> {
        let entry = self.ensure_durable_backend(key)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(keyring::Error::NoStorageAccess(_)) => Err(CredentialError::KeyringLocked),
            Err(_) => Err(CredentialError::StorageFailure),
        }
    }

    fn is_configured(&self, key: ProviderCredentialKey) -> Result<bool, CredentialError> {
        let entry = self.ensure_durable_backend(key)?;
        match entry.get_password() {
            Ok(password) => {
                let _zeroized = Zeroizing::new(password);
                Ok(true)
            }
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(keyring::Error::NoStorageAccess(_)) => Err(CredentialError::KeyringLocked),
            Err(_) => Err(CredentialError::KeyringUnavailable),
        }
    }
}

pub fn status(store: &impl ProviderCredentialStore) -> ProviderConnectionStatusDto {
    match store.is_configured(ProviderCredentialKey::UpstoxAnalyticsToken) {
        Ok(true) => ProviderConnectionStatusDto::connected(),
        Ok(false) => ProviderConnectionStatusDto::not_connected(),
        Err(error) => ProviderConnectionStatusDto::failed(error.safe_message()),
    }
}

pub fn store_token(
    store: &impl ProviderCredentialStore,
    token: SecretValue,
) -> ProviderConnectionStatusDto {
    match store.store(ProviderCredentialKey::UpstoxAnalyticsToken, token) {
        Ok(()) => ProviderConnectionStatusDto::connected(),
        Err(error) => ProviderConnectionStatusDto::failed(error.safe_message()),
    }
}

pub fn disconnect(store: &impl ProviderCredentialStore) -> ProviderConnectionStatusDto {
    match store.delete(ProviderCredentialKey::UpstoxAnalyticsToken) {
        Ok(()) => ProviderConnectionStatusDto::not_connected(),
        Err(error) => ProviderConnectionStatusDto::failed(error.safe_message()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeStore {
        configured: std::sync::Mutex<bool>,
    }

    impl ProviderCredentialStore for FakeStore {
        fn store(
            &self,
            _key: ProviderCredentialKey,
            value: SecretValue,
        ) -> Result<(), CredentialError> {
            if value.as_str().is_empty() {
                return Err(CredentialError::EmptyCredential);
            }
            *self.configured.lock().unwrap() = true;
            Ok(())
        }

        fn delete(&self, _key: ProviderCredentialKey) -> Result<(), CredentialError> {
            *self.configured.lock().unwrap() = false;
            Ok(())
        }

        fn is_configured(&self, _key: ProviderCredentialKey) -> Result<bool, CredentialError> {
            Ok(*self.configured.lock().unwrap())
        }
    }

    #[test]
    fn store_status_disconnect_never_returns_token() {
        let store = FakeStore::default();
        let raw = "y".repeat(24);
        let connected = store_token(&store, SecretValue::new(raw.clone()));
        assert_eq!(connected.state, ProviderConnectionState::Connected);
        let json = serde_json::to_string(&connected).unwrap();
        assert!(!json.contains(&raw));
        assert_eq!(status(&store).state, ProviderConnectionState::Connected);
        assert_eq!(
            disconnect(&store).state,
            ProviderConnectionState::NotConnected
        );
    }
}
