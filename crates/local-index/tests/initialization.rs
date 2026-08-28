use sanket_local_index::LocalIndex;

#[test]
fn new_database_initializes_rebuildable_projection_schema() {
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("index.sqlite3");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::File::create(&database_path).expect("database placeholder should be created");
        std::fs::set_permissions(&database_path, std::fs::Permissions::from_mode(0o644))
            .expect("test should establish an unsafe starting mode");
    }

    let index = LocalIndex::open(&database_path).expect("database should initialize");

    assert_eq!(
        4,
        index.schema_version().expect("schema version should load")
    );
    assert!(
        index
            .has_table("projection_events")
            .expect("table check should work")
    );
    assert!(
        index
            .has_table("allotment_jobs")
            .expect("allotment jobs table should exist")
    );
    assert!(
        index
            .has_table("allotment_attempts")
            .expect("allotment attempts table should exist")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&database_path)
            .expect("database metadata should load")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(0o600, mode);
    }
    drop(index);

    let reopened = LocalIndex::open(&database_path).expect("database should reopen idempotently");
    assert_eq!(
        4,
        reopened
            .schema_version()
            .expect("schema should remain current")
    );
}
