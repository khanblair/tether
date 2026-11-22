use tether_types::{MessagePart, ModelRef, PermissionMode, Provider, Role, ToolCallId};

use crate::Store;

#[test]
fn pending_plan_disappears_after_the_next_user_message() {
    let mut store = Store::open_in_memory().unwrap();
    let project = store.create_project("Plans", 1).unwrap();
    let session = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::OpenAi,
                model_id: "model".into(),
            },
            PermissionMode::Plan,
            1,
        )
        .unwrap();
    store
        .append_message(
            &session.id,
            Role::Assistant,
            &[MessagePart::ToolCall {
                id: ToolCallId::new("call-1"),
                tool: "submit_plan".into(),
                args_json:
                    r#"{"summary":"s","steps":["a"],"files":[],"commands":[],"open_questions":[]}"#
                        .into(),
            }],
            2,
        )
        .unwrap();
    assert_eq!(
        store.pending_plan(&session.id).unwrap().unwrap().summary,
        "s"
    );
    store.begin_turn(&session.id, "Run the plan.", 3).unwrap();
    assert!(store.pending_plan(&session.id).unwrap().is_none());
}

fn plan_session(store: &mut Store, args: &str) -> tether_types::SessionId {
    let project = store.create_project("Plans", 1).unwrap();
    let session = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::OpenAi,
                model_id: "model".into(),
            },
            PermissionMode::Plan,
            1,
        )
        .unwrap();
    store
        .append_message(
            &session.id,
            Role::Assistant,
            &[MessagePart::ToolCall {
                id: ToolCallId::new("call-1"),
                tool: "submit_plan".into(),
                args_json: args.into(),
            }],
            2,
        )
        .unwrap();
    session.id
}

#[test]
fn a_plan_with_only_a_summary_and_steps_is_restored_with_empty_lists() {
    let mut store = Store::open_in_memory().unwrap();
    let id = plan_session(&mut store, r#"{"summary":"s","steps":["a","b"]}"#);
    let plan = store.pending_plan(&id).unwrap().unwrap();
    assert_eq!(plan.steps, ["a", "b"]);
    assert!(plan.files.is_empty() && plan.commands.is_empty() && plan.open_questions.is_empty());
}

#[test]
fn a_plan_with_a_wrong_typed_list_or_missing_steps_is_invalid() {
    for args in [
        r#"{"summary":"s","steps":["a"],"files":"x"}"#,
        r#"{"summary":"s","steps":["a"],"commands":[1]}"#,
        r#"{"summary":"s"}"#,
        r#"{"steps":["a"]}"#,
    ] {
        let mut store = Store::open_in_memory().unwrap();
        let id = plan_session(&mut store, args);
        assert!(store.pending_plan(&id).is_err(), "{args}");
    }
}
