use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tether_types::{CoreError, CoreEvent, SessionId, SessionStatus, ToolCallId, ToolCallStatus};

use super::{NotifyClock, NotifyState};
use crate::callbacks::{NotifyPlatform, RunServicePlatform};

#[derive(Default)]
struct Notifications(Mutex<Vec<String>>);

impl NotifyPlatform for Notifications {
    fn show_approval(
        &self,
        _session_id: String,
        _tool_call_id: String,
        _tool: String,
        _target: String,
        summary: String,
    ) {
        self.0.lock().unwrap().push(format!("approval:{summary}"));
    }

    fn show_question(
        &self,
        _session_id: String,
        _tool_call_id: String,
        question: String,
        _options: Vec<String>,
    ) {
        self.0.lock().unwrap().push(format!("question:{question}"));
    }

    fn show_turn_finished(&self, _session_id: String, _title: String, ok: bool, message: String) {
        self.0
            .lock()
            .unwrap()
            .push(format!("finished:{ok}:{message}"));
    }

    fn update_run_progress(&self, _session_id: String, step: String) {
        self.0.lock().unwrap().push(format!("progress:{step}"));
    }

    fn clear(&self, _session_id: String) {
        self.0.lock().unwrap().push("clear".into());
    }
}

#[derive(Default)]
struct Service {
    starts: AtomicUsize,
    stops: AtomicUsize,
}

impl RunServicePlatform for Service {
    fn start_run_service(&self) {
        self.starts.fetch_add(1, Ordering::SeqCst);
    }

    fn stop_run_service(&self) {
        self.stops.fetch_add(1, Ordering::SeqCst);
    }
}

struct Clock(AtomicI64);

impl NotifyClock for Clock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn session(value: &str) -> SessionId {
    SessionId::new(value)
}

#[test]
fn foreground_events_send_no_background_notifications() {
    let platform = Arc::new(Notifications::default());
    let state = NotifyState::new();
    state.set_platform(platform.clone());
    state.event(&CoreEvent::ApprovalRequired {
        session_id: session("s1"),
        tool_call_id: ToolCallId::new("call-1"),
        tool: "run".into(),
        target: "pytest -q".into(),
    });

    assert!(platform.0.lock().unwrap().is_empty());
}

#[test]
fn background_events_are_sent_once_with_redacted_summary_text() {
    let platform = Arc::new(Notifications::default());
    let state = NotifyState::with_clock(Arc::new(Clock(AtomicI64::new(1_000))));
    state.set_platform(platform.clone());
    state.set_foreground(false);
    let id = session("s1");
    let call = ToolCallId::new("call-1");
    state.event(&CoreEvent::ToolProposed {
        session_id: id.clone(),
        tool_call_id: call.clone(),
        tool: "run".into(),
        target: "pytest -q".into(),
    });
    state.event(&CoreEvent::ApprovalRequired {
        session_id: id.clone(),
        tool_call_id: call,
        tool: "run".into(),
        target: "pytest -q".into(),
    });
    state.event(&CoreEvent::QuestionAsked {
        session_id: id.clone(),
        tool_call_id: ToolCallId::new("question-1"),
        question: "Choose".into(),
        options: vec!["yes".into()],
        allow_free_text: false,
    });
    state.event(&CoreEvent::TurnFinished {
        session_id: id.clone(),
    });
    state.event(&CoreEvent::TurnFailed {
        session_id: id,
        error: CoreError::Network {
            message: "offline".into(),
        },
    });

    assert_eq!(
        *platform.0.lock().unwrap(),
        vec![
            "approval:run: pytest -q",
            "question:Choose",
            "finished:true:Turn finished",
            "finished:false:network error: offline",
        ]
    );
}

#[test]
fn progress_is_throttled_per_session() {
    let platform = Arc::new(Notifications::default());
    let clock = Arc::new(Clock(AtomicI64::new(1_000)));
    let state = NotifyState::with_clock(clock.clone());
    state.set_platform(platform.clone());
    state.set_foreground(false);
    let id = session("s1");
    let call = ToolCallId::new("call-1");
    state.event(&CoreEvent::ToolProposed {
        session_id: id.clone(),
        tool_call_id: call.clone(),
        tool: "run".into(),
        target: "pytest".into(),
    });
    for _ in 0..2 {
        state.event(&CoreEvent::ToolStarted {
            session_id: id.clone(),
            tool_call_id: call.clone(),
        });
    }
    clock.0.store(2_000, Ordering::SeqCst);
    state.event(&CoreEvent::ToolStarted {
        session_id: id,
        tool_call_id: call,
    });

    assert_eq!(
        *platform.0.lock().unwrap(),
        vec!["progress:run: pytest", "progress:run: pytest"]
    );
}

#[test]
fn overlapping_sessions_start_and_stop_the_service_once() {
    let service = Arc::new(Service::default());
    let state = NotifyState::new();
    state.set_service(service.clone());
    for id in [session("s1"), session("s2")] {
        state.event(&CoreEvent::SessionUpdated {
            session_id: id,
            status: SessionStatus::Requesting,
        });
    }
    state.event(&CoreEvent::SessionUpdated {
        session_id: session("s1"),
        status: SessionStatus::Idle,
    });
    state.event(&CoreEvent::SessionUpdated {
        session_id: session("s2"),
        status: SessionStatus::Failed,
    });
    state.event(&CoreEvent::SessionUpdated {
        session_id: session("s2"),
        status: SessionStatus::Failed,
    });

    assert_eq!(service.starts.load(Ordering::SeqCst), 1);
    assert_eq!(service.stops.load(Ordering::SeqCst), 1);
}

#[test]
fn tool_finish_does_not_expose_a_stale_progress_action() {
    let platform = Arc::new(Notifications::default());
    let clock = Arc::new(Clock(AtomicI64::new(1_000)));
    let state = NotifyState::with_clock(clock.clone());
    state.set_platform(platform.clone());
    state.set_foreground(false);
    let id = session("s1");
    let call = ToolCallId::new("call-1");
    state.event(&CoreEvent::ToolProposed {
        session_id: id.clone(),
        tool_call_id: call.clone(),
        tool: "run".into(),
        target: "pytest".into(),
    });
    state.event(&CoreEvent::ToolStarted {
        session_id: id.clone(),
        tool_call_id: call.clone(),
    });
    state.event(&CoreEvent::ToolFinished {
        session_id: id.clone(),
        tool_call_id: call.clone(),
        status: ToolCallStatus::Succeeded,
    });
    clock.0.store(2_000, Ordering::SeqCst);
    state.event(&CoreEvent::ToolStarted {
        session_id: id,
        tool_call_id: call,
    });

    assert_eq!(
        *platform.0.lock().unwrap(),
        vec!["progress:run: pytest", "progress:Running tool"]
    );
}
