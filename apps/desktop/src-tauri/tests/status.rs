use sanket_desktop_lib::AppState;
use sanket_domain::SyncStatus;

#[test]
fn app_status_reports_local_projection_and_pending_sync() {
    let state = AppState::new("device-synthetic-01".into(), 1);

    let status = state.status();

    assert_eq!("device-synthetic-01", status.device_id);
    assert_eq!(1, status.projection_schema_version);
    assert_eq!(SyncStatus::Pending, status.sync_status);
}
