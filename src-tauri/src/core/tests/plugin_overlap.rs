use super::*;
use std::fs;
use std::path::Path;

fn write(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn skill_md(name: &str) -> String {
    format!("---\nname: {name}\ndescription: does {name} things\n---\n\n# {name}\n")
}

/// A plugin listed in enabledPlugins, installed, with a skill named `nextjs`,
/// is reported as providing `nextjs` and attributed to that plugin key.
#[test]
fn reports_enabled_plugin_skill_with_attribution() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let install = h.join("plugins/cache/vercel/vercel-plugin/abc");

    write(
        &h.join("settings.json"),
        r#"{"enabledPlugins":{"vercel-plugin@vercel":true,"off-one@mp":false}}"#,
    );
    write(
        &h.join("plugins/installed_plugins.json"),
        &format!(
            r#"{{"version":2,"plugins":{{"vercel-plugin@vercel":[{{"installPath":{:?}}}]}}}}"#,
            install.to_string_lossy()
        ),
    );
    write(&install.join("skills/nextjs/SKILL.md"), &skill_md("nextjs"));
    // The `.claude/skills` layout is also honored.
    write(
        &install.join(".claude/skills/ai-sdk/SKILL.md"),
        &skill_md("ai-sdk"),
    );

    let got = enabled_plugin_skills(h);
    assert_eq!(
        got.get("nextjs").map(String::as_str),
        Some("vercel-plugin@vercel")
    );
    assert_eq!(
        got.get("ai-sdk").map(String::as_str),
        Some("vercel-plugin@vercel")
    );
    assert_eq!(got.len(), 2);
}

/// A plugin present on disk but NOT enabled contributes nothing.
#[test]
fn ignores_disabled_plugin() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let install = h.join("plugins/cache/x/p/1");
    write(
        &h.join("settings.json"),
        r#"{"enabledPlugins":{"p@x":false}}"#,
    );
    write(
        &h.join("plugins/installed_plugins.json"),
        &format!(
            r#"{{"plugins":{{"p@x":[{{"installPath":{:?}}}]}}}}"#,
            install.to_string_lossy()
        ),
    );
    write(&install.join("skills/nextjs/SKILL.md"), &skill_md("nextjs"));

    assert!(enabled_plugin_skills(h).is_empty());
}

/// Missing config is not an error: an empty map, never a panic.
#[test]
fn missing_config_yields_empty() {
    let home = tempfile::tempdir().unwrap();
    assert!(enabled_plugin_skills(home.path()).is_empty());
}

/// An enabled plugin with no install record is skipped without error.
#[test]
fn enabled_but_not_installed_is_skipped() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    write(
        &h.join("settings.json"),
        r#"{"enabledPlugins":{"ghost@mp":true}}"#,
    );
    write(
        &h.join("plugins/installed_plugins.json"),
        r#"{"plugins":{}}"#,
    );
    assert!(enabled_plugin_skills(h).is_empty());
}
