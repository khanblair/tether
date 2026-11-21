//! Session checklist values.

use crate::text::text_enum;

/// The state of one checklist item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoStatus {
    /// The item has not started.
    Pending,
    /// The item is the current item.
    InProgress,
    /// The item is complete.
    Completed,
}

text_enum!(TodoStatus {
    Pending => "pending",
    InProgress => "in_progress",
    Completed => "completed",
});

/// One item in a session checklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoItem {
    /// The item text.
    pub text: String,
    /// The current item state.
    pub status: TodoStatus,
}
