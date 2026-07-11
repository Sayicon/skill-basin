use std::collections::BTreeMap;

use super::*;

/// In-memory stand-in for the OS keychain — the resolution ORDER is the
/// contract under test, not the credential store plumbing.
struct FakeKeychain(BTreeMap<String, String>);

impl SecretStore for FakeKeychain {
    fn get(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

fn env_file(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
    let path = dir.join("secrets.env");
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn keychain_wins_over_env_file() {
    let dir = tempfile::tempdir().unwrap();
    let env = env_file(dir.path(), "qdrant_api_key=from-env\n");
    let keychain = FakeKeychain(BTreeMap::from([(
        "qdrant_api_key".to_string(),
        "from-keychain".to_string(),
    )]));

    let out = resolve_secrets(&["qdrant_api_key".to_string()], &keychain, Some(&env));
    assert_eq!(out.resolved["qdrant_api_key"], "from-keychain");
    assert_eq!(out.resolved["qdrant_api_key"], "from-keychain");
    assert!(out.missing.is_empty());
    assert_eq!(out.sources["qdrant_api_key"], SecretSource::Keychain);
}

#[test]
fn env_file_fills_keychain_gaps() {
    let dir = tempfile::tempdir().unwrap();
    let env = env_file(
        dir.path(),
        "# yorum satırı atlanır\nminimax_tts_key=env-değeri\n\nexport prefixed=ile de olur\n",
    );
    let keychain = FakeKeychain(BTreeMap::new());

    let out = resolve_secrets(
        &["minimax_tts_key".to_string(), "prefixed".to_string()],
        &keychain,
        Some(&env),
    );
    assert_eq!(out.resolved["minimax_tts_key"], "env-değeri");
    assert_eq!(out.resolved["prefixed"], "ile de olur");
    assert_eq!(out.sources["minimax_tts_key"], SecretSource::EnvFile);
}

#[test]
fn missing_secrets_are_reported_not_invented() {
    let dir = tempfile::tempdir().unwrap();
    let env = env_file(dir.path(), "var_olan=x\n");
    let keychain = FakeKeychain(BTreeMap::new());

    let out = resolve_secrets(
        &["var_olan".to_string(), "eksik_anahtar".to_string()],
        &keychain,
        Some(&env),
    );
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.missing, vec!["eksik_anahtar".to_string()]);
}

#[test]
fn absent_env_file_is_not_an_error() {
    let keychain = FakeKeychain(BTreeMap::from([("a".to_string(), "1".to_string())]));
    let out = resolve_secrets(&["a".to_string(), "b".to_string()], &keychain, None);
    assert_eq!(out.resolved["a"], "1");
    assert_eq!(out.missing, vec!["b".to_string()]);
}

#[test]
fn placeholder_extraction_finds_secret_refs() {
    // ${secret:name} refs inside arbitrary JSON-ish text (mcp/*.json bodies).
    let text = r#"{"env":{"A":"${secret:qdrant_api_key}","B":"düz","C":"${secret:tts_key}"}}"#;
    let mut refs = extract_secret_refs(text);
    refs.sort();
    assert_eq!(
        refs,
        vec!["qdrant_api_key".to_string(), "tts_key".to_string()]
    );
}

#[test]
fn env_file_with_bom_still_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.env");
    std::fs::write(&path, b"\xef\xbb\xbfkey=bom-lu\n").unwrap();
    let keychain = FakeKeychain(BTreeMap::new());
    let out = resolve_secrets(&["key".to_string()], &keychain, Some(&path));
    assert_eq!(out.resolved["key"], "bom-lu");
}
