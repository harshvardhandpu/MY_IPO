use sanket_identity_security::{EncryptedIdentityEnvelope, IdentityCipher, IdentityKey, Pan};
use sanket_member_vault::{MemberVault, MemberVaultError};

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
