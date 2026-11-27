/// What a new session in this project will load, for the New session screen.
#[derive(Debug, Clone, uniffi::Record)]
pub struct NewSessionPreviewView {
    /// File name of the instructions file the prompt will use, or `None` when the project has none.
    pub instructions_file: Option<String>,
    /// Line count of that file after the prompt's 32 KiB cap.
    pub instructions_lines: u32,
    /// Entries in MEMORY.md plus USER.md frozen into the prompt.
    pub memory_entries: u32,
    /// Valid skills that are not disabled.
    pub skills_available: u32,
    /// Title of the most recent non-archived session in this project, if any.
    pub last_session_title: Option<String>,
    /// Update time of that session in milliseconds since the Unix epoch.
    pub last_session_updated_at_ms: Option<i64>,
}
