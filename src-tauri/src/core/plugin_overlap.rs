//! Detect skills that an enabled Claude Code plugin already provides.
//!
//! SkillBasin syncs a skill into `~/.claude/skills/<name>`, but Claude Code
//! ALSO loads skills bundled with enabled plugins, exposed namespaced as
//! `<plugin>:<name>`. A synced skill whose name matches a plugin-provided one
//! produces two live entries in Claude Code's skill listing: context bloat plus
//! non-deterministic routing, with nothing to tell the user it happened.
//!
//! This module resolves the set of plugin-provided skill names so the sync path
//! can surface the overlap instead of duplicating silently. Every file read here
//! is best-effort: a missing or malformed file shrinks the result rather than
//! erroring, so an overlap check can only ever ADD a warning, never break a sync.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Map of skill `name` -> the enabled Claude Code plugin key
/// (`"<plugin>@<marketplace>"`) that provides it, resolved from the Claude Code
/// config under `claude_home` (`~/.claude`). Empty when nothing overlaps or the
/// config can't be read.
pub fn enabled_plugin_skills(claude_home: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let enabled = enabled_plugin_keys(claude_home);
    if enabled.is_empty() {
        return out;
    }
    let installed = installed_plugin_paths(claude_home);
    for key in enabled {
        let Some(install_path) = installed.get(&key) else {
            continue;
        };
        for name in skill_names_under(Path::new(install_path)) {
            // First plugin to claim a name wins the attribution; the warning
            // only needs one concrete culprit to name.
            out.entry(name).or_insert_with(|| key.clone());
        }
    }
    out
}

fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    // Tolerate a UTF-8 BOM the same way the rest of the app's config reads do.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    serde_json::from_str(text).ok()
}

/// Plugin keys with `enabledPlugins["<key>"] == true` in `~/.claude/settings.json`.
fn enabled_plugin_keys(claude_home: &Path) -> Vec<String> {
    let Some(settings) = read_json(&claude_home.join("settings.json")) else {
        return Vec::new();
    };
    let Some(map) = settings.get("enabledPlugins").and_then(Value::as_object) else {
        return Vec::new();
    };
    map.iter()
        .filter(|(_, v)| v.as_bool() == Some(true))
        .map(|(k, _)| k.clone())
        .collect()
}

/// Plugin key -> install path, from `~/.claude/plugins/installed_plugins.json`.
fn installed_plugin_paths(claude_home: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let path = claude_home.join("plugins").join("installed_plugins.json");
    let Some(json) = read_json(&path) else {
        return out;
    };
    let Some(plugins) = json.get("plugins").and_then(Value::as_object) else {
        return out;
    };
    for (key, installs) in plugins {
        // Each plugin key maps to an array of install records; take the first
        // one that carries an installPath.
        if let Some(path) = installs.as_array().and_then(|arr| {
            arr.iter()
                .find_map(|rec| rec.get("installPath").and_then(Value::as_str))
        }) {
            out.insert(key.clone(), path.to_string());
        }
    }
    out
}

/// Skill `name`s declared under a plugin install: both the plugin-root
/// `skills/*/SKILL.md` layout and the `.claude/skills/*/SKILL.md` layout that
/// some plugins use.
fn skill_names_under(install_path: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for skills_dir in [
        install_path.join("skills"),
        install_path.join(".claude").join("skills"),
    ] {
        let Ok(entries) = std::fs::read_dir(&skills_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let skill_md = entry.path().join("SKILL.md");
            if let Some(name) = parse_skill_name(&skill_md) {
                names.push(name);
            }
        }
    }
    names
}

/// The `name` field from a SKILL.md YAML frontmatter block, if present.
fn parse_skill_name(skill_md: &Path) -> Option<String> {
    let text = std::fs::read_to_string(skill_md).ok()?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut lines = text.lines();
    // Frontmatter must open with a `---` fence on the first non-empty line.
    if lines.by_ref().find(|l| !l.trim().is_empty())?.trim() != "---" {
        return None;
    }
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break; // end of frontmatter without a name
        }
        if let Some(rest) = trimmed.strip_prefix("name:") {
            let name = rest.trim().trim_matches(['"', '\'']).trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "tests/plugin_overlap.rs"]
mod tests;
