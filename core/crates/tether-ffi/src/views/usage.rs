//! FFI views for usage history and budgets.

use tether_store::{BudgetSettings, DayUsage, ModelUsage, ProjectUsage, UsageTotals};

use super::ProviderView;

/// Token and cost totals for one usage window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct UsageTotalsView {
    /// Input tokens.
    pub tokens_in: u64,
    /// Output tokens.
    pub tokens_out: u64,
    /// Cost in USD micros.
    pub cost_micros: u64,
}

/// Usage for one UTC day.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DayUsageView {
    /// UTC date in `YYYY-MM-DD` form.
    pub day: String,
    /// Totals for the day.
    pub tokens_in: u64,
    /// Totals for the day.
    pub tokens_out: u64,
    /// Cost for the day.
    pub cost_micros: u64,
}

/// Usage retained for one project.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProjectUsageView {
    /// Project id.
    pub project_id: String,
    /// Current project name, or empty after deletion.
    pub project_name: String,
    /// Input tokens.
    pub tokens_in: u64,
    /// Output tokens.
    pub tokens_out: u64,
    /// Cost in USD micros.
    pub cost_micros: u64,
}

/// Usage grouped by provider and model.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ModelUsageView {
    /// Provider.
    pub provider: ProviderView,
    /// Provider model id.
    pub model_id: String,
    /// Input tokens.
    pub tokens_in: u64,
    /// Output tokens.
    pub tokens_out: u64,
    /// Cost in USD micros.
    pub cost_micros: u64,
}

/// A daily, session, and display-window usage summary.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UsageSummaryView {
    /// Totals for the selected display day.
    pub today: UsageTotalsView,
    /// Totals for the selected display day and six preceding days.
    pub last_7_days: UsageTotalsView,
    /// Totals for the selected display day and 29 preceding days.
    pub last_30_days: UsageTotalsView,
    /// Zero-filled daily rows, oldest first.
    pub by_day: Vec<DayUsageView>,
    /// Project totals.
    pub by_project: Vec<ProjectUsageView>,
    /// Provider/model totals.
    pub by_model: Vec<ModelUsageView>,
}

/// Configured daily and default session budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct BudgetView {
    /// Daily limit in USD micros, or no limit.
    pub daily_limit_micros: Option<u64>,
    /// Default session limit in USD micros, or no limit.
    pub session_limit_micros: Option<u64>,
}

impl From<UsageTotals> for UsageTotalsView {
    fn from(value: UsageTotals) -> Self {
        Self {
            tokens_in: value.tokens_in,
            tokens_out: value.tokens_out,
            cost_micros: value.cost_micros,
        }
    }
}

impl From<DayUsage> for DayUsageView {
    fn from(value: DayUsage) -> Self {
        Self {
            day: value.day,
            ..value.totals.into()
        }
    }
}

impl From<ProjectUsage> for ProjectUsageView {
    fn from(value: ProjectUsage) -> Self {
        Self {
            project_id: value.project_id,
            project_name: value.project_name,
            ..value.totals.into()
        }
    }
}

impl From<ModelUsage> for ModelUsageView {
    fn from(value: ModelUsage) -> Self {
        Self {
            provider: value.provider.into(),
            model_id: value.model_id,
            ..value.totals.into()
        }
    }
}

impl From<BudgetSettings> for BudgetView {
    fn from(value: BudgetSettings) -> Self {
        Self {
            daily_limit_micros: value.daily_limit_micros,
            session_limit_micros: value.session_limit_micros,
        }
    }
}
