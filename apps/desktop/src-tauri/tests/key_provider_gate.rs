use sanket_desktop_lib::service::Application;
use sanket_identity_security::{
    IdentityKey, InMemoryKeyProvider, RuntimeSecurityMode, assert_mode_allows_provider,
};
use uuid::Uuid;

#[test]
fn production_secure_rejects_in_memory_provider() {
    let p = InMemoryKeyProvider::new("dev-key-1", IdentityKey::generate());
    let err = assert_mode_allows_provider(RuntimeSecurityMode::ProductionSecure, &p).unwrap_err();
    assert!(err.to_string().contains("rejects"));
}

#[test]
fn development_mode_allows_synthetic_app() {
    let root = std::env::temp_dir().join(format!("sanket-sec-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&root).unwrap();
    let app = Application::with_mode(
        "dev".into(),
        root.join("vault"),
        root.join("i.sqlite3"),
        RuntimeSecurityMode::DevelopmentSynthetic,
    )
    .unwrap();
    assert_eq!(
        app.security_mode(),
        RuntimeSecurityMode::DevelopmentSynthetic
    );
    let st = app.security_status().expect("status");
    assert!(st.os_keyring_release_blocker);
    assert!(!st.real_pan_allowed);
    assert!(st.blocker.is_some());
}
