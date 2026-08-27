//! Security invariant regression test.
//!
//! Invariant: after creating and persisting a member/friend sensitive identity,
//! searching all normal application projections, event files, logs, AI payloads,
//! and ordinary profile files must NOT reveal the plaintext PAN.

use sanket_identity_security::{
    IdentityCipher, IdentityKey, InMemoryKeyProvider, Pan, SensitiveIdentityRecord,
    SensitiveIdentityService, SensitivePurpose,
};
use sanket_intelligence_vault::InvestmentDecisionPayload;
use sanket_member_vault::MemberVault;

fn key() -> IdentityKey {
    IdentityKey::from_bytes(&[9u8; 32])
}

fn cipher() -> IdentityCipher {
    IdentityCipher::new(key())
}

fn synthetic_pan() -> String {
    // Built at runtime so no format-valid literal is committed to non-test source.
    format!("{}1234{}", "WXYZA", "Q")
}

fn temp_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("sanket-invariant-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("tmp");
    root
}

#[test]
fn persisted_identity_never_leaks_pan() {
    let pan = synthetic_pan();
    let root = temp_root();

    // 1. Create and persist the encrypted identity.
    let vault = MemberVault::open(&root).expect("open vault");
    let envelope = cipher().encrypt(pan.as_bytes(), "kid-1").expect("encrypt");
    vault
        .store_member_identity("member-1", &envelope)
        .expect("store");

    // 2. Purpose-scoped access works and returns the PAN only inside the closure.
    let record = SensitiveIdentityRecord::encrypt_pan(
        Pan::parse(&pan).expect("valid"),
        "member-1",
        &cipher(),
        "kid-1",
    )
    .expect("encrypt record");
    let provider = InMemoryKeyProvider::new("kid-1", key());
    let mut service = SensitiveIdentityService::new(provider);
    let mut seen: Option<String> = None;
    service
        .with_pan(&record, SensitivePurpose::AllotmentCheck, "member-1", |p| {
            seen = Some(p.to_owned());
        })
        .expect("access");
    assert_eq!(seen.as_deref(), Some(pan.as_str()));

    // Audit has no PAN.
    let audit = service.take_last_audit().expect("audit");
    let audit_json = serde_json::to_string(&audit).expect("serialize");
    assert!(!audit_json.contains(&pan));

    // 3. Search every persisted artifact for the plaintext PAN.
    let mut plaintext_found = false;
    for entry in walk(&root) {
        let Ok(bytes) = std::fs::read(&entry) else {
            continue;
        };
        if bytes.windows(pan.len()).any(|w| w == pan.as_bytes()) {
            plaintext_found = true;
        }
    }
    assert!(
        !plaintext_found,
        "plaintext PAN leaked into persisted storage"
    );

    // 4. A sanitized AI payload cannot carry it either.
    let payload = InvestmentDecisionPayload {
        ipo_name: "Example Limited".to_owned(),
        ipo_identifier: "EXLIM-2026".to_owned(),
        price_band_paise_low: 100_000,
        price_band_paise_high: 120_000,
        lot_size: 50,
        subscription_times: 12,
        listing_gain_basis_points: 1_500,
        public_context_hashes: vec!["sha256:beef".to_owned()],
    };
    let payload_json = serde_json::to_string(&payload).expect("serialize");
    assert!(!payload_json.contains(&pan));

    // 5. The envelope's Debug/serialization never contains the PAN.
    let envelope_str = format!("{envelope:?}");
    assert!(!envelope_str.contains(&pan));

    std::fs::remove_dir_all(&root).ok();
}

fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.push(path);
                }
            }
        }
    }
    out
}
