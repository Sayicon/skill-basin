use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use walkdir::{DirEntry, WalkDir};

const IGNORE_NAMES: [&str; 4] = [".git", ".DS_Store", "Thumbs.db", ".gitignore"];

fn is_ignored(entry: &DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    IGNORE_NAMES.iter().any(|name| name == &file_name.as_ref())
}

pub fn hash_dir(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();

    for entry in WalkDir::new(path)
        .follow_links(false)
        // WalkDir yields entries in filesystem order, which differs between
        // machines and filesystems. Normalizing separators (below) is pointless
        // without this: the same content must be visited in the same order to
        // hash identically everywhere. Sort on the lossy string rather than
        // sort_by_file_name so the comparison is byte-identical across
        // platforms for non-ASCII names too.
        .sort_by(|a, b| {
            a.file_name()
                .to_string_lossy()
                .cmp(&b.file_name().to_string_lossy())
        })
        .into_iter()
        .filter_entry(|entry| !is_ignored(entry))
    {
        let entry = entry?;
        if is_ignored(&entry) {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(path)
            .with_context(|| format!("strip prefix {:?}", entry.path()))?;
        // Normalize separators so the same content hashes identically on every OS.
        hasher.update(relative.to_string_lossy().replace('\\', "/").as_bytes());

        if entry.file_type().is_file() {
            let bytes = std::fs::read(entry.path())
                .with_context(|| format!("read file {:?}", entry.path()))?;
            hasher.update(bytes);
        }
    }

    let digest = hasher.finalize();
    Ok(hex::encode(digest))
}

#[cfg(test)]
#[path = "tests/content_hash.rs"]
mod tests;
