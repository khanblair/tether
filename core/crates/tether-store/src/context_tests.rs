use tether_types::{
    Message, MessagePart, ModelRef, PermissionMode, Provider, Role, SessionId, ToolCallId,
};

use crate::{CompactionRecord, Store};

fn session(store: &mut Store) -> SessionId {
    let project = store.create_project("Context", 1).unwrap();
    store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::OpenAi,
                model_id: "model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap()
        .id
}

fn add(store: &mut Store, id: &SessionId, role: Role, text: &str) -> Message {
    store
        .append_message(id, role, &[MessagePart::Text { text: text.into() }], 2)
        .unwrap()
}

fn texts(messages: &[Message]) -> Vec<String> {
    messages
        .iter()
        .map(|message| match &message.parts[0] {
            MessagePart::Text { text } => text.clone(),
            _ => "other".into(),
        })
        .collect()
}

fn record(id: &SessionId, summary: &Message, first: &Message, last: &Message) -> CompactionRecord {
    CompactionRecord {
        id: format!("c-{}", summary.id.as_str()),
        session_id: id.clone(),
        summary_message_id: summary.id.clone(),
        first_message_id: first.id.clone(),
        last_message_id: last.id.clone(),
        tokens_before: 100,
        tokens_after: 10,
        created_at: 3,
        focus: None,
    }
}

#[test]
fn a_compacted_range_is_replaced_by_its_summary_once() {
    let mut store = Store::open_in_memory().unwrap();
    let id = session(&mut store);
    let one = add(&mut store, &id, Role::User, "one");
    let two = add(&mut store, &id, Role::Assistant, "two");
    let three = add(&mut store, &id, Role::User, "three");
    let four = add(&mut store, &id, Role::Assistant, "four");
    // The summary is appended after the recent messages, as the engine does.
    let summary = add(&mut store, &id, Role::User, "summary");
    store
        .insert_compaction(&record(&id, &summary, &one, &two))
        .unwrap();
    let context = store.context_messages(&id).unwrap();
    assert_eq!(texts(&context), ["summary", "three", "four"]);
    assert!(context.iter().all(|m| m.id != one.id && m.id != two.id));
    let _ = (three, four);
}

#[test]
fn two_compactions_each_keep_one_summary_in_order() {
    let mut store = Store::open_in_memory().unwrap();
    let id = session(&mut store);
    let one = add(&mut store, &id, Role::User, "one");
    let two = add(&mut store, &id, Role::Assistant, "two");
    let three = add(&mut store, &id, Role::User, "three");
    let first_summary = add(&mut store, &id, Role::User, "summary-a");
    store
        .insert_compaction(&record(&id, &first_summary, &one, &two))
        .unwrap();
    let four = add(&mut store, &id, Role::Assistant, "four");
    let second_summary = add(&mut store, &id, Role::User, "summary-b");
    store
        .insert_compaction(&record(&id, &second_summary, &three, &four))
        .unwrap();
    let context = store.context_messages(&id).unwrap();
    assert_eq!(texts(&context), ["summary-a", "summary-b"]);
}

#[test]
fn a_session_without_compactions_returns_every_live_message() {
    let mut store = Store::open_in_memory().unwrap();
    let id = session(&mut store);
    add(&mut store, &id, Role::User, "one");
    add(&mut store, &id, Role::Assistant, "two");
    assert_eq!(texts(&store.context_messages(&id).unwrap()), ["one", "two"]);
}

#[test]
fn repair_adds_one_synthetic_result_per_open_call() {
    let mut store = Store::open_in_memory().unwrap();
    let id = session(&mut store);
    store
        .append_message(
            &id,
            Role::Assistant,
            &[MessagePart::ToolCall {
                id: ToolCallId::new("call-1"),
                tool: "read".into(),
                args_json: "{}".into(),
            }],
            2,
        )
        .unwrap();
    assert!(
        store
            .interruption_step(&id)
            .unwrap()
            .unwrap()
            .starts_with("tool: read")
    );
    store.repair_interrupted(&id).unwrap();
    store.repair_interrupted(&id).unwrap();
    let results = store
        .messages(&id)
        .unwrap()
        .iter()
        .flat_map(|message| message.parts.clone())
        .filter(|part| matches!(part, MessagePart::ToolResult { is_error: true, .. }))
        .count();
    assert_eq!(results, 1);
}
