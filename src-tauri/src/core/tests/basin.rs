use std::collections::BTreeMap;
use std::fs;

use super::*;

fn sample_meta() -> SkillMeta {
    let mut versions = BTreeMap::new();
    versions.insert(
        "1.0.0".to_string(),
        VersionInfo {
            content_hash: "sha256:aabbccdd".to_string(),
            added_at: "2026-06-01".to_string(),
        },
    );
    versions.insert(
        "2026-07-08.a3f2".to_string(),
        VersionInfo {
            content_hash: "sha256:a3f2eeff".to_string(),
            added_at: "2026-07-08".to_string(),
        },
    );
    SkillMeta {
        id: "remotion-best-practices".to_string(),
        display_name: "Remotion Best Practices".to_string(),
        source: SkillSource {
            source_type: "git".to_string(),
            url: Some("https://github.com/remotion-dev/skills".to_string()),
            subpath: Some("remotion-best-practices".to_string()),
            git_ref: Some("main".to_string()),
        },
        tags: vec!["video".to_string()],
        latest: "2026-07-08.a3f2".to_string(),
        versions,
        security: SecurityInfo::default(),
    }
}

#[test]
fn version_label_prefers_semver() {
    assert_eq!(
        version_label(Some("2.1.0"), "2026-07-08", "sha256:a3f2eeff"),
        "2.1.0"
    );
}

#[test]
fn version_label_falls_back_to_date_and_hash4() {
    assert_eq!(
        version_label(None, "2026-07-08", "sha256:a3f2eeff"),
        "2026-07-08.a3f2"
    );
    // No algorithm prefix: first 4 chars of the raw hash.
    assert_eq!(
        version_label(None, "2026-07-08", "deadbeef"),
        "2026-07-08.dead"
    );
}

#[test]
fn skill_meta_json_round_trip_keeps_wire_field_names() {
    let meta = sample_meta();
    let json = serde_json::to_string_pretty(&meta).unwrap();
    // Wire format is the documented one: "type" and "ref", camelCase keys.
    assert!(json.contains("\"type\": \"git\""));
    assert!(json.contains("\"ref\": \"main\""));
    assert!(json.contains("\"displayName\""));
    assert!(json.contains("\"contentHash\""));
    let back: SkillMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(back, meta);
}

#[test]
fn skill_meta_defaults_missing_optional_fields() {
    // Older/hand-written files may omit tags and security entirely.
    let json = r#"{
        "id": "x",
        "displayName": "X",
        "source": {"type": "local"},
        "latest": "1.0.0",
        "versions": {"1.0.0": {"contentHash": "sha256:00", "addedAt": "2026-07-08"}}
    }"#;
    let meta: SkillMeta = serde_json::from_str(json).unwrap();
    assert!(meta.tags.is_empty());
    assert_eq!(meta.security.scan_status, "unscanned");
}

#[test]
fn basin_init_writes_manifest_and_git_repo_idempotently() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = basin_init(dir.path(), "kerem-basin", "2026-07-08").unwrap();
    assert_eq!(manifest.schema_version, BASIN_SCHEMA_VERSION);
    assert!(dir.path().join(BASIN_MANIFEST_FILE).exists());
    assert!(dir.path().join(".git").exists());

    // Second init must not fail and must not overwrite the manifest.
    let again = basin_init(dir.path(), "other-name", "2027-01-01").unwrap();
    assert_eq!(again.name, "kerem-basin");
    assert_eq!(read_manifest(dir.path()).unwrap().name, "kerem-basin");
}

#[test]
fn add_skill_version_copies_hashes_and_updates_latest() {
    let basin = tempfile::tempdir().unwrap();
    basin_init(basin.path(), "b", "2026-07-08").unwrap();

    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("SKILL.md"), "---\nname: demo\n---\nbody").unwrap();
    fs::create_dir_all(src.path().join("assets")).unwrap();
    fs::write(src.path().join("assets/a.txt"), "x").unwrap();

    let info = add_skill_version(
        basin.path(),
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-08",
        None,
    )
    .unwrap();
    assert!(info.content_hash.starts_with("sha256:"));

    // Files materialized under versions/<label>/.
    let vdir = version_dir(basin.path(), "demo", "1.0.0");
    assert!(vdir.join("SKILL.md").exists());
    assert!(vdir.join("assets/a.txt").exists());

    // Metadata updated.
    let meta = read_skill_meta(basin.path(), "demo").unwrap();
    assert_eq!(meta.latest, "1.0.0");
    assert_eq!(meta.versions["1.0.0"], info);

    // Same content from a different directory hashes identically (determinism).
    let src2 = tempfile::tempdir().unwrap();
    fs::write(src2.path().join("SKILL.md"), "---\nname: demo\n---\nbody").unwrap();
    fs::create_dir_all(src2.path().join("assets")).unwrap();
    fs::write(src2.path().join("assets/a.txt"), "x").unwrap();
    let info2 = add_skill_version(
        basin.path(),
        "demo",
        src2.path(),
        "1.0.1",
        "2026-07-09",
        None,
    )
    .unwrap();
    assert_eq!(info.content_hash, info2.content_hash);

    // Latest moved forward; the old version directory is still there (rollback is free).
    let meta = read_skill_meta(basin.path(), "demo").unwrap();
    assert_eq!(meta.latest, "1.0.1");
    assert!(version_dir(basin.path(), "demo", "1.0.0").exists());
}

#[test]
fn add_skill_version_rejects_duplicate_label() {
    let basin = tempfile::tempdir().unwrap();
    basin_init(basin.path(), "b", "2026-07-08").unwrap();
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("SKILL.md"), "x").unwrap();

    add_skill_version(
        basin.path(),
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-08",
        None,
    )
    .unwrap();
    let err = add_skill_version(
        basin.path(),
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-08",
        None,
    )
    .unwrap_err();
    assert!(
        format!("{:#}", err).contains("1.0.0"),
        "error should name the duplicate label: {:#}",
        err
    );
}

#[test]
fn migrate_central_moves_skills_as_initial_versions() {
    let central = tempfile::tempdir().unwrap();
    fs::create_dir_all(central.path().join("skill-a")).unwrap();
    fs::write(central.path().join("skill-a/SKILL.md"), "# A").unwrap();
    fs::create_dir_all(central.path().join("skill-b/nested")).unwrap();
    fs::write(central.path().join("skill-b/SKILL.md"), "# B").unwrap();
    fs::write(central.path().join("skill-b/nested/x.txt"), "x").unwrap();
    // Loose file at central root must be ignored, not treated as a skill.
    fs::write(central.path().join("README.md"), "not a skill").unwrap();

    let basin = tempfile::tempdir().unwrap();
    basin_init(basin.path(), "b", "2026-07-08").unwrap();

    let mut sources = BTreeMap::new();
    sources.insert(
        "skill-a".to_string(),
        SkillSource {
            source_type: "git".to_string(),
            url: Some("https://github.com/x/y".to_string()),
            ..Default::default()
        },
    );

    let report =
        migrate_central_to_basin(central.path(), basin.path(), &sources, "2026-07-08").unwrap();
    assert_eq!(report.migrated.len(), 2);
    assert!(report.skipped.is_empty());

    let meta_a = read_skill_meta(basin.path(), "skill-a").unwrap();
    assert_eq!(meta_a.source.source_type, "git");
    assert_eq!(meta_a.versions.len(), 1);
    assert!(version_dir(basin.path(), "skill-a", &meta_a.latest)
        .join("SKILL.md")
        .exists());

    // No source info in the store -> falls back to "local".
    let meta_b = read_skill_meta(basin.path(), "skill-b").unwrap();
    assert_eq!(meta_b.source.source_type, "local");
    assert!(version_dir(basin.path(), "skill-b", &meta_b.latest)
        .join("nested/x.txt")
        .exists());

    // Re-running the migration skips everything (idempotent).
    let report2 =
        migrate_central_to_basin(central.path(), basin.path(), &sources, "2026-07-09").unwrap();
    assert!(report2.migrated.is_empty());
    assert_eq!(report2.skipped.len(), 2);
}

#[test]
fn unix_days_convert_to_civil_dates() {
    assert_eq!(unix_days_to_date(0), "1970-01-01");
    assert_eq!(unix_days_to_date(19723), "2024-01-01");
    assert_eq!(unix_days_to_date(11016), "2000-02-29"); // leap day
}

#[test]
fn basin_commit_all_commits_changes_and_skips_clean_tree() {
    let basin = tempfile::tempdir().unwrap();
    basin_init(basin.path(), "b", "2026-07-08").unwrap();

    // Initial content -> first commit exists.
    let first = basin_commit_all(basin.path(), "init basin").unwrap();
    assert!(first.is_some());

    // Clean tree -> no new commit.
    assert!(basin_commit_all(basin.path(), "noop").unwrap().is_none());

    // New version -> new commit, different id.
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("SKILL.md"), "x").unwrap();
    add_skill_version(
        basin.path(),
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-08",
        None,
    )
    .unwrap();
    let second = basin_commit_all(basin.path(), "add demo 1.0.0").unwrap();
    assert!(second.is_some());
    assert_ne!(first, second);
}

#[test]
fn basin_round_trips_through_a_remote() {
    // Local bare repo stands in for GitHub — full clone/push/pull loop offline.
    let remote = tempfile::tempdir().unwrap();
    let mut opts = git2::RepositoryInitOptions::new();
    opts.bare(true).initial_head("main");
    git2::Repository::init_opts(remote.path(), &opts).unwrap();
    let remote_url = remote.path().to_string_lossy().replace('\\', "/");

    let a = tempfile::tempdir().unwrap();
    let a_dir = a.path().join("basin");
    basin_init(&a_dir, "b", "2026-07-08").unwrap();
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("SKILL.md"), "hello").unwrap();
    add_skill_version(&a_dir, "demo", src.path(), "1.0.0", "2026-07-08", None).unwrap();
    basin_commit_all(&a_dir, "add demo").unwrap();

    {
        let repo = git2::Repository::open(&a_dir).unwrap();
        repo.remote("origin", &remote_url).unwrap();
    }
    basin_push(&a_dir).unwrap();

    // Fresh machine clones the same basin and sees the version.
    let b = tempfile::tempdir().unwrap();
    let b_dir = b.path().join("basin");
    basin_clone_or_pull(&remote_url, &b_dir).unwrap();
    let meta = read_skill_meta(&b_dir, "demo").unwrap();
    assert_eq!(meta.latest, "1.0.0");
    assert!(version_dir(&b_dir, "demo", "1.0.0")
        .join("SKILL.md")
        .exists());
}

#[test]
fn record_update_adds_version_only_when_content_changed() {
    let basin = tempfile::tempdir().unwrap();
    basin_init(basin.path(), "b", "2026-07-08").unwrap();

    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("SKILL.md"), "v1 body").unwrap();
    add_skill_version(
        basin.path(),
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-08",
        None,
    )
    .unwrap();

    // Same content -> no new version, latest unchanged.
    let unchanged =
        record_update_as_version(basin.path(), "demo", src.path(), None, "2026-07-09").unwrap();
    assert!(unchanged.is_none());
    assert_eq!(
        read_skill_meta(basin.path(), "demo").unwrap().latest,
        "1.0.0"
    );

    // Changed content -> new date.hash4 version, old one still on disk.
    fs::write(src.path().join("SKILL.md"), "v2 body").unwrap();
    let (label, info) =
        record_update_as_version(basin.path(), "demo", src.path(), None, "2026-07-09")
            .unwrap()
            .unwrap();
    assert!(label.starts_with("2026-07-09."));
    assert!(info.content_hash.starts_with("sha256:"));
    let meta = read_skill_meta(basin.path(), "demo").unwrap();
    assert_eq!(meta.latest, label);
    assert_eq!(meta.versions.len(), 2);
    assert!(version_dir(basin.path(), "demo", "1.0.0").exists());
}
