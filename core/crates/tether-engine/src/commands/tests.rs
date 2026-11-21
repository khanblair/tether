use super::*;

#[test]
fn builtin_declares_every_f7_command_once() {
    let registry = CommandRegistry::builtin();
    let names: Vec<_> = registry
        .commands()
        .iter()
        .map(|command| command.name.as_str())
        .collect();
    assert_eq!(names.len(), 23);
    assert_eq!(names.iter().filter(|name| **name == "help").count(), 1);
    assert!(names.contains(&"init"));
}

#[test]
fn gated_commands_report_their_feature() {
    let registry = CommandRegistry::builtin();
    let error = registry
        .resolve("/commit", &Capabilities::empty(), &[])
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "/commit needs Git, which is not set up yet"
    );
}

#[test]
fn resolution_substitutes_all_prompt_arguments() {
    let registry = CommandRegistry::builtin();
    let result = registry
        .resolve("/init add $ARGUMENTS", &Capabilities::empty(), &[])
        .unwrap();
    assert!(
        matches!(result.command.kind, CommandKind::Prompt(ref text) if text.ends_with("add $ARGUMENTS"))
    );
}

#[test]
fn invalid_and_large_inputs_never_panic() {
    let registry = CommandRegistry::builtin();
    for input in ["", "/", "//x", "é", &"/x".repeat(5_000)] {
        assert!(
            registry
                .resolve(input, &Capabilities::empty(), &[])
                .is_err()
        );
    }
}

#[test]
fn uppercase_command_is_unknown_with_a_case_insensitive_suggestion() {
    let error = CommandRegistry::builtin()
        .resolve("/COMMIT", &Capabilities::empty(), &[])
        .unwrap_err();
    let CommandError::Unknown { suggestions, .. } = error else {
        panic!("uppercase command should be unknown")
    };
    assert_eq!(suggestions[0], "commit");
}

#[test]
fn mention_parser_skips_code_and_emails() {
    let mentions = parse_mentions("@src/a.rs x@y.test `@inline` ```\n@f.rs\n``` @@literal");
    assert_eq!(
        mentions
            .iter()
            .map(|mention| mention.path.as_str())
            .collect::<Vec<_>>(),
        vec!["src/a.rs"]
    );
}

#[test]
fn mention_parser_handles_boundaries_and_punctuation() {
    let cases = [
        ("@src/a.rs", vec!["src/a.rs"]),
        ("read @src/a.rs, please", vec!["src/a.rs"]),
        ("mail a@src/a.rs", Vec::new()),
        ("@@src/a.rs", Vec::new()),
        ("@../secret", Vec::new()),
        ("`@src/a.rs`", Vec::new()),
    ];
    for (text, expected) in cases {
        assert_eq!(
            parse_mentions(text)
                .iter()
                .map(|mention| mention.path.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn custom_commands_replace_arguments_and_builtin_wins() {
    let registry = CommandRegistry::builtin();
    let custom = [CustomCommand {
        name: "ship".into(),
        description: "Ship it".into(),
        template: "Do $ARGUMENTS twice: $ARGUMENTS".into(),
    }];
    let resolved = registry
        .resolve("/ship carefully", &Capabilities::empty(), &custom)
        .unwrap();
    assert!(
        matches!(resolved.command.kind, CommandKind::Prompt(text) if text == "Do carefully twice: carefully")
    );

    let clash = [CustomCommand {
        name: "help".into(),
        description: "wrong".into(),
        template: "wrong".into(),
    }];
    let resolved = registry
        .resolve("/help", &Capabilities::empty(), &clash)
        .unwrap();
    assert_eq!(resolved.command.description, "List commands");
}

#[test]
fn init_prompt_contains_reviewable_file_instructions() {
    let registry = CommandRegistry::builtin();
    let resolved = registry
        .resolve("/init", &Capabilities::empty(), &[])
        .unwrap();
    let CommandKind::Prompt(prompt) = resolved.command.kind else {
        panic!("init is not a prompt")
    };
    assert!(prompt.contains("TETHER.md"));
    assert!(prompt.contains("normal edit tool"));
    assert!(prompt.contains("under 120 lines"));
}

#[test]
fn export_omits_thinking_and_truncates_tool_results() {
    use tether_types::{Message, MessageId, MessagePart, Role, SessionId, ToolCallId};

    let session = SessionId::new("s");
    let id = ToolCallId::new("call");
    let messages = vec![
        Message {
            id: MessageId::new("m"),
            session_id: session.clone(),
            role: Role::Assistant,
            seq: 0,
            created_at_ms: 0,
            rewound: false,
            parts: vec![
                MessagePart::Thinking {
                    text: "secret thinking".into(),
                },
                MessagePart::ToolCall {
                    id: id.clone(),
                    tool: "read_file".into(),
                    args_json: "{}".into(),
                },
            ],
        },
        Message {
            id: MessageId::new("m2"),
            session_id: session.clone(),
            role: Role::Tool,
            seq: 1,
            created_at_ms: 0,
            rewound: false,
            parts: vec![MessagePart::ToolResult {
                tool_call_id: id,
                is_error: false,
                text: "x".repeat(2_100),
                result_ref: None,
            }],
        },
    ];
    let markdown = export_session_messages_markdown("Title", "model", &messages);
    assert!(!markdown.contains("secret thinking"));
    assert!(markdown.contains("target: read_file"));
    assert!(markdown.len() < 2_400);
}
