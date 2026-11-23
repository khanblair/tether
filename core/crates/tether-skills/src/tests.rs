use crate::{BundledSkill, DiscoverInput, SkillIssue, SkillSource, SkillsCatalog, skills_index};
use crate::{SkillError, parse_skill};
use std::fs;
use std::path::{Path, PathBuf};

fn root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tether-skills-{label}-{}", std::process::id()))
}

fn write_skill(root: &Path, name: &str, description: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\nBody"),
    )
    .unwrap();
}

#[test]
fn agent_skills_example_parses() {
    let source = "---\nname: pdf-review\ndescription: Review PDF files and extract findings.\nlicense: MIT\nmetadata:\n  tether-category: documents\n---\n# PDF review\nRead the file.";
    let skill = parse_skill("pdf-review", source).unwrap();
    assert_eq!(skill.name, "pdf-review");
    assert_eq!(skill.description, "Review PDF files and extract findings.");
    assert_eq!(skill.metadata["tether-category"], "documents");
    assert!(skill.body.contains("Read the file."));
}

#[test]
fn names_must_match_the_folder_and_fit_the_limit() {
    let source = "---\nname: clean-skill\ndescription: A clean fixture.\n---\nBody";
    assert_eq!(parse_skill("other", source), Err(SkillError::InvalidName));
    let long = "a".repeat(65);
    let source = format!("---\nname: {long}\ndescription: Fixture.\n---\nBody");
    assert_eq!(parse_skill(&long, &source), Err(SkillError::InvalidName));
    let uppercase = "---\nname: Bad\ndescription: Fixture.\n---\nBody";
    assert_eq!(parse_skill("Bad", uppercase), Err(SkillError::InvalidName));
}

#[test]
fn missing_and_long_descriptions_fail() {
    let missing = "---\nname: test\n---\nBody";
    assert_eq!(
        parse_skill("test", missing),
        Err(SkillError::MissingField("description"))
    );
    let long = "a".repeat(1025);
    let source = format!("---\nname: test\ndescription: {long}\n---\nBody");
    assert_eq!(
        parse_skill("test", &source),
        Err(SkillError::DescriptionTooLong)
    );
}

#[test]
fn discovery_uses_precedence_and_reports_shadowed_skills() {
    let base = root("precedence");
    let project = base.join("project");
    let user = base.join("user");
    let bundled = base.join("bundled");
    write_skill(&project.join(".tether/skills"), "same", "project");
    write_skill(&user, "same", "user");
    write_skill(&bundled, "same", "bundled");

    let catalog = SkillsCatalog::discover(DiscoverInput {
        project_root: Some(&project),
        project_trusted: true,
        user_dir: &user,
        bundled: &[BundledSkill {
            name: "same".into(),
            source: bundled.join("same"),
        }],
    });

    assert_eq!(catalog.skills.len(), 1);
    assert_eq!(catalog.skills[0].source, SkillSource::Project);
    assert_eq!(catalog.skills[0].description, "project");
    assert_eq!(
        catalog
            .issues
            .iter()
            .filter(|issue| matches!(issue, SkillIssue::Shadowed { .. }))
            .count(),
        2
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn untrusted_project_skills_are_reported_but_not_loaded() {
    let base = root("trust");
    let project = base.join("project");
    let user = base.join("user");
    write_skill(&project.join(".agents/skills"), "project-only", "project");

    let catalog = SkillsCatalog::discover(DiscoverInput {
        project_root: Some(&project),
        project_trusted: false,
        user_dir: &user,
        bundled: &[],
    });

    assert_eq!(catalog.skills.len(), 0);
    assert_eq!(
        catalog.issues,
        vec![SkillIssue::ProjectSkillsNeedTrust { count: 1 }]
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn hidden_user_folders_and_disabled_skills_are_safe() {
    let base = root("hidden");
    let user = base.join("user");
    write_skill(&user.join(".quarantine"), "bad", "hidden");
    write_skill(&user, "visible", "visible");

    let mut catalog = SkillsCatalog::discover(DiscoverInput {
        project_root: None,
        project_trusted: false,
        user_dir: &user,
        bundled: &[],
    });
    catalog.set_disabled("visible", true);

    assert_eq!(catalog.skills.len(), 0);
    assert_eq!(catalog.listed()[0].name, "visible");
    assert!(catalog.view("visible").is_err());
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn index_is_deterministic_and_bounded() {
    let base = root("index");
    let user = base.join("user");
    for name in ["zeta", "alpha"] {
        write_skill(&user, name, "A useful skill description");
    }
    let catalog = SkillsCatalog::discover(DiscoverInput {
        project_root: None,
        project_trusted: false,
        user_dir: &user,
        bundled: &[],
    });
    let first = skills_index(&catalog, 6_000);
    assert_eq!(
        first,
        "- alpha: A useful skill description\n- zeta: A useful skill description"
    );
    assert_eq!(first, skills_index(&catalog, 6_000));
    assert!(skills_index(&catalog, 10).len() <= 10);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn one_hundred_skill_index_fits_the_budget() {
    let base = root("index-budget");
    let user = base.join("user");
    let small_user = base.join("small-user");
    let description = "x".repeat(160);
    for index in 0..100 {
        write_skill(&user, &format!("skill-{index:03}"), &description);
    }
    for index in 0..10 {
        write_skill(&small_user, &format!("skill-{index:03}"), &description);
    }
    let catalog = SkillsCatalog::discover(DiscoverInput {
        project_root: None,
        project_trusted: false,
        user_dir: &user,
        bundled: &[],
    });
    let large = skills_index(&catalog, 6_000);
    let small_catalog = SkillsCatalog::discover(DiscoverInput {
        project_root: None,
        project_trusted: false,
        user_dir: &small_user,
        bundled: &[],
    });
    let small = skills_index(&small_catalog, 6_000);
    assert_eq!(catalog.skills.len(), 100);
    assert_eq!(large.chars().count(), 1_199);
    assert_eq!(small.chars().count(), 1_739);
    fs::remove_dir_all(base).unwrap();
}
