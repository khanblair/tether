//! Prompt index rendering for discovered skills.

use std::fmt::Write;

use crate::SkillsCatalog;

const TRUNCATION_MARKER: &str = "\n[truncated]";

fn cut_chars(source: &str, limit: usize) -> String {
    if source.chars().count() <= limit {
        return source.to_owned();
    }
    let keep = limit.saturating_sub(TRUNCATION_MARKER.chars().count());
    let prefix = source.chars().take(keep).collect::<String>();
    format!("{prefix}{TRUNCATION_MARKER}")
}

/// Renders a bounded, deterministic index of the enabled skills.
#[must_use]
pub fn skills_index(catalog: &SkillsCatalog, budget_chars: usize) -> String {
    if budget_chars == 0 {
        return String::new();
    }
    let lines = |description_limit: usize| {
        catalog
            .skills
            .iter()
            .map(|skill| {
                format!(
                    "- {}: {}",
                    skill.name,
                    cut_chars(&skill.description, description_limit)
                )
            })
            .collect::<Vec<_>>()
    };
    let full = lines(160).join("\n");
    if full.chars().count() <= budget_chars {
        return full;
    }
    let short = lines(80).join("\n");
    if short.chars().count() <= budget_chars {
        return short;
    }
    name_only_index(catalog, budget_chars)
}

fn name_only_index(catalog: &SkillsCatalog, budget_chars: usize) -> String {
    let mut output = String::new();
    let mut remaining = catalog.skills.len();
    for skill in &catalog.skills {
        let line = format!("- {}", skill.name);
        let suffix = if remaining > 1 {
            format!("\n+{} more: call skills_list", remaining - 1)
        } else {
            String::new()
        };
        if output.chars().count() + line.chars().count() + suffix.chars().count() > budget_chars {
            break;
        }
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&line);
        remaining -= 1;
    }
    if remaining > 0 {
        if !output.is_empty() {
            output.push('\n');
        }
        if write!(output, "+{remaining} more: call skills_list").is_err() {
            return output;
        }
    }
    output.chars().take(budget_chars).collect()
}
