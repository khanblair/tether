//! Checklist values exposed to the platform.

use tether_types::{TodoItem, TodoStatus};

/// The FFI view of a checklist item status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TodoStatusView {
    /// Not started.
    Pending,
    /// Current item.
    InProgress,
    /// Finished.
    Completed,
}

impl From<TodoStatus> for TodoStatusView {
    fn from(status: TodoStatus) -> Self {
        match status {
            TodoStatus::Pending => Self::Pending,
            TodoStatus::InProgress => Self::InProgress,
            TodoStatus::Completed => Self::Completed,
        }
    }
}

/// The FFI view of one checklist item.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TodoItemView {
    /// Item text.
    pub text: String,
    /// Item status.
    pub status: TodoStatusView,
}

impl From<TodoItem> for TodoItemView {
    fn from(item: TodoItem) -> Self {
        Self {
            text: item.text,
            status: item.status.into(),
        }
    }
}
