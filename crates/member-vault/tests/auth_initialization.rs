use sanket_member_vault::MemberVault;

fn temp_root(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sanket-member-vault-auth-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn installation_is_uninitialized_only_when_vault_root_has_no_entries() {
    let root = temp_root("empty");
    let vault = MemberVault::open(root.join("vault")).unwrap();
    assert!(vault.is_uninitialized().unwrap());

    std::fs::write(vault.root().join("config-marker"), b"not auth").unwrap();
    assert!(!vault.is_uninitialized().unwrap());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn deleting_projection_does_not_reset_vault_initialization() {
    let root = temp_root("projection");
    let vault = MemberVault::open(root.join("vault")).unwrap();
    std::fs::write(vault.root().join("auth-marker"), b"initialized").unwrap();
    let index = root.join("index.sqlite");
    std::fs::write(&index, b"projection").unwrap();
    std::fs::remove_file(index).unwrap();
    assert!(!vault.is_uninitialized().unwrap());
    let _ = std::fs::remove_dir_all(root);
}
