//! Quality gate parsing, rules, and syntax highlighting.

#![expect(
    clippy::excessive_nesting,
    reason = "Rule heuristics mirror syntax-tree structure."
)]
#![expect(
    clippy::too_many_lines,
    reason = "Rule implementations stay grouped by rule family."
)]
#![expect(
    clippy::too_many_arguments,
    reason = "Finding construction keeps rule output explicit."
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "Source line indexes are bounded by u32 findings."
)]

mod highlight;
mod language;
mod manifest;
mod model;
mod registry;
mod rules;

pub use highlight::{HighlightClass, HighlightSpan, highlight};
pub use language::{
    Language, LanguageProfile, ParsedFile, QualityError, language_profile, parse, ts_language,
};
pub use manifest::manifest_from_files;
pub use model::{
    Batch, FileChange, Finding, ProjectManifest, Rule, RuleContext, Severity, Thresholds,
    changed_lines,
};
pub use registry::Registry;
