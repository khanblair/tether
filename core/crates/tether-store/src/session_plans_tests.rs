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
