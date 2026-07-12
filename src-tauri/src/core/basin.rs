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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Default for SecurityInfo {
    fn default() -> Self {
        Self {
            scan_status: default_scan_status(),
            license: None,
            scanned_at: None,
        }
    }
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
    if let Some(v) = semver {
        return v.to_string();
    }
    let raw = content_hash
        .split_once(':')
        .map_or(content_hash, |(_, rest)| rest);
    let hash4: String = raw.chars().take(4).collect();
    format!("{}.{}", date, hash4)
}

/// Initialize a basin directory: manifest + git repo. Idempotent — an
/// existing manifest is returned untouched.
pub fn basin_init(basin_dir: &Path, name: &str, created_at: &str) -> Result<BasinManifest> {
    std::fs::create_dir_all(basin_dir)
        .with_context(|| format!("create basin dir {:?}", basin_dir))?;

    if !basin_dir.join(".git").exists() {
        // Fixed initial branch: basins must behave the same on every machine
        // regardless of the local init.defaultBranch config.
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        git2::Repository::init_opts(basin_dir, &opts)
            .with_context(|| format!("git init basin {:?}", basin_dir))?;
    }

    let manifest_path = basin_dir.join(BASIN_MANIFEST_FILE);
    if manifest_path.exists() {
        return read_manifest(basin_dir);
    }

    let manifest = BasinManifest {
        schema_version: BASIN_SCHEMA_VERSION,
        name: name.to_string(),
        created_at: created_at.to_string(),
    };
    write_json_pretty(&manifest_path, &manifest)?;
    std::fs::create_dir_all(basin_dir.join("skills"))
        .with_context(|| format!("create skills dir in {:?}", basin_dir))?;
    Ok(manifest)
}

pub fn read_manifest(basin_dir: &Path) -> Result<BasinManifest> {
    let path = basin_dir.join(BASIN_MANIFEST_FILE);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("read basin manifest {:?}", path))?;
    serde_json::from_str(&content).with_context(|| format!("parse basin manifest {:?}", path))
}

pub(crate) fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create dir {:?}", parent))?;
    }
    let json = serde_json::to_string_pretty(value).context("serialize json")?;
    std::fs::write(path, json).with_context(|| format!("write {:?}", path))
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
    let path = skill_dir(basin_dir, &meta.id).join(SKILL_META_FILE);
    write_json_pretty(&path, meta)
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
    let mut meta = match read_skill_meta(basin_dir, id) {
        Ok(existing) => existing,
        Err(_) => SkillMeta {
            id: id.to_string(),
            display_name: id.to_string(),
            source: source.clone().unwrap_or(SkillSource {
                source_type: "local".to_string(),
                ..Default::default()
            }),
            tags: Vec::new(),
            latest: label.to_string(),
            versions: BTreeMap::new(),
            security: SecurityInfo::default(),
        },
    };

    let target = version_dir(basin_dir, id, label);
    if meta.versions.contains_key(label) || target.exists() {
        anyhow::bail!("version {} of skill {} already exists in basin", label, id);
    }

    crate::core::sync_engine::copy_dir_recursive(src_dir, &target)
        .with_context(|| format!("copy skill version into {:?}", target))?;
    let content_hash = format!(
        "sha256:{}",
        crate::core::content_hash::hash_dir(&target)
            .with_context(|| format!("hash skill version {:?}", target))?
    );

    let info = VersionInfo {
        content_hash,
        added_at: added_at.to_string(),
    };
    meta.versions.insert(label.to_string(), info.clone());
    meta.latest = label.to_string();
    if let Some(source) = source {
        meta.source = source;
    }
    write_skill_meta(basin_dir, &meta)?;
    Ok(info)
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
    let mut report = MigrationReport::default();
    let entries = std::fs::read_dir(central_dir)
        .with_context(|| format!("read central repo {:?}", central_dir))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue; // loose files at the central root are not skills
        }
        let Some(id) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if read_skill_meta(basin_dir, &id).is_ok() {
            report.skipped.push(id);
            continue;
        }

        let src_hash = format!(
            "sha256:{}",
            crate::core::content_hash::hash_dir(&path)
                .with_context(|| format!("hash central skill {:?}", path))?
        );
        let label = version_label(None, date, &src_hash);
        let source = sources.get(&id).cloned();
        add_skill_version(basin_dir, &id, &path, &label, date, source)?;
        report.migrated.push(id);
    }

    report.migrated.sort();
    report.skipped.sort();
    Ok(report)
}

/// Stage everything and commit. Returns the new commit id, or `None` when
/// the working tree is clean. Creates the initial commit if the repo has none.
pub fn basin_commit_all(basin_dir: &Path, message: &str) -> Result<Option<String>> {
    let repo = git2::Repository::open(basin_dir)
        .with_context(|| format!("open basin repo {:?}", basin_dir))?;

    let mut index = repo.index().context("open basin index")?;
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .context("stage basin changes")?;
    index.write().context("write basin index")?;
    let tree_id = index.write_tree().context("write basin tree")?;

    let head_commit = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    if let Some(parent) = &head_commit {
        if parent.tree_id() == tree_id {
            return Ok(None); // clean tree, nothing to commit
        }
    }

    let signature = repo
        .signature()
        .or_else(|_| git2::Signature::now("SkillBasin", "basin@skillbasin.local"))
        .context("basin commit signature")?;
    let tree = repo.find_tree(tree_id).context("find basin tree")?;
    let parents: Vec<&git2::Commit> = head_commit.iter().collect();
    let oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .context("commit basin changes")?;
    Ok(Some(oid.to_string()))
}

/// Clone a basin from `url` (any git remote, including a local path) into
/// `dest`, or pull if `dest` already has it. Returns HEAD revision.
pub fn basin_clone_or_pull(url: &str, dest: &Path) -> Result<String> {
    crate::core::git_fetcher::clone_or_pull(url, dest, None, None, None)
}

/// Push the basin to its `origin` remote via the system git binary, which
/// picks up the user's credentials, proxies and OS trust store.
pub fn basin_push(basin_dir: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["push", "-u", "origin", "HEAD"])
        .current_dir(basin_dir)
        .output()
        .with_context(|| format!("run git push in {:?}", basin_dir))?;
    if !output.status.success() {
        anyhow::bail!(
            "git push failed for basin {:?}: {}",
            basin_dir,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Push, and if the remote has moved on, rebase THIS machine's commit on top
/// and push again — once. A fleet agent races the desktop (both push to the
/// same basin); a plain push would be rejected and, crucially, a later
/// `reset --hard` pull would then destroy the un-pushed commit. Rebase keeps
/// the local commit; `git pull` is NOT used because its reset-hard path would
/// discard it. Returns Err only if the retry also fails.
pub fn basin_push_with_rebase(basin_dir: &Path) -> Result<()> {
    if basin_push(basin_dir).is_ok() {
        return Ok(());
    }
    let run = |args: &[&str]| -> Result<std::process::Output> {
        std::process::Command::new("git")
            .args(args)
            .current_dir(basin_dir)
            .output()
            .with_context(|| format!("run git {args:?} in {basin_dir:?}"))
    };
    // Fetch the remote head and replay our single commit on top of it.
    let fetch = run(&["fetch", "origin"])?;
    if !fetch.status.success() {
        anyhow::bail!(
            "git fetch failed during push retry: {}",
            String::from_utf8_lossy(&fetch.stderr).trim()
        );
    }
    let rebase = run(&["rebase", "origin/HEAD"])?;
    if !rebase.status.success() {
        let _ = run(&["rebase", "--abort"]);
        anyhow::bail!(
            "git rebase failed during push retry (conflicting basin edits?): {}",
            String::from_utf8_lossy(&rebase.stderr).trim()
        );
    }
    basin_push(basin_dir)
}

/// Record the current content of `updated_dir` as a new version of `id`
/// unless it is content-identical to the skill's latest version.
/// Pins are never touched here — updates only ADD versions (never overwrite).
pub fn record_update_as_version(
    basin_dir: &Path,
    id: &str,
    updated_dir: &Path,
    semver: Option<&str>,
    date: &str,
) -> Result<Option<(String, VersionInfo)>> {
    let new_hash = format!(
        "sha256:{}",
        crate::core::content_hash::hash_dir(updated_dir)
            .with_context(|| format!("hash updated skill {:?}", updated_dir))?
    );

    if let Ok(meta) = read_skill_meta(basin_dir, id) {
        if let Some(latest) = meta.versions.get(&meta.latest) {
            if latest.content_hash == new_hash {
                return Ok(None); // unchanged content — never churn versions
            }
        }
    }

    let label = version_label(semver, date, &new_hash);
    let info = add_skill_version(basin_dir, id, updated_dir, &label, date, None)?;
    Ok(Some((label, info)))
}

/// Civil date (`YYYY-MM-DD`) from days since the Unix epoch.
/// Howard Hinnant's `civil_from_days` — no chrono dependency needed.
pub fn unix_days_to_date(days: i64) -> String {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    if month <= 2 {
        year += 1;
    }
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Today's UTC date as `YYYY-MM-DD` — the standard `added_at` / label date.
pub fn today_utc_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    unix_days_to_date(secs / 86_400)
}

#[cfg(test)]
#[path = "tests/basin.rs"]
mod tests;
