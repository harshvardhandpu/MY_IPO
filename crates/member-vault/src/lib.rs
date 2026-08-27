//! Private operational storage: encrypted identity envelopes and event persistence.
//!
//! This is the *MemberVault* — the only crate that persists encrypted sensitive
//! identity envelopes. It must never be handed to AI services, and its persistent
//! contents are ciphertext (or metadata) only, never plaintext PAN.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use sanket_domain::EventEnvelope;
use sanket_identity_security::EncryptedIdentityEnvelope;
use thiserror::Error;

/// The deterministic subdirectory layout for encrypted identity envelopes.
///
/// ```text
/// MemberVault/
///   _secure_identity/
///     <member-id>.enc
///     friends/
///       <friend-id>.enc
/// ```
pub const SECURE_IDENTITY_DIR: &str = "_secure_identity";
pub const FRIENDS_DIR: &str = "friends";

#[derive(Debug, Error)]
pub enum MemberVaultError {
    #[error("member vault I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("member vault serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid member/friend id: {0}")]
    InvalidId(String),
    #[error("unsupported identity envelope version {0}")]
    UnsupportedVersion(u16),
}

/// A filesystem-backed MemberVault. All writes are atomic and owner-only.
pub struct MemberVault {
    root: PathBuf,
}

impl MemberVault {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, MemberVaultError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn identity_dir(&self) -> PathBuf {
        self.root.join(SECURE_IDENTITY_DIR)
    }

    fn friends_dir(&self) -> PathBuf {
        self.identity_dir().join(FRIENDS_DIR)
    }

    fn validate_id(id: &str) -> Result<(), MemberVaultError> {
        // Reject ids that could escape the vault directory through path traversal.
        if id.trim().is_empty()
            || id.contains('/')
            || id.contains('\\')
            || id.contains("..")
            || id == "."
        {
            return Err(MemberVaultError::InvalidId(id.to_owned()));
        }
        Ok(())
    }

    /// Persist an encrypted identity envelope for a member, atomically.
    pub fn store_member_identity(
        &self,
        member_id: &str,
        envelope: &EncryptedIdentityEnvelope,
    ) -> Result<(), MemberVaultError> {
        Self::validate_id(member_id)?;
        let dir = self.identity_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{member_id}.enc"));
        Ok(write_atomically(&path, &serde_json::to_vec(envelope)?)?)
    }

    /// Persist an encrypted identity envelope for a friend account.
    pub fn store_friend_identity(
        &self,
        friend_id: &str,
        envelope: &EncryptedIdentityEnvelope,
    ) -> Result<(), MemberVaultError> {
        Self::validate_id(friend_id)?;
        let dir = self.friends_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{friend_id}.enc"));
        Ok(write_atomically(&path, &serde_json::to_vec(envelope)?)?)
    }

    /// Load and validate a member identity envelope.
    pub fn load_member_identity(
        &self,
        member_id: &str,
    ) -> Result<EncryptedIdentityEnvelope, MemberVaultError> {
        Self::validate_id(member_id)?;
        let path = self.identity_dir().join(format!("{member_id}.enc"));
        let bytes = fs::read(path)?;
        let envelope: EncryptedIdentityEnvelope = serde_json::from_slice(&bytes)?;
        validate_envelope(&envelope)?;
        Ok(envelope)
    }

    /// Load and validate a friend identity envelope.
    pub fn load_friend_identity(
        &self,
        friend_id: &str,
    ) -> Result<EncryptedIdentityEnvelope, MemberVaultError> {
        Self::validate_id(friend_id)?;
        let path = self.friends_dir().join(format!("{friend_id}.enc"));
        let bytes = fs::read(path)?;
        let envelope: EncryptedIdentityEnvelope = serde_json::from_slice(&bytes)?;
        validate_envelope(&envelope)?;
        Ok(envelope)
    }

    /// Append an event envelope. Events are metadata/appends only — no plaintext PAN.
    pub fn append_event(&self, event: &EventEnvelope) -> Result<(), MemberVaultError> {
        // Events are stored under a dedicated `_events/` directory keyed by event id.
        let dir = self.root.join("_events");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", event.event_id()));
        Ok(write_atomically(&path, &serde_json::to_vec(event)?)?)
    }
}

fn validate_envelope(envelope: &EncryptedIdentityEnvelope) -> Result<(), MemberVaultError> {
    // Corruption/version gate: reject anything the current build can't understand.
    if envelope.version() != sanket_identity_security::crypto::ENVELOPE_VERSION {
        return Err(MemberVaultError::UnsupportedVersion(envelope.version()));
    }
    if envelope.ciphertext().is_empty() {
        return Err(MemberVaultError::InvalidId("empty ciphertext".to_owned()));
    }
    Ok(())
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let temporary_path = temporary_path(path);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary_path)?;
    if let Err(error) = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary_path, path)?;
        Ok::<(), std::io::Error>(())
    })() {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{}.tmp", std::process::id()));
    PathBuf::from(name)
}
