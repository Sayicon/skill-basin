//! Packaging a basin version as a zip.
//!
//! Sharing a skill needs no registry: a skill is already a directory, and the
//! basin already keeps every version as one. Sharing means either pointing at
//! the basin's git remote or handing over one of these archives.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use zip::write::SimpleFileOptions;

use super::basin::version_dir;

/// Zips `skill`@`version` from the basin into `zip_path`.
///
/// Every entry is nested under a `<skill>/` root so unpacking never scatters
/// files into the working directory, and paths use `/` so the archive opens
/// the same way on every OS.
pub fn export_skill_version(
    basin_dir: &Path,
    skill: &str,
    version: &str,
    zip_path: &Path,
) -> Result<()> {
    let source = version_dir(basin_dir, skill, version);
    if !source.is_dir() {
        anyhow::bail!("VERSION_NOT_FOUND|{skill}@{version}");
    }

    if let Some(parent) = zip_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create export dir {:?}", parent))?;
    }

    // Build into a temporary file so a failure never leaves a truncated
    // archive where the user expects a complete one.
    let staging = zip_path.with_extension("zip.part");
    let result = write_archive(&source, skill, &staging);
    match result {
        Ok(()) => {
            std::fs::rename(&staging, zip_path)
                .with_context(|| format!("finalize export {:?}", zip_path))?;
            Ok(())
        }
        Err(err) => {
            // The staging path is a regular file this function just created.
            // ast-grep-ignore: no-raw-remove-file
            let _ = std::fs::remove_file(&staging);
            Err(err)
        }
    }
}

fn write_archive(source: &Path, root_name: &str, staging: &Path) -> Result<()> {
    let file = std::fs::File::create(staging).with_context(|| format!("create {:?}", staging))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in walkdir::WalkDir::new(source).sort_by_file_name() {
        let entry = entry.context("walk version dir")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(source)
            .context("strip version dir prefix")?;
        let archive_path = format!("{root_name}/{}", to_archive_path(relative));

        zip.start_file(&archive_path, options)
            .with_context(|| format!("add {archive_path}"))?;
        let bytes =
            std::fs::read(entry.path()).with_context(|| format!("read {:?}", entry.path()))?;
        zip.write_all(&bytes)
            .with_context(|| format!("write {archive_path}"))?;
    }

    zip.finish().context("finish zip")?;
    Ok(())
}

/// Zip paths are always `/`-separated, regardless of the host OS.
fn to_archive_path(relative: &Path) -> String {
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// A skill's display name may contain characters that are illegal — or
/// meaningful — in a path, so slug it before it reaches the filesystem.
pub fn default_export_file_name(skill: &str, version: &str) -> String {
    let slug = skill
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug.trim_matches('-').to_string();
    let slug = if slug.is_empty() {
        "skill".to_string()
    } else {
        slug
    };
    format!("{slug}-{version}.zip")
}

#[cfg(test)]
#[path = "tests/export.rs"]
mod tests;
