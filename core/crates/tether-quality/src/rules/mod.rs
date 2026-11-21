//! Built-in quality rules.

mod slop;
mod smell;
mod smell_text;
mod tests_rules;
pub(crate) mod util;

pub(crate) use slop::slop_rules;
pub(crate) use smell::smell_rules;
pub(crate) use tests_rules::test_rules;
