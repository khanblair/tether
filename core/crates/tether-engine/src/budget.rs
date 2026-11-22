//! Budget gates that pause a turn before a provider request.

use std::sync::{Arc, Mutex};

use tether_store::{SessionRecord, Store};
use tether_types::{BudgetScope, CoreError, CoreEvent, SessionStatus};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::EventSink;
use crate::usage::utc_day_from_ms;

#[derive(Default)]
struct GateState {
    paused: Option<BudgetScope>,
    continued_session: Option<u64>,
    continued_day: Option<u64>,
    warned_session: bool,
    warned_day: bool,
    continue_requested: bool,
}

/// Coordinates an app decision while a session waits at a budget limit.
pub(crate) struct BudgetGate {
    state: Mutex<GateState>,
    notify: Notify,
}

/// The session, event sink, and cancel token that one budget check serves.
#[derive(Clone, Copy)]
pub(crate) struct CheckTarget<'a> {
    pub(crate) session: &'a SessionRecord,
    pub(crate) sink: &'a dyn EventSink,
    pub(crate) cancel: &'a CancellationToken,
}

/// One budget scope with its spend, its limit, and any amount already allowed past it.
#[derive(Clone, Copy)]
struct ScopeSpend {
    scope: BudgetScope,
    spent: u64,
    limit: u64,
    saved_continued: Option<u64>,
}

impl BudgetGate {
    /// Creates a gate with no warning or continuation state.
    #[must_use]
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(GateState::default()),
            notify: Notify::new(),
        })
    }

    /// Allows the currently paused request to proceed.
    pub(crate) fn continue_past_budget(&self) {
        if let Ok(mut state) = self.state.lock()
            && state.paused.is_some()
        {
            state.continue_requested = true;
            self.notify.notify_waiters();
        }
    }

    /// Checks both session and daily limits before one provider request.
    ///
    /// # Errors
    /// Returns cancellation or storage errors. Reaching a limit waits until the
    /// gate is continued or the request is cancelled.
    pub(crate) async fn check(
        &self,
        store: &mut Store,
        target: CheckTarget<'_>,
        now_ms: i64,
    ) -> Result<(), CoreError> {
        let settings = store.budgets().map_err(|_| storage_error())?;
        let day = utc_day_from_ms(now_ms);
        let day_spent = store
            .usage_totals(&day)
            .map_err(|_| storage_error())?
            .cost_micros;
        let session_spent = store
            .session_usage(&target.session.id)
            .map_err(|_| storage_error())?
            .cost_micros;
        let override_limit = store
            .session_budget(&target.session.id)
            .map_err(|_| storage_error())?;
        let session_limit =
            override_limit.map_or(settings.session_limit_micros, |value| Some(value.0));
        if let Some(limit) = session_limit {
            let usage = ScopeSpend {
                scope: BudgetScope::Session,
                spent: session_spent,
                limit,
                saved_continued: override_limit.and_then(|value| value.1),
            };
            self.check_scope(store, target, usage).await?;
        }
        if let Some(limit) = settings.daily_limit_micros {
            let usage = ScopeSpend {
                scope: BudgetScope::Day,
                spent: day_spent,
                limit,
                saved_continued: None,
            };
            self.check_scope(store, target, usage).await?;
        }
        Ok(())
    }

    async fn check_scope(
        &self,
        store: &mut Store,
        target: CheckTarget<'_>,
        usage: ScopeSpend,
    ) -> Result<(), CoreError> {
        self.emit_warning_if_needed(target, usage)?;
        if !self.reached(usage) {
            return Ok(());
        }
        self.pause(store, target, usage)?;
        tokio::select! {
            () = target.cancel.cancelled() => Err(cancelled()),
            () = self.notify.notified() => {
                self.finish_continue(store, target, usage)
            },
        }
    }

    fn emit_warning_if_needed(
        &self,
        target: CheckTarget<'_>,
        usage: ScopeSpend,
    ) -> Result<(), CoreError> {
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        let warned = match usage.scope {
            BudgetScope::Session => &mut state.warned_session,
            BudgetScope::Day => &mut state.warned_day,
        };
        if !*warned && usage.spent.saturating_mul(100) >= usage.limit.saturating_mul(80) {
            *warned = true;
            target.sink.on_event(CoreEvent::BudgetWarning {
                session_id: target.session.id.clone(),
                scope: usage.scope,
                percent: percentage(usage.spent, usage.limit),
                spent_micros: usage.spent,
                limit_micros: usage.limit,
            });
        }
        Ok(())
    }

    fn pause(
        &self,
        store: &mut Store,
        target: CheckTarget<'_>,
        usage: ScopeSpend,
    ) -> Result<(), CoreError> {
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        state.paused = Some(usage.scope);
        drop(state);
        store
            .set_session_status(&target.session.id, SessionStatus::Paused, current_ms())
            .map_err(|_| storage_error())?;
        target.sink.on_event(CoreEvent::SessionUpdated {
            session_id: target.session.id.clone(),
            status: SessionStatus::Paused,
        });
        target.sink.on_event(CoreEvent::BudgetReached {
            session_id: target.session.id.clone(),
            scope: usage.scope,
            spent_micros: usage.spent,
            limit_micros: usage.limit,
        });
        Ok(())
    }

    fn finish_continue(
        &self,
        store: &Store,
        target: CheckTarget<'_>,
        usage: ScopeSpend,
    ) -> Result<(), CoreError> {
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        if !state.continue_requested {
            return Err(cancelled());
        }
        state.continue_requested = false;
        state.paused = None;
        match usage.scope {
            BudgetScope::Session => {
                state.continued_session = Some(usage.spent);
                store
                    .set_continued_past(&target.session.id, usage.spent)
                    .map_err(|_| storage_error())?;
            }
            BudgetScope::Day => state.continued_day = Some(usage.spent),
        }
        store
            .set_session_status(&target.session.id, SessionStatus::Requesting, current_ms())
            .map_err(|_| storage_error())?;
        target.sink.on_event(CoreEvent::SessionUpdated {
            session_id: target.session.id.clone(),
            status: SessionStatus::Requesting,
        });
        Ok(())
    }

    fn reached(&self, usage: ScopeSpend) -> bool {
        let continued = usage.saved_continued.or_else(|| {
            self.state.lock().ok().and_then(|state| match usage.scope {
                BudgetScope::Session => state.continued_session,
                BudgetScope::Day => state.continued_day,
            })
        });
        continued.map_or(usage.spent >= usage.limit, |anchor| {
            usage.spent >= anchor.saturating_add(usage.limit / 5)
        })
    }
}

fn percentage(spent: u64, limit: u64) -> u8 {
    spent
        .saturating_mul(100)
        .checked_div(limit)
        .unwrap_or(100)
        .min(100) as u8
}

fn current_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |value| {
            i64::try_from(value.as_millis()).unwrap_or(i64::MAX)
        })
}

fn storage_error() -> CoreError {
    CoreError::Internal {
        message: "could not save budget state".into(),
    }
}

fn cancelled() -> CoreError {
    CoreError::Cancelled {
        message: "budget pause interrupted".into(),
    }
}
