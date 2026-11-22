//! Project manifest package extraction.

use toml::Value;

use crate::model::ProjectManifest;

/// Reads supported manifest files through a caller-owned file reader.
#[must_use]
pub fn manifest_from_files(read: &dyn Fn(&str) -> Option<String>) -> ProjectManifest {
    let mut result = ProjectManifest::default();
    read_toml_manifests(read, &mut result);
    read_json_manifests(read, &mut result);
    read_line_manifests(read, &mut result);
    read_other_manifests(read, &mut result);
    result
}

fn read_toml_manifests(read: &dyn Fn(&str) -> Option<String>, result: &mut ProjectManifest) {
    for path in ["Cargo.toml", "pyproject.toml"] {
        if let Some(text) = read(path) {
            if let Ok(value) = text.parse::<Value>() {
                add_toml_packages(&value, result);
            }
            add_dependency_lines(&text, result);
        }
    }
    for path in ["build.gradle", "build.gradle.kts"] {
        if let Some(text) = read(path) {
            add_colon_packages(&text, result);
        }
    }
}

fn add_dependency_lines(text: &str, result: &mut ProjectManifest) {
    let mut active = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            active = trimmed.contains("dependencies");
            continue;
        }
        if active {
            let name = trimmed.split('=').next().unwrap_or_default().trim();
            if !name.is_empty() && !name.starts_with('#') {
                result
                    .declared_packages
                    .insert(normalize(name.trim_matches('"')));
            }
        }
    }
}

fn add_toml_packages(value: &Value, result: &mut ProjectManifest) {
    for key in [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "workspace.dependencies",
        "project.dependencies",
        "optional-dependencies",
        "poetry.dependencies",
    ] {
        let mut current = value;
        for part in key.split('.') {
            let Some(next) = current.get(part) else {
                current = &Value::Boolean(false);
                break;
            };
            current = next;
        }
        if key == "optional-dependencies" {
            if let Some(table) = current.as_table() {
                for values in table.values().filter_map(Value::as_array) {
                    add_requirement_values(values, result);
                }
            }
        } else if let Some(table) = current.as_table() {
            result
                .declared_packages
                .extend(table.keys().map(|key| normalize(key)));
        } else if let Some(array) = current.as_array() {
            for item in array {
                if let Some(name) = item
                    .as_str()
                    .and_then(|text| text.split(['=', '>', '<']).next())
                {
                    result.declared_packages.insert(normalize(name.trim()));
                }
            }
        }
    }
}

fn add_requirement_values(values: &[Value], result: &mut ProjectManifest) {
    for value in values {
        if let Some(name) = value
            .as_str()
            .and_then(|text| text.split(['=', '>', '<', '[', ';']).next())
            && !name.trim().is_empty()
        {
            result.declared_packages.insert(normalize(name));
        }
    }
}

fn read_json_manifests(read: &dyn Fn(&str) -> Option<String>, result: &mut ProjectManifest) {
    if let Some(text) = read("package.json") {
        if let Ok(value) = text.parse::<toml::Value>() {
            add_toml_packages(&value, result);
        } else {
            add_json_keys(&text, result);
        }
    }
    if let Some(text) = read("composer.json") {
        add_json_keys(&text, result);
    }
}

fn add_json_keys(text: &str, result: &mut ProjectManifest) {
    let sections = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
        "require",
        "require-dev",
    ];
    for section in sections {
        let marker = format!("\"{section}\"");
        let Some(start) = text.find(&marker) else {
            continue;
        };
        let tail = &text[start + marker.len()..];
        let Some(open) = tail.find('{') else { continue };
        let Some(close) = tail[open + 1..].find('}') else {
            continue;
        };
        let body = &tail[open + 1..open + 1 + close];
        for piece in body.split(',') {
            if let Some(name) = piece.split(':').next() {
                let name = name.trim().trim_matches('"');
                if !name.is_empty() {
                    result.declared_packages.insert(normalize(name));
                }
            }
        }
    }
}

fn read_line_manifests(read: &dyn Fn(&str) -> Option<String>, result: &mut ProjectManifest) {
    for path in [
        "requirements.txt",
        "requirements-dev.txt",
        "requirements-test.txt",
    ] {
        if let Some(text) = read(path) {
            for line in text.lines() {
                let name = line
                    .split(['=', '>', '<', '[', ';'])
                    .next()
                    .unwrap_or_default()
                    .trim();
                if !name.is_empty() && !name.starts_with('#') {
                    result.declared_packages.insert(normalize(name));
                }
            }
        }
    }
    if let Some(text) = read("go.mod") {
        let mut in_require = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed == "require (" {
                in_require = true;
                continue;
            }
            if in_require && trimmed == ")" {
                in_require = false;
                continue;
            }
            if let Some(name) = trimmed.strip_prefix("module ") {
                result.declared_packages.insert(normalize(name));
            }
            if let Some(name) = trimmed.strip_prefix("require ") {
                result.declared_packages.insert(normalize(
                    name.split_whitespace().next().unwrap_or_default(),
                ));
            } else if in_require && !trimmed.is_empty() && !trimmed.starts_with("//") {
                result.declared_packages.insert(normalize(
                    trimmed.split_whitespace().next().unwrap_or_default(),
                ));
            }
        }
    }
}

fn read_other_manifests(read: &dyn Fn(&str) -> Option<String>, result: &mut ProjectManifest) {
    for path in ["Gemfile", "Package.swift"] {
        if let Some(text) = read(path) {
            for line in text.lines() {
                if path == "Gemfile" && line.contains("gem ") {
                    add_quoted(line, result);
                }
                if path == "Package.swift" && line.contains(".package(") {
                    add_quoted(line, result);
                }
            }
        }
    }
    if let Some(text) = read("composer.json")
        && text.contains("autoload")
    {
        result.language_hints.insert("php".to_owned());
    }
}

fn add_colon_packages(text: &str, result: &mut ProjectManifest) {
    for line in text.lines() {
        if line.contains(':') && (line.contains("implementation") || line.contains("api ")) {
            let value = line.split(':').nth(1).unwrap_or_default().trim();
            let package = value
                .split(['"', '\'', ' '])
                .find(|part| part.contains('.'));
            if let Some(package) = package {
                let parts: Vec<_> = package.split(':').take(2).collect();
                result
                    .declared_packages
                    .extend(parts.into_iter().map(normalize));
            }
        }
    }
}

fn add_quoted(line: &str, result: &mut ProjectManifest) {
    if let Some(start) = line.find(['"', '\'']) {
        let quote = line.as_bytes()[start] as char;
        if let Some(end) = line[start + 1..].find(quote) {
            result
                .declared_packages
                .insert(normalize(&line[start + 1..start + 1 + end]));
        }
    }
}

fn normalize(value: &str) -> String {
    value.trim().replace('-', "_")
}
