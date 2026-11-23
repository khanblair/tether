//! Tokenization for commands that do not use shell features.

use std::fmt;

/// An unquoted shell feature that WASI argv cannot represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellFeature {
    /// A pipe character.
    Pipe,
    /// A redirection operator.
    Redirect,
    /// The `&&` operator.
    And,
    /// The `||` operator.
    Or,
    /// A command separator.
    Semicolon,
    /// A background operator.
    Background,
    /// A subshell delimiter.
    Subshell,
    /// A glob pattern.
    Glob,
    /// Variable expansion syntax.
    Variable,
    /// An environment assignment before the command.
    EnvironmentAssignment,
    /// A here-document operator.
    HereDoc,
    /// An unterminated quote or escape.
    InvalidSyntax,
}

impl fmt::Display for ShellFeature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Pipe => "pipe",
            Self::Redirect => "redirect",
            Self::And => "&&",
            Self::Or => "||",
            Self::Semicolon => ";",
            Self::Background => "background",
            Self::Subshell => "subshell",
            Self::Glob => "glob",
            Self::Variable => "variable",
            Self::EnvironmentAssignment => "environment assignment prefix",
            Self::HereDoc => "here-doc",
            Self::InvalidSyntax => "invalid shell syntax",
        };
        formatter.write_str(name)
    }
}

/// Tokenizes simple POSIX-style words and rejects shell syntax.
///
/// # Errors
/// Returns the first unquoted shell feature or invalid quote syntax.
#[expect(
    clippy::excessive_nesting,
    clippy::too_many_lines,
    reason = "The tokenizer keeps quote and escape state in one small parser."
)]
pub fn tokenize(command: &str) -> Result<Vec<String>, ShellFeature> {
    if command.chars().count() > 2_000 || command.contains('\0') {
        return Err(ShellFeature::InvalidSyntax);
    }
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut first_word = true;
    let mut word_started = false;
    let chars: Vec<char> = command.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            word.push(ch);
            word_started = true;
            escaped = false;
            index += 1;
            continue;
        }
        if quote == Some('\'') {
            if ch == '\'' {
                quote = None;
            } else {
                word.push(ch);
            }
            word_started = true;
            index += 1;
            continue;
        }
        if quote == Some('"') {
            match ch {
                '"' => quote = None,
                '\\' if chars.get(index + 1).is_some() => {
                    index += 1;
                    word.push(chars[index]);
                    word_started = true;
                }
                _ => {
                    word.push(ch);
                    word_started = true;
                }
            }
            index += 1;
            continue;
        }
        match ch {
            '\'' | '"' => {
                quote = Some(ch);
                word_started = true;
            }
            '\\' => escaped = true,
            ' ' | '\t' | '\r' => {
                flush_word(&mut tokens, &mut word, &mut first_word, &mut word_started);
            }
            '\n' => return Err(ShellFeature::InvalidSyntax),
            '|' if chars.get(index + 1) == Some(&'|') => return Err(ShellFeature::Or),
            '|' => return Err(ShellFeature::Pipe),
            '&' if chars.get(index + 1) == Some(&'&') => return Err(ShellFeature::And),
            '&' => return Err(ShellFeature::Background),
            ';' => return Err(ShellFeature::Semicolon),
            '<' if chars.get(index + 1) == Some(&'<') => return Err(ShellFeature::HereDoc),
            '<' | '>' => return Err(ShellFeature::Redirect),
            '(' | ')' => return Err(ShellFeature::Subshell),
            '*' | '?' | '[' => return Err(ShellFeature::Glob),
            '$' => return Err(ShellFeature::Variable),
            _ => {
                word.push(ch);
                word_started = true;
            }
        }
        if first_word && word.contains('=') && word.split('=').next().is_some_and(valid_env_name) {
            return Err(ShellFeature::EnvironmentAssignment);
        }
        index += 1;
    }
    if escaped || quote.is_some() {
        return Err(ShellFeature::InvalidSyntax);
    }
    flush_word(&mut tokens, &mut word, &mut first_word, &mut word_started);
    Ok(tokens)
}

fn flush_word(
    tokens: &mut Vec<String>,
    word: &mut String,
    first_word: &mut bool,
    word_started: &mut bool,
) {
    if *word_started {
        tokens.push(std::mem::take(word));
        *first_word = false;
        *word_started = false;
    }
}

fn valid_env_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
