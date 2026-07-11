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

// The status schema lives in the shared core so the desktop app's Fleet
// screen and this agent can never drift apart on it.
pub use app_lib::core::fleet::{StatusAction, StatusReport, STATUS_SCHEMA_VERSION};

/// Clone the basin (or reuse an existing clone) and validate the config.
/// Idempotent: running it again on a healthy setup is a no-op pull.
pub fn run_init(config: &AgentConfig) -> Result<()> {
    app_lib::core::basin::basin_clone_or_pull(&config.repo, &config.basin_dir)?;
    // A real basin carries its manifest; anything else is a mis-pointed repo
    // and better refused now than half-applied later.
    app_lib::core::basin::read_manifest(&config.basin_dir)?;
    Ok(())
}

/// Resolve where each tool's skills live on THIS machine: the adapter
/// registry (built-ins + basin agents.json) first, then the config's
/// per-machine overrides on top — the escape hatch for non-standard mounts.
fn resolve_tool_dirs(config: &AgentConfig) -> Result<BTreeMap<String, PathBuf>> {
    use app_lib::core::agent_registry::resolve_all_adapters;
    use app_lib::core::tool_adapters::ToolConfig;

    let mut dirs: BTreeMap<String, PathBuf> =
        resolve_all_adapters(&config.basin_dir, &ToolConfig::default())?
            .into_iter()
            .filter(|adapter| adapter.enabled)
            .filter_map(|adapter| adapter.skills_dir.map(|dir| (adapter.key, dir)))
            .collect();
    for (tool, dir) in &config.tool_dirs {
        dirs.insert(tool.clone(), dir.clone());
    }
    Ok(dirs)
}

fn action_name(action: &app_lib::core::pins::PlanAction) -> &'static str {
    use app_lib::core::pins::PlanAction;
    match action {
        PlanAction::Install { .. } => "install",
        PlanAction::Update { .. } => "update",
        PlanAction::Remove { .. } => "remove",
        PlanAction::Conflict { .. } => "conflict",
    }
}

/// Pull → plan this machine's pins → apply → write + push status.json.
/// A failed action is NOT an Err: it lands in the report with ok=false —
/// the fleet must see partial failure, not lose it (docs/lessons.md L2/L7).
pub fn run_apply(config: &AgentConfig) -> Result<StatusReport> {
    use app_lib::core::basin;
    use app_lib::core::pins;

    basin::basin_clone_or_pull(&config.repo, &config.basin_dir)?;

    let machine_pins = pins::read_machine_pins_or_empty(&config.basin_dir, &config.machine)?;
    let tool_dirs = resolve_tool_dirs(config)?;
    let plan = pins::plan_sync(&machine_pins, &tool_dirs)?;
    let results = pins::apply_plan(&config.basin_dir, &plan)?;

    // apply_plan returns one result per plan action, in order.
    let actions: Vec<StatusAction> = plan
        .iter()
        .zip(results.iter())
        .map(|(action, result)| StatusAction {
            skill: result.skill.clone(),
            tool: result.tool.clone(),
            action: action_name(action).to_string(),
            ok: result.ok,
            error: result.error.clone(),
        })
        .collect();

    let report = StatusReport {
        schema_version: STATUS_SCHEMA_VERSION,
        machine: config.machine.clone(),
        applied_at_epoch: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        ok: actions.iter().all(|a| a.ok),
        actions,
    };

    let machine_dir = config.basin_dir.join("machines").join(&config.machine);
    std::fs::create_dir_all(&machine_dir)?;
    std::fs::write(
        machine_dir.join("status.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    basin::basin_commit_all(
        &config.basin_dir,
        &format!("status: {} apply", config.machine),
    )?;
    basin::basin_push(&config.basin_dir)?;

    Ok(report)
}

/// Load agent config from a JSON file (BOM-tolerant: Windows editors and
/// PowerShell 5.1 prepend one, and a hand-written config must still read).
pub fn load_config(path: &std::path::Path) -> Result<AgentConfig> {
    let bytes = std::fs::read(path)?;
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    Ok(serde_json::from_slice(bytes)?)
}
