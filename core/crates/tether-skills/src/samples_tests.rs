use std::fs;
use std::path::Path;

use super::{SampleInstall, install_sample, sample_skills, validate_sample_path};
use crate::{
    DiscoverInput, ScanVerdict, SkillError, SkillFile, SkillSource, SkillsCatalog, parse_skill,
    scan_skill,
};

const SAMPLE_NAMES: [&str; 11] = [
    "caveman",
    "code-review",
    "debug",
    "diagnosing-bugs",
    "handoff",
    "research",
    "resolving-merge-conflicts",
    "tdd",
    "to-spec",
    "triage",
    "writing-for-agents",
];

#[test]
fn all_sample_names_are_present_and_sorted() {
    let names = sample_skills()
        .iter()
        .map(|sample| sample.name)
        .collect::<Vec<_>>();
    assert_eq!(names, SAMPLE_NAMES);
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn every_sample_skill_parses_with_a_description() {
    for sample in sample_skills() {
        let source = skill_contents(sample.name);
        let document = parse_skill(sample.name, source).unwrap();
        assert!(!document.description.trim().is_empty(), "{}", sample.name);
    }
}

#[test]
fn every_sample_skill_passes_the_static_scan() {
    for sample in sample_skills() {
        let files = sample
            .files
            .iter()
            .map(|file| SkillFile {
                path: format!("{}/{}", sample.name, file.path),
                bytes: file.contents.as_bytes().to_vec(),
            })
            .collect::<Vec<_>>();
        let report = scan_skill(&files);
        println!(
            "{}: {:?} ({:?})",
            sample.name, report.verdict, report.findings
        );
        assert_ne!(report.verdict, ScanVerdict::Blocked, "{}", sample.name);
    }
}

#[test]
fn embedded_skill_files_match_the_sample_tree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
    for sample in sample_skills() {
        let skill_file = sample
            .files
            .iter()
            .find(|file| file.path == "SKILL.md")
            .unwrap();
        let disk = root.join(sample.name).join(skill_file.path);
        assert_eq!(
            skill_file.contents,
            fs::read_to_string(disk).unwrap(),
            "{}",
            sample.name
        );
    }
}

#[test]
fn install_writes_all_files_and_preserves_user_edits() {
    let directory = tempfile::tempdir().unwrap();
    let skills_dir = directory.path().join("skills");
    assert_eq!(
        install_sample(&skills_dir, "tdd").unwrap(),
        SampleInstall::Installed
    );
    let sample = sample_skills()
        .iter()
        .find(|sample| sample.name == "tdd")
        .unwrap();
    for file in sample.files {
        assert_eq!(
            fs::read_to_string(skills_dir.join("tdd").join(file.path)).unwrap(),
            file.contents
        );
    }

    let skill_path = skills_dir.join("tdd/SKILL.md");
    let user_edit = format!("{}\n<!-- user edit -->\n", sample.files[0].contents);
    fs::write(&skill_path, &user_edit).unwrap();
    assert_eq!(
        install_sample(&skills_dir, "tdd").unwrap(),
        SampleInstall::AlreadyInstalled
    );
    assert_eq!(fs::read_to_string(skill_path).unwrap(), user_edit);
    assert!(fs::read_dir(&skills_dir).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".install-")
    }));
    assert_eq!(
        install_sample(&skills_dir, "missing").unwrap_err(),
        SkillError::NotFound
    );

    let catalog = SkillsCatalog::discover(DiscoverInput {
        project_root: None,
        project_trusted: false,
        user_dir: &skills_dir,
        bundled: &[],
    });
    let installed = catalog
        .skills
        .iter()
        .find(|skill| skill.name == "tdd")
        .unwrap();
    assert_eq!(installed.source, SkillSource::User);
}

#[test]
fn sample_paths_reject_absolute_and_parent_paths() {
    assert_eq!(
        validate_sample_path("/SKILL.md"),
        Err(SkillError::InvalidPath)
    );
    assert_eq!(
        validate_sample_path("scripts/../SKILL.md"),
        Err(SkillError::InvalidPath)
    );
}

fn skill_contents(name: &str) -> &str {
    sample_skills()
        .iter()
        .find(|sample| sample.name == name)
        .unwrap()
        .files
        .iter()
        .find(|file| file.path == "SKILL.md")
        .unwrap()
        .contents
}
