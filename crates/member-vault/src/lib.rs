//! Private operational storage: encrypted identity envelopes and event persistence.
//!
//! This is the *MemberVault* — the only crate that persists encrypted sensitive
//! identity envelopes. It must never be handed to AI services, and its persistent
//! contents are ciphertext (or metadata) only, never plaintext PAN.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hmac::{Hmac, Mac};
use sanket_domain::{CoreMember, EventEnvelope, FriendAccount};
use sanket_identity_security::{EncryptedIdentityEnvelope, IdentityKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

/// The deterministic subdirectory layout for encrypted identity envelopes.
///
/// ```text
/// MemberVault/
///   _secure_identity/
///     <member-id>.enc
///     friends/
///       <friend-id>.enc
///   _profiles/
///     members/<member-id>.json   (masked PAN only)
///     friends/<friend-id>.json   (masked PAN only)
///   _events/<event-id>.json
/// ```
pub const SECURE_IDENTITY_DIR: &str = "_secure_identity";
pub const FRIENDS_DIR: &str = "friends";
pub const PROFILES_DIR: &str = "_profiles";
pub const MEMBERS_DIR: &str = "members";

#[derive(Debug, Error)]
pub enum MemberVaultError {
    #[error("member vault I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("member vault serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("member vault event error: {0}")]
    Event(#[from] sanket_domain::EventError),
    #[error("member vault event integrity check failed: {0}")]
    EventIntegrity(String),
    #[error("member vault event id already exists: {0}")]
    EventAlreadyExists(String),
    #[error("event append lock belongs to a different member vault")]
    EventLockOwnerMismatch,
    #[error("event stream integrity anchor is missing")]
    EventStreamAnchorMissing,
    #[error("event stream integrity anchor is invalid: {0}")]
    EventStreamAnchorInvalid(String),
    #[error("event stream integrity anchor is already enrolled")]
    EventStreamAnchorAlreadyEnrolled,
    #[error("invalid member/friend id: {0}")]
    InvalidId(String),
    #[error("unsupported identity envelope version {0}")]
    UnsupportedVersion(u16),
}

/// A filesystem-backed MemberVault. All writes are atomic and owner-only.
pub struct MemberVault {
    root: PathBuf,
    append_lock_owner: Arc<()>,
    event_stream_anchor: Option<EventStreamAnchor>,
}

/// Protected storage for a detached event-stream integrity anchor. Implementations
/// must not persist the anchor under the mutable vault root.
pub trait EventStreamAnchorStore: Send + Sync {
    fn load(&self) -> Result<Option<Vec<u8>>, String>;
    fn store(&self, value: &[u8]) -> Result<(), String>;
}

struct EventStreamAnchor {
    key: IdentityKey,
    store: Arc<dyn EventStreamAnchorStore>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventStreamAnchorV1 {
    version: u8,
    vault_binding: String,
    events: Vec<EventDigest>,
    mac: Vec<u8>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct EventDigest {
    event_id: String,
    content_hash: String,
}

#[derive(Serialize)]
struct UnsignedEventStreamAnchorV1<'a> {
    version: u8,
    vault_binding: &'a str,
    events: &'a [EventDigest],
}

impl MemberVault {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, MemberVaultError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            append_lock_owner: Arc::new(()),
            event_stream_anchor: None,
        })
    }

    /// Open a vault whose event log is authenticated against a detached,
    /// protected anchor. Existing non-empty vaults must be explicitly enrolled
    /// before this handle can read or append events.
    pub fn open_with_event_stream_anchor(
        root: impl Into<PathBuf>,
        key: IdentityKey,
        store: Arc<dyn EventStreamAnchorStore>,
    ) -> Result<Self, MemberVaultError> {
        let mut vault = Self::open(root)?;
        vault.event_stream_anchor = Some(EventStreamAnchor { key, store });
        Ok(vault)
    }

    /// Explicitly create the first detached integrity root for this event log.
    /// This never writes a vault event or SQLite state.
    pub fn enroll_event_stream_anchor(&self) -> Result<(), MemberVaultError> {
        let anchor = self
            .event_stream_anchor
            .as_ref()
            .ok_or(MemberVaultError::EventStreamAnchorMissing)?;
        if anchor
            .store
            .load()
            .map_err(MemberVaultError::EventStreamAnchorInvalid)?
            .is_some()
        {
            return Err(MemberVaultError::EventStreamAnchorAlreadyEnrolled);
        }
        self.store_event_stream_anchor(&self.read_events_unanchored()?)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether this vault-backed installation has no persisted state yet.
    ///
    /// This deliberately examines the authoritative vault root rather than
    /// SQLite or projected member counts, so deleting/replacing a projection
    /// cannot reset first-run authentication.
    pub fn is_uninitialized(&self) -> Result<bool, MemberVaultError> {
        let mut entries = fs::read_dir(&self.root)?;
        Ok(entries.next().transpose()?.is_none())
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

    /// Acquire the cross-process append lock for a compound state-read/append operation.
    pub fn acquire_event_append_lock(&self) -> Result<EventAppendLock, MemberVaultError> {
        let dir = self.root.join("_events");
        fs::create_dir_all(&dir)?;
        acquire_event_append_lock(&dir, Arc::clone(&self.append_lock_owner))
    }

    /// Append an event while the caller holds `lock`.
    pub fn append_event_under_lock(
        &self,
        event: &EventEnvelope,
        lock: &EventAppendLock,
    ) -> Result<(), MemberVaultError> {
        let dir = self.root.join("_events");
        if !Arc::ptr_eq(&self.append_lock_owner, &lock.vault_owner) {
            return Err(MemberVaultError::EventLockOwnerMismatch);
        }
        validate_event_for_append(event)?;
        let previous_events = self.read_events_unanchored()?;
        self.verify_event_stream_anchor(&previous_events)?;
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", event.event_id()));
        let bytes = serde_json::to_vec(event)?;
        if path.exists() {
            let existing = fs::read(&path)?;
            if existing == bytes {
                sync_parent_directory(&path)?;
                return Ok(());
            }
            return Err(MemberVaultError::EventAlreadyExists(
                event.event_id().to_owned(),
            ));
        }
        write_atomically(&path, &bytes)?;
        let current_events = self.read_events_unanchored()?;
        self.store_event_stream_anchor(&current_events)?;
        Ok(())
    }

    /// Append an event envelope. Events are metadata/appends only — no plaintext PAN.
    pub fn append_event(&self, event: &EventEnvelope) -> Result<(), MemberVaultError> {
        validate_event_for_append(event)?;
        let lock = self.acquire_event_append_lock()?;
        self.append_event_under_lock(event, &lock)
    }

    /// Load and integrity-check the append-only event log.
    pub fn list_events(&self) -> Result<Vec<EventEnvelope>, MemberVaultError> {
        let events = self.read_events_unanchored()?;
        self.verify_event_stream_anchor(&events)?;
        Ok(events)
    }

    fn read_events_unanchored(&self) -> Result<Vec<EventEnvelope>, MemberVaultError> {
        let dir = self.root.join("_events");
        let mut events = Vec::new();
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(events),
            Err(error) => return Err(error.into()),
        };
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let event: EventEnvelope = serde_json::from_slice(&fs::read(&path)?)?;
            if !event.verify_integrity()? {
                return Err(MemberVaultError::EventIntegrity(path.display().to_string()));
            }
            events.push(event);
        }
        events.sort_by(|left, right| left.event_id().cmp(right.event_id()));
        Ok(events)
    }

    fn verify_event_stream_anchor(
        &self,
        events: &[EventEnvelope],
    ) -> Result<(), MemberVaultError> {
        let Some(anchor) = &self.event_stream_anchor else {
            return Ok(());
        };
        let bytes = anchor
            .store
            .load()
            .map_err(MemberVaultError::EventStreamAnchorInvalid)?
            .ok_or(MemberVaultError::EventStreamAnchorMissing)?;
        let stored: EventStreamAnchorV1 = serde_json::from_slice(&bytes)
            .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
        if stored.version != 1 || stored.vault_binding != vault_binding(&self.root)? {
            return Err(MemberVaultError::EventStreamAnchorInvalid(
                "wrong version or vault binding".to_owned(),
            ));
        }
        let unsigned = unsigned_anchor_bytes(stored.version, &stored.vault_binding, &stored.events)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(anchor.key.as_bytes())
            .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
        mac.update(&unsigned);
        mac.verify_slice(&stored.mac)
            .map_err(|_| MemberVaultError::EventStreamAnchorInvalid("MAC mismatch".to_owned()))?;
        if stored.events != event_digests(events) {
            return Err(MemberVaultError::EventStreamAnchorInvalid(
                "event set mismatch".to_owned(),
            ));
        }
        Ok(())
    }

    fn store_event_stream_anchor(&self, events: &[EventEnvelope]) -> Result<(), MemberVaultError> {
        let Some(anchor) = &self.event_stream_anchor else {
            return Ok(());
        };
        let vault_binding = vault_binding(&self.root)?;
        let events = event_digests(events);
        let unsigned = unsigned_anchor_bytes(1, &vault_binding, &events)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(anchor.key.as_bytes())
            .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
        mac.update(&unsigned);
        let serialized = serde_json::to_vec(&EventStreamAnchorV1 {
            version: 1,
            vault_binding,
            events,
            mac: mac.finalize().into_bytes().to_vec(),
        })?;
        anchor
            .store
            .store(&serialized)
            .map_err(MemberVaultError::EventStreamAnchorInvalid)
    }

    fn member_profiles_dir(&self) -> PathBuf {
        self.root.join(PROFILES_DIR).join(MEMBERS_DIR)
    }

    fn friend_profiles_dir(&self) -> PathBuf {
        self.root.join(PROFILES_DIR).join(FRIENDS_DIR)
    }

    /// Persist a member profile (masked PAN only — safe plaintext).
    pub fn store_member_profile(&self, member: &CoreMember) -> Result<(), MemberVaultError> {
        Self::validate_id(member.id())?;
        let dir = self.member_profiles_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", member.id()));
        Ok(write_atomically(&path, &serde_json::to_vec(member)?)?)
    }

    /// Persist a friend profile (masked PAN only — safe plaintext).
    pub fn store_friend_profile(&self, friend: &FriendAccount) -> Result<(), MemberVaultError> {
        Self::validate_id(friend.id())?;
        let dir = self.friend_profiles_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", friend.id()));
        Ok(write_atomically(&path, &serde_json::to_vec(friend)?)?)
    }

    /// Load a member profile.
    pub fn load_member_profile(&self, member_id: &str) -> Result<CoreMember, MemberVaultError> {
        Self::validate_id(member_id)?;
        let path = self.member_profiles_dir().join(format!("{member_id}.json"));
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    /// Load a friend profile.
    pub fn load_friend_profile(&self, friend_id: &str) -> Result<FriendAccount, MemberVaultError> {
        Self::validate_id(friend_id)?;
        let path = self.friend_profiles_dir().join(format!("{friend_id}.json"));
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    /// List stored member ids (file stems under `_profiles/members/`).
    pub fn list_member_ids(&self) -> Result<Vec<String>, MemberVaultError> {
        list_ids(&self.member_profiles_dir())
    }

    /// List stored friend ids (file stems under `_profiles/friends/`).
    pub fn list_friend_ids(&self) -> Result<Vec<String>, MemberVaultError> {
        list_ids(&self.friend_profiles_dir())
    }
}

fn vault_binding(root: &Path) -> Result<String, MemberVaultError> {
    let canonical = root.canonicalize()?;
    Ok(canonical.to_string_lossy().into_owned())
}

fn event_digests(events: &[EventEnvelope]) -> Vec<EventDigest> {
    events
        .iter()
        .map(|event| EventDigest {
            event_id: event.event_id().to_owned(),
            content_hash: event.content_hash().to_owned(),
        })
        .collect()
}

fn unsigned_anchor_bytes(
    version: u8,
    vault_binding: &str,
    events: &[EventDigest],
) -> Result<Vec<u8>, MemberVaultError> {
    Ok(serde_json::to_vec(&UnsignedEventStreamAnchorV1 {
        version,
        vault_binding,
        events,
    })?)
}

fn validate_event_for_append(event: &EventEnvelope) -> Result<(), MemberVaultError> {
    if !event.verify_integrity()? {
        return Err(MemberVaultError::EventIntegrity(event.event_id().to_owned()));
    }
    Ok(())
}

pub struct EventAppendLock {
    vault_owner: Arc<()>,
    #[cfg(unix)]
    file: File,
    #[cfg(not(unix))]
    path: PathBuf,
}

fn acquire_event_append_lock(
    dir: &Path,
    vault_owner: Arc<()>,
) -> Result<EventAppendLock, MemberVaultError> {
    #[cfg(unix)]
    {
        let path = dir.join(".append.lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
        if result != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(EventAppendLock {
            vault_owner,
            file,
        })
    }
    #[cfg(not(unix))]
    {
        let path = dir.join(".append.lock");
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        Ok(EventAppendLock {
            vault_owner,
            path,
        })
    }
}

impl Drop for EventAppendLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = self.file.sync_all();
        }
        #[cfg(not(unix))]
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn list_ids(dir: &Path) -> Result<Vec<String>, MemberVaultError> {
    let mut ids = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ids),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.push(stem.to_owned());
            }
        }
    }
    Ok(ids)
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
    let mut renamed = false;
    if let Err(error) = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary_path, path)?;
        renamed = true;
        sync_parent_directory(path)?;
        Ok::<(), std::io::Error>(())
    })() {
        let _ = fs::remove_file(&temporary_path);
        if renamed {
            let _ = fs::remove_file(path);
            let _ = sync_parent_directory(path);
        }
        return Err(error);
    }
    Ok(())
}

fn sync_parent_directory(path: &Path) -> Result<(), std::io::Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let directory = OpenOptions::new().read(true).open(parent)?;
    directory.sync_all()
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{}.tmp", std::process::id()));
    PathBuf::from(name)
}
