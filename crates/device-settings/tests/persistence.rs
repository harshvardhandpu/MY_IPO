use sanket_device_settings::DeviceSettingsStore;

#[test]
fn settings_initialize_once_and_preserve_device_identity() {
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let path = directory.path().join("settings.json");

    let first = DeviceSettingsStore::load_or_initialize_with(
        &path,
        || "device-synthetic-01".to_owned(),
        || "2026-08-28T00:00:00Z".to_owned(),
    )
    .expect("settings should initialize");
    let second = DeviceSettingsStore::load_or_initialize_with(
        &path,
        || panic!("existing settings must not generate another identity"),
        || panic!("existing settings must not generate another timestamp"),
    )
    .expect("settings should reload");

    assert_eq!("device-synthetic-01", first.device_id);
    assert_eq!(first, second);
    assert_eq!(1, first.schema_version);
}
