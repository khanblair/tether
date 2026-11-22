use super::{PromptInputs, PromptSection, SystemPrompt, base, estimate_tokens};

/// Assembles the stable system prompt prefix.
pub struct PromptBuilder;

impl PromptBuilder {
    /// Builds a deterministic prompt in cache-friendly section order.
    #[must_use]
    pub fn build(inputs: &PromptInputs<'_>) -> SystemPrompt {
        let mut text = String::new();
        let mut sections = Vec::new();
        let base = base::render(inputs.mode, inputs.tool_specs);
        add(&mut text, &mut sections, "Base", &base);
        let mode = mode_note(inputs);
        add(&mut text, &mut sections, "Mode note", &mode);
        let environment = environment(inputs);
        add(&mut text, &mut sections, "Environment", &environment);
        if !inputs.global_instructions.is_empty() || !inputs.project_instructions.is_empty() {
            let instructions = instructions(inputs);
            add(&mut text, &mut sections, "Instructions", &instructions);
        }
        if !inputs.memory_block.is_empty() {
            add(&mut text, &mut sections, "Memory", inputs.memory_block);
        }
        if !inputs.vault_block.is_empty() {
            add(&mut text, &mut sections, "Vault slice", inputs.vault_block);
        }
        if let Some(skills) = inputs.skills_index.filter(|text| !text.is_empty()) {
            add(&mut text, &mut sections, "Skills index", skills);
        }
        let date = format!("Today's date: {}", inputs.today);
        add(&mut text, &mut sections, "Date", &date);
        SystemPrompt { text, sections }
    }
}

fn add(text: &mut String, sections: &mut Vec<PromptSection>, name: &str, section: &str) {
    if !text.is_empty() {
        text.push_str("\n\n");
    }
    text.push_str(section);
    let chars = u32::try_from(section.chars().count()).unwrap_or(u32::MAX);
    sections.push(PromptSection {
        name: name.to_owned(),
        chars,
        approx_tokens: estimate_tokens(section),
    });
}

fn mode_note(inputs: &PromptInputs<'_>) -> String {
    match inputs.mode {
        tether_types::PermissionMode::Plan => {
            let ending = if inputs.tool_specs.iter().any(|tool| tool.name == "submit_plan") {
                "Finish by calling submit_plan when it is available."
            } else {
                "Finish by writing the plan as the final reply with numbered steps, files, commands, and open questions."
            };
            format!("Plan mode is read-only investigation. Do not edit files or run commands. {ending}")
        }
        tether_types::PermissionMode::Ask => "Ask mode auto-allows reads. Other actions ask the user.".into(),
        tether_types::PermissionMode::AcceptEdits => "Accept edits mode auto-allows file edits. Other actions ask the user.".into(),
        tether_types::PermissionMode::Auto => "Automatic mode auto-allows sandbox runs and file edits. Other actions ask the user.".into(),
        tether_types::PermissionMode::Bypass => "Fully automatic mode auto-allows actions except force pushes. Other exceptions ask the user.".into(),
    }
}

fn environment(inputs: &PromptInputs<'_>) -> String {
    let executors = if inputs.executors.is_empty() {
        "none".to_owned()
    } else {
        inputs
            .executors
            .iter()
            .map(|executor| executor.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let branch = inputs.branch.unwrap_or("unknown");
    format!(
        "Project: {}\nPlatform: Android\nAvailable executors: {executors}\nCurrent branch: {branch}",
        inputs.project_name
    )
}

fn instructions(inputs: &PromptInputs<'_>) -> String {
    format!(
        "Global instructions:\n{}\n\nProject instructions:\n{}",
        inputs.global_instructions, inputs.project_instructions
    )
}
