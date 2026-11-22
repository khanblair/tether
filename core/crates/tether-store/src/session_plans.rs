//! Queries for the latest submitted session plan.

use serde_json::Value;
use tether_types::{MessagePart, Plan, Role, SessionId};

use crate::{Store, StoreError};

impl Store {
    /// Returns the latest plan unless the session has a later user message.
    ///
    /// # Errors
    /// Returns an error when the transcript or saved plan JSON is invalid.
    pub fn pending_plan(&self, session_id: &SessionId) -> Result<Option<Plan>, StoreError> {
        let messages = self.messages(session_id)?;
        let Some((seq, args_json)) = messages
            .iter()
            .rev()
            .filter(|message| !message.rewound && message.role == Role::Assistant)
            .find_map(|message| {
                message
                    .parts
                    .iter()
                    .rev()
                    .find_map(plan_args)
                    .map(|args| (message.seq, args))
            })
        else {
            return Ok(None);
        };
        let plan = decode_plan(args_json)?;
        if messages
            .iter()
            .any(|message| !message.rewound && message.role == Role::User && message.seq > seq)
        {
            return Ok(None);
        }
        Ok(Some(plan))
    }
}

fn plan_args(part: &MessagePart) -> Option<&str> {
    match part {
        MessagePart::ToolCall {
            tool, args_json, ..
        } if tool == "submit_plan" => Some(args_json),
        _ => None,
    }
}

fn decode_plan(args_json: &str) -> Result<Plan, StoreError> {
    let value: Value = serde_json::from_str(args_json).map_err(StoreError::MessagePartJson)?;
    let object = value.as_object().ok_or_else(|| StoreError::InvalidPlan)?;
    Ok(Plan {
        summary: string(object, "summary")?,
        steps: list(object, "steps")?,
        files: optional_list(object, "files")?,
        commands: optional_list(object, "commands")?,
        open_questions: optional_list(object, "open_questions")?,
    })
}

fn string(object: &serde_json::Map<String, Value>, key: &str) -> Result<String, StoreError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(StoreError::InvalidPlan)
}

/// Reads a list the `submit_plan` schema allows to be omitted; a missing key is an empty list.
fn optional_list(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, StoreError> {
    if object.contains_key(key) {
        list(object, key)
    } else {
        Ok(Vec::new())
    }
}

fn list(object: &serde_json::Map<String, Value>, key: &str) -> Result<Vec<String>, StoreError> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or(StoreError::InvalidPlan)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(StoreError::InvalidPlan)
        })
        .collect()
}
