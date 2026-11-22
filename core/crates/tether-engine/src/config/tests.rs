use super::*;

#[test]
fn parses_known_values_and_reports_unknown_keys() {
    let (config, issues) = parse("[executor]\ndefault = \"linux\"\n[quality]\nmax = 3\n");
    assert_eq!(config.executor.default, Executor::Linux);
    assert_eq!(config.quality["max"], ConfigValue::Number(3.0));
    assert_eq!(issues, Vec::new());
    let (_, issues) = parse("[extra]\nvalue = true\n");
    assert_eq!(issues[0].key, "extra");
}

#[test]
fn malformed_toml_has_line_and_column_and_defaults() {
    let (config, issues) = parse("[executor\ndefault = \"linux\"");
    assert_eq!(config, ProjectConfig::default());
    assert!(issues[0].line.is_some());
    assert!(issues[0].column.is_some());
}

#[test]
fn merge_tracks_the_highest_precedence_origin() {
    let global = parse("[executor]\ndefault=\"linux\"\n").0;
    let project = parse("[executor]\ndefault=\"ci\"\n").0;
    let session = SessionConfig {
        executor_default: Some(Executor::Sandbox),
    };
    let merged = effective_config(&global, &project, &session);
    assert_eq!(merged.executor_default, Executor::Sandbox);
    assert_eq!(merged.executor_origin, ConfigOrigin::Session);
}

#[test]
fn an_explicit_sandbox_project_value_overrides_global_linux() {
    let global = parse("[executor]\ndefault=\"linux\"\n").0;
    let project = parse("[executor]\ndefault=\"sandbox\"\n").0;
    let merged = effective_config(&global, &project, &SessionConfig::default());
    assert_eq!(merged.executor_default, Executor::Sandbox);
    assert_eq!(merged.executor_origin, ConfigOrigin::Project);
}

#[test]
fn quality_values_merge_by_name_and_keep_their_origins() {
    let global = parse("[quality]\nlimit = 10\nstyle = \"warn\"\n").0;
    let project = parse("[quality]\nlimit = 5\n").0;
    let merged = effective_config(&global, &project, &SessionConfig::default());
    assert_eq!(merged.quality["limit"], ConfigValue::Number(5.0));
    assert_eq!(merged.quality["style"], ConfigValue::String("warn".into()));
    assert_eq!(merged.quality_origins["limit"], ConfigOrigin::Project);
    assert_eq!(merged.quality_origins["style"], ConfigOrigin::Global);
}

#[test]
fn allowed_commands_make_only_sandbox_and_linux_rules() {
    let config = EffectiveConfig {
        allowed_commands: vec!["pytest".into()],
        ..EffectiveConfig::default()
    };
    let rules = allowed_command_rules(&config);
    assert_eq!(rules.len(), 2);
    assert!(
        rules
            .iter()
            .all(|rule| rule.pattern == "pytest" && rule.tool == "run")
    );
    assert!(!rules.iter().any(|rule| rule.executor == Some(Executor::Ci)));
}

#[test]
fn untrusted_project_applies_quality_and_only_sandbox_executor() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(directory.path().join(".tether")).unwrap();
    std::fs::write(
        directory.path().join(".tether/config.toml"),
        "[executor]\ndefault = \"linux\"\n[permissions]\nallowed_commands = [\"pytest\"]\n[quality]\nlimit = 5\n[model]\nprovider = \"openai\"\nid = \"x\"\n",
    )
    .unwrap();

    let (untrusted, issues) = load_project(directory.path(), false);
    assert_eq!(untrusted.executor.default, Executor::Sandbox);
    assert_eq!(untrusted.permissions.allowed_commands, Vec::<String>::new());
    assert_eq!(untrusted.model, ModelConfig::default());
    assert_eq!(untrusted.quality["limit"], ConfigValue::Number(5.0));
    assert_eq!(issues.len(), 3);

    let (trusted, issues) = load_project(directory.path(), true);
    assert_eq!(trusted.executor.default, Executor::Linux);
    assert_eq!(trusted.permissions.allowed_commands, vec!["pytest"]);
    assert!(trusted.model.id.is_some());
    assert_eq!(issues, Vec::new());
}
