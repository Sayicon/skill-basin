use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::*;
use crate::core::basin;

/// Basin with two versions of "demo" and one version of "solo".
fn fixture_basin() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    basin::basin_init(dir.path(), "b", "2026-07-08").unwrap();

    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("SKILL.md"), "demo v1").unwrap();
    basin::add_skill_version(dir.path(), "demo", src.path(), "1.0.0", "2026-07-08", None).unwrap();
    fs::write(src.path().join("SKILL.md"), "demo v2").unwrap();
    basin::add_skill_version(dir.path(), "demo", src.path(), "2.0.0", "2026-07-08", None).unwrap();
    fs::write(src.path().join("SKILL.md"), "solo").unwrap();
    basin::add_skill_version(dir.path(), "solo", src.path(), "1.0.0", "2026-07-08", None).unwrap();
    dir
}

fn pin(skill: &str, version: &str, tools: &[&str]) -> PinEntry {
    PinEntry {
        skill: skill.to_string(),
        version: version.to_string(),
        targets: tools
            .iter()
            .map(|t| (t.to_string(), PinTarget::default()))
            .collect(),
    }
}

fn tool_dirs(tools: &[(&str, &tempfile::TempDir)]) -> BTreeMap<String, PathBuf> {
    tools
        .iter()
        .map(|(k, d)| (k.to_string(), d.path().to_path_buf()))
        .collect()
}

#[test]
fn pins_round_trip_with_defaults() {
    let basin_dir = tempfile::tempdir().unwrap();
    let pins = MachinePins {
        machine: "kerem-pc".to_string(),
        pins: vec![pin("demo", "1.0.0", &["claude_code"])],
    };
    write_machine_pins(basin_dir.path(), &pins).unwrap();
    let back = read_machine_pins(basin_dir.path(), "kerem-pc").unwrap();
    assert_eq!(back, pins);

    // Minimal JSON: enabled/strategy default to true/"auto".
    let raw = r#"{"machine":"m","pins":[{"skill":"x","version":"1","targets":{"cursor":{}}}]}"#;
    let parsed: MachinePins = serde_json::from_str(raw).unwrap();
    let target = &parsed.pins[0].targets["cursor"];
    assert!(target.enabled);
    assert_eq!(target.strategy, "auto");
}

#[test]
fn planner_installs_missing_pins() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let pins = MachinePins {
        machine: "m".to_string(),
        pins: vec![pin("demo", "1.0.0", &["claude_code"])],
    };

    let plan = plan_sync(&pins, &tool_dirs(&[("claude_code", &claude)])).unwrap();
    assert_eq!(plan.len(), 1);
    match &plan[0] {
        PlanAction::Install {
            skill,
            version,
            tool,
            target,
            strategy,
        } => {
            assert_eq!(skill, "demo");
            assert_eq!(version, "1.0.0");
            assert_eq!(tool, "claude_code");
            assert_eq!(target, &claude.path().join("demo"));
            assert_eq!(strategy, "auto");
        }
        other => panic!("expected Install, got {:?}", other),
    }
    drop(basin);
}

#[test]
fn same_skill_two_versions_to_two_tools_at_once() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let hermes = tempfile::tempdir().unwrap();
    let pins = MachinePins {
        machine: "m".to_string(),
        pins: vec![
            pin("demo", "2.0.0", &["claude_code"]),
            pin("demo", "1.0.0", &["hermes"]),
        ],
    };

    let dirs = tool_dirs(&[("claude_code", &claude), ("hermes", &hermes)]);
    let plan = plan_sync(&pins, &dirs).unwrap();
    assert_eq!(plan.len(), 2);

    let results = apply_plan(basin.path(), &plan).unwrap();
    assert!(results.iter().all(|r| r.ok), "{:?}", results);

    // Kerem's core requirement: different versions live simultaneously.
    assert_eq!(
        fs::read_to_string(claude.path().join("demo/SKILL.md")).unwrap(),
        "demo v2"
    );
    assert_eq!(
        fs::read_to_string(hermes.path().join("demo/SKILL.md")).unwrap(),
        "demo v1"
    );

    // Sibling manifests written; NOTHING written inside the version dirs.
    let m = read_managed_manifest(&claude.path().join("demo")).unwrap();
    assert_eq!(m.version, "2.0.0");
    assert!(!basin::version_dir(basin.path(), "demo", "2.0.0")
        .join("demo.skillbasin.json")
        .exists());
    assert!(!basin::version_dir(basin.path(), "demo", "2.0.0")
        .join(".skillbasin.json")
        .exists());
}

#[test]
fn planner_updates_when_pin_moves_and_apply_swaps_content() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    let v1 = MachinePins {
        machine: "m".to_string(),
        pins: vec![pin("demo", "1.0.0", &["claude_code"])],
    };
    apply_plan(basin.path(), &plan_sync(&v1, &dirs).unwrap()).unwrap();

    // Pin moves 1.0.0 -> 2.0.0.
    let v2 = MachinePins {
        machine: "m".to_string(),
        pins: vec![pin("demo", "2.0.0", &["claude_code"])],
    };
    let plan = plan_sync(&v2, &dirs).unwrap();
    assert_eq!(plan.len(), 1);
    assert!(matches!(
        &plan[0],
        PlanAction::Update { from_version, to_version, .. }
            if from_version == "1.0.0" && to_version == "2.0.0"
    ));

    apply_plan(basin.path(), &plan).unwrap();
    assert_eq!(
        fs::read_to_string(claude.path().join("demo/SKILL.md")).unwrap(),
        "demo v2"
    );
    assert_eq!(
        read_managed_manifest(&claude.path().join("demo"))
            .unwrap()
            .version,
        "2.0.0"
    );

    // Same pins again -> empty plan (converged).
    assert!(plan_sync(&v2, &dirs).unwrap().is_empty());
}

#[test]
fn planner_removes_only_managed_dirs() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    // demo installed and managed; handmade dir is NOT ours.
    let installed = MachinePins {
        machine: "m".to_string(),
        pins: vec![pin("demo", "1.0.0", &["claude_code"])],
    };
    apply_plan(basin.path(), &plan_sync(&installed, &dirs).unwrap()).unwrap();
    fs::create_dir_all(claude.path().join("handmade")).unwrap();
    fs::write(claude.path().join("handmade/SKILL.md"), "user's own").unwrap();

    // All pins gone -> managed demo removed, handmade untouched.
    let empty = MachinePins {
        machine: "m".to_string(),
        pins: vec![],
    };
    let plan = plan_sync(&empty, &dirs).unwrap();
    assert_eq!(plan.len(), 1);
    assert!(matches!(&plan[0], PlanAction::Remove { skill, .. } if skill == "demo"));

    let results = apply_plan(basin.path(), &plan).unwrap();
    assert!(results.iter().all(|r| r.ok), "{:?}", results);
    assert!(!claude.path().join("demo").exists());
    assert!(!manifest_path_for(&claude.path().join("demo")).exists());
    assert!(claude.path().join("handmade/SKILL.md").exists());

    // Removing a junction/symlink must never delete the basin's version dir.
    assert!(basin::version_dir(basin.path(), "demo", "1.0.0")
        .join("SKILL.md")
        .exists());
}

#[test]
fn planner_ignores_loose_files_and_manifests_in_a_tool_dir() {
    // The manifest sidecars live next to their targets, so scanning a tool
    // directory sees them as entries too; only something with its own
    // manifest counts as managed.
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    let pins = MachinePins {
        machine: "m".to_string(),
        pins: vec![pin("demo", "1.0.0", &["claude_code"])],
    };
    apply_plan(basin.path(), &plan_sync(&pins, &dirs).unwrap()).unwrap();
    assert!(manifest_path_for(&claude.path().join("demo")).exists());

    fs::write(claude.path().join("README.md"), "hand-written note").unwrap();

    // Converged state: the manifest file and the loose file produce no work.
    assert!(plan_sync(&pins, &dirs).unwrap().is_empty());
}

#[test]
fn planner_flags_unmanaged_dir_at_target_as_conflict() {
    let _basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    // User already has an unmanaged dir named "demo".
    fs::create_dir_all(claude.path().join("demo")).unwrap();
    fs::write(claude.path().join("demo/SKILL.md"), "user's own demo").unwrap();

    let pins = MachinePins {
        machine: "m".to_string(),
        pins: vec![pin("demo", "1.0.0", &["claude_code"])],
    };
    let plan = plan_sync(&pins, &tool_dirs(&[("claude_code", &claude)])).unwrap();
    assert_eq!(plan.len(), 1);
    assert!(matches!(&plan[0], PlanAction::Conflict { skill, .. } if skill == "demo"));
}

#[test]
fn disabled_target_counts_as_unpinned() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    let enabled = MachinePins {
        machine: "m".to_string(),
        pins: vec![pin("demo", "1.0.0", &["claude_code"])],
    };
    apply_plan(basin.path(), &plan_sync(&enabled, &dirs).unwrap()).unwrap();

    // Same pin but target disabled -> plan removes the managed install.
    let mut disabled = enabled.clone();
    disabled.pins[0]
        .targets
        .get_mut("claude_code")
        .unwrap()
        .enabled = false;
    let plan = plan_sync(&disabled, &dirs).unwrap();
    assert_eq!(plan.len(), 1);
    assert!(matches!(&plan[0], PlanAction::Remove { .. }));
    drop(basin);
}

#[test]
fn set_pin_creates_and_installs_on_first_call() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    let (pins, results) = set_pin(
        basin.path(),
        "m",
        "demo",
        "1.0.0",
        "claude_code",
        PinTarget::default(),
        &dirs,
    )
    .unwrap();

    assert_eq!(pins.pins.len(), 1);
    assert_eq!(pins.pins[0].version, "1.0.0");
    assert!(results.iter().all(|r| r.ok));
    assert!(claude.path().join("demo").join("SKILL.md").exists());
}

#[test]
fn set_pin_leaves_other_skills_pins_for_the_same_tool_alone() {
    // Pinning one skill must only re-point that skill. The pruning loop keys
    // on `entry.skill == skill && entry.version != version`; relaxing that
    // conjunction unpins every other skill sitting on a different version
    // from the same tool. Note the versions must differ for this to bite —
    // solo is on 1.0.0 while demo moves to 2.0.0.
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    set_pin(
        basin.path(),
        "m",
        "solo",
        "1.0.0",
        "claude_code",
        PinTarget::default(),
        &dirs,
    )
    .unwrap();
    let (pins, results) = set_pin(
        basin.path(),
        "m",
        "demo",
        "2.0.0",
        "claude_code",
        PinTarget::default(),
        &dirs,
    )
    .unwrap();

    assert!(results.iter().all(|r| r.ok), "{results:?}");
    let solo = pins
        .pins
        .iter()
        .find(|entry| entry.skill == "solo")
        .expect("solo's pin must survive pinning demo");
    assert!(solo.targets.contains_key("claude_code"));
    assert!(claude.path().join("solo").exists(), "solo stays installed");
    assert!(claude.path().join("demo").exists());
}

#[test]
fn unset_pin_leaves_other_skills_alone() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    for (skill, version) in [("demo", "1.0.0"), ("solo", "1.0.0")] {
        set_pin(
            basin.path(),
            "m",
            skill,
            version,
            "claude_code",
            PinTarget::default(),
            &dirs,
        )
        .unwrap();
    }

    let (pins, results) = unset_pin(basin.path(), "m", "demo", "claude_code", &dirs).unwrap();

    assert!(results.iter().all(|r| r.ok), "{results:?}");
    assert_eq!(
        pins.pins.len(),
        1,
        "only solo's pin remains: {:?}",
        pins.pins
    );
    assert_eq!(pins.pins[0].skill, "solo");
    assert!(!claude.path().join("demo").exists());
    assert!(claude.path().join("solo").exists(), "solo stays installed");
}

#[test]
fn set_pin_moves_tool_off_previous_version_of_same_skill() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    set_pin(
        basin.path(),
        "m",
        "demo",
        "1.0.0",
        "claude_code",
        PinTarget::default(),
        &dirs,
    )
    .unwrap();
    let (pins, results) = set_pin(
        basin.path(),
        "m",
        "demo",
        "2.0.0",
        "claude_code",
        PinTarget::default(),
        &dirs,
    )
    .unwrap();

    // Only one entry for "demo" survives — the 1.0.0 one is pruned (empty targets).
    let demo_entries: Vec<_> = pins.pins.iter().filter(|e| e.skill == "demo").collect();
    assert_eq!(demo_entries.len(), 1);
    assert_eq!(demo_entries[0].version, "2.0.0");
    assert!(results.iter().all(|r| r.ok));
    let content = fs::read_to_string(claude.path().join("demo").join("SKILL.md")).unwrap();
    assert_eq!(content, "demo v2");
}

#[test]
fn set_pin_lets_two_tools_pin_different_versions_of_same_skill() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let cursor = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude), ("cursor", &cursor)]);

    set_pin(
        basin.path(),
        "m",
        "demo",
        "1.0.0",
        "claude_code",
        PinTarget::default(),
        &dirs,
    )
    .unwrap();
    let (pins, _) = set_pin(
        basin.path(),
        "m",
        "demo",
        "2.0.0",
        "cursor",
        PinTarget::default(),
        &dirs,
    )
    .unwrap();

    let demo_entries: Vec<_> = pins.pins.iter().filter(|e| e.skill == "demo").collect();
    assert_eq!(demo_entries.len(), 2);
    assert_eq!(
        fs::read_to_string(claude.path().join("demo").join("SKILL.md")).unwrap(),
        "demo v1"
    );
    assert_eq!(
        fs::read_to_string(cursor.path().join("demo").join("SKILL.md")).unwrap(),
        "demo v2"
    );
}

#[test]
fn unset_pin_removes_the_managed_install() {
    let basin = fixture_basin();
    let claude = tempfile::tempdir().unwrap();
    let dirs = tool_dirs(&[("claude_code", &claude)]);

    set_pin(
        basin.path(),
        "m",
        "demo",
        "1.0.0",
        "claude_code",
        PinTarget::default(),
        &dirs,
    )
    .unwrap();
    assert!(claude.path().join("demo").exists());

    let (pins, results) = unset_pin(basin.path(), "m", "demo", "claude_code", &dirs).unwrap();
    assert!(pins.pins.iter().all(|e| e.skill != "demo"));
    assert!(results.iter().all(|r| r.ok));
    assert!(!claude.path().join("demo").exists());
}

#[test]
fn read_machine_pins_or_empty_returns_empty_set_when_missing() {
    let basin = fixture_basin();
    let pins = read_machine_pins_or_empty(basin.path(), "never-seen").unwrap();
    assert_eq!(pins.machine, "never-seen");
    assert!(pins.pins.is_empty());
}
