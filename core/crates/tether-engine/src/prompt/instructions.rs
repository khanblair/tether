use std::fs;
use std::path::Path;

use tether_workspace::PathGuard;

const INSTRUCTION_CAP: usize = 32 * 1024;

/// Fresh global and project instruction blocks.
#[derive(Debug, Clone, Default)]
pub struct InstructionBlocks {
    /// Global instructions block.
    pub global: String,
    /// Project instructions block.
    pub project: String,
}

/// A project instruction file and the capped text read from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInstruction {
    /// The selected file name.
    pub file_name: String,
    /// The file text after the prompt's cap is applied.
    pub text: String,
}

/// What reading the project instruction file found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectInstructionRead {
    /// The first safe file in prompt order, with its capped text.
    Found(ProjectInstruction),
    /// A file exists but was not used. `note` is the exact text the prompt shows the model.
    /// `file_name` is empty when the project root itself is unavailable.
    Skipped {
        /// The file that was skipped, or empty when no file was selected.
        file_name: String,
        /// The bracketed note the prompt shows in place of the instructions.
        note: String,
    },
    /// No instruction file exists.
    Missing,
}

/// Reads global and project instructions for one turn.
#[must_use]
pub fn load_instructions(
    global_path: Option<&Path>,
    project_root: Option<&Path>,
) -> InstructionBlocks {
    InstructionBlocks {
        global: global_path.map_or_else(String::new, read_unchecked),
        project: project_root.map_or_else(String::new, read_project),
    }
}

fn read_project(root: &Path) -> String {
    match load_project_instructions(root) {
        ProjectInstructionRead::Found(instruction) => format!(
            "project instruction {}\n<instructions>\n{}\n</instructions>",
            instruction.file_name, instruction.text
        ),
        ProjectInstructionRead::Skipped { note, .. } => note,
        ProjectInstructionRead::Missing => String::new(),
    }
}

/// Reads the first project instruction file in prompt order.
///
/// A file that exists but cannot be used gives `Skipped` with the note the prompt shows.
#[must_use]
pub fn load_project_instructions(root: &Path) -> ProjectInstructionRead {
    let Ok(guard) = PathGuard::new(root) else {
        return ProjectInstructionRead::Skipped {
            file_name: String::new(),
            note: note("project root is unavailable"),
        };
    };
    for name in ["TETHER.md", "AGENTS.md", "CLAUDE.md"] {
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        return read_guarded(&guard, name);
    }
    ProjectInstructionRead::Missing
}

fn read_guarded(guard: &PathGuard, name: &str) -> ProjectInstructionRead {
    if fs::symlink_metadata(guard.root().join(name))
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return skipped(
            name,
            &format!("project instruction {name} was skipped: symlinked file"),
        );
    }
    let Ok(path) = guard.resolve(name) else {
        return skipped(
            name,
            &format!("project instruction {name} was skipped: unsafe path"),
        );
    };
    match read_utf8(&path) {
        Ok(text) => ProjectInstructionRead::Found(ProjectInstruction {
            file_name: name.to_owned(),
            text: truncate_at_line(&text),
        }),
        Err(reason) => skipped(
            name,
            &format!("project instruction {name} were skipped: {reason}"),
        ),
    }
}

fn skipped(name: &str, reason: &str) -> ProjectInstructionRead {
    ProjectInstructionRead::Skipped {
        file_name: name.to_owned(),
        note: note(reason),
    }
}

fn read_unchecked(path: &Path) -> String {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return note("global instructions were skipped: symlinked file");
    }
    read_file(path, "global instructions")
}

fn read_file(path: &Path, label: &str) -> String {
    match read_utf8(path) {
        Ok(text) => {
            let text = truncate_at_line(&text);
            format!("{label}\n<instructions>\n{text}\n</instructions>")
        }
        Err(reason) => note(&format!("{label} were skipped: {reason}")),
    }
}

/// Reads a file as UTF-8. The error is the reason shown in the prompt note.
fn read_utf8(path: &Path) -> Result<String, &'static str> {
    let bytes = fs::read(path).map_err(|_| "unreadable file")?;
    String::from_utf8(bytes).map_err(|_| "non-UTF-8 file")
}

fn truncate_at_line(text: &str) -> String {
    if text.len() <= INSTRUCTION_CAP {
        return text.to_owned();
    }
    let marker = "[truncated]";
    let budget = INSTRUCTION_CAP.saturating_sub(marker.len() + 1);
    let cut = text
        .as_bytes()
        .get(..budget)
        .and_then(|prefix| prefix.iter().rposition(|byte| *byte == b'\n'))
        .unwrap_or(0);
    format!("{}\n{marker}", &text[..cut])
}

fn note(reason: &str) -> String {
    format!("[{reason}]")
}
