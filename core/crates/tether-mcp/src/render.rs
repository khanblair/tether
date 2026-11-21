//! MCP result rendering.

use serde_json::Value;

const MAX_RESULT_BYTES: usize = 1_048_576;

pub(crate) fn render_result(result: &Value) -> (String, bool) {
    let mut parts = Vec::new();
    if let Some(content) = result.get("content").and_then(Value::as_array) {
        for part in content {
            render_part(part, &mut parts);
        }
    }
    if parts.is_empty()
        && let Some(structured) = result.get("structuredContent")
    {
        parts.push(serde_json::to_string(structured).unwrap_or_else(|_| "{}".to_owned()));
    }
    bound_text(&parts.join("\n"))
}

fn render_part(part: &Value, output: &mut Vec<String>) {
    let Some(object) = part.as_object() else {
        return;
    };
    match object.get("type").and_then(Value::as_str) {
        Some("text") => {
            if let Some(text) = object.get("text").and_then(Value::as_str) {
                output.push(text.to_owned());
            }
        }
        Some("image" | "audio") => {
            let kind = object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("media");
            let mime = object
                .get("mimeType")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let size = object
                .get("data")
                .and_then(Value::as_str)
                .map_or(0, str::len);
            output.push(format!("[{kind}: {mime}, {size} bytes]"));
        }
        Some("resource_link") => {
            let name = object
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("resource");
            let uri = object.get("uri").and_then(Value::as_str).unwrap_or("");
            output.push(format!("{name} ({uri})"));
        }
        Some("resource") => {
            if let Some(text) = object
                .get("resource")
                .and_then(|value| value.get("text"))
                .and_then(Value::as_str)
            {
                output.push(text.to_owned());
            }
        }
        _ => {}
    }
}

fn bound_text(text: &str) -> (String, bool) {
    if text.len() <= MAX_RESULT_BYTES {
        return (text.to_owned(), false);
    }
    let marker = "\n[truncated]";
    let mut end = MAX_RESULT_BYTES.saturating_sub(marker.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (format!("{}{}", &text[..end], marker), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_tool_parts_and_structured_content() {
        let result = serde_json::json!({"content":[{"type":"text","text":"hello"},{"type":"image","mimeType":"image/png","data":"abcd"},{"type":"resource_link","name":"doc","uri":"urn:doc"},{"type":"resource","resource":{"text":"embedded"}}]});
        let (text, truncated) = render_result(&result);
        assert_eq!(
            text,
            "hello\n[image: image/png, 4 bytes]\ndoc (urn:doc)\nembedded"
        );
        assert!(!truncated);
        let (text, _) = render_result(&serde_json::json!({"structuredContent":{"ok":true}}));
        assert_eq!(text, r#"{"ok":true}"#);
    }
}
