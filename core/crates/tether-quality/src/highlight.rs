//! Tree-sitter syntax highlighting.

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::language::Language;

/// The fixed highlight classes exposed to the UI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighlightClass {
    /// A language keyword.
    Keyword,
    /// A string literal.
    String,
    /// A numeric literal.
    Number,
    /// A comment.
    Comment,
    /// A function name.
    Function,
    /// A type name.
    Type,
    /// A constant name.
    Constant,
    /// An object property.
    Property,
    /// An operator.
    Operator,
    /// Punctuation.
    Punctuation,
    /// A variable name.
    Variable,
    /// An HTML tag.
    Tag,
    /// An HTML or CSS attribute.
    Attribute,
}

/// One non-overlapping classified source span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HighlightSpan {
    /// Inclusive start byte.
    pub start_byte: usize,
    /// Exclusive end byte.
    pub end_byte: usize,
    /// UI highlight class.
    pub class: HighlightClass,
}

/// Highlights valid UTF-8 source with the grammar's bundled query.
#[must_use]
pub fn highlight(language: Language, source: &str) -> Vec<HighlightSpan> {
    if source.len() > 512 * 1024 {
        return Vec::new();
    }
    let Some(query) = highlights_query(language) else {
        return Vec::new();
    };
    let config = HighlightConfiguration::new(
        language.ts_language(),
        format!("{language:?}"),
        query,
        "",
        "",
    );
    let Ok(mut config) = config else {
        return Vec::new();
    };
    let names = [
        "keyword",
        "string",
        "number",
        "comment",
        "function",
        "type",
        "constant",
        "property",
        "operator",
        "punctuation",
        "variable",
        "tag",
        "attribute",
    ];
    config.configure(&names);
    let mut highlighter = Highlighter::new();
    let Ok(events) = highlighter.highlight(&config, source.as_bytes(), None, None, |_| None) else {
        return Vec::new();
    };
    collect_spans(events, &names)
}

fn collect_spans(
    events: impl Iterator<Item = Result<HighlightEvent, tree_sitter_highlight::Error>>,
    names: &[&str],
) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let mut stack = Vec::new();
    for event in events.flatten() {
        match event {
            HighlightEvent::HighlightStart(highlight) => stack.push(highlight.0),
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                if let Some(&index) = stack.last()
                    && let Some(class) =
                        class_for_name(names.get(index).copied().unwrap_or_default())
                    && start < end
                {
                    spans.push(HighlightSpan {
                        start_byte: start,
                        end_byte: end,
                        class,
                    });
                }
            }
        }
    }
    spans.sort_by_key(|span| (span.start_byte, span.end_byte));
    spans.dedup_by(|left, right| {
        left.start_byte == right.start_byte && left.end_byte == right.end_byte
    });
    spans
}

fn class_for_name(name: &str) -> Option<HighlightClass> {
    match name {
        "keyword" => Some(HighlightClass::Keyword),
        "string" => Some(HighlightClass::String),
        "number" => Some(HighlightClass::Number),
        "comment" => Some(HighlightClass::Comment),
        "function" => Some(HighlightClass::Function),
        "type" => Some(HighlightClass::Type),
        "constant" => Some(HighlightClass::Constant),
        "property" => Some(HighlightClass::Property),
        "operator" => Some(HighlightClass::Operator),
        "punctuation" => Some(HighlightClass::Punctuation),
        "variable" => Some(HighlightClass::Variable),
        "tag" => Some(HighlightClass::Tag),
        "attribute" => Some(HighlightClass::Attribute),
        _ => None,
    }
}

fn highlights_query(language: Language) -> Option<&'static str> {
    match language {
        Language::Python => Some(tree_sitter_python::HIGHLIGHTS_QUERY),
        Language::TypeScript | Language::Tsx => Some(tree_sitter_typescript::HIGHLIGHTS_QUERY),
        Language::Rust => Some(tree_sitter_rust::HIGHLIGHTS_QUERY),
        Language::Go => Some(tree_sitter_go::HIGHLIGHTS_QUERY),
        Language::Java => Some(tree_sitter_java::HIGHLIGHTS_QUERY),
        Language::Swift => Some(tree_sitter_swift::HIGHLIGHTS_QUERY),
        Language::Ruby => Some(tree_sitter_ruby::HIGHLIGHTS_QUERY),
        Language::Php => Some(tree_sitter_php::HIGHLIGHTS_QUERY),
        Language::Html => Some(tree_sitter_html::HIGHLIGHTS_QUERY),
        Language::Css => Some(tree_sitter_css::HIGHLIGHTS_QUERY),
        Language::Json => Some(tree_sitter_json::HIGHLIGHTS_QUERY),
        Language::Yaml => Some(tree_sitter_yaml::HIGHLIGHTS_QUERY),
        Language::Toml => Some(tree_sitter_toml_ng::HIGHLIGHTS_QUERY),
        Language::JavaScript
        | Language::Kotlin
        | Language::C
        | Language::Cpp
        | Language::Markdown
        | Language::Bash => None,
    }
}
