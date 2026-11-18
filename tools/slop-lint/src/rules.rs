//! Rule data and the checks that run over one text.

use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

const WORDS: &str = include_str!("../../../core/crates/tether-quality/data/slop_words.txt");
const PATTERNS: &str = include_str!("../../../core/crates/tether-quality/data/slop_patterns.toml");

const OFF: &str = "<!-- slop-lint:off -->";
const ON: &str = "<!-- slop-lint:on -->";

/// One finding: where it is and what to do about it.
#[derive(Debug, PartialEq, Eq)]
pub struct Finding {
    pub line: usize,
    pub rule: String,
    pub advice: String,
}

#[derive(Deserialize)]
struct PatternFile {
    pattern: Vec<PatternEntry>,
}

#[derive(Deserialize)]
struct PatternEntry {
    name: String,
    regex: String,
    advice: String,
}

/// Where a line sits in its file.
#[derive(Clone, Copy)]
struct LineContext {
    number: usize,
    in_fence: bool,
    is_xml: bool,
}

struct Rule {
    name: String,
    regex: Regex,
    advice: String,
}

/// The compiled rule set.
pub struct Rules {
    rules: Vec<Rule>,
}

impl Rules {
    /// Builds the rule set from the shared data files.
    pub fn load() -> Self {
        let mut rules = word_rules();
        let file: PatternFile = toml::from_str(PATTERNS).expect("slop_patterns.toml parses");
        for entry in file.pattern {
            rules.push(Rule {
                regex: compile(&format!("(?i){}", entry.regex)),
                name: entry.name,
                advice: entry.advice,
            });
        }
        Self { rules }
    }

    /// Checks `text`. `is_xml` limits the exclamation mark rule to element text.
    pub fn check(&self, text: &str, is_xml: bool) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (number, line, in_fence) in checked_lines(text) {
            let at = LineContext {
                number,
                in_fence,
                is_xml,
            };
            self.check_line(&line, at, &mut findings);
        }
        findings
    }

    fn check_line(&self, line: &str, at: LineContext, out: &mut Vec<Finding>) {
        for rule in self.rules.iter().filter(|r| r.regex.is_match(line)) {
            out.push(finding(at.number, &rule.name, &rule.advice));
        }
        if line.contains('\u{2014}') {
            out.push(finding(at.number, "em-dash", "Use two sentences."));
        }
        if !at.in_fence && has_exclamation(line, at.is_xml) {
            let advice = "No exclamation marks in docs or UI copy.";
            out.push(finding(at.number, "exclamation-mark", advice));
        }
    }
}

/// Lines outside exempt blocks, as (line number, text, inside a code fence).
fn checked_lines(text: &str) -> Vec<(usize, String, bool)> {
    let mut state = ScanState::default();
    text.lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            let line = raw.replace('\u{2019}', "'");
            state
                .visit(&line)
                .map(|in_fence| (index + 1, line, in_fence))
        })
        .collect()
}

/// Tracks exempt blocks and code fences while a text is scanned line by line.
#[derive(Default)]
struct ScanState {
    off: bool,
    in_fence: bool,
}

impl ScanState {
    /// Updates the state for `line`. Returns the fence state when the line is checked,
    /// or `None` when the line sits in an exempt block.
    fn visit(&mut self, line: &str) -> Option<bool> {
        if line.contains(OFF) {
            self.off = true;
        }
        let checked = !self.off;
        if checked && line.trim_start().starts_with("```") {
            self.in_fence = !self.in_fence;
        }
        if line.contains(ON) {
            self.off = false;
        }
        checked.then_some(self.in_fence)
    }
}

fn finding(line: usize, rule: &str, advice: &str) -> Finding {
    Finding {
        line,
        rule: rule.to_owned(),
        advice: advice.to_owned(),
    }
}

fn compile(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| panic!("bad pattern {pattern}: {e}"))
}

fn word_rules() -> Vec<Rule> {
    WORDS
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|word| {
            let escaped = regex::escape(word);
            let pattern = if word.contains(' ') {
                format!("(?i){escaped}")
            } else {
                let stem = word.strip_suffix('e').map(regex::escape);
                let ing = stem.map(|st| format!("|{st}ing")).unwrap_or_default();
                format!(r"(?i)\b(?:{escaped}(?:s|es|d|ed|ing|ly)?{ing})\b")
            };
            Rule {
                name: format!("banned-word:{word}"),
                regex: compile(&pattern),
                advice: "Use a plainer word. See docs/project-overview.md writing rules."
                    .to_owned(),
            }
        })
        .collect()
}

fn has_exclamation(line: &str, is_xml: bool) -> bool {
    let prose = if is_xml {
        xml_text(line)
    } else {
        strip_code(line)
    };
    prose.replace("![", "").contains('!')
}

fn xml_text(line: &str) -> String {
    static ELEMENT: LazyLock<Regex> = LazyLock::new(|| compile(r">([^<]*)<"));
    ELEMENT
        .captures_iter(line)
        .map(|c| c[1].to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_code(line: &str) -> String {
    static CODE: LazyLock<Regex> = LazyLock::new(|| compile(r"`[^`]*`|<!--.*?-->"));
    CODE.replace_all(line, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules_of(text: &str) -> Vec<String> {
        Rules::load()
            .check(text, false)
            .into_iter()
            .map(|f| f.rule)
            .collect()
    }

    #[test]
    fn flags_banned_word_and_inflection() {
        let mut found = rules_of("We leverage a robust core.");
        found.sort();
        assert_eq!(found, ["banned-word:leverage", "banned-word:robust"]);
        assert_eq!(
            rules_of("It is leveraging nothing."),
            ["banned-word:leverage"]
        );
    }

    #[test]
    fn plain_text_passes() {
        assert!(rules_of("The core runs the turn loop.").is_empty());
    }

    #[test]
    fn off_block_is_exempt() {
        let text = "<!-- slop-lint:off -->\nrobust\n<!-- slop-lint:on -->\nrobust";
        assert_eq!(Rules::load().check(text, false).len(), 1);
    }

    #[test]
    fn em_dash_and_exclamation_are_flagged_outside_code() {
        assert!(rules_of("One \u{2014} two").contains(&"em-dash".to_owned()));
        assert!(rules_of("Done!").contains(&"exclamation-mark".to_owned()));
        assert!(rules_of("Use `!=` here").is_empty());
        assert!(rules_of("![logo](a.png)").is_empty());
        assert!(rules_of("<!-- a comment -->").is_empty());
    }

    #[test]
    fn pattern_rules_match() {
        assert!(
            rules_of("It is not just an app, but a workspace.")
                .contains(&"not-just-x-but-y".to_owned())
        );
    }
}
