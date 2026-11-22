use super::{ConfigIssue, ConfigValue, ProjectConfig, issue};

const SECTIONS: [&str; 4] = ["executor", "permissions", "quality", "model"];

pub(super) fn parse_value(value: &toml::Value) -> (ProjectConfig, Vec<ConfigIssue>) {
    let mut config = ProjectConfig::default();
    let mut issues = Vec::new();
    let Some(table) = value.as_table() else {
        return (
            config,
            vec![issue("file", "top-level TOML value must be a table")],
        );
    };
    check_top_level(table, &mut issues);
    parse_executor(table, &mut config, &mut issues);
    parse_permissions(table, &mut config, &mut issues);
    parse_quality(table, &mut config, &mut issues);
    parse_model(table, &mut config, &mut issues);
    (config, issues)
}

fn check_top_level(table: &toml::Table, issues: &mut Vec<ConfigIssue>) {
    for key in table.keys() {
        if !SECTIONS.contains(&key.as_str()) {
            issues.push(issue(key, "unknown key ignored"));
        }
    }
    for key in SECTIONS {
        if table.get(key).is_some_and(|value| !value.is_table()) {
            issues.push(issue(key, "expected a table"));
        }
    }
}

fn parse_executor(table: &toml::Table, config: &mut ProjectConfig, issues: &mut Vec<ConfigIssue>) {
    let Some(table) = table.get("executor").and_then(toml::Value::as_table) else {
        return;
    };
    config.executor_set = table.contains_key("default");
    if let Some(value) = table.get("default") {
        match value.as_str().and_then(|text| text.parse().ok()) {
            Some(executor) => config.executor.default = executor,
            None => issues.push(issue("executor.default", "expected sandbox, linux, or ci")),
        }
    }
    unknown_nested(table, "executor", &["default"], issues);
}

fn parse_permissions(
    table: &toml::Table,
    config: &mut ProjectConfig,
    issues: &mut Vec<ConfigIssue>,
) {
    let Some(table) = table.get("permissions").and_then(toml::Value::as_table) else {
        return;
    };
    config.permissions_set = table.contains_key("allowed_commands");
    let values = table
        .get("allowed_commands")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten();
    for value in values {
        match value.as_str().filter(|program| valid_program(program)) {
            Some(program) => config.permissions.allowed_commands.push(program.to_owned()),
            None => issues.push(issue("permissions.allowed_commands", "program names only")),
        }
    }
    unknown_nested(table, "permissions", &["allowed_commands"], issues);
}

fn parse_quality(table: &toml::Table, config: &mut ProjectConfig, issues: &mut Vec<ConfigIssue>) {
    let Some(table) = table.get("quality").and_then(toml::Value::as_table) else {
        return;
    };
    for (key, value) in table {
        if let Some(entry) = quality_value(value) {
            config.quality.insert(key.clone(), entry);
        } else {
            issues.push(issue(
                &format!("quality.{key}"),
                "expected a number or string",
            ));
        }
    }
}

fn quality_value(value: &toml::Value) -> Option<ConfigValue> {
    value
        .as_float()
        .or_else(|| value.as_integer().and_then(integer_as_f64))
        .map(ConfigValue::Number)
        .or_else(|| {
            value
                .as_str()
                .map(|text| ConfigValue::String(text.to_owned()))
        })
}

/// Converts an integer to the nearest `f64` without a lossy `as` cast.
///
/// Decimal text parses to the nearest `f64`, which is the same value `as`
/// would give, so every integer the config accepted before is still accepted.
fn integer_as_f64(number: i64) -> Option<f64> {
    number.to_string().parse().ok()
}

fn parse_model(table: &toml::Table, config: &mut ProjectConfig, issues: &mut Vec<ConfigIssue>) {
    let Some(table) = table.get("model").and_then(toml::Value::as_table) else {
        return;
    };
    config.model_set = true;
    config.model.provider = table
        .get("provider")
        .and_then(toml::Value::as_str)
        .and_then(|text| text.parse().ok());
    config.model.id = table
        .get("id")
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    if table.get("provider").is_some() && config.model.provider.is_none() {
        issues.push(issue("model.provider", "unknown provider"));
    }
    if config.model.provider.is_some() != config.model.id.is_some() {
        issues.push(issue("model", "provider and id must be set together"));
    }
    unknown_nested(table, "model", &["provider", "id"], issues);
}

fn unknown_nested(
    table: &toml::map::Map<String, toml::Value>,
    prefix: &str,
    known: &[&str],
    issues: &mut Vec<ConfigIssue>,
) {
    for key in table.keys().filter(|key| !known.contains(&key.as_str())) {
        issues.push(issue(&format!("{prefix}.{key}"), "unknown key ignored"));
    }
}

fn valid_program(program: &str) -> bool {
    !program.is_empty()
        && program
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}
