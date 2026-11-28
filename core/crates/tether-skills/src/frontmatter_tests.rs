use crate::{SkillError, parse_skill};

fn parse(name: &str, frontmatter: &str) -> crate::SkillDocument {
    parse_skill(name, frontmatter).expect("fixture should parse")
}

#[test]
fn parses_real_block_scalar_shapes() {
    let caveman = parse(
        "caveman",
        "---\nname: caveman\ndescription: >\n  Ultra-compressed communication mode that cuts output tokens while keeping\n  technical accuracy. Levels: lite, full, ultra and the wenyan variants.\n  Use for /caveman, \"caveman mode\", \"talk like caveman\", \"be brief\" or \"less tokens\".\n---\nBody",
    );
    assert_eq!(
        caveman.description,
        "Ultra-compressed communication mode that cuts output tokens while keeping technical accuracy. Levels: lite, full, ultra and the wenyan variants. Use for /caveman, \"caveman mode\", \"talk like caveman\", \"be brief\" or \"less tokens\"."
    );

    let folded = parse(
        "folded",
        "---\nname: folded\ndescription: >-\n  First line.\n  Second line.\n---\nBody",
    );
    assert_eq!(folded.description, "First line. Second line.");

    let literal = parse(
        "literal",
        "---\nname: literal\ndescription: |\n  First line.\n  Second line.\n---\nBody",
    );
    assert_eq!(literal.description, "First line.\nSecond line.");

    let literal_strip = parse(
        "literal-strip",
        "---\nname: literal-strip\ndescription: |-\n  First line.\n  Second line.\n---\nBody",
    );
    assert_eq!(literal_strip.description, "First line.\nSecond line.");
}

#[test]
fn folded_blocks_keep_blank_and_more_indented_lines() {
    let blank = parse(
        "blank",
        "---\nname: blank\ndescription: >\n  first\n  second\n\n  third\n---\nBody",
    );
    assert_eq!(blank.description, "first second\nthird");

    let indented = parse(
        "indented",
        "---\nname: indented\ndescription: >\n  first\n    more indented\n  third\n---\nBody",
    );
    assert_eq!(indented.description, "first\n  more indented\nthird");
}

#[test]
fn block_ends_before_later_top_level_fields() {
    let skill = parse(
        "later-fields",
        "---\nname: later-fields\ndescription: >-\n  A description.\nlicense: MIT\nallowed-tools: Bash, Read\n---\nBody",
    );
    assert_eq!(skill.description, "A description.");
    assert_eq!(skill.license.as_deref(), Some("MIT"));
    assert_eq!(skill.allowed_tools.as_deref(), Some("Bash, Read"));
}

#[test]
fn plain_continuations_fold_and_quoted_values_keep_escaped_quotes() {
    let skill = parse(
        "continued",
        r#"---
name: continued
description: first line
  second line
license: "MIT \"license\""
---
Body"#,
    );
    assert_eq!(skill.description, "first line second line");
    assert_eq!(skill.license.as_deref(), Some(r#"MIT \"license\""#));
}

#[test]
fn unknown_nested_blocks_are_skipped() {
    let skill = parse(
        "nested",
        "---\nname: nested\ndescription: A nested skill.\nlicense: MIT\nhooks:\n  pre:\n    - Read\nallowed-tools:\n  - Bash\ncompatibility: android\n---\nBody",
    );
    assert_eq!(skill.license.as_deref(), Some("MIT"));
    assert_eq!(skill.compatibility.as_deref(), Some("android"));
    assert_eq!(skill.allowed_tools.as_deref(), Some(""));
}

#[test]
fn metadata_block_scalar_is_stored_in_metadata() {
    let skill = parse(
        "metadata-block",
        "---\nname: metadata-block\ndescription: Skill.\nmetadata:\n  summary: >-\n    First line.\n    Second line.\n---\nBody",
    );
    assert_eq!(skill.metadata["summary"], "First line. Second line.");
}

#[test]
fn rejects_malformed_or_invalid_block_frontmatter() {
    let no_colon = "---\nname: malformed\ndescription: Fine.\nbad line\n---\nBody";
    assert_eq!(
        parse_skill("malformed", no_colon),
        Err(SkillError::InvalidFrontmatter)
    );

    let missing_close = "---\nname: missing-close\ndescription: Fine.";
    assert_eq!(
        parse_skill("missing-close", missing_close),
        Err(SkillError::MissingFrontmatter)
    );

    let wrong_name = "---\nname: other\ndescription: Fine.\n---\nBody";
    assert_eq!(
        parse_skill("expected", wrong_name),
        Err(SkillError::InvalidName)
    );

    let long_description = "a".repeat(1_025);
    let long_source = format!("---\nname: long\ndescription: |-\n  {long_description}\n---\nBody");
    assert_eq!(
        parse_skill("long", &long_source),
        Err(SkillError::DescriptionTooLong)
    );

    let empty_description = "---\nname: empty\ndescription: >-\n---\nBody";
    assert_eq!(
        parse_skill("empty", empty_description),
        Err(SkillError::MissingField("description"))
    );
}

#[test]
fn parses_short_versions_of_real_skill_frontmatter() {
    let cases = [
        (
            "caveman",
            "---\nname: caveman\ndescription: >\n  Ultra-compressed communication mode.\n  Use for /caveman.\n---\nBody",
            "Ultra-compressed communication mode. Use for /caveman.",
        ),
        (
            "computer-use",
            "---\nname: computer-use\ndescription: >-\n  Operate the browser.\n  Use when a task needs a visible app.\n---\nBody",
            "Operate the browser. Use when a task needs a visible app.",
        ),
        (
            "tool-skill",
            "---\nname: tool-skill\ndescription: Use the tool.\nallowed-tools: Bash, Read\nargument-hint: [path]\n---\nBody",
            "Use the tool.",
        ),
        (
            "no-invoke",
            "---\nname: no-invoke\ndescription: Do not invoke automatically.\ndisable-model-invocation: true\n---\nBody",
            "Do not invoke automatically.",
        ),
        (
            "quoted",
            "---\nname: quoted\ndescription: \"A quoted description.\"\n---\nBody",
            "A quoted description.",
        ),
    ];

    for (name, source, expected) in cases {
        assert_eq!(parse(name, source).description, expected);
    }
}
