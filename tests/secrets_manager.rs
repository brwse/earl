mod common;

use secrecy::SecretString;

#[test]
fn get_returns_metadata_after_set() {
    let ws = common::temp_workspace();
    let index_path = ws.root.path().join("state/secrets-index.json");
    let manager = common::in_memory_secret_manager(&index_path);

    manager
        .set(
            "github.token",
            SecretString::new("token-1".to_string().into()),
        )
        .unwrap();

    let meta = manager.get("github.token").unwrap().unwrap();
    assert_eq!(meta.key, "github.token");
}

#[test]
fn list_returns_stored_secrets() {
    let ws = common::temp_workspace();
    let index_path = ws.root.path().join("state/secrets-index.json");
    let manager = common::in_memory_secret_manager(&index_path);

    manager
        .set(
            "github.token",
            SecretString::new("token-1".to_string().into()),
        )
        .unwrap();

    let list = manager.list().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].key, "github.token");
}

#[test]
fn delete_returns_true_when_key_exists() {
    let ws = common::temp_workspace();
    let index_path = ws.root.path().join("state/secrets-index.json");
    let manager = common::in_memory_secret_manager(&index_path);

    manager
        .set(
            "github.token",
            SecretString::new("token-1".to_string().into()),
        )
        .unwrap();

    let deleted = manager.delete("github.token").unwrap();
    assert!(deleted);
}

#[test]
fn delete_removes_secret_from_store() {
    let ws = common::temp_workspace();
    let index_path = ws.root.path().join("state/secrets-index.json");
    let manager = common::in_memory_secret_manager(&index_path);

    manager
        .set(
            "github.token",
            SecretString::new("token-1".to_string().into()),
        )
        .unwrap();

    manager.delete("github.token").unwrap();
    assert!(manager.get("github.token").unwrap().is_none());
}

#[test]
fn repeated_set_preserves_created_at() {
    let ws = common::temp_workspace();
    let index_path = ws.root.path().join("state/secrets-index.json");
    let manager = common::in_memory_secret_manager(&index_path);

    manager
        .set(
            "service.token",
            SecretString::new("first".to_string().into()),
        )
        .unwrap();
    let first = manager.get("service.token").unwrap().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(2));

    manager
        .set(
            "service.token",
            SecretString::new("second".to_string().into()),
        )
        .unwrap();
    let second = manager.get("service.token").unwrap().unwrap();

    assert_eq!(first.created_at, second.created_at);
}

#[test]
fn repeated_set_advances_updated_at() {
    let ws = common::temp_workspace();
    let index_path = ws.root.path().join("state/secrets-index.json");
    let manager = common::in_memory_secret_manager(&index_path);

    manager
        .set(
            "service.token",
            SecretString::new("first".to_string().into()),
        )
        .unwrap();
    let first = manager.get("service.token").unwrap().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(2));

    manager
        .set(
            "service.token",
            SecretString::new("second".to_string().into()),
        )
        .unwrap();
    let second = manager.get("service.token").unwrap().unwrap();

    assert!(second.updated_at > first.updated_at);
}
