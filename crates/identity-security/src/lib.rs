//! Sensitive identity security for Sanket IPO.
//!
//! This crate owns every type that may touch plaintext PAN (or future sensitive
//! secrets): PAN validation/masking, redacted secret wrappers, an authenticated
//! encryption envelope, a key-provider abstraction, and a purpose-scoped,
//! audited access service. Nothing here must ever serialize, display, or log a
//! full PAN by accident.

mod pan;
mod redacted;
mod secret;

pub mod crypto;

pub use crypto::{CipherError, EncryptedIdentityEnvelope, IdentityCipher, IdentityKey};
pub use pan::{MaskedPan, Pan, PanError};
pub use redacted::Redacted;
pub use secret::{IdentitySecret, IdentitySecretError};

use serde::{Deserialize, Serialize};

/// A purpose for which a sensitive value may be temporarily decrypted.
///
/// New purposes must be added deliberately; unknown values are rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SensitivePurpose {
    /// Registrar allotment checking requires the full PAN locally.
    AllotmentCheck,
    /// Any unrecognized purpose (rejected by the access service).
    #[serde(other)]
    Unknown,
}

/// An opaque, stable identifier for a key in a [`KeyProvider`] implementation.
pub type KeyId = String;

/// A provider that resolves stable key identifiers to [`IdentityKey`] values.
///
/// Production implementations will back this with Windows Credential Manager or
/// the Linux Secret Service. Phase 2A ships an in-memory provider for tests and
/// development; the OS-keyring implementations are documented but deferred.
pub trait KeyProvider: Send + Sync {
    fn key(&self, key_id: &str) -> Option<IdentityKey>;
}

/// A development/test key provider that holds keys in memory only.
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
    fn key(&self, key_id: &str) -> Option<IdentityKey> {
        if key_id == self.key_id {
            Some(self.key.clone())
        } else {
            None
        }
    }
}

/// A minimally-descriptive, non-sensitive reference to an identity record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SensitiveIdentityRecord {
    account_id: String,
    masked_pan: MaskedPan,
    envelope: EncryptedIdentityEnvelope,
}

impl SensitiveIdentityRecord {
    /// Build a record that stores only the encrypted PAN; the plaintext is gone
    /// from this struct the moment the envelope is produced.
    pub fn encrypt_pan(
        pan: Pan,
        account_id: impl Into<String>,
        cipher: &IdentityCipher,
        key_id: impl Into<String>,
    ) -> Result<Self, CipherError> {
        let masked = pan.mask();
        let envelope = cipher.encrypt(pan.as_normalized().as_bytes(), key_id)?;
        Ok(Self {
            account_id: account_id.into(),
            masked_pan: masked,
            envelope,
        })
    }

    /// Build a record from the full identity payload (PAN + optional UPI).
    /// The payload is serialized to JSON and encrypted; only ciphertext remains.
    pub fn encrypt_identity(
        secret: IdentitySecret,
        account_id: impl Into<String>,
        cipher: &IdentityCipher,
        key_id: impl Into<String>,
    ) -> Result<Self, CipherError> {
        let masked = secret.pan.mask();
        let envelope = cipher.encrypt(secret.to_json().as_bytes(), key_id)?;
        Ok(Self {
            account_id: account_id.into(),
            masked_pan: masked,
            envelope,
        })
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn masked_pan(&self) -> &MaskedPan {
        &self.masked_pan
    }

    pub fn envelope(&self) -> &EncryptedIdentityEnvelope {
        &self.envelope
    }
}

// `Debug` is derived above: `MaskedPan` and the envelope both redact their
// payloads, so the derived `Debug` never leaks the PAN. Keep it that way.

/// Audit record emitted whenever a sensitive value is decrypted for a purpose.
///
/// Deliberately contains only identifiers and metadata — never the value itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensitiveAccessAudit {
    pub account_id: String,
    pub purpose: String,
    pub occurred_at: String,
}

/// Errors the access service can produce.
#[derive(Debug, thiserror::Error)]
pub enum AccessError {
    #[error("purpose {0} is not authorized")]
    UnauthorizedPurpose(String),
    #[error("encryption/decryption failed: {0}")]
    Cipher(#[from] CipherError),
    #[error("no key available for key id {0}")]
    MissingKey(String),
    #[error("identity payload is invalid")]
    Payload,
}

/// Purpose-scoped sensitive access.
///
/// The only safe way to touch a plaintext PAN: authorize, load, decrypt transiently,
/// and audit — all without ever handing the plaintext to a long-lived caller.
pub struct SensitiveIdentityService {
    provider: Box<dyn KeyProvider>,
    last_audit: Option<SensitiveAccessAudit>,
}

impl SensitiveIdentityService {
    pub fn new(provider: impl KeyProvider + 'static) -> Self {
        Self {
            provider: Box::new(provider),
            last_audit: None,
        }
    }

    /// Decrypt the record's PAN only for the duration of `f`, then drop it.
    /// Works with both raw-PAN envelopes (Phase 2A) and JSON identity payloads.
    pub fn with_pan<F, R>(
        &mut self,
        record: &SensitiveIdentityRecord,
        purpose: SensitivePurpose,
        actor_account_id: &str,
        f: F,
    ) -> Result<R, AccessError>
    where
        F: FnOnce(&str) -> R,
    {
        let plaintext = self.decrypt_authorized(record, purpose, actor_account_id)?;
        let secret = decode_payload(&plaintext)?;
        Ok(f(secret.pan.as_normalized()))
    }

    /// Decrypt the full identity payload (PAN + UPI) for the duration of `f`.
    pub fn with_identity<F, R>(
        &mut self,
        record: &SensitiveIdentityRecord,
        purpose: SensitivePurpose,
        actor_account_id: &str,
        f: F,
    ) -> Result<R, AccessError>
    where
        F: FnOnce(&IdentitySecret) -> R,
    {
        let plaintext = self.decrypt_authorized(record, purpose, actor_account_id)?;
        let secret = decode_payload(&plaintext)?;
        Ok(f(&secret))
    }

    /// Shared authorize → resolve key → decrypt path with audit emission.
    fn decrypt_authorized(
        &mut self,
        record: &SensitiveIdentityRecord,
        purpose: SensitivePurpose,
        actor_account_id: &str,
    ) -> Result<Vec<u8>, AccessError> {
        let purpose_str = match purpose {
            SensitivePurpose::AllotmentCheck => "ALLOTMENT_CHECK",
            SensitivePurpose::Unknown => {
                return Err(AccessError::UnauthorizedPurpose("UNKNOWN".to_owned()));
            }
        };

        let key_id = record.envelope().key_id();
        let key = self
            .provider
            .key(key_id)
            .ok_or_else(|| AccessError::MissingKey(key_id.to_owned()))?;
        let decrypt_cipher = IdentityCipher::new(key);
        let plaintext = decrypt_cipher.decrypt(record.envelope())?;

        self.last_audit = Some(SensitiveAccessAudit {
            account_id: actor_account_id.to_owned(),
            purpose: purpose_str.to_owned(),
            occurred_at: now_rfc3339(),
        });

        Ok(plaintext)
    }

    /// The audit record for the most recent access, for the caller to persist.
    pub fn take_last_audit(&mut self) -> Option<SensitiveAccessAudit> {
        self.last_audit.take()
    }
}

/// Decode a decrypted payload into an [`IdentitySecret`].
///
/// Supports both envelope formats: Phase 2A raw-PAN bytes and Phase 2B JSON
/// payloads. JSON is tried first; anything that is not valid JSON falls back to
/// raw-PAN interpretation. Invalid PAN content fails closed.
fn decode_payload(plaintext: &[u8]) -> Result<IdentitySecret, AccessError> {
    let text = std::str::from_utf8(plaintext).map_err(|_| AccessError::Payload)?;
    if let Ok(secret) = IdentitySecret::from_json(text) {
        return Ok(secret);
    }
    let pan = Pan::parse(text).map_err(|_| AccessError::Payload)?;
    Ok(IdentitySecret { pan, upi_id: None })
}

fn now_rfc3339() -> String {
    // std-only RFC3339 UTC timestamp; avoids pulling a clock dependency here.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    let sec_of_day = secs % 86_400;
    let (hour, minute, second) = (sec_of_day / 3600, (sec_of_day % 3600) / 60, sec_of_day % 60);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    // Howard Hinnant's `civil_from_days` algorithm.
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    #[test]
    fn rfc3339_shape_is_stable() {
        let s = super::now_rfc3339();
        assert_eq!(s.len(), 20);
        assert!(s.ends_with('Z'));
    }
}
