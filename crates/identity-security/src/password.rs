//! Password credential hashing and verification.
//!
//! This module owns only the verifier metadata. It does not establish sessions,
//! call services, or persist events. Passwords are kept in an owned, zeroizing
//! wrapper while they are in this crate; only the salted PHC verifier leaves it.

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier as _, Version,
};
use serde::{Deserialize, Deserializer, Serialize};
use zeroize::Zeroizing;

/// Version of the verifier policy stored alongside the verifier in auth events.
pub const PASSWORD_VERIFIER_VERSION: u16 = 1;
/// Argon2id v1.3, encoded as the PHC `v=19` version field.
pub const ARGON2_VERSION: &str = "v=19";
/// Fixed memory cost in KiB for the approved policy.
pub const ARGON2_MEMORY_KIB: u32 = 19_456;
/// Fixed number of Argon2 passes for the approved policy.
pub const ARGON2_TIME_COST: u32 = 2;
/// Fixed Argon2 parallelism for the approved policy.
pub const ARGON2_LANES: u32 = 1;

const POLICY_PREFIX: &str = "$argon2id$v=19$m=19456,t=2,p=1$";

/// An owned password that never reveals its bytes through formatting or serde.
pub struct Password(Zeroizing<Vec<u8>>);

impl Password {
    /// Copy password bytes into zeroizing storage for the duration of credential work.
    pub fn new(password: impl AsRef<[u8]>) -> Self {
        Self(Zeroizing::new(password.as_ref().to_vec()))
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password([REDACTED])")
    }
}

/// A serialized Argon2id PHC verifier containing only salted password metadata.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PasswordVerifier(String);

impl PasswordVerifier {
    /// Return the verifier string for authenticated vault-event metadata.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for PasswordVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordVerifier([REDACTED_METADATA])")
    }
}

impl<'de> Deserialize<'de> for PasswordVerifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if is_approved_verifier(&value) {
            Ok(Self(value))
        } else {
            Err(serde::de::Error::custom("invalid password verifier"))
        }
    }
}

/// Generic operation failure. Verification itself intentionally has no detailed
/// error channel, so wrong and malformed credentials are indistinguishable.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("password credential operation failed")]
pub struct PasswordCredentialError;

/// Generic result of credential verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordVerification {
    Verified,
    Rejected,
}

/// Hash a password into a salted, versioned Argon2id PHC verifier.
pub fn hash_password(password: &Password) -> Result<PasswordVerifier, PasswordCredentialError> {
    let password_bytes = Zeroizing::new(password.as_bytes().to_vec());
    configured_argon2()
        .map_err(|_| PasswordCredentialError)?
        .hash_password(&password_bytes)
        .map(|hash| PasswordVerifier(hash.to_string()))
        .map_err(|_| PasswordCredentialError)
}

/// Verify a password, exposing only a generic verified/rejected classification.
pub fn verify_password(password: &Password, verifier: &str) -> PasswordVerification {
    if !is_approved_verifier(verifier) {
        return PasswordVerification::Rejected;
    }

    let password_bytes = Zeroizing::new(password.as_bytes().to_vec());
    let Ok(parsed) = PasswordHash::new(verifier) else {
        return PasswordVerification::Rejected;
    };

    match configured_argon2() {
        Ok(argon2) if argon2.verify_password(&password_bytes, &parsed).is_ok() => {
            PasswordVerification::Verified
        }
        _ => PasswordVerification::Rejected,
    }
}

fn configured_argon2() -> Result<Argon2<'static>, argon2::Error> {
    let params = Params::new(ARGON2_MEMORY_KIB, ARGON2_TIME_COST, ARGON2_LANES, None)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn is_approved_verifier(value: &str) -> bool {
    value.starts_with(POLICY_PREFIX) && PasswordHash::new(value).is_ok()
}
