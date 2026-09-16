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
use sha2::{Digest, Sha256};
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
    #[error("event stream append recovery requires explicit owner confirmation")]
    EventStreamRecoveryRequired,
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

/// Test/development-only detached anchor store. Production must select the
/// keyring store; this store exists so synthetic restart tests retain the
/// exact same event-set commitment without placing it in the vault or SQLite.
pub struct FileEventStreamAnchorStore {
    path: PathBuf,
}

impl FileEventStreamAnchorStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl EventStreamAnchorStore for FileEventStreamAnchorStore {
    fn load(&self) -> Result<Option<Vec<u8>>, String> {
        match fs::read(&self.path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn store(&self, value: &[u8]) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        write_atomically(&self.path, value).map_err(|error| error.to_string())
    }
}

/// OS-keyring-backed detached anchor storage. The anchor lives outside the
/// mutable vault and SQLite projection; its HMAC key is a separate keyring
/// item supplied by the caller.
pub struct KeyringEventStreamAnchorStore {
    account: String,
}

impl KeyringEventStreamAnchorStore {
    const SERVICE: &'static str = "sanket-ipo-event-stream-anchor";

    pub fn for_vault(root: &Path) -> Result<Self, MemberVaultError> {
        let canonical = root.canonicalize()?;
        let digest = Sha256::digest(canonical.to_string_lossy().as_bytes());
        Ok(Self {
            account: format!("v1:{digest:x}"),
        })
    }

    fn entry(&self) -> Result<keyring::Entry, String> {
        keyring::Entry::new(Self::SERVICE, &self.account).map_err(|error| error.to_string())
    }

    fn ensure_durable(&self) -> Result<keyring::Entry, String> {
        let entry = self.entry()?;
        if entry
            .get_credential()
            .downcast_ref::<keyring::mock::MockCredential>()
            .is_some()
        {
            return Err("no durable OS credential store available".to_owned());
        }
        Ok(entry)
    }
}

impl EventStreamAnchorStore for KeyringEventStreamAnchorStore {
    fn load(&self) -> Result<Option<Vec<u8>>, String> {
        let entry = self.ensure_durable()?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value.into_bytes())),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn store(&self, value: &[u8]) -> Result<(), String> {
        let value = std::str::from_utf8(value).map_err(|error| error.to_string())?;
        self.ensure_durable()?
            .set_password(value)
            .map_err(|error| error.to_string())
    }
}

struct EventStreamAnchor {
    key: IdentityKey,
    store: Arc<dyn EventStreamAnchorStore>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EventStreamAnchorV1 {
    version: u8,
    vault_binding: String,
    event_count: u64,
    events: Vec<EventDigest>,
    mac: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct EventDigest {
    event_id: String,
    content_hash: String,
}

#[derive(Serialize)]
struct UnsignedEventStreamAnchorV1<'a> {
    version: u8,
    vault_binding: &'a str,
    event_count: u64,
    events: &'a [EventDigest],
}

#[derive(Debug, Serialize, Deserialize)]
struct PendingEventAppendV1 {
    version: u8,
    transaction_id: String,
    vault_binding: String,
    previous_events: Vec<EventDigest>,
    expected_events: Vec<EventDigest>,
    expected_event_id: String,
    expected_event_hash: String,
    mac: Vec<u8>,
}

#[derive(Serialize)]
struct UnsignedPendingEventAppendV1<'a> {
    version: u8,
    transaction_id: &'a str,
    vault_binding: &'a str,
    previous_events: &'a [EventDigest],
    expected_events: &'a [EventDigest],
    expected_event_id: &'a str,
    expected_event_hash: &'a str,
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

    /// Verify the authenticated event stream before handing the vault to an
    /// application security boundary. This acquires the same cross-process
    /// lock used by appends, so startup cannot observe a transition halfway
    /// through the pending-append protocol.
    pub fn verified_open(&self) -> Result<(), MemberVaultError> {
        if self.event_stream_anchor.is_none() {
            return Err(MemberVaultError::EventStreamAnchorMissing);
        }
        let _lock = self.acquire_event_append_lock()?;
        self.verified_open_under_lock()
    }

    fn verified_open_under_lock(&self) -> Result<(), MemberVaultError> {
        let events = self.read_events_unanchored()?;
        self.recover_pending_append(&events, false)
    }

    fn pending_path(&self) -> PathBuf {
        self.root.join("_events/.anchor-pending-v1.json")
    }

    fn clear_pending_append(&self) -> Result<(), MemberVaultError> {
        match fs::remove_file(self.pending_path()) {
            Ok(()) => sync_parent_directory(&self.pending_path())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    fn recover_pending_append(
        &self,
        events: &[EventEnvelope],
        owner_confirmed: bool,
    ) -> Result<(), MemberVaultError> {
        let anchor_config = self
            .event_stream_anchor
            .as_ref()
            .ok_or(MemberVaultError::EventStreamAnchorMissing)?;
        let pending_bytes = match fs::read(self.pending_path()) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let stored = self.read_anchor(anchor_config)?;
        let actual = event_digests(events);
        let Some(pending_bytes) = pending_bytes else {
            if stored.events != actual {
                return Err(MemberVaultError::EventStreamAnchorInvalid(
                    "event set mismatch".to_owned(),
                ));
            }
            return Ok(());
        };
        let pending: PendingEventAppendV1 = serde_json::from_slice(&pending_bytes)
            .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
        self.verify_pending(&pending, anchor_config)?;
        if pending.vault_binding != vault_binding(&self.root)?
            || stored.events != pending.previous_events && stored.events != pending.expected_events
            || actual != pending.previous_events && actual != pending.expected_events
        {
            return Err(MemberVaultError::EventStreamAnchorInvalid(
                "pending event transition mismatch".to_owned(),
            ));
        }

        if actual == pending.previous_events && stored.events == pending.previous_events {
            // The event was not durable. Discarding the authenticated intent
            // cannot hide an authoritative event because the event set did not
            // change.
            self.clear_pending_append()?;
            return Ok(());
        }
        if actual == pending.expected_events && stored.events == pending.expected_events {
            // The anchor update completed; only pending-record cleanup was
            // interrupted.
            self.clear_pending_append()?;
            return Ok(());
        }
        if actual == pending.expected_events && stored.events == pending.previous_events {
            if !owner_confirmed {
                return Err(MemberVaultError::EventStreamRecoveryRequired);
            }
            self.store_event_stream_anchor(events)?;
            self.verify_event_stream_anchor(events)?;
            self.clear_pending_append()?;
            return Ok(());
        }
        Err(MemberVaultError::EventStreamAnchorInvalid(
            "pending anchor state mismatch".to_owned(),
        ))
    }

    /// Complete an append whose event file is durable but whose detached anchor
    /// update was interrupted. The caller must first authenticate an owner and
    /// obtain an explicit confirmation; ordinary startup never invokes this
    /// recovery path.
    pub fn recover_pending_event_append_after_owner_confirmation(
        &self,
        owner_confirmed: bool,
    ) -> Result<(), MemberVaultError> {
        if !owner_confirmed {
            return Err(MemberVaultError::EventStreamRecoveryRequired);
        }
        let _lock = self.acquire_event_append_lock()?;
        let events = self.read_events_unanchored()?;
        self.recover_pending_append(&events, true)
    }

    fn read_anchor(
        &self,
        anchor_config: &EventStreamAnchor,
    ) -> Result<EventStreamAnchorV1, MemberVaultError> {
        let bytes = anchor_config
            .store
            .load()
            .map_err(MemberVaultError::EventStreamAnchorInvalid)?
            .ok_or(MemberVaultError::EventStreamAnchorMissing)?;
        let stored: EventStreamAnchorV1 = serde_json::from_slice(&bytes)
            .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
        if stored.version != 1
            || stored.vault_binding != vault_binding(&self.root)?
            || stored.event_count != stored.events.len() as u64
            || !is_canonical_event_digests(&stored.events)
        {
            return Err(MemberVaultError::EventStreamAnchorInvalid(
                "wrong version, vault binding, count, or event ordering".to_owned(),
            ));
        }
        let unsigned = unsigned_anchor_bytes(
            stored.version,
            &stored.vault_binding,
            stored.event_count,
            &stored.events,
        )?;
        let mut mac = Hmac::<Sha256>::new_from_slice(anchor_config.key.as_bytes())
            .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
        mac.update(&unsigned);
        mac.verify_slice(&stored.mac)
            .map_err(|_| MemberVaultError::EventStreamAnchorInvalid("MAC mismatch".to_owned()))?;
        Ok(stored)
    }

    fn verify_pending(
        &self,
        pending: &PendingEventAppendV1,
        anchor_config: &EventStreamAnchor,
    ) -> Result<(), MemberVaultError> {
        let mut expected_from_previous = pending.previous_events.clone();
        expected_from_previous.push(EventDigest {
            event_id: pending.expected_event_id.clone(),
            content_hash: pending.expected_event_hash.clone(),
        });
        expected_from_previous.sort_by(|left, right| left.event_id.cmp(&right.event_id));
        if pending.version != 1
            || pending.transaction_id != format!("append:{}", pending.expected_event_id)
            || pending.expected_events != expected_from_previous
            || pending.previous_events.iter().any(|event| event.event_id == pending.expected_event_id)
            || !is_canonical_event_digests(&pending.previous_events)
            || !is_canonical_event_digests(&pending.expected_events)
        {
            return Err(MemberVaultError::EventStreamAnchorInvalid(
                "malformed pending append".to_owned(),
            ));
        }
        let unsigned = unsigned_pending_bytes(pending)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(anchor_config.key.as_bytes())
            .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
        mac.update(&unsigned);
        mac.verify_slice(&pending.mac).map_err(|_| {
            MemberVaultError::EventStreamAnchorInvalid("pending MAC mismatch".to_owned())
        })
    }

    fn verify_event_stream_anchor(
        &self,
        events: &[EventEnvelope],
    ) -> Result<(), MemberVaultError> {
        let Some(anchor_config) = &self.event_stream_anchor else {
            return Ok(());
        };
        let stored = self.read_anchor(anchor_config)?;
        if stored.events != event_digests(events) {
            return Err(MemberVaultError::EventStreamAnchorInvalid(
                "event set mismatch".to_owned(),
            ));
        }
        Ok(())
    }

    /// Create the first detached integrity root for an empty event stream.
    /// Existing history must use the explicit owner-confirmed enrollment API.
    pub fn enroll_event_stream_anchor(&self) -> Result<(), MemberVaultError> {
        let _lock = self.acquire_event_append_lock()?;
        self.enroll_event_stream_anchor_under_lock(false)
    }

    /// Explicitly enroll a verified legacy event stream. This is never called
    /// by ordinary startup and is intentionally gated by owner confirmation.
    pub fn enroll_legacy_event_stream_anchor_after_owner_confirmation(
        &self,
        owner_confirmed: bool,
    ) -> Result<(), MemberVaultError> {
        if !owner_confirmed {
            return Err(MemberVaultError::EventStreamRecoveryRequired);
        }
        let _lock = self.acquire_event_append_lock()?;
        self.enroll_event_stream_anchor_under_lock(true)
    }

    fn enroll_event_stream_anchor_under_lock(
        &self,
        owner_confirmed: bool,
    ) -> Result<(), MemberVaultError> {
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
        match fs::metadata(self.pending_path()) {
            Ok(_) => return Err(MemberVaultError::EventStreamRecoveryRequired),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let events = self.read_events_unanchored()?;
        if !events.is_empty() && !owner_confirmed {
            return Err(MemberVaultError::EventStreamRecoveryRequired);
        }
        self.store_event_stream_anchor(&events)?;
        self.verify_event_stream_anchor(&events)
    }

    /// Reads the event directory with envelope verification but does not
    /// consult an anchor. This is only for first-run state classification.
    pub fn event_log_is_empty(&self) -> Result<bool, MemberVaultError> {
        Ok(self.read_events_unanchored()?.is_empty())
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
        if !Arc::ptr_eq(&self.append_lock_owner, &lock.vault_owner) {
            return Err(MemberVaultError::EventLockOwnerMismatch);
        }
        validate_event_for_append(event)?;
        if self.event_stream_anchor.is_some() {
            self.verified_open_under_lock()?;
        }
        let previous_events = self.read_events_unanchored()?;
        fs::create_dir_all(self.root.join("_events"))?;
        let path = self.root.join("_events").join(format!("{}.json", event.event_id()));
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
        if self.event_stream_anchor.is_none() {
            write_atomically(&path, &bytes)?;
            return Ok(());
        }
        let mut expected_events = event_digests(&previous_events);
        expected_events.push(EventDigest {
            event_id: event.event_id().to_owned(),
            content_hash: event.content_hash().to_owned(),
        });
        expected_events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
        let vault_binding = vault_binding(&self.root)?;
        let pending = PendingEventAppendV1 {
            version: 1,
            transaction_id: format!("append:{}", event.event_id()),
            vault_binding,
            previous_events: event_digests(&previous_events),
            expected_events,
            expected_event_id: event.event_id().to_owned(),
            expected_event_hash: event.content_hash().to_owned(),
            mac: Vec::new(),
        };
        let anchor = self
            .event_stream_anchor
            .as_ref()
            .ok_or(MemberVaultError::EventStreamAnchorMissing)?;
        write_atomically(&self.pending_path(), &sign_pending(&anchor.key, pending)?)?;
        write_atomically(&path, &bytes)?;
        let current_events = self.read_events_unanchored()?;
        self.store_event_stream_anchor(&current_events)?;
        self.clear_pending_append()?;
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
        if self.event_stream_anchor.is_some() {
            let lock = self.acquire_event_append_lock()?;
            return self.list_events_under_lock(&lock);
        }
        self.read_events_unanchored()
    }

    /// Read the event log while the caller already owns the append lock.
    ///
    /// This is the only safe way for a compound application operation to
    /// re-read authoritative state after acquiring the cross-process lock:
    /// opening another lock file descriptor can block on Unix `flock`.
    pub fn list_events_under_lock(
        &self,
        lock: &EventAppendLock,
    ) -> Result<Vec<EventEnvelope>, MemberVaultError> {
        if !Arc::ptr_eq(&self.append_lock_owner, &lock.vault_owner) {
            return Err(MemberVaultError::EventLockOwnerMismatch);
        }
        let events = self.read_events_unanchored()?;
        if self.event_stream_anchor.is_some() {
            self.recover_pending_append(&events, false)?;
        }
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
            if path.file_name().and_then(|name| name.to_str()) == Some(".anchor-pending-v1.json") {
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let event: EventEnvelope = serde_json::from_slice(&fs::read(&path)?)?;
            if path.file_stem().and_then(|stem| stem.to_str()) != Some(event.event_id()) {
                return Err(MemberVaultError::EventStreamAnchorInvalid(
                    "event filename does not match event id".to_owned(),
                ));
            }
            if !event.verify_integrity()? {
                return Err(MemberVaultError::EventIntegrity(path.display().to_string()));
            }
            events.push(event);
        }
        events.sort_by(|left, right| left.event_id().cmp(right.event_id()));
        if events
            .windows(2)
            .any(|pair| pair[0].event_id() == pair[1].event_id())
        {
            return Err(MemberVaultError::EventStreamAnchorInvalid(
                "duplicate event id".to_owned(),
            ));
        }
        Ok(events)
    }

    fn store_event_stream_anchor(&self, events: &[EventEnvelope]) -> Result<(), MemberVaultError> {
        let Some(anchor) = &self.event_stream_anchor else {
            return Ok(());
        };
        let vault_binding = vault_binding(&self.root)?;
        let events = event_digests(events);
        let event_count = events.len() as u64;
        let unsigned = unsigned_anchor_bytes(1, &vault_binding, event_count, &events)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(anchor.key.as_bytes())
            .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
        mac.update(&unsigned);
        let serialized = serde_json::to_vec(&EventStreamAnchorV1 {
            version: 1,
            vault_binding,
            event_count,
            events: events.clone(),
            mac: mac.finalize().into_bytes().to_vec(),
        })?;
        anchor
            .store
            .store(&serialized)
            .map_err(MemberVaultError::EventStreamAnchorInvalid)?;
        let stored = self.read_anchor(anchor)?;
        if stored.events != events || stored.event_count != event_count {
            return Err(MemberVaultError::EventStreamAnchorInvalid(
                "anchor readback mismatch".to_owned(),
            ));
        }
        Ok(())
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
    let mut digests = events
        .iter()
        .map(|event| EventDigest {
            event_id: event.event_id().to_owned(),
            content_hash: event.content_hash().to_owned(),
        })
        .collect::<Vec<_>>();
    digests.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    digests
}

fn is_canonical_event_digests(events: &[EventDigest]) -> bool {
    events
        .windows(2)
        .all(|pair| pair[0].event_id < pair[1].event_id)
}

fn unsigned_anchor_bytes(
    version: u8,
    vault_binding: &str,
    event_count: u64,
    events: &[EventDigest],
) -> Result<Vec<u8>, MemberVaultError> {
    Ok(serde_json::to_vec(&UnsignedEventStreamAnchorV1 {
        version,
        vault_binding,
        event_count,
        events,
    })?)
}

fn unsigned_pending_bytes(
    pending: &PendingEventAppendV1,
) -> Result<Vec<u8>, MemberVaultError> {
    Ok(serde_json::to_vec(&UnsignedPendingEventAppendV1 {
        version: pending.version,
        transaction_id: &pending.transaction_id,
        vault_binding: &pending.vault_binding,
        previous_events: &pending.previous_events,
        expected_events: &pending.expected_events,
        expected_event_id: &pending.expected_event_id,
        expected_event_hash: &pending.expected_event_hash,
    })?)
}

fn sign_pending(
    key: &IdentityKey,
    mut pending: PendingEventAppendV1,
) -> Result<Vec<u8>, MemberVaultError> {
    let unsigned = unsigned_pending_bytes(&pending)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
        .map_err(|error| MemberVaultError::EventStreamAnchorInvalid(error.to_string()))?;
    mac.update(&unsigned);
    pending.mac = mac.finalize().into_bytes().to_vec();
    Ok(serde_json::to_vec(&pending)?)
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
