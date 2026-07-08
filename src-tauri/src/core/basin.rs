//! Versioned skill pool ("basin") on top of the flat central repo.
//!
//! Layout inside a basin directory (a git repo owned by the user):
//!
//! ```text
//! basin/
//! ├── basin.json                     # manifest: schema version, name
//! └── skills/
//!     └── <skill-id>/
//!         ├── skill.json             # metadata: source, tags, versions, latest
//!         └── versions/
//!             └── <label>/           # full skill directory, one per version
//! ```
//!
//! Versions are materialized side by side so different tools on the same
//! machine can use different versions of the same skill at the same time.
//! `content_hash::hash_dir` output is OS-independent (separators normalized),
//! which makes version identity portable across machines.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const BASIN_SCHEMA_VERSION: u32 = 1;
pub const BASIN_MANIFEST_FILE: &str = "basin.json";
pub const SKILL_META_FILE: &str = "skill.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BasinManifest {
    pub schema_version: u32,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillSource {
    /// "git" | "skills_sh" | "local" | "zip"
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpath: Option<String>,
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub content_hash: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecurityInfo {
    /// "clean" | "flagged" | "unscanned"
    #[serde(default = "default_scan_status")]
    pub scan_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scanned_at: Option<String>,
}

fn default_scan_status() -> String {
    "unscanned".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillMeta {
    pub id: String,
    pub display_name: String,
    pub source: SkillSource,
    #[serde(default)]
    pub tags: Vec<String>,
    pub latest: String,
    /// version label -> info; BTreeMap keeps serialization deterministic.
    pub versions: BTreeMap<String, VersionInfo>,
    #[serde(default)]
    pub security: SecurityInfo,
}

#[derive(Debug, Default, PartialEq)]
pub struct MigrationReport {
    pub migrated: Vec<String>,
    pub skipped: Vec<String>,
}

/// Version label for a new snapshot: semver from the source wins; otherwise
/// `YYYY-MM-DD.<hash4>` where hash4 is the first 4 hex chars of the content
/// hash (after the algorithm prefix, if any).
pub fn version_label(semver: Option<&str>, date: &str, content_hash: &str) -> String {
    let _ = (semver, date, content_hash);
    unimplemented!("FAZ 1B")
}

/// Initialize a basin directory: manifest + git repo. Idempotent.
pub fn basin_init(basin_dir: &Path, name: &str, created_at: &str) -> Result<BasinManifest> {
    let _ = (basin_dir, name, created_at);
    unimplemented!("FAZ 1B")
}

pub fn read_manifest(basin_dir: &Path) -> Result<BasinManifest> {
    let _ = basin_dir;
    unimplemented!("FAZ 1B")
}

pub fn skill_dir(basin_dir: &Path, id: &str) -> PathBuf {
    basin_dir.join("skills").join(id)
}

pub fn version_dir(basin_dir: &Path, id: &str, label: &str) -> PathBuf {
    skill_dir(basin_dir, id).join("versions").join(label)
}

pub fn read_skill_meta(basin_dir: &Path, id: &str) -> Result<SkillMeta> {
    let path = skill_dir(basin_dir, id).join(SKILL_META_FILE);
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("read skill meta {:?}", path))?;
    serde_json::from_str(&content).with_context(|| format!("parse skill meta {:?}", path))
}

pub fn write_skill_meta(basin_dir: &Path, meta: &SkillMeta) -> Result<()> {
    let _ = (basin_dir, meta);
    unimplemented!("FAZ 1B")
}

/// Copy `src_dir` into the basin as a new version of `id` and update the
/// skill metadata (`latest`, `versions`). Fails if the label already exists.
pub fn add_skill_version(
    basin_dir: &Path,
    id: &str,
    src_dir: &Path,
    label: &str,
    added_at: &str,
    source: Option<SkillSource>,
) -> Result<VersionInfo> {
    let _ = (basin_dir, id, src_dir, label, added_at, source);
    unimplemented!("FAZ 1B")
}

/// Migrate a flat central repo (one directory per skill) into basin layout.
/// Every skill becomes an initial version; source info is carried from the
/// store when available. Skills already present in the basin are skipped.
pub fn migrate_central_to_basin(
    central_dir: &Path,
    basin_dir: &Path,
    sources: &BTreeMap<String, SkillSource>,
    date: &str,
) -> Result<MigrationReport> {
    let _ = (central_dir, basin_dir, sources, date);
    unimplemented!("FAZ 1B")
}

#[cfg(test)]
#[path = "tests/basin.rs"]
mod tests;
