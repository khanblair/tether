//! Usage and budget runtime for the platform.

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_store::{BudgetSettings, Store, StoreError, usage_day_offset};
use tether_types::SessionId;

use crate::{
    BudgetView, DayUsageView, ModelUsageView, ProjectUsageView, SessionRuntime,
    SessionRuntimeError, UsageSummaryView,
};

/// A usage operation failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum UsageRuntimeError {
    /// The database or usage rows are unavailable.
    #[error("usage is unavailable")]
    Unavailable,
    /// The requested session does not exist.
    #[error("session not found")]
    NotFound,
    /// A budget is below one cent or above ten thousand dollars.
    #[error("budget is outside the supported range")]
    InvalidBudget,
}

/// Reads usage history and edits the application's budgets.
#[derive(uniffi::Object)]
pub struct UsageRuntime {
    database_path: String,
}

#[uniffi::export]
impl UsageRuntime {
    /// Opens the usage database.
    ///
    /// # Errors
    /// Returns an error when the database cannot be opened.
    #[uniffi::constructor]
    pub fn new(database_path: &str) -> Result<Arc<Self>, UsageRuntimeError> {
        Store::open(Path::new(database_path)).map_err(|_| UsageRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            database_path: database_path.into(),
        }))
    }

    /// Returns usage history for one display window.
    ///
    /// The offset selects the client's display day. Stored rows remain UTC, so this
    /// is an approximation around a local midnight.
    ///
    /// # Errors
    /// Returns an error when the database or usage rows are unavailable.
    pub fn usage_summary(
        &self,
        days: u32,
        utc_offset_minutes: i32,
    ) -> Result<UsageSummaryView, UsageRuntimeError> {
        let store = self.store()?;
        let today = display_today(utc_offset_minutes);
        let count = days.clamp(1, 90);
        let from = usage_day_offset(&today, -(count as i32 - 1)).map_err(map_store)?;
        let windows = store.usage_windows(&today).map_err(map_store)?;
        let rows = store.usage_by_day(&from, &today).map_err(map_store)?;
        let by_day = (0..count)
            .map(|index| {
                let day = usage_day_offset(&from, index as i32).map_err(map_store)?;
                Ok(rows
                    .iter()
                    .find(|row| row.day == day)
                    .cloned()
                    .map_or_else(|| zero_day(day), Into::into))
            })
            .collect::<Result<Vec<DayUsageView>, UsageRuntimeError>>()?;
        Ok(UsageSummaryView {
            today: windows.today.into(),
            last_7_days: windows.last_7_days.into(),
            last_30_days: windows.last_30_days.into(),
            by_day,
            by_project: store
                .usage_by_project(&from, &today)
                .map_err(map_store)?
                .into_iter()
                .map(ProjectUsageView::from)
                .collect(),
            by_model: store
                .usage_by_model(&from, &today)
                .map_err(map_store)?
                .into_iter()
                .map(ModelUsageView::from)
                .collect(),
        })
    }

    /// Reads the configured daily and default session budgets.
    ///
    /// # Errors
    /// Returns an error when the database cannot be read.
    pub fn budgets(&self) -> Result<BudgetView, UsageRuntimeError> {
        self.store()?.budgets().map(Into::into).map_err(map_store)
    }

    /// Saves the daily and default session budgets.
    ///
    /// # Errors
    /// Returns an error when a limit is invalid or storage fails.
    pub fn set_budgets(
        &self,
        daily_micros: Option<u64>,
        session_micros: Option<u64>,
    ) -> Result<(), UsageRuntimeError> {
        self.store()?
            .set_budgets(
                BudgetSettings {
                    daily_limit_micros: daily_micros,
                    session_limit_micros: session_micros,
                },
                now_ms(),
            )
            .map_err(map_store)
    }

    /// Sets or removes one session budget override.
    ///
    /// # Errors
    /// Returns an error when a limit is invalid, the session is absent, or storage fails.
    pub fn set_session_budget(
        &self,
        session_id: &str,
        limit_micros: Option<u64>,
    ) -> Result<(), UsageRuntimeError> {
        self.store()?
            .set_session_budget(&SessionId::new(session_id), limit_micros)
            .map_err(map_store)
    }
}

#[uniffi::export]
impl SessionRuntime {
    /// Continues a turn paused at a budget limit. It is a no-op if no runner is paused.
    ///
    /// The same limit pauses again only after spending another 20% of that limit.
    ///
    /// # Errors
    /// Returns an error when the session runner is unavailable.
    pub fn continue_past_budget(&self, session_id: &str) -> Result<(), SessionRuntimeError> {
        let id = SessionId::new(session_id);
        let runner = self
            .runners
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .get(&id)
            .cloned()
            .ok_or(SessionRuntimeError::NotFound)?;
        runner.handle.continue_past_budget();
        Ok(())
    }
}

impl UsageRuntime {
    fn store(&self) -> Result<Store, UsageRuntimeError> {
        Store::open(Path::new(&self.database_path)).map_err(map_store)
    }
}

fn zero_day(day: String) -> tether_store::DayUsage {
    tether_store::DayUsage {
        day,
        totals: tether_store::UsageTotals::default(),
    }
}

fn display_today(offset_minutes: i32) -> String {
    let now = now_ms().saturating_add(i64::from(offset_minutes).saturating_mul(60_000));
    let days = now.div_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = (if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    }) / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| {
            i64::try_from(value.as_millis()).unwrap_or(i64::MAX)
        })
}

fn map_store(error: StoreError) -> UsageRuntimeError {
    match error {
        StoreError::InvalidBudget => UsageRuntimeError::InvalidBudget,
        StoreError::MissingSessionRow => UsageRuntimeError::NotFound,
        _ => UsageRuntimeError::Unavailable,
    }
}
