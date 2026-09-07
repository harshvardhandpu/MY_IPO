use sanket_identity_security::{
    ARGON2_LANES, ARGON2_MEMORY_KIB, ARGON2_TIME_COST, ARGON2_VERSION, PASSWORD_VERIFIER_VERSION,
    Password, PasswordVerification, hash_password, verify_password,
};

const TEST_PASSWORD: &str = "correct horse battery staple";

#[test]
fn correct_password_is_verified() {
    let password = Password::new(TEST_PASSWORD);
    let verifier = hash_password(&password).expect("hash password");

    assert_eq!(
        verify_password(&password, verifier.as_str()),
        PasswordVerification::Verified
    );
}

#[test]
fn wrong_password_is_only_a_generic_rejection() {
    let password = Password::new(TEST_PASSWORD);
    let wrong_password = Password::new("wrong password");
    let verifier = hash_password(&password).expect("hash password");

    assert_eq!(
        verify_password(&wrong_password, verifier.as_str()),
        PasswordVerification::Rejected
    );
}

#[test]
fn malformed_verifier_is_only_a_generic_rejection() {
    let password = Password::new(TEST_PASSWORD);
    assert_eq!(
        verify_password(&password, "not-a-password-verifier"),
        PasswordVerification::Rejected
    );
}

#[test]
fn password_and_verifier_outputs_never_contain_plaintext() {
    let password = Password::new(TEST_PASSWORD);
    let verifier = hash_password(&password).expect("hash password");
    let serialized = serde_json::to_string(&verifier).expect("serialize verifier");
    let verifier_debug = format!("{verifier:?}");
    let password_debug = format!("{password:?}");

    assert!(!serialized.contains(TEST_PASSWORD));
    assert!(!verifier_debug.contains(TEST_PASSWORD));
    assert!(!password_debug.contains(TEST_PASSWORD));
    assert_eq!(password_debug, "Password([REDACTED])");
}

#[test]
fn verifier_uses_the_pinned_argon2id_policy_and_version() {
    let password = Password::new(TEST_PASSWORD);
    let verifier = hash_password(&password).expect("hash password");
    let fields: Vec<&str> = verifier.as_str().split('$').collect();

    assert_eq!(PASSWORD_VERIFIER_VERSION, 1);
    assert_eq!(fields[1], "argon2id");
    assert_eq!(fields[2], ARGON2_VERSION);
    assert_eq!(
        fields[3],
        format!("m={ARGON2_MEMORY_KIB},t={ARGON2_TIME_COST},p={ARGON2_LANES}")
    );
}
