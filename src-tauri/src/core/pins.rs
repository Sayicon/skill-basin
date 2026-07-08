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
    let path = machine_pins_path(basin_dir, &pins.machine);
    crate::core::basin::write_json_pretty(&path, pins)
}

/// Like [`read_machine_pins`], but a missing pins.json (first pin ever set
/// on this machine) is an empty pin set rather than an error.
pub fn read_machine_pins_or_empty(basin_dir: &Path, machine: &str) -> Result<MachinePins> {
    if !machine_pins_path(basin_dir, machine).exists() {
        return Ok(MachinePins {
            machine: machine.to_string(),
            pins: Vec::new(),
        });
    }
    read_machine_pins(basin_dir, machine)
}

/// Pins `tool` to `skill`@`version` (moving it off any other version of the
/// same skill it was pinned to), writes pins.json, and immediately
/// re-applies the sync plan so the change takes effect on disk.
pub fn set_pin(
    basin_dir: &Path,
    machine: &str,
    skill: &str,
    version: &str,
    tool: &str,
    target: PinTarget,
    tool_dirs: &BTreeMap<String, PathBuf>,
) -> Result<(MachinePins, Vec<ApplyResult>)> {
    let mut pins = read_machine_pins_or_empty(basin_dir, machine)?;

    for entry in pins.pins.iter_mut() {
        if entry.skill == skill && entry.version != version {
            entry.targets.remove(tool);
        }
    }
    pins.pins
        .retain(|entry| !(entry.skill == skill && entry.targets.is_empty()));

    match pins
        .pins
        .iter_mut()
        .find(|entry| entry.skill == skill && entry.version == version)
    {
        Some(entry) => {
            entry.targets.insert(tool.to_string(), target);
        }
        None => {
            let mut targets = BTreeMap::new();
            targets.insert(tool.to_string(), target);
            pins.pins.push(PinEntry {
                skill: skill.to_string(),
                version: version.to_string(),
                targets,
            });
        }
    }

    write_machine_pins(basin_dir, &pins)?;
    let plan = plan_sync(&pins, tool_dirs)?;
    let results = apply_plan(basin_dir, &plan)?;
    Ok((pins, results))
}

/// Removes `tool`'s pin for `skill` (whichever version it was pinned to)
/// and re-applies — the managed install is removed from disk.
pub fn unset_pin(
    basin_dir: &Path,
    machine: &str,
    skill: &str,
    tool: &str,
    tool_dirs: &BTreeMap<String, PathBuf>,
) -> Result<(MachinePins, Vec<ApplyResult>)> {
    let mut pins = read_machine_pins_or_empty(basin_dir, machine)?;
    for entry in pins.pins.iter_mut() {
        if entry.skill == skill {
            entry.targets.remove(tool);
        }
    }
    pins.pins
        .retain(|entry| !(entry.skill == skill && entry.targets.is_empty()));

    write_machine_pins(basin_dir, &pins)?;
    let plan = plan_sync(&pins, tool_dirs)?;
    let results = apply_plan(basin_dir, &plan)?;
    Ok((pins, results))
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
    // Desired: (tool, skill) -> (version, strategy), enabled targets only.
    let mut desired: BTreeMap<(String, String), (String, String)> = BTreeMap::new();
    for entry in &pins.pins {
        for (tool, target) in &entry.targets {
            if !target.enabled || !tool_dirs.contains_key(tool) {
                continue;
            }
            desired.insert(
                (tool.clone(), entry.skill.clone()),
                (entry.version.clone(), target.strategy.clone()),
            );
        }
    }

    // Actual: (tool, skill) -> manifest, for entries carrying our manifest.
    let mut managed: BTreeMap<(String, String), ManagedManifest> = BTreeMap::new();
    for (tool, dir) in tool_dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue; // tool dir may not exist yet — nothing managed there
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() && std::fs::symlink_metadata(&path).is_err() {
                continue;
            }
            if let Some(manifest) = read_managed_manifest(&path) {
                let skill = manifest.skill.clone();
                managed.insert((tool.clone(), skill), manifest);
            }
        }
    }

    let mut plan = Vec::new();

    for ((tool, skill), (version, strategy)) in &desired {
        let target = tool_dirs[tool].join(skill);
        match managed.get(&(tool.clone(), skill.clone())) {
            Some(manifest) if &manifest.version == version => {} // converged
            Some(manifest) => plan.push(PlanAction::Update {
                skill: skill.clone(),
                from_version: manifest.version.clone(),
                to_version: version.clone(),
                tool: tool.clone(),
                target,
                strategy: strategy.clone(),
            }),
            None => {
                // Occupied by something without our manifest? Hands off.
                if std::fs::symlink_metadata(&target).is_ok() {
                    plan.push(PlanAction::Conflict {
                        skill: skill.clone(),
                        tool: tool.clone(),
                        target,
                    });
                } else {
                    plan.push(PlanAction::Install {
                        skill: skill.clone(),
                        version: version.clone(),
                        tool: tool.clone(),
                        target,
                        strategy: strategy.clone(),
                    });
                }
            }
        }
    }

    for (tool, skill) in managed.keys() {
        if !desired.contains_key(&(tool.clone(), skill.clone())) {
            plan.push(PlanAction::Remove {
                skill: skill.clone(),
                tool: tool.clone(),
                target: tool_dirs[tool].join(skill),
            });
        }
    }

    Ok(plan)
}

fn sync_with_strategy(source: &Path, target: &Path, strategy: &str) -> Result<()> {
    if strategy == "copy" {
        crate::core::sync_engine::sync_dir_copy_with_overwrite(source, target, true)?;
    } else {
        crate::core::sync_engine::sync_dir_hybrid_with_overwrite(source, target, true)?;
    }
    Ok(())
}

fn install_version(
    basin_dir: &Path,
    skill: &str,
    version: &str,
    target: &Path,
    strategy: &str,
) -> Result<()> {
    let source = crate::core::basin::version_dir(basin_dir, skill, version);
    if !source.exists() {
        anyhow::bail!("version {} of {} not found in basin", version, skill);
    }
    sync_with_strategy(&source, target, strategy)?;

    // Content hash from metadata when recorded; hash the source otherwise.
    let content_hash = crate::core::basin::read_skill_meta(basin_dir, skill)
        .ok()
        .and_then(|meta| meta.versions.get(version).map(|v| v.content_hash.clone()))
        .map_or_else(
            || -> Result<String> {
                Ok(format!(
                    "sha256:{}",
                    crate::core::content_hash::hash_dir(&source)?
                ))
            },
            Ok,
        )?;
    let manifest = ManagedManifest {
        skill: skill.to_string(),
        version: version.to_string(),
        content_hash,
    };
    crate::core::basin::write_json_pretty(&manifest_path_for(target), &manifest)
}

/// Execute a plan against the basin: sources come from
/// `skills/<id>/versions/<version>/`. Returns per-action results.
/// One failing action never aborts the rest of the plan.
pub fn apply_plan(basin_dir: &Path, plan: &[PlanAction]) -> Result<Vec<ApplyResult>> {
    let mut results = Vec::with_capacity(plan.len());
    for action in plan {
        let (skill, tool, outcome) = match action {
            PlanAction::Install {
                skill,
                version,
                tool,
                target,
                strategy,
            }
            | PlanAction::Update {
                skill,
                to_version: version,
                tool,
                target,
                strategy,
                ..
            } => (
                skill,
                tool,
                install_version(basin_dir, skill, version, target, strategy),
            ),
            PlanAction::Remove {
                skill,
                tool,
                target,
            } => {
                let removed = crate::core::sync_engine::remove_path_any(target).and_then(|()| {
                    let manifest = manifest_path_for(target);
                    if manifest.exists() {
                        std::fs::remove_file(&manifest)
                            .with_context(|| format!("remove manifest {:?}", manifest))?;
                    }
                    Ok(())
                });
                (skill, tool, removed)
            }
            PlanAction::Conflict {
                skill,
                tool,
                target,
            } => (
                skill,
                tool,
                Err(anyhow::anyhow!(
                    "unmanaged directory at {:?} — refusing to overwrite; \
                     remove it or import it into the basin first",
                    target
                )),
            ),
        };
        results.push(ApplyResult {
            skill: skill.clone(),
            tool: tool.clone(),
            ok: outcome.is_ok(),
            error: outcome.err().map(|err| format!("{:#}", err)),
        });
    }
    Ok(results)
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
