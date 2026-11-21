use super::{SkillsRuntime, SkillsRuntimeError};
use std::fs;

#[test]
fn skills_runtime_lists_valid_and_invalid_and_reads_detail() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("skills");
    let runtime = SkillsRuntime::new(root.to_str().unwrap()).unwrap();
    assert_eq!(runtime.list().unwrap(), []);
    fs::create_dir_all(root.join("good-skill")).unwrap();
    fs::create_dir_all(root.join("bad-skill")).unwrap();
    fs::write(root.join("good-skill/SKILL.md"), "---\nname: good-skill\ndescription: Helps test.\nlicense: MIT\nmetadata:\n  tether-mode: careful\n---\n# Steps\nRun tests.").unwrap();
    fs::write(root.join("bad-skill/SKILL.md"), "wrong").unwrap();
    assert_skill_rows(&runtime);
    let detail = runtime.read("good-skill".into()).unwrap();
    assert_eq!(
        (
            detail.folder,
            detail.name,
            detail.description,
            detail.license,
            detail.body
        ),
        (
            "good-skill".into(),
            "good-skill".into(),
            "Helps test.".into(),
            Some("MIT".into()),
            "# Steps\nRun tests.".into()
        )
    );
    assert_eq!(detail.metadata.get("tether-mode"), Some(&"careful".into()));
    assert!(matches!(
        runtime.read("bad-skill".into()),
        Err(SkillsRuntimeError::InvalidSkill)
    ));
    assert!(matches!(
        runtime.read("../escape".into()),
        Err(SkillsRuntimeError::InvalidPath)
    ));
}

fn assert_skill_rows(runtime: &SkillsRuntime) {
    let list = runtime.list().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(
        (
            &list[0].folder,
            &list[0].name,
            &list[0].description,
            list[0].valid,
            &list[0].error
        ),
        (
            &"bad-skill".into(),
            &String::new(),
            &String::new(),
            false,
            &Some("missing frontmatter".into())
        )
    );
    assert_eq!(
        (
            &list[1].folder,
            &list[1].name,
            &list[1].description,
            list[1].valid,
            &list[1].error
        ),
        (
            &"good-skill".into(),
            &"good-skill".into(),
            &"Helps test.".into(),
            true,
            &None
        )
    );
}

#[cfg(unix)]
#[test]
fn skills_runtime_rejects_symlinked_skill_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("skills");
    let runtime = SkillsRuntime::new(root.to_str().unwrap()).unwrap();
    fs::create_dir_all(root.join("linked")).unwrap();
    let outside = dir.path().join("outside.md");
    fs::write(
        &outside,
        "---\nname: linked\ndescription: Outside\n---\nPrivate",
    )
    .unwrap();
    std::os::unix::fs::symlink(outside, root.join("linked/SKILL.md")).unwrap();
    assert!(matches!(
        runtime.read("linked".into()),
        Err(SkillsRuntimeError::InvalidPath)
    ));
}
