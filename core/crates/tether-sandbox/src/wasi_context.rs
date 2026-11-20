//! Narrow input contract for a future WASI context and its streamed output.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// The only host folder a sandbox run may see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasiContextSpec {
    /// Canonical host project folder mapped to `/workspace` in the guest.
    pub workspace: PathBuf,
    /// Guest variables built from a fixed safe set, never inherited from the host.
    pub env: BTreeMap<String, String>,
}

/// A rejected sandbox context.
#[derive(Debug)]
pub enum ContextError {
    /// Workspace path is missing, a symlink, or not a directory.
    InvalidWorkspace,
    /// Host filesystem access failed.
    Io(io::Error),
}

impl WasiContextSpec {
    /// Plans a `/workspace` mount with a fixed guest environment.
    ///
    /// # Errors
    /// Rejects symlink and non-directory workspaces.
    pub fn new(workspace: impl Into<PathBuf>) -> Result<Self, ContextError> {
        let workspace = workspace.into();
        let metadata = fs::symlink_metadata(&workspace).map_err(ContextError::Io)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ContextError::InvalidWorkspace);
        }
        let workspace = fs::canonicalize(workspace).map_err(ContextError::Io)?;
        let env = BTreeMap::from([
            ("HOME".into(), "/workspace".into()),
            ("LANG".into(), "C.UTF-8".into()),
            ("PATH".into(), "/usr/bin:/bin".into()),
            ("TMPDIR".into(), "/tmp".into()),
        ]);
        Ok(Self { workspace, env })
    }
}

/// Receives output chunks while a command runs.
pub trait OutputSink {
    /// A stdout chunk from the guest adapter.
    fn stdout(&mut self, bytes: &[u8]);
    /// A stderr chunk from the guest adapter.
    fn stderr(&mut self, bytes: &[u8]);
}

/// Which guest stream an adapter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputChannel { Stdout, Stderr }

/// `Write` adapter for a guest stream. It forwards chunks without buffering a whole run.
pub struct StreamWriter<'a> {
    sink: &'a mut dyn OutputSink,
    channel: OutputChannel,
}

impl<'a> StreamWriter<'a> {
    /// Attaches one guest output channel to a sink.
    pub fn new(sink: &'a mut dyn OutputSink, channel: OutputChannel) -> Self {
        Self { sink, channel }
    }
}

impl Write for StreamWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.channel {
            OutputChannel::Stdout => self.sink.stdout(bytes),
            OutputChannel::Stderr => self.sink.stderr(bytes),
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Chunks(Vec<Vec<u8>>, Vec<Vec<u8>>);
    impl OutputSink for Chunks {
        fn stdout(&mut self, bytes: &[u8]) { self.0.push(bytes.to_vec()); }
        fn stderr(&mut self, bytes: &[u8]) { self.1.push(bytes.to_vec()); }
    }

    #[test]
    fn guest_environment_never_inherits_host_keys() {
        let dir = tempfile::tempdir().unwrap();
        let spec = WasiContextSpec::new(dir.path()).unwrap();
        assert_eq!(spec.workspace, fs::canonicalize(dir.path()).unwrap());
        assert_eq!(spec.env.get("HOME").map(String::as_str), Some("/workspace"));
        assert!(!spec.env.keys().any(|key| key.contains("KEY") || key.contains("TOKEN")));
    }

    #[test]
    fn output_is_forwarded_in_separate_chunks_and_channels() {
        let mut chunks = Chunks(Vec::new(), Vec::new());
        StreamWriter::new(&mut chunks, OutputChannel::Stdout).write_all(b"one").unwrap();
        StreamWriter::new(&mut chunks, OutputChannel::Stdout).write_all(b"two").unwrap();
        StreamWriter::new(&mut chunks, OutputChannel::Stderr).write_all(b"error").unwrap();
        assert_eq!(chunks.0, vec![b"one".to_vec(), b"two".to_vec()]);
        assert_eq!(chunks.1, vec![b"error".to_vec()]);
    }
}
