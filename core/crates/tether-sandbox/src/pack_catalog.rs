//! Queries over verified installed runtime packs.

use std::fs;
use std::path::Path;

use crate::pack::validate_name;
use crate::pack_install::InstalledPack;

/// Returns program names from every verified installed pack.
#[must_use]
pub fn installed_programs(packs_root: &Path) -> std::collections::BTreeSet<String> {
    let mut programs = std::collections::BTreeSet::new();
    let Ok(entries) = fs::read_dir(packs_root) else {
        return programs;
    };
    for id_entry in entries.flatten() {
        let id = id_entry.file_name().to_string_lossy().into_owned();
        if validate_name(&id).is_err() {
            continue;
        }
        let Ok(versions) = fs::read_dir(id_entry.path()) else {
            continue;
        };
        for version_entry in versions.flatten() {
            add_version_programs(packs_root, &id, &version_entry.path(), &mut programs);
        }
    }
    programs
}

fn add_version_programs(
    packs_root: &Path,
    id: &str,
    version_root: &Path,
    programs: &mut std::collections::BTreeSet<String>,
) {
    let version = version_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let Some(version) = version else {
        return;
    };
    if validate_name(&version).is_err() {
        return;
    }
    let Ok(pack) = InstalledPack::open(packs_root, id, &version) else {
        return;
    };
    if pack.verify().is_err() {
        return;
    }
    programs.extend(pack.entry().programs.iter().cloned());
}
