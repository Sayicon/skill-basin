//! This installation's stable identifier for `machines/<id>/pins.json`.
//!
//! Best-effort human-readable (env hostname vars), falling back to a
//! generated slug — persisted once so it never changes underfoot.

use anyhow::{Context, Result};
use uuid::Uuid;

use super::skill_store::SkillStore;

pub const MACHINE_ID_SETTING: &str = "machine_id";

fn detect_hostname() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

fn slugify(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    slug.trim_matches('-').to_string()
}

/// Returns this machine's stable id, generating and persisting one the
/// first time it's needed.
pub fn current_machine_id(store: &SkillStore) -> Result<String> {
    if let Some(existing) = store
        .get_setting(MACHINE_ID_SETTING)
        .context("read machine_id setting")?
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(existing);
    }

    let generated = detect_hostname()
        .map(|name| slugify(&name))
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| format!("machine-{}", Uuid::new_v4().simple()));

    store
        .set_setting(MACHINE_ID_SETTING, &generated)
        .context("persist machine_id setting")?;
    Ok(generated)
}

#[cfg(test)]
#[path = "tests/machine.rs"]
mod tests;
