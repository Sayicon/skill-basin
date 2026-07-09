use std::io::Read;

use super::*;
use crate::core::basin;

fn read_zip_entries(path: &std::path::Path) -> Vec<(String, String)> {
    let file = std::fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let mut body = String::new();
        entry.read_to_string(&mut body).unwrap();
        entries.push((name, body));
    }
    entries.sort();
    entries
}

fn basin_with_version() -> (tempfile::TempDir, std::path::PathBuf) {
    let basin = tempfile::tempdir().unwrap();
    basin::basin_init(basin.path(), "b", "2026-07-09").unwrap();

    let src = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("SKILL.md"), "demo body").unwrap();
    std::fs::create_dir_all(src.path().join("refs")).unwrap();
    std::fs::write(src.path().join("refs/extra.md"), "nested").unwrap();
    basin::add_skill_version(
        basin.path(),
        "demo",
        src.path(),
        "1.0.0",
        "2026-07-09",
        None,
    )
    .unwrap();

    let out_dir = basin.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    (basin, out_dir)
}

#[test]
fn export_writes_every_file_under_a_named_root() {
    let (basin, out_dir) = basin_with_version();
    let zip_path = out_dir.join("demo.zip");

    export_skill_version(basin.path(), "demo", "1.0.0", &zip_path).unwrap();

    let entries = read_zip_entries(&zip_path);
    assert_eq!(
        entries,
        vec![
            // Unpacking must produce one self-contained directory, never
            // scatter files into the user's current directory.
            ("demo/SKILL.md".to_string(), "demo body".to_string()),
            ("demo/refs/extra.md".to_string(), "nested".to_string()),
        ]
    );
}

#[test]
fn export_uses_forward_slashes_so_the_archive_is_portable() {
    let (basin, out_dir) = basin_with_version();
    let zip_path = out_dir.join("demo.zip");

    export_skill_version(basin.path(), "demo", "1.0.0", &zip_path).unwrap();

    for (name, _) in read_zip_entries(&zip_path) {
        assert!(
            !name.contains('\\'),
            "backslash leaked into archive: {name}"
        );
    }
}

#[test]
fn export_rejects_a_version_that_is_not_in_the_basin() {
    let (basin, out_dir) = basin_with_version();
    let zip_path = out_dir.join("demo.zip");

    let err = export_skill_version(basin.path(), "demo", "9.9.9", &zip_path).unwrap_err();
    assert!(format!("{err:#}").contains("VERSION_NOT_FOUND"), "{err:#}");
    assert!(!zip_path.exists(), "no half-written archive left behind");
}

#[test]
fn export_rejects_an_unknown_skill() {
    let (basin, out_dir) = basin_with_version();
    let zip_path = out_dir.join("ghost.zip");

    let err = export_skill_version(basin.path(), "ghost", "1.0.0", &zip_path).unwrap_err();
    assert!(format!("{err:#}").contains("VERSION_NOT_FOUND"), "{err:#}");
}

#[test]
fn default_export_file_name_is_slugged_and_stable() {
    assert_eq!(
        default_export_file_name("frontend-design", "2026-07-09.85fe"),
        "frontend-design-2026-07-09.85fe.zip"
    );
    // Version labels are filesystem-safe already, but a display name may not
    // be; never let a name build a path.
    assert_eq!(
        default_export_file_name("my skill/v2", "1.0.0"),
        "my-skill-v2-1.0.0.zip"
    );
}
