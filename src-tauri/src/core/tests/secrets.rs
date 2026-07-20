use std::collections::BTreeMap;

use super::*;

/// In-memory stand-in for the OS keychain — the resolution ORDER is the
/// contract under test, not the credential store plumbing.
struct FakeKeychain(BTreeMap<String, String>);

impl SecretStore for FakeKeychain {
    fn get(&self, name: &str) -> SecretLookup {
        match self.0.get(name) {
            Some(value) => SecretLookup::Found(value.clone()),
            None => SecretLookup::Absent,
        }
    }
}

/// A keychain that cannot be consulted at all — locked, or no backend.
struct BrokenKeychain;

impl SecretStore for BrokenKeychain {
    fn get(&self, _name: &str) -> SecretLookup {
        SecretLookup::Unavailable("keychain is locked".to_string())
    }
}

#[test]
fn locked_keychain_is_reported_as_unavailable_not_missing() {
    // Collapsing these two told the user to re-enter a secret they already
    // have, and hid the fact that the credential store was unreachable.
    let out = resolve_secrets(&["token".to_string()], &BrokenKeychain, None);
    assert!(
        out.missing.is_empty(),
        "must not claim the secret is absent"
    );
    assert_eq!(
        out.unavailable.get("token").map(String::as_str),
        Some("keychain is locked")
    );

    // A genuinely absent secret still reports as missing.
    let out = resolve_secrets(&["token".to_string()], &FakeKeychain(BTreeMap::new()), None);
    assert_eq!(out.missing, vec!["token".to_string()]);
    assert!(out.unavailable.is_empty());
}

#[test]
fn env_file_still_answers_when_the_keychain_is_unreachable() {
    // A locked keychain must not shadow a value the env file can supply.
    let dir = tempfile::tempdir().unwrap();
    let env = env_file(dir.path(), "token=from-env\n");
    let out = resolve_secrets(&["token".to_string()], &BrokenKeychain, Some(&env));
    assert_eq!(
        out.resolved.get("token").map(String::as_str),
        Some("from-env")
    );
    assert_eq!(out.sources.get("token"), Some(&SecretSource::EnvFile));
    assert!(out.unavailable.is_empty());
    assert!(out.missing.is_empty());
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

#[test]
fn env_file_unwraps_quotes_and_preserves_inner_space() {
    let dir = tempfile::tempdir().unwrap();
    let path = env_file(
        dir.path(),
        "dquoted=\"a b \"\nsquoted='  x'\nplain=  düz değer  \nbare=tek\n",
    );
    let keychain = FakeKeychain(BTreeMap::new());
    let out = resolve_secrets(
        &[
            "dquoted".to_string(),
            "squoted".to_string(),
            "plain".to_string(),
            "bare".to_string(),
        ],
        &keychain,
        Some(&path),
    );
    // Quoted: inner text verbatim (trailing space kept, quotes gone).
    assert_eq!(out.resolved["dquoted"], "a b ");
    assert_eq!(out.resolved["squoted"], "  x");
    // Unquoted: trimmed.
    assert_eq!(out.resolved["plain"], "düz değer");
    assert_eq!(out.resolved["bare"], "tek");
}
