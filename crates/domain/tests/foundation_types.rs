use sanket_domain::{Role, SyncStatus};

#[test]
fn only_owner_role_can_manage_members() {
    assert!(Role::Owner.can_manage_members());
    assert!(!Role::CoreMember.can_manage_members());
}

#[test]
fn sync_states_have_stable_external_names() {
    let states = [
        (SyncStatus::Synced, "SYNCED"),
        (SyncStatus::Syncing, "SYNCING"),
        (SyncStatus::Pending, "PENDING"),
        (SyncStatus::Offline, "OFFLINE"),
        (SyncStatus::Conflict, "CONFLICT"),
        (
            SyncStatus::AuthenticationRequired,
            "AUTHENTICATION_REQUIRED",
        ),
    ];

    for (state, expected) in states {
        assert_eq!(
            format!("\"{expected}\""),
            serde_json::to_string(&state).unwrap()
        );
    }
}
