//! Lexical path cleanup shared by the file tools and the run executor.

use std::path::{Component, Path, PathBuf};

/// Removes current-directory components and rejects workspace escapes.
pub(crate) fn lexical_path(raw: &str) -> Result<PathBuf, String> {
    let mut output = PathBuf::new();
    for component in Path::new(raw).components() {
        push_component(&mut output, component)?;
    }
    Ok(output)
}

fn push_component(output: &mut PathBuf, component: Component<'_>) -> Result<(), String> {
    match component {
        Component::Normal(value) => output.push(value),
        Component::CurDir => {}
        Component::ParentDir if output.pop() => {}
        Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
            return Err("path leaves the workspace".into());
        }
    }
    Ok(())
}
