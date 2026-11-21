use tether_types::{Executor, NewRule, PermissionMode, ProjectId, ToolCallId};

use crate::{ApprovalRecord, Store};

fn projects() -> (Store, ProjectId, ProjectId) {
    let mut store = Store::open_in_memory().unwrap();
    let first = store.create_project("First", 1).unwrap();
    let second = store.create_project("Second", 2).unwrap();
    (store, first.id, second.id)
}

#[test]
fn permission_rule_add_is_idempotent_and_delete_is_project_scoped() {
    let (store, first, second) = projects();
    let rule = NewRule {
        tool: "write_file".into(),
        pattern: "src/main.rs".into(),
        executor: None,
    };
    let saved = store.add_permission_rule(&first, &rule, 3).unwrap();
    let repeated = store.add_permission_rule(&first, &rule, 4).unwrap();
    assert_eq!(repeated, saved);
    assert_eq!(store.permission_rules(&first).unwrap(), vec![saved.clone()]);
    assert!(!store.delete_permission_rule(&second, &saved.id).unwrap());
    assert!(store.delete_permission_rule(&first, &saved.id).unwrap());
    assert_eq!(store.permission_rules(&first).unwrap(), Vec::new());
}

#[test]
fn deleting_a_project_removes_its_permission_rules() {
    let (mut store, first, _second) = projects();
    store
        .add_permission_rule(
            &first,
            &NewRule {
                tool: "run".into(),
                pattern: "pytest".into(),
                executor: Some(Executor::Sandbox),
            },
            3,
        )
        .unwrap();
    store.delete_project(&first).unwrap();
    assert_eq!(store.permission_rules(&first).unwrap(), Vec::new());
}

#[test]
fn approval_answers_round_trip_for_a_session() {
    let (mut store, project, _second) = projects();
    let session = store
        .create_session(
            &project,
            &tether_types::ModelRef {
                provider: tether_types::Provider::OpenAi,
                model_id: "m".into(),
            },
            PermissionMode::Ask,
            5,
        )
        .unwrap();
    store
        .record_approval(&ApprovalRecord {
            id: "a1".into(),
            session_id: session.id.clone(),
            tool_call_id: ToolCallId::new("call"),
            tool: "write_file".into(),
            target: "note.txt".into(),
            decision: "deny".into(),
            reason: Some("no".into()),
            decided_at: 6,
            via: "app".into(),
        })
        .unwrap();
    let rows = store.list_approvals(&session.id).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].session_id, session.id);
    assert_eq!(rows[0].tool_call_id, ToolCallId::new("call"));
    assert_eq!(rows[0].decision, "deny");
    assert_eq!(rows[0].reason.as_deref(), Some("no"));
}
