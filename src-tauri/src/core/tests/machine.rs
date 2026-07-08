use super::*;
use crate::core::skill_store::SkillStore;

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().unwrap();
    (dir, store)
}

#[test]
fn generates_and_persists_a_machine_id() {
    let (_dir, store) = make_store();
    let first = current_machine_id(&store).unwrap();
    assert!(!first.is_empty());

    let second = current_machine_id(&store).unwrap();
    assert_eq!(first, second, "machine id must be stable across calls");
}

#[test]
fn slugify_lowercases_and_replaces_non_alphanumeric() {
    assert_eq!(slugify("Kerem-PC"), "kerem-pc");
    assert_eq!(slugify("kerem_pc.local"), "kerem-pc-local");
    assert_eq!(slugify("---weird---"), "weird");
}
