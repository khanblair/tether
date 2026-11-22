use std::cell::Cell;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use zip::ZipWriter;
use zip::write::{FileOptions, SimpleFileOptions};

use super::{ArchiveError, CreateOptions, ExtractLimits, create_zip, extract_zip};

struct Dir(PathBuf);

impl Dir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("tether-archive-{}", ulid::Ulid::new()));
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }

    fn sub(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

enum Item<'a> {
    File(&'a str, &'a [u8], u32),
    Dir(&'a str),
    Link(&'a str, &'a str),
}

fn build_zip(path: &Path, items: &[Item<'_>]) {
    let mut writer = ZipWriter::new(File::create(path).unwrap());
    for item in items {
        match item {
            Item::File(name, bytes, mode) => {
                let options: SimpleFileOptions = FileOptions::default().unix_permissions(*mode);
                writer.start_file(*name, options).unwrap();
                writer.write_all(bytes).unwrap();
            }
            Item::Dir(name) => {
                let options: SimpleFileOptions = FileOptions::default();
                writer.add_directory(*name, options).unwrap();
            }
            Item::Link(name, target) => {
                let options: SimpleFileOptions = FileOptions::default();
                writer.add_symlink(*name, *target, options).unwrap();
            }
        }
    }
    writer.finish().unwrap();
}

fn extract(zip: &Path, dest: &Path) -> Result<super::ExtractReport, ArchiveError> {
    extract_zip(zip, dest, &ExtractLimits::default(), &|| false, false)
}

fn listing(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    collect(root, root, &mut found);
    found
}

fn collect(root: &Path, folder: &Path, found: &mut BTreeSet<String>) {
    for item in fs::read_dir(folder).unwrap() {
        let path = item.unwrap().path();
        let relative = path.strip_prefix(root).unwrap();
        found.insert(relative.to_string_lossy().into_owned());
        if path.is_dir() && !path.is_symlink() {
            collect(root, &path, found);
        }
    }
}

fn names(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn replace_bytes(path: &Path, from: &[u8], to: &[u8]) {
    let mut bytes = fs::read(path).unwrap();
    let mut index = 0;
    while index + from.len() <= bytes.len() {
        if &bytes[index..index + from.len()] == from {
            bytes[index..index + from.len()].copy_from_slice(to);
            index += from.len();
        } else {
            index += 1;
        }
    }
    fs::write(path, bytes).unwrap();
}

#[test]
fn every_unsafe_entry_name_is_rejected_and_nothing_is_written() {
    let names = [
        "../evil",
        "/abs/evil",
        "a/../../b",
        "a/../b",
        "C:/x",
        "C:\\x",
        "a\\b",
        "a\0b",
        "a//b",
        "./a",
        "a/./b",
    ];
    for name in names {
        let dir = Dir::new();
        let zip = dir.0.join("bad.zip");
        build_zip(
            &zip,
            &[
                Item::File("ok.txt", b"ok", 0o644),
                Item::File(name, b"x", 0o644),
            ],
        );
        let dest = dir.sub("dest");
        let error = extract(&zip, &dest).unwrap_err();
        assert!(
            matches!(error, ArchiveError::InvalidEntryName),
            "{name:?} gave {error:?}"
        );
        assert!(listing(&dest).is_empty(), "{name:?} left files behind");
        assert!(!dir.0.join("evil").exists() && !dir.0.join("b").exists());
    }
}

#[test]
fn duplicate_paths_and_file_folder_conflicts_are_rejected() {
    let dir = Dir::new();
    let duplicate = dir.0.join("dup.zip");
    build_zip(
        &duplicate,
        &[
            Item::File("dupA.txt", b"x", 0o644),
            Item::File("dupB.txt", b"y", 0o644),
        ],
    );
    replace_bytes(&duplicate, b"dupB.txt", b"dupA.txt");
    let cases: Vec<(&str, Vec<Item<'_>>)> = vec![
        (
            "file then child",
            vec![Item::File("a", b"x", 0o644), Item::File("a/b", b"y", 0o644)],
        ),
        (
            "child then file",
            vec![Item::File("a/b", b"y", 0o644), Item::File("a", b"x", 0o644)],
        ),
        (
            "folder and file",
            vec![Item::Dir("a/"), Item::File("a", b"x", 0o644)],
        ),
    ];
    for (label, items) in cases {
        let zip = dir.0.join("case.zip");
        build_zip(&zip, &items);
        let dest = dir.sub(&format!("dest-{}", label.replace(' ', "-")));
        assert!(
            matches!(extract(&zip, &dest), Err(ArchiveError::InvalidEntryName)),
            "{label}"
        );
        assert!(listing(&dest).is_empty(), "{label}");
    }
    let dest = dir.sub("dest-dup");
    let result = extract(&duplicate, &dest);
    assert!(
        matches!(result, Err(ArchiveError::InvalidEntryName)),
        "{result:?}"
    );
}

#[test]
fn symlink_entries_are_skipped_and_counted() {
    let dir = Dir::new();
    let zip = dir.0.join("links.zip");
    build_zip(
        &zip,
        &[
            Item::Link("escape", "/etc"),
            Item::File("real.txt", b"data", 0o644),
        ],
    );
    let dest = dir.sub("dest");
    let report = extract(&zip, &dest).unwrap();
    assert_eq!(report.skipped_symlinks, 1);
    assert_eq!(report.files, 1);
    assert_eq!(listing(&dest), names(&["real.txt"]));
}

#[test]
fn an_entry_written_through_a_symlink_entry_rejects_the_whole_archive() {
    let dir = Dir::new();
    for items in [
        vec![
            Item::Link("escape", "/etc"),
            Item::File("escape/passwd", b"x", 0o644),
        ],
        vec![
            Item::File("escape/passwd", b"x", 0o644),
            Item::Link("escape", "/etc"),
        ],
    ] {
        let zip = dir.0.join("links.zip");
        build_zip(&zip, &items);
        let dest = dir.sub(&format!("d{}", ulid::Ulid::new()));
        assert!(matches!(
            extract(&zip, &dest),
            Err(ArchiveError::InvalidEntryName)
        ));
        assert!(listing(&dest).is_empty());
    }
}

#[test]
fn the_destination_must_be_an_empty_real_folder() {
    let dir = Dir::new();
    let zip = dir.0.join("a.zip");
    build_zip(&zip, &[Item::File("a.txt", b"x", 0o644)]);
    let full = dir.sub("full");
    fs::write(full.join("old"), "x").unwrap();
    assert!(matches!(
        extract(&zip, &full),
        Err(ArchiveError::InvalidDestination)
    ));
    let real = dir.sub("real");
    symlink(&real, dir.0.join("link")).unwrap();
    assert!(matches!(
        extract(&zip, &dir.0.join("link")),
        Err(ArchiveError::InvalidDestination)
    ));
    assert!(matches!(
        extract(&zip, &dir.0.join("missing")),
        Err(ArchiveError::Io(_))
    ));
    assert!(matches!(
        extract(&dir.0.join("a.txt-not-a-zip"), &real),
        Err(ArchiveError::Io(_))
    ));
    fs::write(dir.0.join("junk.zip"), "not a zip").unwrap();
    assert!(matches!(
        extract(&dir.0.join("junk.zip"), &real),
        Err(ArchiveError::NotZip)
    ));
}

#[test]
fn entry_count_total_size_and_file_size_limits_apply() {
    let dir = Dir::new();
    let zip = dir.0.join("sizes.zip");
    build_zip(
        &zip,
        &[
            Item::File("a", &[1; 100], 0o644),
            Item::File("b", &[2; 100], 0o644),
            Item::File("c", &[3; 100], 0o644),
        ],
    );
    let run = |limits: ExtractLimits| {
        let dest = dir.sub(&format!("d{}", ulid::Ulid::new()));
        let result = extract_zip(&zip, &dest, &limits, &|| false, false);
        assert!(listing(&dest).is_empty() || result.is_ok());
        result
    };
    let roomy = ExtractLimits::default();
    assert!(run(roomy).is_ok());
    assert!(matches!(
        run(ExtractLimits {
            max_entries: 2,
            ..roomy
        }),
        Err(ArchiveError::TooManyEntries)
    ));
    assert!(matches!(
        run(ExtractLimits {
            max_total_bytes: 299,
            ..roomy
        }),
        Err(ArchiveError::TooLarge)
    ));
    assert!(
        run(ExtractLimits {
            max_total_bytes: 300,
            ..roomy
        })
        .is_ok()
    );
    assert!(matches!(
        run(ExtractLimits {
            max_file_bytes: 99,
            ..roomy
        }),
        Err(ArchiveError::TooLarge)
    ));
}

#[test]
fn a_declared_ratio_bomb_is_rejected_before_anything_is_written() {
    let dir = Dir::new();
    let zip = dir.0.join("bomb.zip");
    build_zip(&zip, &[Item::File("zeros.bin", &[0; 4096], 0o644)]);
    // Rewrite the declared uncompressed size to 120 MiB in the local and central headers.
    let mut bytes = fs::read(&zip).unwrap();
    let declared = (120_u32 << 20).to_le_bytes();
    let mut patched = 0;
    for index in 0..bytes.len().saturating_sub(46) {
        if bytes[index..index + 4] == [0x50, 0x4b, 0x03, 0x04] {
            bytes[index + 22..index + 26].copy_from_slice(&declared);
            patched += 1;
        }
        if bytes[index..index + 4] == [0x50, 0x4b, 0x01, 0x02] {
            bytes[index + 24..index + 28].copy_from_slice(&declared);
            patched += 1;
        }
    }
    assert_eq!(patched, 2);
    fs::write(&zip, bytes).unwrap();
    let dest = dir.sub("dest");
    let limits = ExtractLimits {
        max_file_bytes: 1 << 30,
        ..ExtractLimits::default()
    };
    let result = extract_zip(&zip, &dest, &limits, &|| false, false);
    assert!(matches!(result, Err(ArchiveError::Ratio)), "{result:?}");
    assert!(listing(&dest).is_empty());
}

#[test]
fn cancelling_stops_extraction_and_removes_partial_output() {
    let dir = Dir::new();
    let zip = dir.0.join("many.zip");
    build_zip(
        &zip,
        &[
            Item::File("d/one.txt", b"1", 0o644),
            Item::File("d/two.txt", b"2", 0o644),
            Item::File("three.txt", b"3", 0o644),
        ],
    );
    let dest = dir.sub("dest");
    let calls = Cell::new(0_u32);
    let cancel = || {
        calls.set(calls.get() + 1);
        calls.get() > 4
    };
    let result = extract_zip(&zip, &dest, &ExtractLimits::default(), &cancel, false);
    assert!(matches!(result, Err(ArchiveError::Cancelled)), "{result:?}");
    assert!(listing(&dest).is_empty());
}

#[test]
fn strip_single_top_level_only_drops_a_shared_folder() {
    let dir = Dir::new();
    let strip = |items: &[Item<'_>]| {
        let zip = dir.0.join("s.zip");
        build_zip(&zip, items);
        let dest = dir.sub(&format!("d{}", ulid::Ulid::new()));
        extract_zip(&zip, &dest, &ExtractLimits::default(), &|| false, true).unwrap();
        listing(&dest)
    };
    assert_eq!(
        strip(&[
            Item::Dir("proj/"),
            Item::File("proj/a.txt", b"a", 0o644),
            Item::File("proj/sub/b.txt", b"b", 0o644),
        ]),
        names(&["a.txt", "sub", "sub/b.txt"])
    );
    assert_eq!(
        strip(&[
            Item::File("proj/a.txt", b"a", 0o644),
            Item::File("other.txt", b"b", 0o644)
        ]),
        names(&["proj", "proj/a.txt", "other.txt"])
    );
    assert_eq!(
        strip(&[Item::File("only.txt", b"a", 0o644)]),
        names(&["only.txt"])
    );
}

#[test]
fn extracted_modes_never_carry_setuid_or_lock_out_the_owner() {
    let dir = Dir::new();
    let zip = dir.0.join("modes.zip");
    build_zip(
        &zip,
        &[
            Item::File("suid", b"x", 0o4755),
            Item::File("none", b"x", 0o000),
            Item::File("script.sh", b"x", 0o755),
        ],
    );
    let dest = dir.sub("dest");
    extract(&zip, &dest).unwrap();
    let mode = |name: &str| fs::metadata(dest.join(name)).unwrap().permissions().mode() & 0o7777;
    assert_eq!(mode("suid"), 0o755);
    assert_eq!(mode("none"), 0o600);
    assert_eq!(mode("script.sh"), 0o755);
}

fn make_project(root: &Path) {
    fs::create_dir_all(root.join("src/deep")).unwrap();
    fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".gitignore"), "*.log\n").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(root.join("src/deep/empty"), "").unwrap();
    fs::write(root.join("run.sh"), "#!/bin/sh\n").unwrap();
    fs::set_permissions(root.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(root.join("debug.log"), "noise\n").unwrap();
    fs::write(root.join("node_modules/pkg/index.js"), "x\n").unwrap();
    fs::write(root.join(".git/config"), "secret\n").unwrap();
    symlink(root.join("src/main.rs"), root.join("link")).unwrap();
}

#[test]
fn create_then_extract_round_trips_files_and_modes() {
    let dir = Dir::new();
    let project = dir.sub("project");
    make_project(&project);
    let archive = dir.0.join("out/project.zip");
    let report = create_zip(&project, &archive, &CreateOptions::default(), &|| false).unwrap();
    assert_eq!(report.files, 4);
    assert_eq!(report.skipped_symlinks, 1);
    let restored = dir.sub("restored");
    extract(&archive, &restored).unwrap();
    assert_eq!(
        listing(&restored),
        names(&[
            ".gitignore",
            "run.sh",
            "src",
            "src/deep",
            "src/deep/empty",
            "src/main.rs"
        ])
    );
    assert_eq!(
        fs::read(restored.join("src/main.rs")).unwrap(),
        b"fn main() {}\n"
    );
    let mode = |root: &Path| {
        fs::metadata(root.join("run.sh"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode(&restored), mode(&project));
}

#[test]
fn include_ignored_adds_ignored_files_but_never_git() {
    let dir = Dir::new();
    let project = dir.sub("project");
    make_project(&project);
    let archive = dir.0.join("all.zip");
    let options = CreateOptions {
        include_ignored: true,
        ..CreateOptions::default()
    };
    create_zip(&project, &archive, &options, &|| false).unwrap();
    let restored = dir.sub("restored");
    extract(&archive, &restored).unwrap();
    let found = listing(&restored);
    assert!(found.contains("debug.log") && found.contains("node_modules/pkg/index.js"));
    assert!(
        !found
            .iter()
            .any(|path| path.starts_with(".git/") || path == ".git")
    );
}

#[test]
fn create_refuses_a_destination_inside_the_project_and_cleans_up_on_cancel() {
    let dir = Dir::new();
    let project = dir.sub("project");
    make_project(&project);
    let inside = project.join("src/out.zip");
    assert!(matches!(
        create_zip(&project, &inside, &CreateOptions::default(), &|| false),
        Err(ArchiveError::InvalidDestination)
    ));
    assert!(!inside.exists());
    let outside = dir.sub("outside").join("p.zip");
    let result = create_zip(&project, &outside, &CreateOptions::default(), &|| true);
    assert!(matches!(result, Err(ArchiveError::Cancelled)));
    assert!(listing(&dir.0.join("outside")).is_empty());
    let result = create_zip(
        &project,
        &outside,
        &CreateOptions {
            max_total_bytes: 3,
            ..CreateOptions::default()
        },
        &|| false,
    );
    assert!(matches!(result, Err(ArchiveError::TooLarge)));
    assert!(listing(&dir.0.join("outside")).is_empty());
}
