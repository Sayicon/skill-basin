//! hub-agent spec: pull → plan → apply → status.json, against a real bare
//! remote (file path), the same shape a GitHub remote presents to git.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use app_lib::core::basin;
use app_lib::core::pins::{MachinePins, PinEntry, PinTarget};
use hub_agent::{run_apply, run_init, AgentConfig, StatusReport, STATUS_SCHEMA_VERSION};

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn pins_with(entries: Vec<PinEntry>) -> MachinePins {
    MachinePins {
        machine: "m1".to_string(),
        pins: entries,
    }
}

fn pin(skill: &str, version: &str, tool: &str) -> PinEntry {
    let mut targets = BTreeMap::new();
    targets.insert(tool.to_string(), PinTarget::default());
    PinEntry {
        skill: skill.to_string(),
        version: version.to_string(),
        targets,
    }
}

/// Bare remote + seeded basin: demo@1.0.0 exists, m1 pins it to tool t0.
/// Returns (tempdir, bare_repo_path).
fn seeded_remote(pins: MachinePins) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let bare = dir.path().join("remote.git");
    let out = Command::new("git")
        .args(["init", "--bare", "-b", "main"])
        .arg(&bare)
        .output()
        .unwrap();
    assert!(out.status.success());

    let seed = dir.path().join("seed");
    basin::basin_init(&seed, "fleet-test", "2026-07-11").unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("SKILL.md"), "demo v1 fleet body").unwrap();
    basin::add_skill_version(&seed, "demo", &src, "1.0.0", "2026-07-11", None).unwrap();

    let machine_dir = seed.join("machines").join("m1");
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(
        machine_dir.join("pins.json"),
        serde_json::to_string_pretty(&pins).unwrap(),
    )
    .unwrap();

    basin::basin_commit_all(&seed, "seed fleet basin").unwrap();
    git(&seed, &["remote", "add", "origin", bare.to_str().unwrap()]);
    git(&seed, &["push", "-u", "origin", "main"]);
    (dir, bare)
}

fn agent_config(root: &Path, bare: &Path) -> AgentConfig {
    let tool_dir = root.join("tool-t0");
    std::fs::create_dir_all(&tool_dir).unwrap();
    let mut tool_dirs = BTreeMap::new();
    tool_dirs.insert("t0".to_string(), tool_dir);
    AgentConfig {
        repo: bare.to_string_lossy().to_string(),
        machine: "m1".to_string(),
        basin_dir: root.join("agent-basin"),
        tool_dirs,
    }
}

/// Read a file straight out of the bare remote's main branch — proves the
/// agent PUSHED it, not just wrote it locally.
fn show_from_remote(bare: &Path, path: &str) -> Option<String> {
    let out = Command::new("git")
        .current_dir(bare)
        .args(["show", &format!("main:{path}")])
        .output()
        .unwrap();
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

#[test]
fn init_clones_and_is_idempotent() {
    let (dir, bare) = seeded_remote(pins_with(vec![pin("demo", "1.0.0", "t0")]));
    let cfg = agent_config(dir.path(), &bare);

    run_init(&cfg).unwrap();
    assert!(cfg.basin_dir.join("basin.json").exists(), "clone must land");
    assert!(cfg.basin_dir.join("machines/m1/pins.json").exists());

    run_init(&cfg).unwrap(); // second init on a healthy clone: no-op, no error
}

#[test]
fn apply_installs_pins_and_pushes_status() {
    let (dir, bare) = seeded_remote(pins_with(vec![pin("demo", "1.0.0", "t0")]));
    let cfg = agent_config(dir.path(), &bare);
    run_init(&cfg).unwrap();

    let report = run_apply(&cfg).unwrap();
    assert!(report.ok, "clean apply must report ok: {report:?}");
    assert_eq!(report.schema_version, STATUS_SCHEMA_VERSION);
    assert_eq!(report.machine, "m1");
    assert!(report.applied_at_epoch > 1_700_000_000);
    assert_eq!(report.actions.len(), 1);
    assert_eq!(report.actions[0].action, "install");
    assert!(report.actions[0].ok);

    // The skill actually landed in the tool dir.
    let body =
        std::fs::read_to_string(cfg.tool_dirs["t0"].join("demo").join("SKILL.md")).unwrap();
    assert_eq!(body, "demo v1 fleet body");

    // status.json exists locally AND on the remote (Fleet reads via git).
    let local: StatusReport = serde_json::from_str(
        &std::fs::read_to_string(cfg.basin_dir.join("machines/m1/status.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(local, report);
    let remote_raw = show_from_remote(&bare, "machines/m1/status.json")
        .expect("status.json must be pushed to the remote");
    let remote: StatusReport = serde_json::from_str(&remote_raw).unwrap();
    assert_eq!(remote, report);
}

#[test]
fn apply_picks_up_remote_pin_changes() {
    let (dir, bare) = seeded_remote(pins_with(vec![pin("demo", "1.0.0", "t0")]));
    let cfg = agent_config(dir.path(), &bare);
    run_init(&cfg).unwrap();
    run_apply(&cfg).unwrap();

    // Someone (the desktop app) unpins everything and pushes.
    let editor = dir.path().join("editor");
    let out = Command::new("git")
        .args(["clone", bare.to_str().unwrap()])
        .arg(&editor)
        .output()
        .unwrap();
    assert!(out.status.success());
    std::fs::write(
        editor.join("machines/m1/pins.json"),
        serde_json::to_string_pretty(&pins_with(vec![])).unwrap(),
    )
    .unwrap();
    git(&editor, &["add", "-A"]);
    git(&editor, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-m", "unpin"]);
    git(&editor, &["push"]);

    // Next apply pulls that change and removes the managed install.
    let report = run_apply(&cfg).unwrap();
    assert!(report.ok, "{report:?}");
    assert_eq!(report.actions.len(), 1);
    assert_eq!(report.actions[0].action, "remove");
    assert!(!cfg.tool_dirs["t0"].join("demo").exists());
}

#[test]
fn failed_action_is_reported_not_swallowed() {
    // Pin points at a version the basin doesn't have: the apply must not Err
    // out, must not claim success, and the bad row must be visible in status.
    let (dir, bare) = seeded_remote(pins_with(vec![
        pin("demo", "1.0.0", "t0"),
        pin("demo-eksik", "9.9.9", "t0"),
    ]));
    let cfg = agent_config(dir.path(), &bare);
    run_init(&cfg).unwrap();

    let report = run_apply(&cfg).unwrap();
    assert!(!report.ok, "partial failure must flip ok=false");
    let bad: Vec<_> = report.actions.iter().filter(|a| !a.ok).collect();
    assert_eq!(bad.len(), 1);
    assert_eq!(bad[0].skill, "demo-eksik");
    assert!(bad[0].error.as_deref().unwrap_or("").contains("9.9.9"));

    // The healthy pin still applied — one bad row must not block the rest.
    assert!(cfg.tool_dirs["t0"].join("demo").join("SKILL.md").exists());

    // And the failure is on the remote for the Fleet screen to see.
    let remote: StatusReport =
        serde_json::from_str(&show_from_remote(&bare, "machines/m1/status.json").unwrap())
            .unwrap();
    assert!(!remote.ok);
}

#[test]
fn config_round_trips_from_json_with_bom() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = AgentConfig {
        repo: "https://example.com/x.git".to_string(),
        machine: "m1".to_string(),
        basin_dir: dir.path().join("b"),
        tool_dirs: BTreeMap::new(),
    };
    let path = dir.path().join("agent.json");
    let mut bytes = b"\xef\xbb\xbf".to_vec(); // Windows editors love a BOM
    bytes.extend(serde_json::to_vec_pretty(&cfg).unwrap());
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(hub_agent::load_config(&path).unwrap(), cfg);
}
