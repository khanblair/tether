//! Built-in quality rules.

mod slop;
mod smell;
mod smell_text;
mod tests_rules;
pub(crate) mod util;

pub(crate) use slop::slop_rules;
pub(crate) use smell::smell_rules;
pub(crate) use tests_rules::test_rules;

pub(crate) fn is_known_rule_id(id: &str) -> bool {
    [
        "smell.function_length",
        "smell.nesting_depth",
        "smell.parameter_count",
        "smell.file_length",
        "smell.large_type",
        "smell.duplicate_block",
        "smell.dead_code",
        "smell.commented_out_code",
        "smell.magic_number",
        "smell.swallowed_error",
        "smell.debug_print",
        "smell.todo_without_issue",
        "slop.restating_comment",
        "slop.placeholder_body",
        "slop.unknown_import",
        "slop.broad_catch",
        "slop.unused_added_function",
        "slop.generic_name",
        "slop.banned_word_identifier",
        "slop.emoji_in_log",
        "slop.test_constant_assertion",
        "slop.test_without_assertion",
        "slop.test_removed_or_skipped",
        "slop.test_mocks_unit_under_test",
        "slop.test_snapshot_without_change",
    ]
    .contains(&id)
}
