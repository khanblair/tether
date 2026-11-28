use tether_engine::commands::{CommandRegistry, CustomCommand};
use tether_skills::{DiscoverInput, SkillsCatalog};

use crate::SessionRuntime;

/// Returns enabled user skills that can be invoked from the composer.
pub(crate) fn skill_commands(runtime: &SessionRuntime) -> Vec<CustomCommand> {
    let Ok(store) = runtime.store() else {
        return Vec::new();
    };
    let mut catalog = SkillsCatalog::discover(DiscoverInput {
        project_root: None,
        project_trusted: false,
        user_dir: &runtime.files_root.join("skills"),
        bundled: &[],
    });
    let Ok(disabled) = store.disabled_skills() else {
        return Vec::new();
    };
    for name in disabled {
        catalog.set_disabled(&name, true);
    }

    let builtins = CommandRegistry::builtin();
    catalog
        .skills
        .into_iter()
        .filter(|skill| valid_command_name(&skill.name))
        .filter(|skill| {
            !builtins
                .commands()
                .iter()
                .any(|command| command.name == skill.name)
        })
        .map(|skill| {
            let name = skill.name;
            CustomCommand {
                name: name.clone(),
                description: command_description(&skill.description),
                template: format!(
                    "Use the \"{name}\" skill for this request. Load it first with skill_view, then follow its instructions. $ARGUMENTS"
                ),
            }
        })
        .collect()
}

fn valid_command_name(name: &str) -> bool {
    (1..=40).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn command_description(description: &str) -> String {
    let description = description.split_whitespace().collect::<Vec<_>>().join(" ");
    if description.chars().count() <= 100 {
        return description;
    }
    let prefix = description.chars().take(99).collect::<String>();
    format!("{prefix}…")
}

#[cfg(test)]
#[path = "commands_skills_tests.rs"]
mod commands_skills_tests;
