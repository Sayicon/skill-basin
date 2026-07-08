//! Per-machine version pins and the desired-state sync planner.
//!
//! `machines/<machine-id>/pins.json` inside the basin is the lockfile: it
//! says which version of which skill goes to which tool on that machine.
//! The planner diffs this desired state against what is actually on disk
//! and produces actions; `apply_plan` executes them through the existing
//! symlink → junction → copy chain.
//!
//! Ownership rule: SkillBasin only ever touches directories it installed
//! itself. Every managed install leaves a sibling manifest next to the
//! target (`<target>.skillbasin.json`) — NEVER inside it, because a
//! symlinked target would write into the basin's version directory and
//! corrupt its content hash. No sibling manifest → not ours → untouched.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const PINS_FILE: &str = "pins.json";
pub const MANAGED_MANIFEST_SUFFIX: &str = ".skillbasin.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MachinePins {
    pub machine: String,
    #[serde(default)]
    pub pins: Vec<PinEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PinEntry {
    pub skill: String,
    pub version: String,
    /// tool key -> per-target settings
    pub targets: BTreeMap<String, PinTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PinTarget {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// "auto" (symlink→junction→copy) | "copy"
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

fn default_true() -> bool {
    true
}

fn default_strategy() -> String {
    "auto".to_string()
}

impl Default for PinTarget {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: default_strategy(),
        }
    }
}

/// Sibling manifest proving a target directory is managed by SkillBasin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedManifest {
    pub skill: String,
    pub version: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlanAction {
    /// Nothing at the target yet — install this version.
    Install {
        skill: String,
        version: String,
        tool: String,
        target: PathBuf,
        strategy: String,
    },
    /// Managed target exists but pin points at a different version.
    Update {
        skill: String,
        from_version: String,
        to_version: String,
        tool: String,
        target: PathBuf,
        strategy: String,
    },
    /// Managed target exists but no enabled pin wants it anymore.
    Remove {
        skill: String,
        tool: String,
        target: PathBuf,
    },
    /// Unmanaged directory occupies the target path — refuse to touch it.
    Conflict {
        skill: String,
        tool: String,
        target: PathBuf,
    },
}

pub fn machine_pins_path(basin_dir: &Path, machine: &str) -> PathBuf {
    basin_dir.join("machines").join(machine).join(PINS_FILE)
}

pub fn read_machine_pins(basin_dir: &Path, machine: &str) -> Result<MachinePins> {
    let path = machine_pins_path(basin_dir, machine);
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("read pins {:?}", path))?;
    serde_json::from_str(&content).with_context(|| format!("parse pins {:?}", path))
}

pub fn write_machine_pins(basin_dir: &Path, pins: &MachinePins) -> Result<()> {
    let _ = (basin_dir, pins);
    unimplemented!("FAZ 2B")
}

/// Path of the sibling manifest for a given target directory.
pub fn manifest_path_for(target: &Path) -> PathBuf {
    let mut name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    name.push_str(MANAGED_MANIFEST_SUFFIX);
    target.with_file_name(name)
}

pub fn read_managed_manifest(target: &Path) -> Option<ManagedManifest> {
    let content = std::fs::read_to_string(manifest_path_for(target)).ok()?;
    serde_json::from_str(&content).ok()
}

/// Diff desired pins against the actual state of the given tool skill
/// directories. `tool_dirs` maps tool key -> that tool's skills directory.
pub fn plan_sync(
    pins: &MachinePins,
    tool_dirs: &BTreeMap<String, PathBuf>,
) -> Result<Vec<PlanAction>> {
    let _ = (pins, tool_dirs);
    unimplemented!("FAZ 2B")
}

/// Execute a plan against the basin: sources come from
/// `skills/<id>/versions/<version>/`. Returns per-action results.
pub fn apply_plan(basin_dir: &Path, plan: &[PlanAction]) -> Result<Vec<ApplyResult>> {
    let _ = (basin_dir, plan);
    unimplemented!("FAZ 2B")
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub skill: String,
    pub tool: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
#[path = "tests/pins.rs"]
mod tests;
