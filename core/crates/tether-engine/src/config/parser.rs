use super::{ConfigIssue, ConfigValue, ProjectConfig, issue};

pub(super) fn parse_value(value: &toml::Value) -> (ProjectConfig, Vec<ConfigIssue>) {
    let mut config = ProjectConfig::default();
    let mut issues = Vec::new();
    let Some(table) = value.as_table() else {
        return (
            config,
            vec![issue("file", "top-level TOML value must be a table")],
        );
    };
    for key in table.keys() {
        if !matches!(
            key.as_str(),
            "executor" | "permissions" | "quality" | "model"
        ) {
            issues.push(issue(key, "unknown key ignored"));
        }
    }
    for key in ["executor", "permissions", "quality", "model"] {
        if table.get(key).is_some_and(|value| !value.is_table()) {
            issues.push(issue(key, "expected a table"));
        }
    }
    if let Some(table) = table.get("executor").and_then(toml::Value::as_table) {
        config.executor_set = table.contains_key("default");
        if let Some(value) = table.get("default") {
            match value.as_str().and_then(|text| text.parse().ok()) {
                Some(executor) => config.executor.default = executor,
                None => issues.push(issue("executor.default", "expected sandbox, linux, or ci")),
            }
        }
        unknown_nested(table, "executor", &["default"], &mut issues);
    }
    if let Some(table) = table.get("permissions").and_then(toml::Value::as_table) {
        config.permissions_set = table.contains_key("allowed_commands");
        if let Some(values) = table
            .get("allowed_commands")
            .and_then(toml::Value::as_array)
        {
            for value in values {
                if let Some(program) = value.as_str().filter(|program| valid_program(program)) {
                    config.permissions.allowed_commands.push(program.to_owned());
                } else {
                    issues.push(issue("permissions.allowed_commands", "program names only"));
                }
            }
        }
        unknown_nested(table, "permissions", &["allowed_commands"], &mut issues);
    }
    if let Some(table) = table.get("quality").and_then(toml::Value::as_table) {
        for (key, value) in table {
            if let Some(number) = value
                .as_float()
                .or_else(|| value.as_integer().map(|number| number as f64))
            {
                config
                    .quality
                    .insert(key.clone(), ConfigValue::Number(number));
            } else if let Some(text) = value.as_str() {
                config
                    .quality
                    .insert(key.clone(), ConfigValue::String(text.to_owned()));
            } else {
                issues.push(issue(
                    &format!("quality.{key}"),
                    "expected a number or string",
                ));
            }
        }
    }
    if let Some(table) = table.get("model").and_then(toml::Value::as_table) {
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
        unknown_nested(table, "model", &["provider", "id"], &mut issues);
    }
    (config, issues)
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
