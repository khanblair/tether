//! Stable names for tools exposed by remote servers.

const MAX_NAME: usize = 64;
const PREFIX: &str = "mcp__";

/// Creates a sanitized, bounded name for an MCP tool.
#[must_use]
pub fn namespaced_tool_name(server_name: &str, tool_name: &str) -> String {
    let server = sanitize(server_name);
    let tool = sanitize(tool_name);
    let plain = format!("{PREFIX}{server}__{tool}");
    if plain.len() <= MAX_NAME {
        return plain;
    }
    let hash = stable_hash(server_name, tool_name);
    let budget = MAX_NAME - PREFIX.len() - 2 - hash.len() - 1;
    let (server, tool) = if server.len() >= tool.len() {
        let tool_budget = tool.len().min(budget / 2);
        (
            truncate(&server, budget - tool_budget),
            truncate(&tool, tool_budget),
        )
    } else {
        let server_budget = server.len().min(budget / 2);
        (
            truncate(&server, server_budget),
            truncate(&tool, budget - server_budget),
        )
    };
    format!("{PREFIX}{server}__{tool}_{hash}")
}

/// Parses a name created by [`namespaced_tool_name`].
#[must_use]
pub fn parse_namespaced_tool_name(name: &str) -> Option<(String, String)> {
    let rest = name.strip_prefix(PREFIX)?;
    let (server, tool) = rest.split_once("__")?;
    if server.is_empty() || tool.is_empty() || !valid_part(server) || !valid_part(tool) {
        return None;
    }
    Some((server.to_owned(), tool.to_owned()))
}

fn sanitize(value: &str) -> String {
    let result: String = value
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' | '_' | '-' => character,
            'A'..='Z' => character.to_ascii_lowercase(),
            _ => '_',
        })
        .collect();
    if result.is_empty() {
        "_".to_owned()
    } else {
        result
    }
}

fn valid_part(value: &str) -> bool {
    value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
    })
}

fn truncate(value: &str, budget: usize) -> String {
    value.chars().take(budget).collect()
}

fn stable_hash(server: &str, tool: &str) -> String {
    let mut hash = 2_166_136_261_u32;
    for byte in server.bytes().chain([0xff]).chain(tool.bytes()) {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    format!("{hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_and_parses_names() {
        let name = namespaced_tool_name("My Server", "Read.File");
        assert_eq!(name, "mcp__my_server__read_file");
        assert_eq!(
            parse_namespaced_tool_name(&name),
            Some(("my_server".to_owned(), "read_file".to_owned()))
        );
    }

    #[test]
    fn long_names_are_bounded_and_distinct() {
        let first = namespaced_tool_name(&"server".repeat(40), &"tool".repeat(40));
        let second = namespaced_tool_name(&"server".repeat(40), &format!("{}x", "tool".repeat(40)));
        assert!(first.len() <= MAX_NAME);
        assert_ne!(first, second);
    }
}
