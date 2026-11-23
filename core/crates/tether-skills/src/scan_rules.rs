//! Pattern checks that the skill scanner applies to each line and file.

pub(crate) fn reads_sensitive_path(line: &str) -> bool {
    line.contains("~/.ssh")
        || line.contains("~/.aws")
        || line.contains("browser profile")
        || line.contains("keychain")
}

pub(crate) fn sends_secret(line: &str) -> bool {
    let secret = ["secret", "key", "token", "credential", ".env", "ssh key"]
        .iter()
        .any(|word| line.contains(word));
    let destination = [
        "send",
        "upload",
        "post",
        "email",
        "exfiltrate",
        "forward",
        "curl",
        "wget",
    ]
    .iter()
    .any(|word| line.contains(word));
    secret && destination
}

pub(crate) fn overrides_instructions(line: &str) -> bool {
    [
        "ignore previous",
        "ignore system",
        "ignore developer",
        "override previous",
        "override system",
        "override developer",
    ]
    .iter()
    .any(|phrase| line.contains(phrase))
}

pub(crate) fn bypasses_controls(line: &str) -> bool {
    [
        "disable approval",
        "skip approval",
        "bypass approval",
        "disable permission",
        "bypass permission",
        "disable sandbox",
        "skip sandbox",
        "bypass sandbox",
    ]
    .iter()
    .any(|phrase| line.contains(phrase))
}

pub(crate) fn has_hidden_character(text: &str) -> bool {
    text.chars().any(|character| {
        matches!(
            character,
            '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{e0000}'..='\u{e007f}'
        )
    })
}

pub(crate) fn contains_url(text: &str) -> bool {
    text.contains("http://") || text.contains("https://")
}

pub(crate) fn is_script(path: &str) -> bool {
    [".sh", ".py", ".js", ".ts", ".rb"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
}

pub(crate) fn is_image(path: &str) -> bool {
    [".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
}
