//! Phase 2B: CoreMember/FriendAccount profile persistence in the MemberVault.
//!
//! Profiles carry only masked PANs — safe to persist as JSON. The full PAN
//! remains exclusively in `_secure_identity/*.enc`.

use sanket_domain::{CoreMember, EventPayload, FriendAccount, MemberStatus, Role};
use sanket_identity_security::MaskedPan;
use sanket_member_vault::MemberVault;

fn masked() -> MaskedPan {
    MaskedPan::from_parts("ABCDE", "F")
}

fn temp_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("sanket-profiles-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("tmp");
    root
}

#[test]
fn member_profile_round_trips() {
    let root = temp_root("member-rt");
    let vault = MemberVault::open(&root).expect("open");
    let mut member = CoreMember::onboard("member-1", "Sanket", Role::Owner, masked(), "rec-1");
    member.designate_primary_account("account-1");

    vault.store_member_profile(&member).expect("store");
    let loaded = vault.load_member_profile("member-1").expect("load");
    assert_eq!(loaded, member);
    assert_eq!(loaded.primary_account_id(), Some("account-1"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn friend_profile_round_trips() {
    let root = temp_root("friend-rt");
    let vault = MemberVault::open(&root).expect("open");
    let mut friend = FriendAccount::create("friend-1", "member-1", "Broker", masked(), "rec-2");
    friend.archive();

    vault.store_friend_profile(&friend).expect("store");
    let loaded = vault.load_friend_profile("friend-1").expect("load");
    assert_eq!(loaded, friend);
    assert_eq!(loaded.status(), MemberStatus::Archived);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn profile_files_contain_only_masked_pan() {
    let root = temp_root("masked-only");
    let vault = MemberVault::open(&root).expect("open");
    let member = CoreMember::onboard("member-1", "Sanket", Role::Owner, masked(), "rec-1");
    vault.store_member_profile(&member).expect("store");

    let bytes = std::fs::read(root.join("_profiles/members/member-1.json")).expect("read");
    let text = String::from_utf8(bytes).expect("utf8");
    assert!(text.contains("ABCDE****F"));
    // No interior digits of any PAN can appear in a profile file.
    assert!(!text.contains("1234"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn profile_writes_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root("perms");
    let vault = MemberVault::open(&root).expect("open");
    let member = CoreMember::onboard("member-1", "Sanket", Role::Owner, masked(), "rec-1");
    vault.store_member_profile(&member).expect("store");

    let mode = std::fs::metadata(root.join("_profiles/members/member-1.json"))
        .expect("meta")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn listing_profiles_returns_stored_ids() {
    let root = temp_root("list");
    let vault = MemberVault::open(&root).expect("open");
    vault
        .store_member_profile(&CoreMember::onboard(
            "m-1",
            "A",
            Role::Owner,
            masked(),
            "r-1",
        ))
        .expect("store");
    vault
        .store_member_profile(&CoreMember::onboard(
            "m-2",
            "B",
            Role::CoreMember,
            masked(),
            "r-2",
        ))
        .expect("store");
    vault
        .store_friend_profile(&FriendAccount::create(
            "f-1",
            "m-1",
            "Broker",
            masked(),
            "r-3",
        ))
        .expect("store");

    let mut members = vault.list_member_ids().expect("list members");
    members.sort();
    assert_eq!(members, vec!["m-1", "m-2"]);
    assert_eq!(vault.list_friend_ids().expect("list friends"), vec!["f-1"]);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn friend_added_notification_event_persists_without_pan() {
    use sanket_domain::{EventEnvelope, NewEvent};

    let root = temp_root("notify");
    let vault = MemberVault::open(&root).expect("open");

    let payload = EventPayload::FriendAdded {
        friend_id: "friend-1".to_owned(),
        owner_member_id: "member-1".to_owned(),
        label: "Broker".to_owned(),
        share_basis_points: 1_000,
    };
    let event = EventEnvelope::seal(NewEvent {
        event_id: "evt-1".to_owned(),
        aggregate_type: "friend_account".to_owned(),
        aggregate_id: "friend-1".to_owned(),
        aggregate_revision: 1,
        actor_member_id: "member-1".to_owned(),
        device_id: "device-1".to_owned(),
        occurred_at: "2026-08-28T00:00:00Z".to_owned(),
        app_version: "0.1.0".to_owned(),
        previous_event_hash: None,
        payload,
    })
    .expect("seal");
    vault.append_event(&event).expect("append");

    let bytes = std::fs::read(root.join("_events/evt-1.json")).expect("read");
    let text = String::from_utf8(bytes).expect("utf8");
    assert!(text.contains("FRIEND_ADDED"));
    assert!(text.contains("friend-1"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn profile_ids_reject_path_traversal() {
    let root = temp_root("traversal");
    let vault = MemberVault::open(&root).expect("open");
    let member = CoreMember::onboard("../evil", "X", Role::Owner, masked(), "r-1");
    assert!(vault.store_member_profile(&member).is_err());

    let _ = std::fs::remove_dir_all(&root);
}
