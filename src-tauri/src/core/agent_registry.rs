//! Basin-level agent adapter registry — additive overlay on top of the
//! hardcoded `tool_adapters::ToolId` enum (47 tools).
//!
//! `basin/agents.json` holds two kinds of entries: vendored Tier2 tools
//! (from vercel-labs/skills' `agents.ts`, `verified: false` + `source`) and
//! user-defined custom tools (`custom: true`, including dirless `mcp`
//! adapters). Neither the enum nor `tool_adapters::default_tool_adapters()`
//! is touched — this module only reads/merges on top.
//!
//! Tier1 (`claude_code`, `cursor`, `antigravity`, `hermes_agent`) is
//! Kerem-verified by hand and always wins: an `agents.json` entry with one
//! of those keys is ignored by `resolve_all_adapters`. Every other key —
//! built-in or not — is overridden by `agents.json` when present.
//!
//! Tauri-independent by design (see `core/sync_engine.rs`/`core/installer.rs`
//! for the same pattern): the FAZ 7 hub-agent CLI reuses this directly.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::tool_adapters::{
    self, expand_custom_tool_path, is_builtin_tool_enabled, is_valid_custom_tool_key, ToolConfig,
};

pub const AGENTS_FILE: &str = "agents.json";
const BUNDLED_DEFAULT: &str = include_str!("../../../agents.default.json");

/// Tier1: Kerem-verified by hand, never overridden by agents.json.
pub const TIER1_KEYS: [&str; 4] = ["claude_code", "cursor", "antigravity", "hermes_agent"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AdapterKind {
    Dir,
    Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub display_name: String,
    pub adapter_kind: AdapterKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_skills_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_skills_dir: Option<String>,
    #[serde(default)]
    pub detect: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_endpoint: Option<String>,
    #[serde(default = "default_strategy")]
    pub default_strategy: String,
    #[serde(default)]
    pub verified: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub custom: bool,
}

fn default_strategy() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRegistry {
    pub version: u32,
    #[serde(default)]
    pub adapters: BTreeMap<String, AgentEntry>,
}

/// Fully-resolved adapter: built-in enum entry or agents.json entry,
/// normalized to one shape for callers (commands/mod.rs, hub-agent).
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAdapter {
    pub key: String,
    pub display_name: String,
    pub adapter_kind: AdapterKind,
    pub skills_dir: Option<PathBuf>,
    pub project_skills_dir: Option<String>,
    pub mcp_endpoint: Option<String>,
    pub default_strategy: String,
    pub is_custom: bool,
    pub verified: bool,
    pub enabled: bool,
}

pub fn default_agent_registry() -> AgentRegistry {
    serde_json::from_str(BUNDLED_DEFAULT).expect("bundled agents.default.json must parse")
}

pub fn read_agent_registry(basin_dir: &Path) -> Result<AgentRegistry> {
    let path = basin_dir.join(AGENTS_FILE);
    if !path.exists() {
        return Ok(default_agent_registry());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

pub fn write_agent_registry(basin_dir: &Path, registry: &AgentRegistry) -> Result<()> {
    let path = basin_dir.join(AGENTS_FILE);
    let json = serde_json::to_string_pretty(registry)?;
    std::fs::write(&path, json).with_context(|| format!("write {}", path.display()))
}

/// Writes the bundled default registry into the basin if `agents.json` is
/// missing — makes pre-FAZ3 basins (no agents.json yet) forward-compatible
/// without a manual migration step.
pub fn ensure_agent_registry_seeded(basin_dir: &Path) -> Result<bool> {
    let path = basin_dir.join(AGENTS_FILE);
    if path.exists() {
        return Ok(false);
    }
    write_agent_registry(basin_dir, &default_agent_registry())?;
    Ok(true)
}

fn resolved_from_builtin(
    adapter: &tool_adapters::ToolAdapter,
    tool_config: &ToolConfig,
) -> ResolvedAdapter {
    let key = adapter.id.as_key().to_string();
    ResolvedAdapter {
        enabled: is_builtin_tool_enabled(tool_config, &key),
        key,
        display_name: adapter.display_name.to_string(),
        adapter_kind: AdapterKind::Dir,
        skills_dir: tool_adapters::resolve_default_path(adapter).ok(),
        project_skills_dir: Some(tool_adapters::project_relative_skills_dir(adapter).to_string()),
        mcp_endpoint: None,
        default_strategy: "auto".to_string(),
        is_custom: false,
        verified: true,
    }
}

fn resolved_from_entry(key: &str, entry: &AgentEntry) -> ResolvedAdapter {
    let skills_dir = entry
        .global_skills_dir
        .as_deref()
        .and_then(|raw| expand_custom_tool_path(raw).ok());
    ResolvedAdapter {
        key: key.to_string(),
        display_name: entry.display_name.clone(),
        adapter_kind: entry.adapter_kind,
        skills_dir,
        project_skills_dir: entry.project_skills_dir.clone(),
        mcp_endpoint: entry.mcp_endpoint.clone(),
        default_strategy: entry.default_strategy.clone(),
        is_custom: entry.custom,
        verified: entry.verified,
        enabled: true,
    }
}

/// Merges built-in enum adapters with the basin's `agents.json`.
///
/// Resolution order: (1) Tier1 always built-in, agents.json ignored for
/// those keys; (2) other built-in keys — agents.json wins if present;
/// (3) remaining agents.json-only entries (Tier2 vendored + custom).
pub fn resolve_all_adapters(
    basin_dir: &Path,
    tool_config: &ToolConfig,
) -> Result<Vec<ResolvedAdapter>> {
    let registry = read_agent_registry(basin_dir)?;
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for adapter in tool_adapters::default_tool_adapters() {
        let key = adapter.id.as_key().to_string();
        if TIER1_KEYS.contains(&key.as_str()) {
            out.push(resolved_from_builtin(&adapter, tool_config));
            seen.insert(key);
        }
    }

    for adapter in tool_adapters::default_tool_adapters() {
        let key = adapter.id.as_key().to_string();
        if seen.contains(&key) {
            continue;
        }
        match registry.adapters.get(&key) {
            Some(entry) => out.push(resolved_from_entry(&key, entry)),
            None => out.push(resolved_from_builtin(&adapter, tool_config)),
        }
        seen.insert(key);
    }

    for (key, entry) in &registry.adapters {
        if seen.contains(key) || TIER1_KEYS.contains(&key.as_str()) {
            continue;
        }
        out.push(resolved_from_entry(key, entry));
    }

    Ok(out)
}

fn validate_custom_key(key: &str) -> Result<()> {
    if TIER1_KEYS.contains(&key) {
        anyhow::bail!("custom agent key conflicts with a Tier1 built-in tool: {key}");
    }
    if !is_valid_custom_tool_key(key) {
        anyhow::bail!("custom agent key contains invalid characters: {key}");
    }
    Ok(())
}

/// Adds a new custom agent entry. Fails if the key is Tier1 or already
/// present (use `update_custom_agent` for existing keys).
pub fn add_custom_agent(basin_dir: &Path, key: &str, entry: AgentEntry) -> Result<AgentRegistry> {
    validate_custom_key(key)?;
    let mut registry = read_agent_registry(basin_dir)?;
    if registry.adapters.contains_key(key) {
        anyhow::bail!("agent key already exists: {key}");
    }
    let mut entry = entry;
    entry.custom = true;
    registry.adapters.insert(key.to_string(), entry);
    write_agent_registry(basin_dir, &registry)?;
    Ok(registry)
}

/// Updates an existing custom agent entry. Refuses to touch vendored
/// (non-custom) entries — those come from `scripts/vendor-agents.mjs`.
pub fn update_custom_agent(
    basin_dir: &Path,
    key: &str,
    entry: AgentEntry,
) -> Result<AgentRegistry> {
    validate_custom_key(key)?;
    let mut registry = read_agent_registry(basin_dir)?;
    match registry.adapters.get(key) {
        Some(existing) if !existing.custom => {
            anyhow::bail!("cannot modify a vendored (non-custom) agent entry: {key}")
        }
        Some(_) => {}
        None => anyhow::bail!("agent key not found: {key}"),
    }
    let mut entry = entry;
    entry.custom = true;
    registry.adapters.insert(key.to_string(), entry);
    write_agent_registry(basin_dir, &registry)?;
    Ok(registry)
}

/// Removes a custom agent entry. Refuses to remove vendored entries.
pub fn remove_custom_agent(basin_dir: &Path, key: &str) -> Result<AgentRegistry> {
    let mut registry = read_agent_registry(basin_dir)?;
    match registry.adapters.get(key) {
        Some(existing) if !existing.custom => {
            anyhow::bail!("cannot remove a vendored (non-custom) agent entry: {key}")
        }
        Some(_) => {}
        None => anyhow::bail!("agent key not found: {key}"),
    }
    registry.adapters.remove(key);
    write_agent_registry(basin_dir, &registry)?;
    Ok(registry)
}

#[cfg(test)]
#[path = "tests/agent_registry.rs"]
mod tests;
