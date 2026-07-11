//! skillbasin-agent — the fleet side of SkillBasin.
//!
//! A machine that can't run the desktop app (a VPS hosting an agent like
//! Hermes) runs this instead: `init` clones the basin repo and records which
//! machine it speaks for; `apply` pulls, diffs this machine's pins against
//! the tool directories, applies the plan with the same engine the desktop
//! app uses, and pushes a status report back into the basin so the Fleet
//! screen can show it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Local agent configuration (lives OUTSIDE the basin: `agent.json` next to
/// wherever the user keeps it, default `~/.skillbasin/agent.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// Git URL (or local path) of the basin repository.
    pub repo: String,
    /// Which machine's pins this agent applies (basin `machines/<id>/`).
    pub machine: String,
    /// Where the basin clone lives on this machine.
    pub basin_dir: PathBuf,
    /// Per-machine tool directory overrides. Tools NOT listed here resolve
    /// through the normal adapter registry (built-ins + basin agents.json);
    /// an entry here always wins — the escape hatch for non-standard mounts.
    #[serde(default)]
    pub tool_dirs: BTreeMap<String, PathBuf>,
}

pub const STATUS_SCHEMA_VERSION: u32 = 1;

/// One applied (or refused) plan action, as reported in status.json.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusAction {
    pub skill: String,
    pub tool: String,
    /// "install" | "update" | "remove" | "conflict"
    pub action: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// machines/<id>/status.json — written after every apply and pushed to the
/// basin remote, so the Fleet screen reads machine health straight from the
/// repo with no extra channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    pub schema_version: u32,
    pub machine: String,
    /// Unix seconds; the UI formats it (the core stays chrono-free).
    pub applied_at_epoch: u64,
    /// False when ANY action failed — partial success must be visible.
    pub ok: bool,
    pub actions: Vec<StatusAction>,
}

/// Clone the basin (or reuse an existing clone) and validate the config.
/// Idempotent: running it again on a healthy setup is a no-op.
pub fn run_init(config: &AgentConfig) -> Result<()> {
    let _ = config;
    todo!("clone basin, validate machine dir")
}

/// Pull → plan this machine's pins → apply → write + push status.json.
/// A failed action is NOT an Err: it lands in the report with ok=false —
/// the fleet must see partial failure, not lose it (docs/lessons.md L2/L7).
pub fn run_apply(config: &AgentConfig) -> Result<StatusReport> {
    let _ = config;
    todo!("pull, plan, apply, report")
}

/// Load agent config from a JSON file (BOM-tolerant).
pub fn load_config(path: &std::path::Path) -> Result<AgentConfig> {
    let _ = path;
    todo!("read agent.json")
}
