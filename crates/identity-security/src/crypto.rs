//! Authenticated encryption for sensitive identity envelopes.

use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

pub const ENVELOPE_VERSION: u16 = 1;
pub const ALGORITHM: &str = "XChaCha20-Poly1305";

/// A 256-bit encryption key. Zeroized on drop; construction is explicit so keys
/// are never accidentally derived from a plain string.
#[derive(Clone)]
pub struct IdentityKey([u8; 32]);

impl IdentityKey {
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut bytes);
        Self(bytes)
    }
}

impl Zeroize for IdentityKey {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl Drop for IdentityKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// The encrypted, authenticated, and versioned identity envelope.
///
/// Serialization is by design metadata + ciphertext only: there is no plaintext
/// PAN field anywhere in this struct, so a derived `Serialize` cannot leak it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedIdentityEnvelope {
    pub version: u16,
    pub algorithm: String,
    pub key_id: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub created_at: String,
}

impl EncryptedIdentityEnvelope {
    pub fn version(&self) -> u16 {
        self.version
    }
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
    pub fn nonce(&self) -> &[u8] {
        &self.nonce
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("unsupported envelope version {0}")]
    UnsupportedVersion(u16),
    #[error("unsupported algorithm {0}")]
    UnsupportedAlgorithm(String),
    #[error("invalid nonce length {0}")]
    InvalidNonce(usize),
    #[error("authentication failed: ciphertext or key is wrong")]
    Authentication,
    #[error("nonce generation failed")]
    NonceFailure,
}

pub struct IdentityCipher {
    key: IdentityKey,
}

impl IdentityCipher {
    pub fn new(key: IdentityKey) -> Self {
        Self { key }
    }

    pub fn encrypt(
        &self,
        plaintext: &[u8],
        key_id: impl Into<String>,
    ) -> Result<EncryptedIdentityEnvelope, CipherError> {
        let mut nonce = [0u8; 24];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce);

        let cipher = XChaCha20Poly1305::new((&self.key.0).into());
        let ciphertext = cipher
            .encrypt(XNonce::from_slice(&nonce), plaintext)
            .map_err(|_| CipherError::NonceFailure)?;

        Ok(EncryptedIdentityEnvelope {
            version: ENVELOPE_VERSION,
            algorithm: ALGORITHM.to_owned(),
            key_id: key_id.into(),
            nonce: nonce.to_vec(),
            ciphertext,
            created_at: super::now_rfc3339(),
        })
    }

    pub fn decrypt(&self, envelope: &EncryptedIdentityEnvelope) -> Result<Vec<u8>, CipherError> {
        if envelope.version != ENVELOPE_VERSION {
            return Err(CipherError::UnsupportedVersion(envelope.version));
        }
        if envelope.algorithm != ALGORITHM {
            return Err(CipherError::UnsupportedAlgorithm(
                envelope.algorithm.clone(),
            ));
        }
        let nonce = XNonce::from_slice(&envelope.nonce);
        let cipher = XChaCha20Poly1305::new((&self.key.0).into());
        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &envelope.ciphertext,
                    aad: b"",
                },
            )
            .map_err(|_| CipherError::Authentication)
    }
}
