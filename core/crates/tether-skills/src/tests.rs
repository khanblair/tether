use crate::{SkillError, parse_skill};

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
    assert_eq!(parse_skill("test", missing), Err(SkillError::MissingField("description")));
    let long = "a".repeat(1025);
    let source = format!("---\nname: test\ndescription: {long}\n---\nBody");
    assert_eq!(parse_skill("test", &source), Err(SkillError::DescriptionTooLong));
}
