//! Secret resolution for basin-carried configs.
//!
//! The basin only ever carries `${secret:name}` placeholders; real values
//! live machine-local. Resolution order: OS keychain (Windows Credential
//! Manager / macOS Keychain / libsecret) first, then `~/.skillbasin/
//! secrets.env` — and anything still unresolved is REPORTED, never invented,
//! so the Settings screen can show exactly which keys a machine is missing.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

/// Where a secret's value came from — shown as a chip in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretSource {
    Keychain,
    EnvFile,
}

/// Abstraction over the OS credential store so resolution order is testable
/// without touching a real keychain.
pub trait SecretStore {
    fn get(&self, name: &str) -> Option<String>;
}

/// The real OS keychain, service-scoped to SkillBasin.
pub struct OsKeychain;

impl SecretStore for OsKeychain {
    fn get(&self, name: &str) -> Option<String> {
        let _ = name;
        todo!("keyring lookup")
    }
}

#[derive(Debug, Default)]
pub struct SecretResolution {
    pub resolved: BTreeMap<String, String>,
    pub sources: BTreeMap<String, SecretSource>,
    /// Names nothing could satisfy — the Settings "eksik" list.
    pub missing: Vec<String>,
}

/// Resolve `names` against the keychain first, then the env file.
pub fn resolve_secrets(
    names: &[String],
    keychain: &dyn SecretStore,
    env_file: Option<&Path>,
) -> SecretResolution {
    let _ = (names, keychain, env_file);
    todo!("keychain -> env file -> missing report")
}

/// Find every `${secret:name}` reference in a config body.
pub fn extract_secret_refs(text: &str) -> Vec<String> {
    let _ = text;
    todo!("scan for placeholders")
}

#[cfg(test)]
#[path = "tests/secrets.rs"]
mod tests;
