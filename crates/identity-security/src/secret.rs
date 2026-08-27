//! Unified sensitive identity payload: PAN plus optional UPI, encrypted as JSON.
//!
//! `IdentitySecret` is the plaintext that lives only inside the encryption
//! envelope (and transiently inside a purpose-scoped closure). Its `Debug`
//! redacts the UPI; the PAN field is redacted by `Pan` itself.

use serde::{Deserialize, Serialize};

use crate::pan::{Pan, PanError};

/// Errors produced while building or decoding an [`IdentitySecret`].
#[derive(Debug, thiserror::Error)]
pub enum IdentitySecretError {
    #[error("PAN is invalid")]
    Pan(#[from] PanError),
    #[error("UPI ID is invalid")]
    InvalidUpi,
    #[error("identity payload is invalid")]
    Payload,
    #[error("encryption failed: {0}")]
    Cipher(#[from] crate::crypto::CipherError),
}

/// The plaintext sensitive identity payload carried by an encrypted envelope.
///
/// Fields are public so callers can build the value directly; UPI validation is
/// enforced at the trust boundaries (encryption and decoding), which is where
/// untrusted input enters.
#[derive(Clone, PartialEq, Eq)]
pub struct IdentitySecret {
    pub pan: Pan,
    pub upi_id: Option<String>,
}

/// Wire form: the only serialized shape of the payload (inside ciphertext).
#[derive(Serialize, Deserialize)]
struct Wire {
    pan: String,
    upi_id: Option<String>,
}

impl IdentitySecret {
    /// Minimal fail-closed UPI validation: non-empty, no whitespace, exactly one
    /// `@` with non-empty local part and handle. Full NPCI validation is out of
    /// scope; this blocks obviously malformed values from entering the vault.
    pub fn validate_upi(upi: &str) -> Result<(), IdentitySecretError> {
        if upi.is_empty() || upi.chars().any(char::is_whitespace) {
            return Err(IdentitySecretError::InvalidUpi);
        }
        let mut parts = upi.split('@');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(local), Some(handle), None) if !local.is_empty() && !handle.is_empty() => Ok(()),
            _ => Err(IdentitySecretError::InvalidUpi),
        }
    }

    /// Serialize to the JSON wire form encrypted into the envelope.
    pub(crate) fn to_json(&self) -> String {
        let wire = Wire {
            pan: self.pan.as_normalized().to_owned(),
            upi_id: self.upi_id.clone(),
        };
        serde_json::to_string(&wire).expect("wire form always serializes")
    }

    /// Decode and re-validate the JSON wire form. Fails closed on any invalid
    /// field so corrupted payloads never produce a half-valid identity.
    pub(crate) fn from_json(json: &str) -> Result<Self, IdentitySecretError> {
        let wire: Wire = serde_json::from_str(json).map_err(|_| IdentitySecretError::Payload)?;
        let pan = Pan::parse(&wire.pan)?;
        if let Some(upi) = &wire.upi_id {
            Self::validate_upi(upi)?;
        }
        Ok(Self {
            pan,
            upi_id: wire.upi_id,
        })
    }
}

impl std::fmt::Debug for IdentitySecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentitySecret")
            .field("pan", &self.pan) // Pan's Debug is redacted
            .field("upi_id", &"[REDACTED]")
            .finish()
    }
}
