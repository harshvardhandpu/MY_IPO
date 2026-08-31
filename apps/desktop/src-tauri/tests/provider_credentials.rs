use sanket_desktop_lib::provider_credentials::{
    CredentialError, OsProviderCredentialStore, ProviderConnectionStatusDto, ProviderCredentialKey,
    ProviderCredentialStore, SecretValue,
};

#[test]
fn provider_token_namespace_is_separate_from_identity_namespace() {
    assert_ne!(
        ProviderCredentialKey::UpstoxAnalyticsToken.key_id(),
        "os-keyring:v1:identity-key-v1"
    );
    assert_eq!(
        ProviderCredentialKey::UpstoxAnalyticsToken.service_name(),
        "sanket-ipo-provider"
    );
}

#[test]
fn secret_debug_output_is_redacted() {
    let raw = "x".repeat(24);
    let secret = SecretValue::new(raw.clone());
    let debug = format!("{secret:?}");
    assert!(!debug.contains(&raw));
    assert!(debug.contains("REDACTED"));
}

#[test]
fn connection_status_dto_has_no_secret_field() {
    let status = ProviderConnectionStatusDto::not_connected();
    let json = serde_json::to_string(&status).unwrap();
    assert!(!json.contains("token"));
    assert!(!json.contains("secret"));
    assert!(json.contains("NOT_CONNECTED"));
}

#[test]
fn oversized_input_is_rejected_before_keyring_access() {
    let store = OsProviderCredentialStore;
    let result = store.store(
        ProviderCredentialKey::UpstoxAnalyticsToken,
        SecretValue::new("x".repeat(4097)),
    );
    assert!(matches!(result, Err(CredentialError::CredentialTooLong)));
}
