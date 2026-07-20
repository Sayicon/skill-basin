//! Secret resolution for basin-carried configs.
//!
//! The basin only ever carries `${secret:name}` placeholders; real values
//! live machine-local. Resolution order: OS keychain (Windows Credential
//! Manager / macOS Keychain / libsecret) first, then `~/.skillbasin/
//! secrets.env` — and anything still unresolved is REPORTED, never invented,
//! so the Settings screen can show exactly which keys a machine is missing.
//!
//! # Scope: reporting only, on purpose (docs/DECISIONS.md D16)
//!
//! This module answers "can this machine satisfy that secret?" and nothing
//! else. It deliberately does NOT substitute resolved values into any file:
//! nothing reads or writes the basin's `mcp/` directory today, and the fleet
//! agent never touches it. That is a drawn boundary, not an unfinished
//! implementation — read it as such before "completing" it.
//!
//! Materializing configs needs product answers first (which tool, which path
//! and format, merge vs overwrite of the user's existing config, plaintext on
//! disk with what permissions, and whether the agent does it unattended on a
//! remote machine). Getting those wrong writes a plaintext credential to the
//! wrong place, so the code stops here until they are decided.

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

/// Outcome of a keychain lookup.
///
/// "Not stored" and "could not ask" are different facts and must not collapse
/// into one: a locked keychain reported as `Absent` sends the user off to
/// re-enter a secret they already have, and silently hides that the machine's
/// credential store is unreachable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretLookup {
    Found(String),
    /// The store answered, and there is no such entry.
    Absent,
    /// The store could not be consulted (locked, no backend, D-Bus down...).
    Unavailable(String),
}

/// Abstraction over the OS credential store so resolution order is testable
/// without touching a real keychain.
pub trait SecretStore {
    fn get(&self, name: &str) -> SecretLookup;
}

/// The real OS keychain, service-scoped to SkillBasin.
pub struct OsKeychain;

pub const KEYCHAIN_SERVICE: &str = "skillbasin";

impl SecretStore for OsKeychain {
    fn get(&self, name: &str) -> SecretLookup {
        let entry = match keyring::Entry::new(KEYCHAIN_SERVICE, name) {
            Ok(entry) => entry,
            Err(err) => return SecretLookup::Unavailable(err.to_string()),
        };
        match entry.get_password() {
            Ok(value) => SecretLookup::Found(value),
            Err(keyring::Error::NoEntry) => SecretLookup::Absent,
            Err(err) => SecretLookup::Unavailable(err.to_string()),
        }
    }
}

#[derive(Debug, Default)]
pub struct SecretResolution {
    pub resolved: BTreeMap<String, String>,
    pub sources: BTreeMap<String, SecretSource>,
    /// Names nothing could satisfy — the Settings "eksik" list.
    pub missing: Vec<String>,
    /// Names whose keychain lookup could not be performed at all, with why.
    /// Distinct from `missing`: the secret may well be stored, we just could
    /// not ask. Telling the user to re-enter it would be wrong advice.
    pub unavailable: BTreeMap<String, String>,
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
            let value = value.trim();
            // A quoted value keeps its inner text verbatim (spaces included);
            // an unquoted one is trimmed. Without this, a secret like
            // `TOKEN="a b "` resolves WITH the quotes and loses the trailing
            // space — i.e. to the wrong credential.
            let value = if value.len() >= 2
                && ((value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\'')))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };
            out.insert(key.trim().to_string(), value.to_string());
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
        let lookup = keychain.get(name);
        if let SecretLookup::Found(value) = lookup {
            out.resolved.insert(name.clone(), value);
            out.sources.insert(name.clone(), SecretSource::Keychain);
            continue;
        }
        // The env file is still a legitimate answer even when the keychain is
        // unreachable, so fall through to it either way.
        if let Some(value) = env_map.get(name) {
            out.resolved.insert(name.clone(), value.clone());
            out.sources.insert(name.clone(), SecretSource::EnvFile);
            continue;
        }
        match lookup {
            SecretLookup::Unavailable(reason) => {
                out.unavailable.insert(name.clone(), reason);
            }
            _ => out.missing.push(name.clone()),
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
