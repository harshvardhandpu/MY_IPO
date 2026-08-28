//! OS-backed and in-memory key providers.
//!
//! Production mode requires a real, durable OS credential store (Linux
//! Secret Service / keyutils, Windows Credential Manager, macOS Keychain).
//! The `keyring` crate falls back to a non-persistent *mock* store when no
//! platform feature applies to a target; that mock must never be treated as
//! secure, so this module fails closed whenever the active backend is the
//! mock.

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use keyring::Entry;
use serde::{Deserialize, Serialize};

use crate::crypto::IdentityKey;

/// Opaque key id.
pub type KeyId = String;

/// Runtime security posture for sensitive identity operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeSecurityMode {
    /// Real member PAN allowed only with OS-backed keys.
    ProductionSecure,
    /// Synthetic fixtures / local development only.
    DevelopmentSynthetic,
}

impl RuntimeSecurityMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProductionSecure => "PRODUCTION_SECURE",
            Self::DevelopmentSynthetic => "DEVELOPMENT_SYNTHETIC",
        }
    }

    /// Strict parse: only an explicit production token selects production.
    /// Anything unrecognized fails closed to development-synthetic, because
    /// selecting real-PAN mode by accident is more dangerous than staying
    /// locked down.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "PRODUCTION_SECURE" | "PRODUCTION" | "SECURE" => Self::ProductionSecure,
            _ => Self::DevelopmentSynthetic,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KeyProviderClass {
    OsSecure,
    InMemoryDev,
}

#[derive(Debug, thiserror::Error)]
pub enum KeyProviderError {
    #[error("keyring unavailable: {0}")]
    KeyringUnavailable(String),
    #[error("keyring store inaccessible or locked: {0}")]
    KeyringLocked(String),
    #[error("key not found: {0}")]
    MissingKey(String),
    #[error("invalid key material: {0}")]
    InvalidKey(String),
    #[error("no durable OS credential store available on this platform")]
    NoDurableBackend,
    #[error("production secure mode rejects non-OS key provider")]
    InsecureProviderRejected,
    #[error("keyring I/O failed: {0}")]
    Io(String),
}

/// Resolves and optionally stores identity keys.
pub trait KeyProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn provider_class(&self) -> KeyProviderClass;
    fn key(&self, key_id: &str) -> Result<IdentityKey, KeyProviderError>;
    fn store_key(&self, key_id: &str, key: &IdentityKey) -> Result<(), KeyProviderError>;
    fn delete_key(&self, key_id: &str) -> Result<(), KeyProviderError> {
        let _ = key_id;
        Ok(())
    }
}

/// Ensure production mode never uses in-memory/dev providers.
pub fn assert_mode_allows_provider(
    mode: RuntimeSecurityMode,
    provider: &dyn KeyProvider,
) -> Result<(), KeyProviderError> {
    match mode {
        RuntimeSecurityMode::DevelopmentSynthetic => Ok(()),
        RuntimeSecurityMode::ProductionSecure => {
            if provider.provider_class() != KeyProviderClass::OsSecure {
                return Err(KeyProviderError::InsecureProviderRejected);
            }
            Ok(())
        }
    }
}

/// Development/test provider — forbidden in ProductionSecure for real PAN.
pub struct InMemoryKeyProvider {
    key_id: KeyId,
    key: IdentityKey,
}

impl InMemoryKeyProvider {
    pub fn new(key_id: impl Into<KeyId>, key: IdentityKey) -> Self {
        Self {
            key_id: key_id.into(),
            key,
        }
    }
}

impl KeyProvider for InMemoryKeyProvider {
    fn provider_id(&self) -> &'static str {
        "in-memory-dev"
    }

    fn provider_class(&self) -> KeyProviderClass {
        KeyProviderClass::InMemoryDev
    }

    fn key(&self, key_id: &str) -> Result<IdentityKey, KeyProviderError> {
        if key_id == self.key_id {
            Ok(self.key.clone())
        } else {
            Err(KeyProviderError::MissingKey(key_id.to_owned()))
        }
    }

    fn store_key(&self, _key_id: &str, _key: &IdentityKey) -> Result<(), KeyProviderError> {
        // In-memory provider is fixed at construction for tests.
        Err(KeyProviderError::Io(
            "in-memory provider does not persist new keys".into(),
        ))
    }
}

const SERVICE_NAME: &str = "sanket-ipo-identity";

/// Linux Secret Service / Windows Credential Manager / macOS Keychain via `keyring`.
///
/// This provider refuses to operate when the entry it uses is backed by the
/// keyring *mock* store. The keyring crate silently selects a non-persistent
/// mock store when no platform feature applies to a target; that mock deletes
/// generated keys between accesses, so treating it as secure would make
/// enrolled ciphertext undecryptable and misrepresent the security posture.
/// Every read/write therefore runs the runtime backend check below and fails
/// closed (`NoDurableBackend`) on a mock.
pub struct OsKeyringKeyProvider {
    service: String,
}

impl OsKeyringKeyProvider {
    pub fn new() -> Self {
        Self {
            service: SERVICE_NAME.into(),
        }
    }

    #[cfg(test)]
    fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key_id: &str) -> Result<Entry, KeyProviderError> {
        Entry::new(&self.service, key_id)
            .map_err(|e| KeyProviderError::KeyringUnavailable(format!("open entry failed: {e}")))
    }

    /// The mock credential store is non-persistent and deletes generated keys
    /// immediately. Its presence at runtime means no durable OS store is
    /// active, so production must not proceed.
    fn is_mock_backend(&self, key_id: &str) -> Result<bool, KeyProviderError> {
        let entry = self.entry(key_id)?;
        Ok(entry
            .get_credential()
            .downcast_ref::<keyring::mock::MockCredential>()
            .is_some())
    }

    fn ensure_durable_backend(&self, key_id: &str) -> Result<(), KeyProviderError> {
        if self.is_mock_backend(key_id)? {
            return Err(KeyProviderError::NoDurableBackend);
        }
        Ok(())
    }

    /// Ensure a key exists; generate and store if missing.
    ///
    /// A locked/unavailable/corrupted store is *not* a signal to generate a
    /// new key over it — that would silently replace production key material.
    /// Only a positively-confirmed missing key triggers generation.
    pub fn ensure_key(&self, key_id: &str) -> Result<IdentityKey, KeyProviderError> {
        self.ensure_durable_backend(key_id)?;
        match self.key(key_id) {
            Ok(k) => Ok(k),
            Err(KeyProviderError::MissingKey(_)) => {
                let k = IdentityKey::generate();
                self.store_key(key_id, &k)?;
                Ok(k)
            }
            Err(e) => Err(e),
        }
    }
}

impl Default for OsKeyringKeyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyProvider for OsKeyringKeyProvider {
    fn provider_id(&self) -> &'static str {
        "os-keyring"
    }

    /// `OsSecure` is asserted here because the *runtime* backend check in
    /// `ensure_key`/`key`/`store_key` is what actually enforces durability.
    /// Every real operation fails closed on a mock backend.
    fn provider_class(&self) -> KeyProviderClass {
        KeyProviderClass::OsSecure
    }

    fn key(&self, key_id: &str) -> Result<IdentityKey, KeyProviderError> {
        self.ensure_durable_backend(key_id)?;
        let entry = self.entry(key_id)?;
        let material = entry.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => KeyProviderError::MissingKey(key_id.to_owned()),
            keyring::Error::NoStorageAccess(inner) => {
                KeyProviderError::KeyringLocked(inner.to_string())
            }
            other => KeyProviderError::KeyringUnavailable(other.to_string()),
        })?;
        let bytes = B64
            .decode(material.trim())
            .map_err(|_| KeyProviderError::InvalidKey("not base64".into()))?;
        if bytes.len() != 32 {
            return Err(KeyProviderError::InvalidKey(format!(
                "expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(IdentityKey::from_bytes(&arr))
    }

    fn store_key(&self, key_id: &str, key: &IdentityKey) -> Result<(), KeyProviderError> {
        self.ensure_durable_backend(key_id)?;
        let entry = self.entry(key_id)?;
        let encoded = B64.encode(key.as_bytes());
        entry.set_password(&encoded).map_err(|e| match e {
            keyring::Error::NoStorageAccess(inner) => {
                KeyProviderError::KeyringLocked(inner.to_string())
            }
            other => KeyProviderError::Io(other.to_string()),
        })
    }

    fn delete_key(&self, key_id: &str) -> Result<(), KeyProviderError> {
        self.ensure_durable_backend(key_id)?;
        let entry = self.entry(key_id)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(KeyProviderError::Io(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_rejects_in_memory() {
        let p = InMemoryKeyProvider::new("k", IdentityKey::generate());
        let err = assert_mode_allows_provider(RuntimeSecurityMode::ProductionSecure, &p)
            .expect_err("must reject");
        assert!(matches!(err, KeyProviderError::InsecureProviderRejected));
    }

    #[test]
    fn development_allows_in_memory() {
        let p = InMemoryKeyProvider::new("k", IdentityKey::generate());
        assert_mode_allows_provider(RuntimeSecurityMode::DevelopmentSynthetic, &p).unwrap();
    }

    #[test]
    fn parse_defaults_to_development_not_production() {
        assert_eq!(
            RuntimeSecurityMode::parse(""),
            RuntimeSecurityMode::DevelopmentSynthetic
        );
        assert_eq!(
            RuntimeSecurityMode::parse("totally-unknown"),
            RuntimeSecurityMode::DevelopmentSynthetic
        );
        assert_eq!(
            RuntimeSecurityMode::parse("PRODUCTION_SECURE"),
            RuntimeSecurityMode::ProductionSecure
        );
    }

    #[test]
    fn mock_detection_reports_mock_backend() {
        // In a headless test environment with no D-Bus Secret Service, the
        // keyring crate falls back to the mock store. This must be detected
        // as non-durable, never silently accepted as `OsSecure`.
        let p = OsKeyringKeyProvider::with_service("sanket-ipo-test-unit");
        if p.is_mock_backend("probe-key").unwrap_or(false) {
            let err = p.ensure_key("probe-key").map(|_k| ()).unwrap_err();
            assert!(matches!(err, KeyProviderError::NoDurableBackend));
        }
    }
}
