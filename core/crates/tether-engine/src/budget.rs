//! Budget gates that pause a turn before a provider request.

use std::sync::{Arc, Mutex};

use tether_store::{SessionRecord, Store};
use tether_types::{BudgetScope, CoreError, CoreEvent, SessionStatus};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::EventSink;
use crate::usage::utc_day_from_ms;

#[derive(Clone, Copy)]
struct PausedBudget {
    scope: BudgetScope,
    spent_micros: u64,
    limit_micros: u64,
}

#[derive(Default)]
struct GateState {
    paused: Option<PausedBudget>,
    continued_session: Option<u64>,
    continued_day: Option<u64>,
    warned_session: bool,
    warned_day: bool,
    continue_requested: bool,
}

/// Coordinates an app decision while a session waits at a budget limit.
pub struct BudgetGate {
    state: Mutex<GateState>,
    notify: Notify,
}

impl BudgetGate {
    /// Creates a gate with no warning or continuation state.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(GateState::default()),
            notify: Notify::new(),
        })
    }

    /// Allows the currently paused request to proceed.
    pub fn continue_past_budget(&self) {
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
    pub async fn check(
        &self,
        store: &mut Store,
        session: &SessionRecord,
        sink: &dyn EventSink,
        cancel: &CancellationToken,
        now_ms: i64,
    ) -> Result<(), CoreError> {
        let settings = store.budgets().map_err(|_| storage_error())?;
        let day = utc_day_from_ms(now_ms);
        let day_spent = store
            .usage_totals(&day)
            .map_err(|_| storage_error())?
            .cost_micros;
        let session_spent = store
            .session_usage(&session.id)
            .map_err(|_| storage_error())?
            .cost_micros;
        let override_limit = store
            .session_budget(&session.id)
            .map_err(|_| storage_error())?;
        let session_limit =
            override_limit.map_or(settings.session_limit_micros, |value| Some(value.0));
        if let Some(limit) = session_limit {
            self.check_scope(
                store,
                session,
                sink,
                cancel,
                BudgetScope::Session,
                session_spent,
                limit,
                override_limit.and_then(|value| value.1),
            )
            .await?;
        }
        if let Some(limit) = settings.daily_limit_micros {
            self.check_scope(
                store,
                session,
                sink,
                cancel,
                BudgetScope::Day,
                day_spent,
                limit,
                None,
            )
            .await?;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the gate passes the complete scope and pause context together"
    )]
    async fn check_scope(
        &self,
        store: &mut Store,
        session: &SessionRecord,
        sink: &dyn EventSink,
        cancel: &CancellationToken,
        scope: BudgetScope,
        spent: u64,
        limit: u64,
        saved_continued: Option<u64>,
    ) -> Result<(), CoreError> {
        self.emit_warning_if_needed(sink, session, scope, spent, limit)?;
        if !self.reached(scope, spent, limit, saved_continued) {
            return Ok(());
        }
        self.pause(store, session, sink, scope, spent, limit)?;
        tokio::select! {
            () = cancel.cancelled() => Err(cancelled()),
            () = self.notify.notified() => {
                self.finish_continue(store, session, sink, scope, spent)
            },
        }
    }

    fn emit_warning_if_needed(
        &self,
        sink: &dyn EventSink,
        session: &SessionRecord,
        scope: BudgetScope,
        spent: u64,
        limit: u64,
    ) -> Result<(), CoreError> {
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        let warned = match scope {
            BudgetScope::Session => &mut state.warned_session,
            BudgetScope::Day => &mut state.warned_day,
        };
        if !*warned && spent.saturating_mul(100) >= limit.saturating_mul(80) {
            *warned = true;
            sink.on_event(CoreEvent::BudgetWarning {
                session_id: session.id.clone(),
                scope,
                percent: percentage(spent, limit),
                spent_micros: spent,
                limit_micros: limit,
            });
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "pausing needs the saved session, event sink, and exact limit"
    )]
    fn pause(
        &self,
        store: &mut Store,
        session: &SessionRecord,
        sink: &dyn EventSink,
        scope: BudgetScope,
        spent: u64,
        limit: u64,
    ) -> Result<(), CoreError> {
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        state.paused = Some(PausedBudget {
            scope,
            spent_micros: spent,
            limit_micros: limit,
        });
        drop(state);
        store
            .set_session_status(&session.id, SessionStatus::Paused, current_ms())
            .map_err(|_| storage_error())?;
        sink.on_event(CoreEvent::SessionUpdated {
            session_id: session.id.clone(),
            status: SessionStatus::Paused,
        });
        sink.on_event(CoreEvent::BudgetReached {
            session_id: session.id.clone(),
            scope,
            spent_micros: spent,
            limit_micros: limit,
        });
        Ok(())
    }

    fn finish_continue(
        &self,
        store: &Store,
        session: &SessionRecord,
        sink: &dyn EventSink,
        scope: BudgetScope,
        spent: u64,
    ) -> Result<(), CoreError> {
        let mut state = self.state.lock().map_err(|_| storage_error())?;
        if !state.continue_requested {
            return Err(cancelled());
        }
        state.continue_requested = false;
        state.paused = None;
        match scope {
            BudgetScope::Session => {
                state.continued_session = Some(spent);
                store
                    .set_continued_past(&session.id, spent)
                    .map_err(|_| storage_error())?;
            }
            BudgetScope::Day => state.continued_day = Some(spent),
        }
        store
            .set_session_status(&session.id, SessionStatus::Requesting, current_ms())
            .map_err(|_| storage_error())?;
        sink.on_event(CoreEvent::SessionUpdated {
            session_id: session.id.clone(),
            status: SessionStatus::Requesting,
        });
        Ok(())
    }

    fn reached(&self, scope: BudgetScope, spent: u64, limit: u64, saved: Option<u64>) -> bool {
        let continued = saved.or_else(|| {
            self.state.lock().ok().and_then(|state| match scope {
                BudgetScope::Session => state.continued_session,
                BudgetScope::Day => state.continued_day,
            })
        });
        continued.map_or(spent >= limit, |anchor| {
            spent >= anchor.saturating_add(limit / 5)
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
