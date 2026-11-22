//! Background notification and foreground-service dispatch.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_engine::EventSink as EngineSink;
use tether_engine::notify::summarize_call;
use tether_types::{CoreEvent, SessionId, ToolCallId};

use crate::callbacks::{EventSink, NotifyPlatform, RunServicePlatform};
use crate::{SessionRuntime, SessionRuntimeError};

pub(crate) struct NotifyState {
    platform: Mutex<Option<Arc<dyn NotifyPlatform>>>,
    service: Mutex<Option<Arc<dyn RunServicePlatform>>>,
    foreground: AtomicBool,
    running: AtomicUsize,
    targets: Mutex<HashMap<ToolCallId, String>>,
    last_progress: Mutex<HashMap<SessionId, i64>>,
}

impl NotifyState {
    pub(crate) fn new() -> Self {
        Self {
            platform: Mutex::new(None),
            service: Mutex::new(None),
            foreground: AtomicBool::new(true),
            running: AtomicUsize::new(0),
            targets: Mutex::new(HashMap::new()),
            last_progress: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn set_platform(&self, platform: Arc<dyn NotifyPlatform>) {
        if let Ok(mut current) = self.platform.lock() {
            *current = Some(platform);
        }
    }

    pub(crate) fn set_service(&self, service: Arc<dyn RunServicePlatform>) {
        if let Ok(mut current) = self.service.lock() {
            *current = Some(service);
        }
    }

    pub(crate) fn set_foreground(&self, foreground: bool) {
        self.foreground.store(foreground, Ordering::Release);
    }

    fn platform(&self) -> Option<Arc<dyn NotifyPlatform>> {
        self.platform.lock().ok().and_then(|value| value.clone())
    }

    fn service(&self) -> Option<Arc<dyn RunServicePlatform>> {
        self.service.lock().ok().and_then(|value| value.clone())
    }

    fn event(&self, event: &CoreEvent) {
        let session_id = event_session(event);
        self.track_run_service(event);
        self.track_target(event);
        if self.foreground.load(Ordering::Acquire) {
            return;
        }
        let Some(platform) = self.platform() else {
            return;
        };
        self.dispatch(&platform, &session_id, event);
    }

    fn track_run_service(&self, event: &CoreEvent) {
        let CoreEvent::SessionUpdated { status, .. } = event else {
            return;
        };
        if matches!(status, tether_types::SessionStatus::Requesting) {
            self.note_run_started();
        } else if matches!(
            status,
            tether_types::SessionStatus::Idle
                | tether_types::SessionStatus::Failed
                | tether_types::SessionStatus::Interrupted
        ) {
            self.note_run_finished();
        }
    }

    fn note_run_started(&self) {
        if self.running.fetch_add(1, Ordering::AcqRel) == 0 {
            self.start_service();
        }
    }

    fn note_run_finished(&self) {
        if self.running.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.stop_service();
        }
    }

    fn start_service(&self) {
        if let Some(service) = self.service() {
            service.start_run_service();
        }
    }

    fn stop_service(&self) {
        if let Some(service) = self.service() {
            service.stop_run_service();
        }
    }

    fn track_target(&self, event: &CoreEvent) {
        if let CoreEvent::ToolProposed {
            tool_call_id,
            tool,
            target,
            ..
        } = event
            && let Ok(mut targets) = self.targets.lock()
        {
            targets.insert(tool_call_id.clone(), summarize_call(tool, target));
        }
    }

    fn dispatch(&self, platform: &Arc<dyn NotifyPlatform>, session_id: &str, event: &CoreEvent) {
        match event {
            CoreEvent::ApprovalRequired {
                tool_call_id,
                tool,
                target,
                ..
            } => platform.show_approval(
                session_id.to_owned(),
                tool_call_id.to_string(),
                tool.clone(),
                target.clone(),
                target.clone(),
            ),
            CoreEvent::QuestionAsked {
                tool_call_id,
                question,
                options,
                ..
            } => platform.show_question(
                session_id.to_owned(),
                tool_call_id.to_string(),
                question.clone(),
                options.clone(),
            ),
            CoreEvent::TurnFinished { .. } => platform.show_turn_finished(
                session_id.to_owned(),
                session_id.to_owned(),
                true,
                "Turn finished".into(),
            ),
            CoreEvent::TurnFailed { error, .. } => platform.show_turn_finished(
                session_id.to_owned(),
                session_id.to_owned(),
                false,
                error.to_string(),
            ),
            CoreEvent::ToolStarted { tool_call_id, .. } => {
                self.progress(platform, session_id, tool_call_id);
            }
            _ => {}
        }
    }

    fn progress(&self, platform: &Arc<dyn NotifyPlatform>, session_id: &str, id: &ToolCallId) {
        if !self.claim_progress_slot(session_id) {
            return;
        }
        let step = self
            .targets
            .lock()
            .ok()
            .and_then(|targets| targets.get(id).cloned())
            .unwrap_or_else(|| "Running tool".into());
        platform.update_run_progress(session_id.to_owned(), step);
    }

    /// Returns true when the session may send progress now, and records the send time.
    fn claim_progress_slot(&self, session_id: &str) -> bool {
        let now = now_ms();
        let Ok(mut last) = self.last_progress.lock() else {
            return false;
        };
        let key = SessionId::new(session_id);
        let previous = last.get(&key).copied().unwrap_or(0);
        if now.saturating_sub(previous) < 1_000 {
            return false;
        }
        last.insert(key, now);
        true
    }
}

pub(crate) struct SessionSink {
    pub(crate) sink: Arc<dyn EventSink>,
    pub(crate) state: Arc<NotifyState>,
}

impl EngineSink for SessionSink {
    fn on_event(&self, event: CoreEvent) {
        self.sink
            .on_event(crate::views::CoreEventView::from(event.clone()));
        self.state.event(&event);
    }
}

fn event_session(event: &CoreEvent) -> String {
    match event {
        CoreEvent::SessionUpdated { session_id, .. }
        | CoreEvent::SessionRewound { session_id }
        | CoreEvent::TextDelta { session_id, .. }
        | CoreEvent::ThinkingDelta { session_id, .. }
        | CoreEvent::ToolProposed { session_id, .. }
        | CoreEvent::ApprovalRequired { session_id, .. }
        | CoreEvent::ToolStarted { session_id, .. }
        | CoreEvent::ToolOutput { session_id, .. }
        | CoreEvent::ToolFinished { session_id, .. }
        | CoreEvent::TodoUpdated { session_id, .. }
        | CoreEvent::QuestionAsked { session_id, .. }
        | CoreEvent::PlanReady { session_id, .. }
        | CoreEvent::UsageUpdated { session_id, .. }
        | CoreEvent::TurnFinished { session_id }
        | CoreEvent::TurnInterrupted { session_id }
        | CoreEvent::TurnFailed { session_id, .. }
        | CoreEvent::NetworkPaused { session_id }
        | CoreEvent::NetworkResumed { session_id }
        | CoreEvent::BudgetWarning { session_id, .. }
        | CoreEvent::BudgetReached { session_id, .. }
        | CoreEvent::ContextCompacted { session_id, .. } => session_id.to_string(),
        CoreEvent::Ready { .. } | CoreEvent::Test { .. } | CoreEvent::ProviderStatus { .. } => {
            String::new()
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

#[uniffi::export]
impl SessionRuntime {
    /// Sets the callback used for background approval and turn notifications.
    ///
    /// # Errors
    /// Returns `Unavailable` when the callback cannot be stored.
    pub fn set_notify_platform(
        &self,
        platform: Arc<dyn NotifyPlatform>,
    ) -> Result<(), SessionRuntimeError> {
        self.notify.set_platform(platform);
        Ok(())
    }

    /// Sets the callback used for the foreground run service.
    ///
    /// # Errors
    /// Returns `Unavailable` when the callback cannot be stored.
    pub fn set_run_service_platform(
        &self,
        platform: Arc<dyn RunServicePlatform>,
    ) -> Result<(), SessionRuntimeError> {
        self.notify.set_service(platform);
        Ok(())
    }

    /// Records whether the app is visible to the user.
    pub fn set_app_foreground(&self, foreground: bool) {
        self.notify.set_foreground(foreground);
        if !foreground {
            return;
        }
        let Ok(runners) = self.runners.lock() else {
            return;
        };
        let Some(platform) = self.notify.platform() else {
            return;
        };
        for session_id in runners.keys() {
            platform.clear(session_id.to_string());
        }
    }
}
