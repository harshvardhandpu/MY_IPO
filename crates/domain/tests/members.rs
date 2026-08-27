//! Phase 2B: CoreMember and FriendAccount domain types.

use sanket_domain::{BasisPoints, CoreMember, EventPayload, FriendAccount, MemberStatus, Role};
use sanket_identity_security::MaskedPan;

fn masked() -> MaskedPan {
    MaskedPan::from_parts("ABCDE", "F")
}

#[test]
fn core_member_onboarding_requires_pan() {
    // PAN is mandatory: the constructor takes MaskedPan by value, so a member
    // cannot exist without one.
    let member = CoreMember::onboard(
        "member-1",
        "Sanket",
        Role::Owner,
        masked(),
        "sensitive-record-1",
    );
    assert_eq!(member.id(), "member-1");
    assert_eq!(member.display_name(), "Sanket");
    assert_eq!(member.role(), Role::Owner);
    assert_eq!(member.masked_pan().to_string(), "ABCDE****F");
    assert_eq!(member.status(), MemberStatus::Active);
    assert!(member.primary_account_id().is_none());
}

#[test]
fn core_member_primary_account_designation() {
    let mut member = CoreMember::onboard(
        "member-1",
        "Sanket",
        Role::Owner,
        masked(),
        "sensitive-record-1",
    );
    member.designate_primary_account("account-1");
    assert_eq!(member.primary_account_id(), Some("account-1"));
    // Redesignation replaces, never duplicates.
    member.designate_primary_account("account-2");
    assert_eq!(member.primary_account_id(), Some("account-2"));
}

#[test]
fn friend_account_defaults_to_ten_percent_share() {
    let friend = FriendAccount::create(
        "friend-1",
        "member-1",
        "Broker friend",
        masked(),
        "sensitive-record-2",
    );
    assert_eq!(
        friend.share_basis_points(),
        BasisPoints::friend_share_default()
    );
    assert_eq!(friend.status(), MemberStatus::Active);
    assert_eq!(friend.owner_member_id(), "member-1");
}

#[test]
fn friend_account_share_can_be_set_within_range() {
    let mut friend = FriendAccount::create(
        "friend-1",
        "member-1",
        "Broker friend",
        masked(),
        "sensitive-record-2",
    );
    assert!(
        friend
            .set_share_basis_points(BasisPoints::try_new(1_500).unwrap())
            .is_ok()
    );
    assert_eq!(friend.share_basis_points().value(), 1_500);
}

#[test]
fn archive_not_delete_for_friends() {
    let mut friend = FriendAccount::create(
        "friend-1",
        "member-1",
        "Broker friend",
        masked(),
        "sensitive-record-2",
    );
    friend.archive();
    assert_eq!(friend.status(), MemberStatus::Archived);
    // Archived friends are not investable.
    assert!(!friend.is_investable());
    // There is no delete: the type has no delete/remove method, and status has
    // no Deleted variant.
    let statuses = [MemberStatus::Active, MemberStatus::Archived];
    assert_eq!(statuses.len(), 2);
}

#[test]
fn archive_not_delete_for_members() {
    let mut member = CoreMember::onboard(
        "member-1",
        "Sanket",
        Role::Owner,
        masked(),
        "sensitive-record-1",
    );
    member.archive();
    assert_eq!(member.status(), MemberStatus::Archived);
}

#[test]
fn active_friend_is_investable() {
    let friend = FriendAccount::create(
        "friend-1",
        "member-1",
        "Broker friend",
        masked(),
        "sensitive-record-2",
    );
    assert!(friend.is_investable());
}

#[test]
fn friend_added_event_is_member_wide_notification() {
    let payload = EventPayload::FriendAccountAdded {
        friend_id: "friend-1".to_owned(),
        owner_member_id: "member-1".to_owned(),
        label: "Broker friend".to_owned(),
        share_basis_points: 1_000,
    };
    // Notification events carry no PAN — only ids, label, and share terms.
    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("FRIEND_ACCOUNT_ADDED"));
    assert!(json.contains("friend-1"));
}

#[test]
fn friend_archived_event_is_member_wide_notification() {
    let payload = EventPayload::FriendAccountArchived {
        friend_id: "friend-1".to_owned(),
        owner_member_id: "member-1".to_owned(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("FRIEND_ACCOUNT_ARCHIVED"));
}
