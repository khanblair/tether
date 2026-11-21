use super::{branch_name_for_task, parse_github_url, should_clone_shallow, validate_branch_name};

#[test]
fn branch_slug_is_bounded_and_nonempty() {
    assert_eq!(
        branch_name_for_task("Fix: API / retries!"),
        "tether/fix-api-retries"
    );
    assert_eq!(branch_name_for_task("..."), "tether/work");
    assert!(branch_name_for_task(&"a".repeat(100)).len() <= 47);
}

#[test]
fn branch_ref_validation_rejects_git_metacharacters() {
    for name in [
        "",
        "-main",
        "main..next",
        "main~1",
        "main^1",
        "main:next",
        "main?",
        "main*",
        "main[",
        "main\\next",
        "main\0next",
        "main\nnext",
        "main@{next}",
        "main//next",
        "main.",
        "main.lock",
        "main/",
    ] {
        assert!(validate_branch_name(name).is_err(), "{name}");
    }
    assert!(validate_branch_name("feature/task-1").is_ok());
}

#[test]
fn github_url_parser_accepts_canonical_forms_only() {
    assert_eq!(
        parse_github_url("https://github.com/o/r"),
        Some(("o".into(), "r".into()))
    );
    assert_eq!(
        parse_github_url("https://github.com/o/r.git/"),
        Some(("o".into(), "r".into()))
    );
    assert!(parse_github_url("http://github.com/o/r").is_none());
    assert!(parse_github_url("https://github.com/o/r?x=1").is_none());
}

#[test]
fn shallow_clone_threshold_is_strictly_above_200_mib() {
    assert!(!should_clone_shallow(200 * 1024));
    assert!(should_clone_shallow(200 * 1024 + 1));
}
