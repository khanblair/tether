//! Short, redacted notification text for tool progress.

use serde_json::Value;

/// Summarizes a prepared tool call without exposing file contents.
#[must_use]
pub fn summarize_call(tool: &str, payload: &str) -> String {
    let value = serde_json::from_str::<Value>(payload).unwrap_or(Value::Null);
    let text = match tool {
        "edit" | "edit_file" => {
            let path = string(&value, "path").unwrap_or("unknown");
            let lines = value
                .get("new_text")
                .and_then(Value::as_str)
                .map_or(0, |value| value.lines().count());
            format!("edit {path}: {lines} lines changed")
        }
        "write" | "write_file" => {
            let path = string(&value, "path").unwrap_or("unknown");
            let lines = value
                .get("content")
                .and_then(Value::as_str)
                .map_or(0, |value| value.lines().count());
            format!("write {path}: {lines} lines")
        }
        "delete" | "delete_file" => {
            format!("delete {}", string(&value, "path").unwrap_or("unknown"))
        }
        "run" | "run_command" => format!("run: {}", string(&value, "command").unwrap_or("command")),
        _ => string(&value, "target").map_or_else(|| tool.to_owned(), ToOwned::to_owned),
    };
    text.chars().take(120).collect()
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use serde_json::to_string;

    use super::summarize_call;

    #[test]
    fn summaries_are_short_and_do_not_include_content() {
        assert_eq!(
            summarize_call("edit", r#"{"path":"src/a.rs","new_text":"a\nb\nc"}"#),
            "edit src/a.rs: 3 lines changed"
        );
        assert_eq!(
            summarize_call(
                "write",
                &format!(
                    r#"{{"path":"src/b.rs","content":{}}}"#,
                    to_string(&"x\n".repeat(120)).unwrap()
                )
            ),
            "write src/b.rs: 120 lines"
        );
        assert_eq!(summarize_call("delete", r#"{"path":"x"}"#), "delete x");
        assert_eq!(
            summarize_call("run", r#"{"command":"pytest -q"}"#),
            "run: pytest -q"
        );
        assert_eq!(summarize_call("other", r#"{"target":"remote"}"#), "remote");
    }
}
