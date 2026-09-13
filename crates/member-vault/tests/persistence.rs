use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sanket_domain::{EventEnvelope, EventPayload, NewEvent};
use sanket_identity_security::{EncryptedIdentityEnvelope, IdentityCipher, IdentityKey, Pan};
use sanket_member_vault::{EventStreamAnchorStore, MemberVault, MemberVaultError};

fn key() -> IdentityKey {
    IdentityKey::from_bytes(&[5u8; 32])
}

fn cipher() -> IdentityCipher {
    IdentityCipher::new(key())
}

fn pan() -> Pan {
    Pan::parse("ABCDE1234F").expect("valid PAN")
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "sanket-member-vault-test-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    root
}

#[test]
fn duplicate_event_id_cannot_replace_authoritative_bytes() {
    let root = temp_root("duplicate-event");
    let vault = MemberVault::open(&root).expect("open");
    let first = EventEnvelope::seal(NewEvent {
        event_id: "event-1".into(),
        aggregate_type: "lookup_authorization".into(),
        aggregate_id: "auth-1".into(),
        aggregate_revision: 1,
        actor_member_id: "owner-1".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-01-01T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::LookupAuthorizationGranted {
            authorization_id: "auth-1".into(),
            application_id: "application-1".into(),
            provider_id: "mufg-intime-live".into(),
            expiry_time: "9999999999".into(),
        },
    })
    .expect("seal first");
    let second = EventEnvelope::seal(NewEvent {
        event_id: "event-1".into(),
        aggregate_type: "lookup_authorization".into(),
        aggregate_id: "auth-1".into(),
        aggregate_revision: 2,
        actor_member_id: "SYSTEM".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-01-01T00:00:01Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::LookupAuthorizationConsumed {
            authorization_id: "auth-1".into(),
            application_id: "application-1".into(),
            provider_id: "mufg-intime-live".into(),
            execution_id: "job-1".into(),
            account_ids: vec!["account-1".into()],
            timestamp: "2026-01-01T00:00:01Z".into(),
        },
    })
    .expect("seal second");
    vault.append_event(&first).expect("append first");
    assert!(matches!(
        vault.append_event(&second),
        Err(MemberVaultError::EventAlreadyExists(_))
    ));
    assert_eq!(vault.list_events().expect("events").len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn malformed_event_id_cannot_escape_the_event_directory() {
    let root = temp_root("event-id-traversal");
    let vault = MemberVault::open(&root).expect("open");
    let event = EventEnvelope::seal(NewEvent {
        event_id: "event-safe".into(),
        aggregate_type: "member".into(),
        aggregate_id: "member-1".into(),
        aggregate_revision: 1,
        actor_member_id: "member-1".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-01-01T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::MemberCreated {
            member_id: "member-1".into(),
            display_name: "Owner".into(),
            role: sanket_domain::Role::Owner,
        },
    })
    .expect("seal");

    for unsafe_id in ["../escaped", "nested/event", "nested\\event"] {
        let mut serialized = serde_json::to_value(&event).expect("serialize");
        serialized["event_id"] = serde_json::json!(unsafe_id);
        assert!(serde_json::from_value::<EventEnvelope>(serialized).is_err());
        assert!(!root.parent().unwrap().join("escaped.json").exists());
    }
    assert!(vault.list_events().expect("events").is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn tampered_event_envelope_is_not_written() {
    let root = temp_root("tampered-event");
    let vault = MemberVault::open(&root).expect("open");
    let event = EventEnvelope::seal(NewEvent {
        event_id: "event-tampered".into(),
        aggregate_type: "member".into(),
        aggregate_id: "member-1".into(),
        aggregate_revision: 1,
        actor_member_id: "member-1".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-01-01T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::MemberCreated {
            member_id: "member-1".into(),
            display_name: "Owner".into(),
            role: sanket_domain::Role::Owner,
        },
    })
    .expect("seal");
    let mut serialized = serde_json::to_value(event).expect("serialize");
    serialized["aggregate_revision"] = serde_json::json!(2);
    let tampered: EventEnvelope = serde_json::from_value(serialized).expect("deserialize");

    assert!(matches!(
        vault.append_event(&tampered),
        Err(MemberVaultError::EventIntegrity(_))
    ));
    assert!(!root.join("_events/event-tampered.json").exists());
    assert!(vault.list_events().expect("events").is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn append_lock_cannot_be_used_with_another_vault() {
    let first_root = temp_root("lock-owner-first");
    let second_root = temp_root("lock-owner-second");
    let first = MemberVault::open(&first_root).expect("open first");
    let second = MemberVault::open(&second_root).expect("open second");
    let lock = first.acquire_event_append_lock().expect("first lock");
    let event = EventEnvelope::seal(NewEvent {
        event_id: "event-lock-owner".into(),
        aggregate_type: "member".into(),
        aggregate_id: "member-1".into(),
        aggregate_revision: 1,
        actor_member_id: "member-1".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-01-01T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::MemberCreated {
            member_id: "member-1".into(),
            display_name: "Owner".into(),
            role: sanket_domain::Role::Owner,
        },
    })
    .expect("seal");

    assert!(matches!(
        second.append_event_under_lock(&event, &lock),
        Err(MemberVaultError::EventLockOwnerMismatch)
    ));
    assert!(second.list_events().expect("second events").is_empty());
    let _ = std::fs::remove_dir_all(first_root);
    let _ = std::fs::remove_dir_all(second_root);
}

#[test]
fn append_lock_cannot_be_used_with_another_handle_to_the_same_vault() {
    let root = temp_root("lock-owner-same-root");
    let first = MemberVault::open(&root).expect("open first");
    let second = MemberVault::open(&root).expect("open second");
    let lock = first.acquire_event_append_lock().expect("first lock");
    let event = EventEnvelope::seal(NewEvent {
        event_id: "event-same-root-lock".into(),
        aggregate_type: "member".into(),
        aggregate_id: "member-1".into(),
        aggregate_revision: 1,
        actor_member_id: "member-1".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-01-01T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::MemberCreated {
            member_id: "member-1".into(),
            display_name: "Owner".into(),
            role: sanket_domain::Role::Owner,
        },
    })
    .expect("seal");

    assert!(matches!(
        second.append_event_under_lock(&event, &lock),
        Err(MemberVaultError::EventLockOwnerMismatch)
    ));
    assert!(first.list_events().expect("events").is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn encrypted_file_contains_no_plaintext_pan() {
    let root = temp_root("no-plaintext");
    let vault = MemberVault::open(&root).expect("open");
    let envelope = cipher()
        .encrypt(pan().as_normalized().as_bytes(), "kid-1")
        .expect("encrypt");

    vault
        .store_member_identity("member-1", &envelope)
        .expect("store");

    let path = root.join("_secure_identity/member-1.enc");
    let contents = std::fs::read_to_string(&path).expect("read");
    assert!(!contents.contains("ABCDE1234F"));
    assert!(!contents.contains("1234"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn round_trip_reload_decrypts() {
    let root = temp_root("roundtrip");
    let vault = MemberVault::open(&root).expect("open");
    let envelope = cipher()
        .encrypt(pan().as_normalized().as_bytes(), "kid-1")
        .expect("encrypt");

    vault
        .store_member_identity("member-1", &envelope)
        .expect("store");
    let loaded = vault.load_member_identity("member-1").expect("load");

    let recovered = cipher().decrypt(&loaded).expect("decrypt");
    assert_eq!(recovered, pan().as_normalized().as_bytes());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn malformed_envelope_fails_safely() {
    let root = temp_root("malformed");
    let vault = MemberVault::open(&root).expect("open");
    let dir = root.join("_secure_identity");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("member-1.enc"), b"not json").expect("write");

    assert!(matches!(
        vault.load_member_identity("member-1"),
        Err(MemberVaultError::Json(_))
    ));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn unsupported_version_fails() {
    let root = temp_root("version");
    let vault = MemberVault::open(&root).expect("open");
    let mut envelope = cipher().encrypt(b"x", "kid-1").expect("encrypt");
    envelope.version = 99;
    vault
        .store_member_identity("member-1", &envelope)
        .expect("store");

    assert!(matches!(
        vault.load_member_identity("member-1"),
        Err(MemberVaultError::UnsupportedVersion(99))
    ));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn friend_identity_round_trips_in_friends_dir() {
    let root = temp_root("friend");
    let vault = MemberVault::open(&root).expect("open");
    let envelope = cipher().encrypt(b"friend-pan", "kid-1").expect("encrypt");

    vault
        .store_friend_identity("friend-1", &envelope)
        .expect("store");
    let loaded = vault.load_friend_identity("friend-1").expect("load");
    assert_eq!(cipher().decrypt(&loaded).expect("decrypt"), b"friend-pan");

    // deterministic layout
    assert!(root.join("_secure_identity/friends/friend-1.enc").exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn rejects_path_traversal_ids() {
    let root = temp_root("traversal");
    let vault = MemberVault::open(&root).expect("open");
    let envelope: EncryptedIdentityEnvelope = cipher().encrypt(b"x", "kid-1").expect("encrypt");

    for bad in ["../etc", "a/b", "a\\b", "..", ".", ""] {
        assert!(
            matches!(
                vault.store_member_identity(bad, &envelope),
                Err(MemberVaultError::InvalidId(_))
            ),
            "should reject id {bad:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn writes_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root("perms");
    let vault = MemberVault::open(&root).expect("open");
    let envelope = cipher().encrypt(b"x", "kid-1").expect("encrypt");
    vault
        .store_member_identity("member-1", &envelope)
        .expect("store");

    let mode = std::fs::metadata(root.join("_secure_identity/member-1.enc"))
        .expect("meta")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn append_lock_serializes_across_processes() {
    let root = if let Some(root) = std::env::var_os("SANKET_MEMBER_VAULT_LOCK_ROOT") {
        std::path::PathBuf::from(root)
    } else {
        temp_root("cross-process-lock")
    };
    if std::env::var_os("SANKET_MEMBER_VAULT_LOCK_CHILD").is_some() {
        let vault = MemberVault::open(&root).expect("child open");
        let _lock = vault
            .acquire_event_append_lock()
            .expect("child acquires lock");
        return;
    }
    let vault = MemberVault::open(&root).expect("parent open");
    let lock = vault.acquire_event_append_lock().expect("parent lock");
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "append_lock_serializes_across_processes", "--nocapture"])
        .env("SANKET_MEMBER_VAULT_LOCK_CHILD", "1")
        .env("SANKET_MEMBER_VAULT_LOCK_ROOT", &root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child");
    thread::sleep(Duration::from_millis(150));
    assert!(child.try_wait().expect("poll child").is_none());
    drop(lock);
    assert!(child.wait().expect("wait child").success());
    let _ = std::fs::remove_dir_all(root);
}

#[derive(Default)]
struct MemoryAnchor {
    value: Mutex<Option<Vec<u8>>>,
}

impl EventStreamAnchorStore for MemoryAnchor {
    fn load(&self) -> Result<Option<Vec<u8>>, String> {
        Ok(self.value.lock().expect("anchor lock").clone())
    }

    fn store(&self, value: &[u8]) -> Result<(), String> {
        *self.value.lock().expect("anchor lock") = Some(value.to_vec());
        Ok(())
    }
}

fn member_created_event(event_id: &str, revision: u64) -> EventEnvelope {
    EventEnvelope::seal(NewEvent {
        event_id: event_id.into(),
        aggregate_type: "member".into(),
        aggregate_id: "member-1".into(),
        aggregate_revision: revision,
        actor_member_id: "member-1".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-01-01T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::MemberCreated {
            member_id: "member-1".into(),
            display_name: "Owner".into(),
            role: sanket_domain::Role::Owner,
        },
    })
    .expect("seal")
}

#[test]
fn anchored_event_stream_rejects_recomputed_self_hash_forgery() {
    let root = temp_root("anchored-forgery");
    let anchor = Arc::new(MemoryAnchor::default());
    let vault = MemberVault::open_with_event_stream_anchor(
        &root,
        IdentityKey::from_bytes(&[9u8; 32]),
        anchor,
    )
    .expect("open anchored vault");
    vault.enroll_event_stream_anchor().expect("explicit enrollment");
    vault
        .append_event(&member_created_event("anchored-event", 1))
        .expect("append event");

    let forged = member_created_event("anchored-event", 2);
    std::fs::write(
        root.join("_events/anchored-event.json"),
        serde_json::to_vec(&forged).expect("bytes"),
    )
    .expect("replace event");

    assert!(vault.list_events().is_err(), "anchor must reject a valid self-hash forgery");
    let _ = std::fs::remove_dir_all(root);
}
