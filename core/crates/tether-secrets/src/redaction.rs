//! Redacts known key shapes and values held in the secret vault before text leaves the process.

use crate::Secret;

const MASK: &str = "[REDACTED]";
const PREFIXES: &[&str] = &[
    "sk-ant-",
    "sk-proj-",
    "sk-",
    "AIza",
    "ghp_",
    "github_pat_",
    "gsk_",
];
const MIN_KEY_CHARS: usize = 8;

/// Holds copies of active secret values; copies are zeroized on drop.
#[derive(Default)]
pub struct Redactor {
    values: Vec<Secret>,
}

impl Redactor {
    /// Includes a value even when its format is not recognized.
    pub fn register(&mut self, secret: &Secret) {
        if secret.expose().len() >= MIN_KEY_CHARS {
            self.values.push(Secret::new(secret.expose().to_vec()));
        }
    }

    /// Replaces registered values and recognized key-shaped tokens.
    #[must_use]
    pub fn redact(&self, input: &str) -> String {
        let mut output = input.to_owned();
        for text in self
            .values
            .iter()
            .filter_map(|value| value.expose_text().ok())
        {
            output = output.replace(text, MASK);
        }
        redact_shapes(&output)
    }
}

fn redact_shapes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let tail = &input[index..];
        if let Some(end) = key_shape_length(tail) {
            output.push_str(MASK);
            index += end;
            continue;
        }
        let character = tail.chars().next().expect("index is inside input");
        output.push(character);
        index += character.len_utf8();
    }
    output
}

fn key_shape_length(tail: &str) -> Option<usize> {
    let prefix = PREFIXES.iter().find(|prefix| tail.starts_with(**prefix))?;
    let end = tail
        .char_indices()
        .take_while(|(_, character)| is_key_char(*character))
        .last()
        .map_or(0, |(offset, character)| offset + character.len_utf8());
    (end >= prefix.len() + MIN_KEY_CHARS).then_some(end)
}

fn is_key_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
}
