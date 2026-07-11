//! Fleet health: the status report every hub-agent writes to
//! `machines/<id>/status.json` after an apply, and the reader the Fleet
//! screen uses. One schema, defined once — the agent and the app must
//! never drift apart on it.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
/// basin remote, so fleet health travels with the repo itself.
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

/// Read a machine's status report if it has one. A machine that never ran
/// an apply (or predates status reporting) simply has no report — that is
/// a valid state, not an error. A CORRUPT report is an error: hiding it
/// would show a broken machine as merely "quiet".
pub fn read_status(basin_dir: &Path, machine: &str) -> Result<Option<StatusReport>> {
    let path = basin_dir.join("machines").join(machine).join("status.json");
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("read {path:?}")),
    };
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let report =
        serde_json::from_slice(bytes).with_context(|| format!("parse {path:?}"))?;
    Ok(Some(report))
}

/// List every machine directory in the basin (the fleet roster).
pub fn list_machines(basin_dir: &Path) -> Result<Vec<String>> {
    let machines_dir = basin_dir.join("machines");
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&machines_dir) {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(err) => return Err(err).with_context(|| format!("read {machines_dir:?}")),
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            out.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    out.sort();
    Ok(out)
}
