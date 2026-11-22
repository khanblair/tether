//! Session-local checklist, question, and plan tools.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tether_providers::{BoxFuture, ToolSpec};
use tether_store::Store;
use tether_types::{CoreEvent, PermissionMode, Plan, SessionStatus, TodoItem, TodoStatus};

use super::registry::{ExecuteContext, PrepareContext, PreparedCall, ToolHandler, ToolOutput};
use crate::permissions::Effect;
use crate::{Answer, Asker, QuestionRequest};

mod validation;
use validation::{
    object, prepare_plan, prepare_question, prepare_todos, required_string, schema, strings,
};

#[cfg(test)]
mod tests;

const TODO_TOOL: &str = "todo_write";
const ASK_TOOL: &str = "ask_user";
const PLAN_TOOL: &str = "submit_plan";

/// The specification table entry used by the prompt builder.
#[derive(Debug, Clone, Copy)]
pub struct SessionToolSpec {
    /// Provider-facing name.
    pub name: &'static str,
    /// Short model-facing description.
    pub description: &'static str,
}

impl SessionToolSpec {
    /// Returns whether the tool should be advertised in a permission mode.
    #[must_use]
    pub fn advertised_in(self, mode: PermissionMode) -> bool {
        if self.name == PLAN_TOOL {
            matches!(mode, PermissionMode::Plan)
        } else {
            true
        }
    }
}

/// The session tool specification table.
pub const SESSION_TOOL_SPECS: [SessionToolSpec; 3] = [
    SessionToolSpec {
        name: TODO_TOOL,
        description: "Replace the session checklist with the current work items.",
    },
    SessionToolSpec {
        name: ASK_TOOL,
        description: "Ask the user for a needed decision with optional answer choices.",
    },
    SessionToolSpec {
        name: PLAN_TOOL,
        description: "Submit a plan for review when working in Plan mode.",
    },
];

/// Owns the three tools that change only session state.
pub struct SessionTools {
    asker: Arc<dyn Asker>,
    store: Arc<Mutex<Store>>,
}

impl SessionTools {
    /// Creates the session tool handler with its platform question resolver.
    #[must_use]
    pub fn new(asker: Arc<dyn Asker>, store: Arc<Mutex<Store>>) -> Self {
        Self { asker, store }
    }
}

impl ToolHandler for SessionTools {
    fn specs(&self) -> Vec<ToolSpec> {
        SESSION_TOOL_SPECS
            .into_iter()
            .map(|spec| ToolSpec {
                name: spec.name.into(),
                description: spec.description.into(),
                input_schema_json: schema(spec.name).to_string(),
            })
            .collect()
    }

    fn prepare(
        &self,
        name: &str,
        args_json: &str,
        context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String> {
        let value = object(args_json)?;
        let (target, payload) = match name {
            TODO_TOOL => prepare_todos(&value)?,
            ASK_TOOL => prepare_question(&value)?,
            PLAN_TOOL => prepare_plan(&value, context)?,
            _ => return Err("unknown tool".into()),
        };
        Ok(PreparedCall {
            tool: name.into(),
            effect: Effect::Internal,
            target,
            executor: None,
            payload,
        })
    }

    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        mut context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        Box::pin(async move {
            match call.tool.as_str() {
                TODO_TOOL => self.execute_todos(&call.payload, &mut context),
                ASK_TOOL => self.execute_question(&call, &mut context).await,
                PLAN_TOOL => execute_plan(&call, &mut context),
                _ => Err("unknown tool".into()),
            }
        })
    }
}

impl SessionTools {
    fn execute_todos(
        &self,
        value: &Value,
        context: &mut ExecuteContext<'_>,
    ) -> Result<ToolOutput, String> {
        let items = value
            .get("todos")
            .and_then(Value::as_array)
            .ok_or("todos must be an array")?
            .iter()
            .map(todo_item)
            .collect::<Result<Vec<_>, _>>()?;
        let mut store = self
            .store
            .lock()
            .map_err(|_| "could not save checklist".to_owned())?;
        store
            .replace_todos(context.session_id, &items, now_ms())
            .map_err(|_| "could not save checklist".to_owned())?;
        drop(store);
        if let Some(sink) = context.sink {
            sink.on_event(CoreEvent::TodoUpdated {
                session_id: context.session_id.clone(),
                items: items.clone(),
            });
        }
        let completed = items
            .iter()
            .filter(|item| item.status == TodoStatus::Completed)
            .count();
        Ok(ToolOutput {
            text: format!("checklist updated: {completed} of {} done", items.len()),
        })
    }
}

fn todo_item(value: &Value) -> Result<TodoItem, String> {
    let object = value.as_object().ok_or("each todo must be an object")?;
    let text = required_string(object, "text")?.to_owned();
    let status = required_string(object, "status")
        .and_then(|value| value.parse().map_err(|_| "invalid todo status".to_owned()))?;
    Ok(TodoItem { text, status })
}

impl SessionTools {
    async fn execute_question(
        &self,
        call: &PreparedCall,
        context: &mut ExecuteContext<'_>,
    ) -> Result<ToolOutput, String> {
        let object = call
            .payload
            .as_object()
            .ok_or("question arguments must be an object")?;
        let question = required_string(object, "question")?.to_owned();
        let options = strings(object, "options")?;
        let allow_free_text = object
            .get("allow_free_text")
            .and_then(Value::as_bool)
            .ok_or("allow_free_text must be a boolean")?;
        if !self.asker.interactive() {
            return Ok(ToolOutput {
                text: "No one is available to answer. Continue with your best assumption and say which assumption you made.".into(),
            });
        }
        self.set_status(context, SessionStatus::AwaitingApproval)?;
        if let Some(sink) = context.sink {
            sink.on_event(CoreEvent::QuestionAsked {
                session_id: context.session_id.clone(),
                tool_call_id: context.tool_call_id.clone(),
                question: question.clone(),
                options: options.clone(),
                allow_free_text,
            });
        }
        let request = QuestionRequest {
            session_id: context.session_id.clone(),
            tool_call_id: context.tool_call_id.clone(),
            question,
            options: options.clone(),
            allow_free_text,
        };
        let answer = self.asker.ask(request, context.cancel.clone()).await;
        self.set_status(context, SessionStatus::RunningTool)?;
        Ok(ToolOutput {
            text: answer_text(answer, &options),
        })
    }
    fn set_status(
        &self,
        context: &mut ExecuteContext<'_>,
        status: SessionStatus,
    ) -> Result<(), String> {
        let store = self
            .store
            .lock()
            .map_err(|_| "could not save question state".to_owned())?;
        store
            .set_session_status(context.session_id, status, now_ms())
            .map_err(|_| "could not save question state".to_owned())?;
        drop(store);
        if let Some(sink) = context.sink {
            sink.on_event(CoreEvent::SessionUpdated {
                session_id: context.session_id.clone(),
                status,
            });
        }
        Ok(())
    }
}

fn answer_text(answer: Answer, options: &[String]) -> String {
    let text = match answer {
        Answer::Cancelled => "The user did not answer".to_owned(),
        Answer::Text(text) if options.iter().any(|option| option == &text) => {
            format!("The user chose: {text}")
        }
        Answer::Text(text) => format!("The user answered: {text}"),
    };
    text.chars().take(4_000).collect()
}

fn execute_plan(
    call: &PreparedCall,
    context: &mut ExecuteContext<'_>,
) -> Result<ToolOutput, String> {
    let object = call
        .payload
        .as_object()
        .ok_or("plan arguments must be an object")?;
    let plan = Plan {
        summary: required_string(object, "summary")?.to_owned(),
        steps: strings(object, "steps")?,
        files: strings(object, "files")?,
        commands: strings(object, "commands")?,
        open_questions: strings(object, "open_questions")?,
    };
    if let Some(sink) = context.sink {
        sink.on_event(CoreEvent::PlanReady {
            session_id: context.session_id.clone(),
            tool_call_id: context.tool_call_id.clone(),
            plan,
        });
    }
    Ok(ToolOutput {
        text: "Plan shown to the user. Stop and wait for their decision.".into(),
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}
