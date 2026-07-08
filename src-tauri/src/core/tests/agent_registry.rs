use super::*;
use crate::core::tool_adapters::ToolConfig;

fn dir_entry(display_name: &str) -> AgentEntry {
    AgentEntry {
        display_name: display_name.to_string(),
        adapter_kind: AdapterKind::Dir,
        global_skills_dir: Some("~/.fake/skills".to_string()),
        project_skills_dir: Some(".fake/skills".to_string()),
        detect: vec!["~/.fake".to_string()],
        mcp_endpoint: None,
        default_strategy: "auto".to_string(),
        verified: false,
        source: Some("https://example.com/docs".to_string()),
        custom: false,
    }
}

fn mcp_entry(display_name: &str) -> AgentEntry {
    AgentEntry {
        display_name: display_name.to_string(),
        adapter_kind: AdapterKind::Mcp,
        global_skills_dir: None,
        project_skills_dir: None,
        detect: vec![],
        mcp_endpoint: Some("stdio:skills-mcp".to_string()),
        default_strategy: "auto".to_string(),
        verified: true,
        source: None,
        custom: true,
    }
}

fn write_registry(basin_dir: &std::path::Path, adapters: &[(&str, AgentEntry)]) {
    let mut registry = AgentRegistry {
        version: 1,
        adapters: BTreeMap::new(),
    };
    for (key, entry) in adapters {
        registry.adapters.insert(key.to_string(), entry.clone());
    }
    write_agent_registry(basin_dir, &registry).unwrap();
}

#[test]
fn seeds_bundled_default_when_agents_json_missing() {
    let basin = tempfile::tempdir().unwrap();
    assert!(!basin.path().join(AGENTS_FILE).exists());

    let seeded = ensure_agent_registry_seeded(basin.path()).unwrap();
    assert!(seeded);
    assert!(basin.path().join(AGENTS_FILE).exists());

    let read_back = read_agent_registry(basin.path()).unwrap();
    assert_eq!(read_back, default_agent_registry());

    // Second call is a no-op — file already exists.
    let seeded_again = ensure_agent_registry_seeded(basin.path()).unwrap();
    assert!(!seeded_again);
}

#[test]
fn read_without_file_falls_back_to_bundled_default() {
    let basin = tempfile::tempdir().unwrap();
    let registry = read_agent_registry(basin.path()).unwrap();
    assert_eq!(registry, default_agent_registry());
}

#[test]
fn tier1_builtin_always_wins_over_agents_json() {
    let basin = tempfile::tempdir().unwrap();
    write_registry(
        basin.path(),
        &[("claude_code", dir_entry("Fake Claude Code Override"))],
    );

    let resolved = resolve_all_adapters(basin.path(), &ToolConfig::default()).unwrap();
    let claude = resolved
        .iter()
        .find(|a| a.key == "claude_code")
        .expect("claude_code must resolve");

    assert_eq!(claude.display_name, "Claude Code");
    assert!(!claude.is_custom);
    assert_eq!(
        claude.project_skills_dir.as_deref(),
        Some(".claude/skills")
    );
}

#[test]
fn non_tier1_builtin_is_overridden_by_agents_json() {
    let basin = tempfile::tempdir().unwrap();
    write_registry(basin.path(), &[("codex", dir_entry("Codex (vendored override)"))]);

    let resolved = resolve_all_adapters(basin.path(), &ToolConfig::default()).unwrap();
    let codex = resolved
        .iter()
        .find(|a| a.key == "codex")
        .expect("codex must resolve");

    assert_eq!(codex.display_name, "Codex (vendored override)");
    assert_eq!(codex.project_skills_dir.as_deref(), Some(".fake/skills"));
}

#[test]
fn builtin_without_agents_json_entry_falls_back_to_rust_default() {
    let basin = tempfile::tempdir().unwrap();
    // No agents.json at all — every builtin resolves from the Rust default.
    let resolved = resolve_all_adapters(basin.path(), &ToolConfig::default()).unwrap();
    let cursor = resolved
        .iter()
        .find(|a| a.key == "cursor")
        .expect("cursor must resolve");
    assert_eq!(cursor.display_name, "Cursor");
    assert!(!cursor.is_custom);
}

#[test]
fn custom_mcp_entry_resolves_without_skills_dir() {
    let basin = tempfile::tempdir().unwrap();
    add_custom_agent(basin.path(), "hermes_via_mcp", mcp_entry("Hermes MCP")).unwrap();

    let resolved = resolve_all_adapters(basin.path(), &ToolConfig::default()).unwrap();
    let hermes = resolved
        .iter()
        .find(|a| a.key == "hermes_via_mcp")
        .expect("custom mcp entry must resolve");

    assert_eq!(hermes.adapter_kind, AdapterKind::Mcp);
    assert!(hermes.skills_dir.is_none());
    assert_eq!(hermes.mcp_endpoint.as_deref(), Some("stdio:skills-mcp"));
    assert!(hermes.is_custom);
}

#[test]
fn tier2_vendored_entries_are_never_dropped_by_seen_set() {
    let basin = tempfile::tempdir().unwrap();
    // "junie" already exists as a builtin (43-set); "zzz-brand-new" does not.
    write_registry(
        basin.path(),
        &[
            ("junie", dir_entry("Junie override")),
            ("zzz-brand-new", dir_entry("Brand New Tool")),
        ],
    );

    let resolved = resolve_all_adapters(basin.path(), &ToolConfig::default()).unwrap();
    assert!(resolved.iter().any(|a| a.key == "junie"
        && a.display_name == "Junie override"));
    assert!(resolved
        .iter()
        .any(|a| a.key == "zzz-brand-new" && a.display_name == "Brand New Tool"));
}

#[test]
fn add_custom_agent_rejects_tier1_key() {
    let basin = tempfile::tempdir().unwrap();
    let err = add_custom_agent(basin.path(), "cursor", dir_entry("Nope")).unwrap_err();
    assert!(err.to_string().contains("Tier1"));
}

#[test]
fn add_custom_agent_rejects_invalid_key_format() {
    let basin = tempfile::tempdir().unwrap();
    let err = add_custom_agent(basin.path(), "Not Valid Key!", dir_entry("Nope")).unwrap_err();
    assert!(err.to_string().contains("invalid characters"));
}

#[test]
fn add_custom_agent_rejects_duplicate_key() {
    let basin = tempfile::tempdir().unwrap();
    add_custom_agent(basin.path(), "my_tool", dir_entry("My Tool")).unwrap();
    let err = add_custom_agent(basin.path(), "my_tool", dir_entry("My Tool 2")).unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn update_custom_agent_round_trip() {
    let basin = tempfile::tempdir().unwrap();
    add_custom_agent(basin.path(), "my_tool", dir_entry("My Tool")).unwrap();
    update_custom_agent(basin.path(), "my_tool", dir_entry("My Tool Renamed")).unwrap();

    let registry = read_agent_registry(basin.path()).unwrap();
    assert_eq!(
        registry.adapters["my_tool"].display_name,
        "My Tool Renamed"
    );
}

#[test]
fn update_custom_agent_rejects_missing_key() {
    let basin = tempfile::tempdir().unwrap();
    let err = update_custom_agent(basin.path(), "ghost", dir_entry("X")).unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[test]
fn update_custom_agent_rejects_vendored_entry() {
    let basin = tempfile::tempdir().unwrap();
    write_registry(basin.path(), &[("codex", dir_entry("Codex vendored"))]);
    let err = update_custom_agent(basin.path(), "codex", dir_entry("Hijacked")).unwrap_err();
    assert!(err.to_string().contains("vendored"));
}

#[test]
fn remove_custom_agent_round_trip() {
    let basin = tempfile::tempdir().unwrap();
    add_custom_agent(basin.path(), "my_tool", dir_entry("My Tool")).unwrap();
    remove_custom_agent(basin.path(), "my_tool").unwrap();

    let registry = read_agent_registry(basin.path()).unwrap();
    assert!(!registry.adapters.contains_key("my_tool"));
}

#[test]
fn remove_custom_agent_rejects_vendored_entry() {
    let basin = tempfile::tempdir().unwrap();
    write_registry(basin.path(), &[("codex", dir_entry("Codex vendored"))]);
    let err = remove_custom_agent(basin.path(), "codex").unwrap_err();
    assert!(err.to_string().contains("vendored"));
}

#[test]
fn disabled_builtin_tools_still_resolve_but_marked_disabled() {
    let basin = tempfile::tempdir().unwrap();
    let mut config = ToolConfig::default();
    config.disabled_builtin_tools.push("cursor".to_string());

    let resolved = resolve_all_adapters(basin.path(), &config).unwrap();
    let cursor = resolved.iter().find(|a| a.key == "cursor").unwrap();
    assert!(!cursor.enabled);
}
