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

pub const KEYCHAIN_SERVICE: &str = "skillbasin";

impl SecretStore for OsKeychain {
    fn get(&self, name: &str) -> Option<String> {
        keyring::Entry::new(KEYCHAIN_SERVICE, name)
            .ok()?
            .get_password()
            .ok()
    }
}

#[derive(Debug, Default)]
pub struct SecretResolution {
    pub resolved: BTreeMap<String, String>,
    pub sources: BTreeMap<String, SecretSource>,
    /// Names nothing could satisfy — the Settings "eksik" list.
    pub missing: Vec<String>,
}

/// Parse a dotenv-style file: `KEY=value` lines, `#` comments, optional
/// `export ` prefix, UTF-8 BOM tolerated. Values are taken verbatim.
fn read_env_file(path: &Path) -> BTreeMap<String, String> {
    let Ok(bytes) = std::fs::read(path) else {
        return BTreeMap::new();
    };
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let text = String::from_utf8_lossy(bytes);
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        if let Some((key, value)) = line.split_once('=') {
            out.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    out
}

/// Resolve `names` against the keychain first, then the env file.
pub fn resolve_secrets(
    names: &[String],
    keychain: &dyn SecretStore,
    env_file: Option<&Path>,
) -> SecretResolution {
    let env_map = env_file.map(read_env_file).unwrap_or_default();
    let mut out = SecretResolution::default();
    for name in names {
        if let Some(value) = keychain.get(name) {
            out.resolved.insert(name.clone(), value);
            out.sources.insert(name.clone(), SecretSource::Keychain);
        } else if let Some(value) = env_map.get(name) {
            out.resolved.insert(name.clone(), value.clone());
            out.sources.insert(name.clone(), SecretSource::EnvFile);
        } else {
            out.missing.push(name.clone());
        }
    }
    out
}

/// Find every `${secret:name}` reference in a config body.
pub fn extract_secret_refs(text: &str) -> Vec<String> {
    const OPEN: &str = "${secret:";
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        rest = &rest[start + OPEN.len()..];
        let Some(end) = rest.find('}') else { break };
        let name = rest[..end].trim();
        if !name.is_empty() && !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
        rest = &rest[end + 1..];
    }
    out
}

#[cfg(test)]
#[path = "tests/secrets.rs"]
mod tests;
