//! Tree-sitter languages and language-specific rule data.

use std::path::Path;

use thiserror::Error;
use tree_sitter::{Language as TsLanguage, Parser, Tree};

/// The source languages understood by the quality gate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Language {
    /// Python source.
    Python,
    /// JavaScript source.
    JavaScript,
    /// TypeScript source.
    TypeScript,
    /// TSX source.
    Tsx,
    /// Rust source.
    Rust,
    /// Go source.
    Go,
    /// Kotlin source.
    Kotlin,
    /// Java source.
    Java,
    /// Swift source.
    Swift,
    /// C source.
    C,
    /// C++ source.
    Cpp,
    /// Ruby source.
    Ruby,
    /// PHP source.
    Php,
    /// HTML source.
    Html,
    /// CSS source.
    Css,
    /// JSON source.
    Json,
    /// YAML source.
    Yaml,
    /// TOML source.
    Toml,
    /// Markdown source.
    Markdown,
    /// Bash source.
    Bash,
}

/// Data used by rules without language-specific rule branches.
#[derive(Debug)]
pub struct LanguageProfile {
    /// Function and method node kinds.
    pub function_kinds: &'static [&'static str],
    /// Parameter list node kinds.
    pub parameter_kinds: &'static [&'static str],
    /// Nesting block node kinds.
    pub block_kinds: &'static [&'static str],
    /// Comment node kinds.
    pub comment_kinds: &'static [&'static str],
    /// String literal node kinds.
    pub string_kinds: &'static [&'static str],
    /// Numeric literal node kinds.
    pub number_kinds: &'static [&'static str],
    /// Catch or except node kinds.
    pub handler_kinds: &'static [&'static str],
    /// Call expression node kinds.
    pub call_kinds: &'static [&'static str],
    /// Test name and annotation markers.
    pub test_markers: &'static [&'static str],
    /// Import statement node kinds.
    pub import_kinds: &'static [&'static str],
    /// Standard package prefixes.
    pub standard_packages: &'static [&'static str],
    /// Calls that can be safely recognized as ignored-result calls.
    pub ignored_result_calls: &'static [&'static str],
}

static CODE_FUNCTIONS: &[&str] = &[
    "function_definition",
    "function_declaration",
    "method_definition",
    "function_item",
    "method_declaration",
    "function_definition",
    "function_declaration",
    "method",
    "method_definition",
    "singleton_method",
];
static CODE_PARAMS: &[&str] = &[
    "parameters",
    "formal_parameters",
    "parameter_list",
    "lambda_parameters",
    "function_parameters",
];
static CODE_BLOCKS: &[&str] = &[
    "if_statement",
    "if_expression",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "while_expression",
    "match_expression",
    "match_statement",
    "try_statement",
    "catch_clause",
    "closure_expression",
    "block",
    "compound_statement",
];
static COMMENTS: &[&str] = &["comment", "line_comment", "block_comment", "shebang"];
static STRINGS: &[&str] = &[
    "string",
    "string_literal",
    "interpreted_string_literal",
    "raw_string_literal",
    "template_string",
    "heredoc_body",
];
static NUMBERS: &[&str] = &[
    "integer",
    "integer_literal",
    "float",
    "float_literal",
    "number",
    "decimal_integer_literal",
];
static HANDLERS: &[&str] = &["except_clause", "catch_clause", "rescue", "handler"];
static CALLS: &[&str] = &[
    "call",
    "call_expression",
    "method_invocation",
    "function_call",
    "macro_invocation",
];
static TEST_MARKERS: &[&str] = &[
    "test_",
    "#[test]",
    "@Test",
    "func test",
    "TestXxx",
    "it(",
    "test(",
];
static IMPORTS: &[&str] = &[
    "import_statement",
    "import_declaration",
    "use_declaration",
    "include_statement",
    "require_statement",
];
static EMPTY: &[&str] = &[];
static COMMON_STD: &[&str] = &[
    "std",
    "core",
    "alloc",
    "math",
    "os",
    "sys",
    "json",
    "re",
    "pathlib",
    "typing",
    "collections",
    "fmt",
    "io",
    "strings",
    "net",
    "time",
    "java",
    "kotlin",
    "swift",
    "fs",
    "path",
    "http",
    "https",
    "url",
    "crypto",
    "util",
    "assert",
];
static IGNORED_RESULT: &[&str] = &["write", "send", "remove_file", "remove", "delete"];

/// Returns the profile data for a language.
#[must_use]
pub fn language_profile(language: Language) -> &'static LanguageProfile {
    static CODE: LanguageProfile = LanguageProfile {
        function_kinds: CODE_FUNCTIONS,
        parameter_kinds: CODE_PARAMS,
        block_kinds: CODE_BLOCKS,
        comment_kinds: COMMENTS,
        string_kinds: STRINGS,
        number_kinds: NUMBERS,
        handler_kinds: HANDLERS,
        call_kinds: CALLS,
        test_markers: TEST_MARKERS,
        import_kinds: IMPORTS,
        standard_packages: COMMON_STD,
        ignored_result_calls: IGNORED_RESULT,
    };
    static DATA: LanguageProfile = LanguageProfile {
        function_kinds: EMPTY,
        parameter_kinds: EMPTY,
        block_kinds: EMPTY,
        comment_kinds: COMMENTS,
        string_kinds: STRINGS,
        number_kinds: NUMBERS,
        handler_kinds: EMPTY,
        call_kinds: EMPTY,
        test_markers: EMPTY,
        import_kinds: EMPTY,
        standard_packages: EMPTY,
        ignored_result_calls: EMPTY,
    };
    match language {
        Language::Html
        | Language::Css
        | Language::Json
        | Language::Yaml
        | Language::Toml
        | Language::Markdown => &DATA,
        _ => &CODE,
    }
}

/// An error returned when a grammar cannot be installed in a parser.
#[derive(Debug, Error)]
pub enum QualityError {
    /// The selected grammar was rejected by tree-sitter.
    #[error("could not load the {language:?} grammar: {message}")]
    Language {
        /// The language whose grammar failed.
        language: Language,
        /// The grammar error text.
        message: String,
    },
    /// The parser could not be created.
    #[error("could not create a tree-sitter parser")]
    Parser,
}

impl Language {
    /// Infers a language from a path extension or well-known file name.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_ascii_lowercase();
        let extension = path.extension().and_then(|value| value.to_str());
        match (name.as_str(), extension) {
            ("makefile", _) | (_, Some("sh" | "bash" | "zsh")) => Some(Self::Bash),
            ("gemfile" | "rakefile", _) | (_, Some("rb")) => Some(Self::Ruby),
            (_, Some("py")) => Some(Self::Python),
            (_, Some("js" | "mjs" | "cjs" | "jsx")) => Some(Self::JavaScript),
            (_, Some("ts")) => Some(Self::TypeScript),
            (_, Some("tsx")) => Some(Self::Tsx),
            (_, Some("rs")) => Some(Self::Rust),
            (_, Some("go")) => Some(Self::Go),
            (_, Some("kt" | "kts")) => Some(Self::Kotlin),
            (_, Some("java")) => Some(Self::Java),
            (_, Some("swift")) => Some(Self::Swift),
            (_, Some("c" | "h")) => Some(Self::C),
            (_, Some("cc" | "cp" | "cpp" | "cxx" | "hh" | "hpp")) => Some(Self::Cpp),
            (_, Some("php")) => Some(Self::Php),
            (_, Some("html" | "htm")) => Some(Self::Html),
            (_, Some("css")) => Some(Self::Css),
            (_, Some("json")) => Some(Self::Json),
            (_, Some("yaml" | "yml")) => Some(Self::Yaml),
            (_, Some("toml")) => Some(Self::Toml),
            (_, Some("md" | "markdown")) => Some(Self::Markdown),
            _ => None,
        }
    }

    /// Returns the tree-sitter grammar for this language.
    #[must_use]
    pub fn ts_language(self) -> TsLanguage {
        match self {
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Swift => tree_sitter_swift::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Html => tree_sitter_html::LANGUAGE.into(),
            Self::Css => tree_sitter_css::LANGUAGE.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::Yaml => tree_sitter_yaml::LANGUAGE.into(),
            Self::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
            Self::Markdown => tree_sitter_md::LANGUAGE.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
        }
    }
}

/// Returns the tree-sitter grammar for a language.
#[must_use]
pub fn ts_language(language: Language) -> TsLanguage {
    language.ts_language()
}

/// A parsed source file and its original source text.
pub struct ParsedFile {
    /// The selected language.
    pub language: Language,
    /// The UTF-8 source text used to create the tree.
    pub source: String,
    /// The syntax tree.
    pub tree: Tree,
}

impl ParsedFile {
    /// Reports whether the tree contains any syntax error nodes.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.tree.root_node().has_error()
    }
}

/// Parses source text with the selected grammar.
///
/// # Errors
///
/// Returns an error when tree-sitter rejects the grammar.
pub fn parse(language: Language, source: &str) -> Result<ParsedFile, QualityError> {
    let mut parser = Parser::new();
    parser
        .set_language(&language.ts_language())
        .map_err(|error| QualityError::Language {
            language,
            message: error.to_string(),
        })?;
    let tree = parser.parse(source, None).ok_or(QualityError::Parser)?;
    Ok(ParsedFile {
        language,
        source: source.to_owned(),
        tree,
    })
}
