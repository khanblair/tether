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
    let Ok(guard) = PathGuard::new(root) else {
        return note("project root is unavailable");
    };
    for name in ["TETHER.md", "AGENTS.md", "CLAUDE.md"] {
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        return read_guarded(&guard, name);
    }
    String::new()
}

fn read_guarded(guard: &PathGuard, name: &str) -> String {
    if fs::symlink_metadata(guard.root().join(name))
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return note(&format!(
            "project instruction {name} was skipped: symlinked file"
        ));
    }
    let Ok(path) = guard.resolve(name) else {
        return note(&format!(
            "project instruction {name} was skipped: unsafe path"
        ));
    };
    read_file(&path, &format!("project instruction {name}"))
}

fn read_unchecked(path: &Path) -> String {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return note("global instructions were skipped: symlinked file");
    }
    read_file(path, "global instructions")
}

fn read_file(path: &Path, label: &str) -> String {
    let Ok(bytes) = fs::read(path) else {
        return note(&format!("{label} were skipped: unreadable file"));
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return note(&format!("{label} were skipped: non-UTF-8 file"));
    };
    let text = truncate_at_line(&text);
    format!("{label}\n<instructions>\n{text}\n</instructions>")
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
