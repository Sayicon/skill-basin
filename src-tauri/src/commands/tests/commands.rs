use super::*;
use crate::core::skill_store::SkillRecord;

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

#[test]
fn format_anyhow_error_passthrough_prefixes() {
    let err = anyhow::anyhow!("MULTI_SKILLS|abc");
    assert_eq!(format_anyhow_error(err), "MULTI_SKILLS|abc");
}

#[test]
fn format_anyhow_error_redacts_clone_temp_path() {
    let err = anyhow::anyhow!("clone https://example.com/a/b into /tmp/skills-hub-git-123");
    let msg = format_anyhow_error(err);
    assert!(msg.contains("已省略临时目录"));
    assert!(!msg.contains("/tmp/skills-hub-git-123"));
}

#[test]
fn format_anyhow_error_github_hint_auth() {
    let err = anyhow::anyhow!("git clone https://github.com/a/b failed: authentication failed");
    let msg = format_anyhow_error(err);
    assert!(msg.contains("无法访问该仓库"));
}

#[test]
fn expand_home_path_basic() {
    let home = dirs::home_dir().expect("home");
    assert_eq!(expand_home_path("~").unwrap(), home);
    assert_eq!(expand_home_path("~/abc").unwrap(), home.join("abc"));
}

#[test]
fn expand_home_path_empty_is_error() {
    let err = expand_home_path("  ").unwrap_err().to_string();
    assert!(err.contains("storage path is empty"));
}

#[test]
fn saving_custom_tool_config_creates_enabled_skills_dir() {
    let (dir, store) = make_store();
    let existing = dir.path().join("existing-skills");
    std::fs::create_dir_all(&existing).unwrap();
    let created = dir.path().join("created-skills");
    assert!(!created.exists());

    save_tool_config(
        &store,
        ToolConfig {
            disabled_builtin_tools: Vec::new(),
            custom_tools: vec![
                CustomToolConfig {
                    key: "custom_existing".to_string(),
                    label: "Existing".to_string(),
                    skills_dir: existing.to_string_lossy().to_string(),
                    project_skills_dir: None,
                    enabled: true,
                },
                CustomToolConfig {
                    key: "custom_created".to_string(),
                    label: "Created".to_string(),
                    skills_dir: created.to_string_lossy().to_string(),
                    project_skills_dir: None,
                    enabled: true,
                },
            ],
        },
    )
    .unwrap();
    assert!(created.is_dir());

    let tools = runtime_tools(&store, true).unwrap();
    let existing_tool = tools
        .iter()
        .find(|tool| tool.key == "custom_existing")
        .unwrap();
    let created_tool = tools
        .iter()
        .find(|tool| tool.key == "custom_created")
        .unwrap();

    assert!(existing_tool.enabled);
    assert!(existing_tool.installed);
    assert!(created_tool.enabled);
    assert!(created_tool.installed);
}

#[test]
fn disabled_tool_with_existing_dir_stays_detected() {
    // Regression: `installed` conflated "dir exists" with "user enabled it",
    // so toggling a tool off made it drop into the "not detected" bucket and
    // vanish from the management UI.
    let (dir, store) = make_store();
    let skills_dir = dir.path().join("real-tool-skills");
    std::fs::create_dir_all(&skills_dir).unwrap();

    save_tool_config(
        &store,
        ToolConfig {
            disabled_builtin_tools: Vec::new(),
            custom_tools: vec![CustomToolConfig {
                key: "custom_real".to_string(),
                label: "Real Tool".to_string(),
                skills_dir: skills_dir.to_string_lossy().to_string(),
                project_skills_dir: None,
                enabled: false,
            }],
        },
    )
    .unwrap();

    let tools = runtime_tools(&store, true).unwrap();
    let tool = tools.iter().find(|t| t.key == "custom_real").unwrap();
    assert!(tool.detected, "dir exists -> stays detected");
    assert!(!tool.enabled);
    assert!(!tool.installed, "disabled tool is not a sync target");
}

fn make_basin(dir: &std::path::Path, store: &SkillStore) -> std::path::PathBuf {
    let basin_dir = dir.join("basin");
    crate::core::basin::basin_init(&basin_dir, "test", "2026-07-09").unwrap();
    store
        .set_setting("basin_path", &basin_dir.to_string_lossy())
        .unwrap();
    basin_dir
}

fn mcp_agent_entry() -> AgentEntry {
    AgentEntry {
        display_name: "My MCP Tool".to_string(),
        adapter_kind: AdapterKind::Mcp,
        global_skills_dir: None,
        project_skills_dir: None,
        detect: vec![],
        mcp_endpoint: Some("stdio:my-tool".to_string()),
        default_strategy: "auto".to_string(),
        verified: true,
        source: None,
        custom: true,
    }
}

#[test]
fn set_skill_pin_core_reports_conflict_instead_of_silent_success() {
    let (dir, store) = make_store();
    let basin_dir = make_basin(dir.path(), &store);

    let src = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("SKILL.md"), "v1").unwrap();
    crate::core::basin::add_skill_version(
        &basin_dir,
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-09",
        None,
    )
    .unwrap();

    // An UNMANAGED dir (no sidecar manifest) already occupies the target.
    let tool_dir = dir.path().join("tool-skills");
    std::fs::create_dir_all(tool_dir.join("demo")).unwrap();
    std::fs::write(tool_dir.join("demo").join("SKILL.md"), "user's own").unwrap();
    save_tool_config(
        &store,
        ToolConfig {
            disabled_builtin_tools: Vec::new(),
            custom_tools: vec![CustomToolConfig {
                key: "custom_qa".to_string(),
                label: "QA".to_string(),
                skills_dir: tool_dir.to_string_lossy().to_string(),
                project_skills_dir: None,
                enabled: true,
            }],
        },
    )
    .unwrap();

    let out =
        set_skill_pin_core(&store, "demo", "1.0.0", "custom_qa", PinTarget::default()).unwrap();

    // The pin is recorded, but the refused sync must surface in `results`
    // instead of being dropped (the UI showed "pinned" over a no-op).
    assert_eq!(out.pins.pins.len(), 1);
    let failed: Vec<_> = out.results.iter().filter(|r| !r.ok).collect();
    assert_eq!(
        failed.len(),
        1,
        "conflict must be reported: {:?}",
        out.results
    );
    assert!(failed[0]
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("unmanaged"));
    assert_eq!(
        std::fs::read_to_string(tool_dir.join("demo").join("SKILL.md")).unwrap(),
        "user's own",
        "unmanaged dir must stay untouched"
    );
}

#[test]
fn online_dto_from_github_repo_labels_origin_and_keeps_unknown_license() {
    let repo = RepoSummary {
        full_name: "vercel-labs/skills".to_string(),
        html_url: "https://github.com/vercel-labs/skills".to_string(),
        description: None,
        stars: 42,
        updated_at: "2026-07-09T00:00:00Z".to_string(),
        clone_url: "https://github.com/vercel-labs/skills.git".to_string(),
        license: None,
    };

    let dto = OnlineSkillDto::from(repo);
    assert_eq!(dto.origin, ORIGIN_GITHUB);
    assert_eq!(dto.name, "skills", "card shows the repo's last segment");
    assert_eq!(dto.source, "vercel-labs/skills");
    assert_eq!(dto.installs, 42, "stars stand in for the install count");
    assert_eq!(dto.license, None, "unknown license must not be invented");
}

#[test]
fn online_dto_from_skills_sh_carries_its_license_and_origin() {
    let dto = OnlineSkillDto::from(OnlineSkillResult {
        name: "react-expert".to_string(),
        installs: 100,
        source: "a/b".to_string(),
        source_url: "https://github.com/a/b".to_string(),
        license: Some("MIT".to_string()),
    });

    assert_eq!(dto.origin, ORIGIN_SKILLS_SH);
    assert_eq!(dto.license.as_deref(), Some("MIT"));
}

#[test]
fn skill_versions_put_the_latest_first_even_within_one_day() {
    // `added_at` is a date, not a timestamp, so two versions added the same
    // day tie — and the newest one rendered at the bottom of the list under
    // a "latest" chip. The latest version leads regardless.
    let mut versions = vec![
        SkillVersionDto {
            label: "2026-07-09.65c9".to_string(),
            added_at: "2026-07-09".to_string(),
            is_latest: false,
        },
        SkillVersionDto {
            label: "2026-07-09.8a4a".to_string(),
            added_at: "2026-07-09".to_string(),
            is_latest: true,
        },
        SkillVersionDto {
            label: "2026-07-01.aaaa".to_string(),
            added_at: "2026-07-01".to_string(),
            is_latest: false,
        },
    ];

    sort_skill_versions(&mut versions);

    assert_eq!(versions[0].label, "2026-07-09.8a4a", "latest leads");
    assert_eq!(versions[1].label, "2026-07-09.65c9");
    assert_eq!(versions[2].label, "2026-07-01.aaaa", "older days follow");
}

#[test]
fn skill_versions_are_ordered_newest_day_first() {
    let mut versions = vec![
        SkillVersionDto {
            label: "old".to_string(),
            added_at: "2026-07-01".to_string(),
            is_latest: false,
        },
        SkillVersionDto {
            label: "new".to_string(),
            added_at: "2026-07-09".to_string(),
            is_latest: false,
        },
    ];

    sort_skill_versions(&mut versions);
    assert_eq!(versions[0].label, "new");
}

#[test]
fn basin_latest_versions_core_maps_every_skill_to_its_latest_label() {
    let (dir, store) = make_store();
    let basin_dir = make_basin(dir.path(), &store);
    let src = tempfile::tempdir().unwrap();

    std::fs::write(src.path().join("SKILL.md"), "demo v1").unwrap();
    crate::core::basin::add_skill_version(
        &basin_dir,
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-09",
        None,
    )
    .unwrap();
    std::fs::write(src.path().join("SKILL.md"), "demo v2").unwrap();
    crate::core::basin::add_skill_version(
        &basin_dir,
        "demo",
        src.path(),
        "2.0.0",
        "2026-07-10",
        None,
    )
    .unwrap();
    std::fs::write(src.path().join("SKILL.md"), "solo").unwrap();
    crate::core::basin::add_skill_version(
        &basin_dir,
        "solo",
        src.path(),
        "1.0.0",
        "2026-07-09",
        None,
    )
    .unwrap();

    let map = basin_latest_versions_core(&store).unwrap();
    assert_eq!(map.get("demo").map(String::as_str), Some("2.0.0"));
    assert_eq!(map.get("solo").map(String::as_str), Some("1.0.0"));
}

#[test]
fn basin_latest_versions_core_is_empty_without_a_basin() {
    // Pre-onboarding the UI still renders; an unconfigured basin means "no
    // version info", not an error the cards have to handle.
    let (_dir, store) = make_store();
    assert!(basin_latest_versions_core(&store).unwrap().is_empty());
}

#[test]
fn require_basin_dir_errors_when_unconfigured() {
    let (_dir, store) = make_store();
    let err = require_basin_dir(&store).unwrap_err().to_string();
    assert!(err.contains("no basin configured"));
}

#[test]
fn custom_agent_crud_commits_to_basin_and_surfaces_in_runtime_tools() {
    let (dir, store) = make_store();
    let basin_dir = make_basin(dir.path(), &store);

    let registry = add_custom_agent(&basin_dir, "my_mcp_tool", mcp_agent_entry()).unwrap();
    crate::core::basin::basin_commit_all(&basin_dir, "add custom agent: my_mcp_tool").unwrap();
    assert!(registry.adapters["my_mcp_tool"].custom);

    let tools = runtime_tools(&store, true).unwrap();
    let tool = tools
        .iter()
        .find(|tool| tool.key == "my_mcp_tool")
        .expect("custom mcp agent must surface in runtime_tools");
    assert!(tool.is_custom);
    assert_eq!(tool.adapter_kind, "mcp");
    assert_eq!(tool.mcp_endpoint.as_deref(), Some("stdio:my-tool"));

    let mut renamed = mcp_agent_entry();
    renamed.display_name = "Renamed".to_string();
    update_custom_agent(&basin_dir, "my_mcp_tool", renamed).unwrap();
    let after_update = read_agent_registry(&basin_dir).unwrap();
    assert_eq!(after_update.adapters["my_mcp_tool"].display_name, "Renamed");

    remove_custom_agent(&basin_dir, "my_mcp_tool").unwrap();
    let after_remove = read_agent_registry(&basin_dir).unwrap();
    assert!(!after_remove.adapters.contains_key("my_mcp_tool"));
}

#[test]
fn agents_json_custom_entry_takes_precedence_over_legacy_sqlite_key_collision() {
    let (dir, store) = make_store();
    let basin_dir = make_basin(dir.path(), &store);
    add_custom_agent(&basin_dir, "shared_key", mcp_agent_entry()).unwrap();

    save_tool_config(
        &store,
        ToolConfig {
            disabled_builtin_tools: Vec::new(),
            custom_tools: vec![CustomToolConfig {
                key: "shared_key".to_string(),
                label: "Legacy SQLite Entry".to_string(),
                skills_dir: dir.path().to_string_lossy().to_string(),
                project_skills_dir: None,
                enabled: true,
            }],
        },
    )
    .unwrap();

    let tools = runtime_tools(&store, true).unwrap();
    let matches: Vec<_> = tools.iter().filter(|t| t.key == "shared_key").collect();
    assert_eq!(matches.len(), 1, "duplicate key must not appear twice");
    assert_eq!(matches[0].adapter_kind, "mcp"); // agents.json entry won
}

#[test]
fn normalize_scope_defaults_to_global_and_rejects_unknown() {
    assert_eq!(normalize_scope(None).unwrap(), "global");
    assert_eq!(normalize_scope(Some("global")).unwrap(), "global");
    assert_eq!(normalize_scope(Some("project")).unwrap(), "project");
    assert!(normalize_scope(Some("workspace")).is_err());
}

#[test]
fn recent_projects_are_deduped_ordered_and_limited() {
    let (_dir, store) = make_store();
    let project_root = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    for i in 0..9 {
        let path = project_root.path().join(format!("project-{i}"));
        std::fs::create_dir_all(&path).unwrap();
        paths.push(path);
    }

    for path in &paths {
        save_recent_project_impl(&store, path.to_string_lossy().as_ref()).unwrap();
    }

    let recent = get_recent_projects_impl(&store).unwrap();
    assert_eq!(recent.len(), 8);
    assert_eq!(recent[0], paths[8].to_string_lossy());
    assert_eq!(recent[7], paths[1].to_string_lossy());
    assert!(!recent.contains(&paths[0].to_string_lossy().to_string()));

    save_recent_project_impl(&store, paths[3].to_string_lossy().as_ref()).unwrap();
    let recent = get_recent_projects_impl(&store).unwrap();
    assert_eq!(recent.len(), 8);
    assert_eq!(recent[0], paths[3].to_string_lossy());
    assert_eq!(
        recent
            .iter()
            .filter(|item| *item == &paths[3].to_string_lossy())
            .count(),
        1
    );
}

#[test]
fn save_recent_project_rejects_missing_directory() {
    let (_dir, store) = make_store();
    let missing = tempfile::tempdir().unwrap().path().join("missing-project");
    let err = save_recent_project_impl(&store, missing.to_string_lossy().as_ref())
        .unwrap_err()
        .to_string();
    assert!(err.contains("projectPath must be an existing directory"));
}

#[test]
fn remove_path_any_handles_file_dir_and_missing() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.txt");
    std::fs::write(&file, b"1").unwrap();
    remove_path_any(file.to_string_lossy().as_ref()).unwrap();
    assert!(!file.exists());

    let sub = dir.path().join("d");
    std::fs::create_dir_all(&sub).unwrap();
    remove_path_any(sub.to_string_lossy().as_ref()).unwrap();
    assert!(!sub.exists());

    remove_path_any(dir.path().join("missing").to_string_lossy().as_ref()).unwrap();
}

#[test]
#[cfg(unix)]
fn remove_path_any_removes_symlink_only() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real");
    std::fs::create_dir_all(&target).unwrap();
    let link = dir.path().join("link");
    symlink(&target, &link).unwrap();

    remove_path_any(link.to_string_lossy().as_ref()).unwrap();
    assert!(!link.exists());
    assert!(target.exists());
}

#[test]
fn remove_path_any_removes_dir_link_without_touching_source() {
    // Regression: dir junctions/symlinks died with os error 5 on Windows
    // because the old helper only ever tried remove_file on links.
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), "content").unwrap();
    let link = dir.path().join("link");
    crate::core::sync_engine::sync_dir_hybrid(&source, &link).unwrap();
    assert!(link.join("SKILL.md").exists());

    remove_path_any(link.to_string_lossy().as_ref()).unwrap();

    assert!(!link.exists(), "link itself must be gone");
    assert!(source.join("SKILL.md").exists(), "source must be untouched");
}

fn seed_deletable_skill(
    store: &SkillStore,
    dir: &std::path::Path,
    tools: &[&str],
) -> (String, std::path::PathBuf, Vec<std::path::PathBuf>) {
    use crate::core::skill_store::SkillTargetRecord;

    let central = dir.join("central").join("demo");
    std::fs::create_dir_all(&central).unwrap();
    std::fs::write(central.join("SKILL.md"), "demo").unwrap();

    let skill_id = "skill-under-delete".to_string();
    store
        .upsert_skill(&SkillRecord {
            id: skill_id.clone(),
            name: "demo".to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: central.to_string_lossy().to_string(),
            content_hash: None,
            created_at: 0,
            updated_at: 0,
            last_sync_at: None,
            last_seen_at: 0,
            enabled: true,
            status: "active".to_string(),
        })
        .unwrap();

    let mut links = Vec::new();
    for (i, tool) in tools.iter().enumerate() {
        let link = dir.join("tools").join(tool).join("demo");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        crate::core::sync_engine::sync_dir_hybrid(&central, &link).unwrap();
        store
            .upsert_skill_target(&SkillTargetRecord {
                id: format!("target-{i}"),
                skill_id: skill_id.clone(),
                tool: tool.to_string(),
                scope: "global".to_string(),
                project_path: None,
                target_path: link.to_string_lossy().to_string(),
                mode: "junction".to_string(),
                status: "ok".to_string(),
                last_error: None,
                synced_at: None,
            })
            .unwrap();
        links.push(link);
    }
    (skill_id, central, links)
}

#[test]
fn delete_managed_skill_core_removes_links_central_and_record() {
    let (dir, store) = make_store();
    let (skill_id, central, links) =
        seed_deletable_skill(&store, dir.path(), &["claude_code", "cursor"]);

    delete_managed_skill_core(&store, &skill_id).unwrap();

    for link in &links {
        assert!(!link.exists(), "synced link must be removed: {link:?}");
    }
    assert!(!central.exists(), "central copy must be removed");
    assert!(store.get_skill_by_id(&skill_id).unwrap().is_none());
}

#[test]
fn delete_managed_skill_core_also_unpins_and_removes_pinned_installs() {
    // Pins live in the basin, not in the SQLite target table, so a skill
    // installed by pinning left an orphaned link in every tool directory when
    // it was deleted: the record vanished, the junction stayed.
    let (dir, store) = make_store();
    let basin_dir = make_basin(dir.path(), &store);

    let src = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("SKILL.md"), "v1").unwrap();
    crate::core::basin::add_skill_version(
        &basin_dir,
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-09",
        None,
    )
    .unwrap();

    let tool_dir = dir.path().join("tool-skills");
    std::fs::create_dir_all(&tool_dir).unwrap();
    save_tool_config(
        &store,
        ToolConfig {
            disabled_builtin_tools: Vec::new(),
            custom_tools: vec![CustomToolConfig {
                key: "custom_qa".to_string(),
                label: "QA".to_string(),
                skills_dir: tool_dir.to_string_lossy().to_string(),
                project_skills_dir: None,
                enabled: true,
            }],
        },
    )
    .unwrap();

    let central = dir.path().join("central").join("demo");
    std::fs::create_dir_all(&central).unwrap();
    std::fs::write(central.join("SKILL.md"), "v1").unwrap();
    store
        .upsert_skill(&SkillRecord {
            id: "pinned-skill".to_string(),
            name: "demo".to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: central.to_string_lossy().to_string(),
            content_hash: None,
            created_at: 0,
            updated_at: 0,
            last_sync_at: None,
            last_seen_at: 0,
            enabled: true,
            status: "active".to_string(),
        })
        .unwrap();

    set_skill_pin_core(&store, "demo", "1.0.0", "custom_qa", PinTarget::default()).unwrap();
    let installed = tool_dir.join("demo");
    assert!(installed.exists(), "pin should have installed the skill");

    delete_managed_skill_core(&store, "pinned-skill").unwrap();

    assert!(!installed.exists(), "pinned install must be removed too");
    let pins =
        read_machine_pins_or_empty(&basin_dir, &current_machine_id(&store).unwrap()).unwrap();
    assert!(pins.pins.is_empty(), "pin must be dropped: {:?}", pins.pins);
    // The basin keeps its history; only the tool-side install disappears.
    assert!(crate::core::basin::version_dir(&basin_dir, "demo", "1.0.0").exists());
}

#[test]
fn delete_managed_skill_core_handles_tools_sharing_one_target_path() {
    use crate::core::skill_store::SkillTargetRecord;

    let (dir, store) = make_store();
    let (skill_id, central, links) = seed_deletable_skill(&store, dir.path(), &["cline"]);
    // Second tool points at the SAME directory (shared skills dir, like
    // cline/loaf/warp all using ~/.agents/skills).
    store
        .upsert_skill_target(&SkillTargetRecord {
            id: "target-shared".to_string(),
            skill_id: skill_id.clone(),
            tool: "loaf".to_string(),
            scope: "global".to_string(),
            project_path: None,
            target_path: links[0].to_string_lossy().to_string(),
            mode: "junction".to_string(),
            status: "ok".to_string(),
            last_error: None,
            synced_at: None,
        })
        .unwrap();

    delete_managed_skill_core(&store, &skill_id).unwrap();

    assert!(!links[0].exists());
    assert!(!central.exists());
    assert!(store.get_skill_by_id(&skill_id).unwrap().is_none());
}

#[test]
fn get_managed_skills_impl_maps_targets() {
    let (_dir, store) = make_store();
    let skill = SkillRecord {
        id: "s1".to_string(),
        name: "S1".to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: Some("/tmp/src".to_string()),
        source_subpath: None,
        source_revision: None,
        central_path: "/tmp/central".to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 2,
        last_sync_at: None,
        last_seen_at: 1,
        enabled: true,
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill).unwrap();

    let target = SkillTargetRecord {
        id: "t1".to_string(),
        skill_id: "s1".to_string(),
        tool: "cursor".to_string(),
        scope: "global".to_string(),
        project_path: None,
        target_path: "/tmp/target".to_string(),
        mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
    };
    store.upsert_skill_target(&target).unwrap();
    let tag = store.create_tag("Frontend").unwrap();
    store.set_skill_tags("s1", &[tag.id]).unwrap();

    let out = get_managed_skills_impl(&store).unwrap();
    assert_eq!(out.len(), 1);
    assert!(out[0].enabled);
    assert_eq!(out[0].tags.len(), 1);
    assert_eq!(out[0].tags[0].name, "Frontend");
    assert_eq!(out[0].targets.len(), 1);
    assert_eq!(out[0].targets[0].tool, "cursor");
    assert_eq!(out[0].targets[0].scope, "global");
    assert!(out[0].targets[0].project_path.is_none());
}

#[test]
fn fleet_machines_lists_roster_with_status_and_pin_counts() {
    let (dir, store) = make_store();
    let basin_dir = make_basin(dir.path(), &store);

    // Remote machine m1: two pins on two tools + a failing status report.
    let m1 = basin_dir.join("machines/m1");
    std::fs::create_dir_all(&m1).unwrap();
    std::fs::write(
        m1.join("pins.json"),
        r#"{"machine":"m1","pins":[
            {"skill":"a","version":"1.0.0","targets":{"t0":{},"t1":{}}},
            {"skill":"b","version":"1.0.0","targets":{"t0":{}}}]}"#,
    )
    .unwrap();
    std::fs::write(
        m1.join("status.json"),
        serde_json::to_string(&crate::core::fleet::StatusReport {
            schema_version: 1,
            machine: "m1".to_string(),
            applied_at_epoch: 1_760_000_000,
            ok: false,
            actions: vec![crate::core::fleet::StatusAction {
                skill: "a".to_string(),
                tool: "t0".to_string(),
                action: "install".to_string(),
                ok: false,
                error: Some("permission denied".to_string()),
            }],
        })
        .unwrap(),
    )
    .unwrap();

    // Machine with a CORRUPT status: must surface as an error string, not
    // vanish and not sink the whole roster.
    let m2 = basin_dir.join("machines/m2");
    std::fs::create_dir_all(&m2).unwrap();
    std::fs::write(m2.join("status.json"), "{bozuk json").unwrap();

    let out = fleet_machines_impl(&store).unwrap();
    let current_id = current_machine_id(&store).unwrap();

    // Roster: m1, m2, and this machine (even with no machines/ entry yet).
    let names: Vec<_> = out.iter().map(|m| m.machine.as_str()).collect();
    assert!(names.contains(&"m1"), "{names:?}");
    assert!(names.contains(&"m2"), "{names:?}");
    assert!(names.contains(&current_id.as_str()), "{names:?}");

    let m1 = out.iter().find(|m| m.machine == "m1").unwrap();
    assert!(!m1.is_current);
    assert_eq!(m1.pinned_by_tool.get("t0"), Some(&2));
    assert_eq!(m1.pinned_by_tool.get("t1"), Some(&1));
    let status = m1.status.as_ref().unwrap();
    assert!(!status.ok);
    assert_eq!(status.actions[0].error.as_deref(), Some("permission denied"));

    let m2 = out.iter().find(|m| m.machine == "m2").unwrap();
    assert!(m2.status.is_none());
    assert!(m2.status_error.as_deref().unwrap_or("").contains("parse"));

    let me = out.iter().find(|m| m.machine == current_id).unwrap();
    assert!(me.is_current);
    assert!(me.status.is_none());
    assert!(me.status_error.is_none());
}

fn custom_tool(key: &str, skills_dir: &std::path::Path, enabled: bool) -> CustomToolConfig {
    CustomToolConfig {
        key: key.to_string(),
        label: key.to_string(),
        skills_dir: skills_dir.to_string_lossy().to_string(),
        project_skills_dir: None,
        enabled,
    }
}

#[test]
fn resolved_tool_dirs_excludes_disabled_tools() {
    // A disabled tool must not be a pin-sync target: leaving it in the map
    // keeps the planner installing/updating into a directory the user turned off.
    let dir = tempfile::tempdir().unwrap();
    let config = ToolConfig {
        disabled_builtin_tools: vec!["claude_code".to_string()],
        custom_tools: vec![
            custom_tool("custom_on", &dir.path().join("on-skills"), true),
            custom_tool("custom_off", &dir.path().join("off-skills"), false),
        ],
    };

    let dirs = resolved_tool_dirs(dir.path(), &config).unwrap();
    assert!(
        !dirs.contains_key("claude_code"),
        "disabled builtin must drop out of pin targets"
    );
    assert!(dirs.contains_key("cursor"), "enabled builtin stays");
    assert!(dirs.contains_key("custom_on"), "enabled custom tool stays");
    assert!(
        !dirs.contains_key("custom_off"),
        "disabled custom tool must drop out of pin targets"
    );
}

fn basin_with_demo_v1(dir: &std::path::Path, store: &SkillStore) -> std::path::PathBuf {
    let basin_dir = make_basin(dir, store);
    let src = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("SKILL.md"), "v1").unwrap();
    crate::core::basin::add_skill_version(
        &basin_dir,
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-10",
        None,
    )
    .unwrap();
    basin_dir
}

#[test]
fn pinning_to_disabled_tool_is_an_error() {
    let (dir, store) = make_store();
    let _basin_dir = basin_with_demo_v1(dir.path(), &store);

    let tool_dir = dir.path().join("off-tool-skills");
    save_tool_config(
        &store,
        ToolConfig {
            disabled_builtin_tools: Vec::new(),
            custom_tools: vec![custom_tool("custom_off", &tool_dir, false)],
        },
    )
    .unwrap();

    let err = set_skill_pin_core(&store, "demo", "1.0.0", "custom_off", PinTarget::default())
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("TOOL_DIR_UNKNOWN"),
        "pin to a disabled tool must be refused, got: {err}"
    );
}

#[test]
fn unpinning_while_tool_disabled_leaves_its_files_alone() {
    let (dir, store) = make_store();
    let _basin_dir = basin_with_demo_v1(dir.path(), &store);

    let tool_dir = dir.path().join("tool-skills");
    let config = |enabled: bool| ToolConfig {
        disabled_builtin_tools: Vec::new(),
        custom_tools: vec![custom_tool("custom_qa", &tool_dir, enabled)],
    };
    save_tool_config(&store, config(true)).unwrap();
    set_skill_pin_core(&store, "demo", "1.0.0", "custom_qa", PinTarget::default()).unwrap();
    let target = tool_dir.join("demo");
    assert!(
        target.exists(),
        "pin must install while the tool is enabled"
    );

    // User turns the tool off: SkillBasin stops managing that directory. The
    // pin record still goes away, but files on disk are hands-off (a later
    // re-enable lets the planner remove the now-unpinned install).
    save_tool_config(&store, config(false)).unwrap();
    let out = unset_skill_pin_core(&store, "demo", "custom_qa").unwrap();
    assert!(out.pins.pins.is_empty(), "pin record must go away");
    assert!(target.exists(), "disabled tool's files are hands-off");
    assert!(
        crate::core::pins::manifest_path_for(&target).exists(),
        "manifest stays with the files it describes"
    );
}

#[test]
fn deleting_a_skill_cleans_pin_manifest_when_legacy_target_covers_same_dir() {
    let (dir, store) = make_store();
    let basin_dir = basin_with_demo_v1(dir.path(), &store);

    let tool_dir = dir.path().join("tool-skills");
    save_tool_config(
        &store,
        ToolConfig {
            disabled_builtin_tools: Vec::new(),
            custom_tools: vec![custom_tool("custom_qa", &tool_dir, true)],
        },
    )
    .unwrap();
    set_skill_pin_core(&store, "demo", "1.0.0", "custom_qa", PinTarget::default()).unwrap();
    let target = tool_dir.join("demo");
    let manifest = crate::core::pins::manifest_path_for(&target);
    assert!(target.exists() && manifest.exists());

    // Drifted state: a legacy SQLite target row also claims the pinned dir.
    store
        .upsert_skill(&SkillRecord {
            id: "s1".to_string(),
            name: "demo".to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: dir
                .path()
                .join("central-demo")
                .to_string_lossy()
                .to_string(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            enabled: true,
            status: "ok".to_string(),
        })
        .unwrap();
    store
        .upsert_skill_target(&SkillTargetRecord {
            id: "t1".to_string(),
            skill_id: "s1".to_string(),
            tool: "custom_qa".to_string(),
            scope: "global".to_string(),
            project_path: None,
            target_path: target.to_string_lossy().to_string(),
            mode: "copy".to_string(),
            status: "ok".to_string(),
            last_error: None,
            synced_at: None,
        })
        .unwrap();

    delete_managed_skill_core(&store, "s1").unwrap();

    assert!(!target.exists(), "target dir must be removed");
    assert!(
        !manifest.exists(),
        "sidecar manifest must not be orphaned after delete"
    );
    let pins =
        read_machine_pins_or_empty(&basin_dir, &current_machine_id(&store).unwrap()).unwrap();
    assert!(pins.pins.is_empty(), "pin record must be gone");
}
