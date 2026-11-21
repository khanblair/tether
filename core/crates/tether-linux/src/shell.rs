use std::ffi::OsString;
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::LinuxError;
use crate::proot::{ProotConfig, SessionKind};

/// A live interactive shell inside the Alpine guest.
pub struct LinuxShell {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    output: Option<Receiver<Vec<u8>>>,
}

impl LinuxShell {
    /// Starts `sh -l` in a PRoot-backed pty.
    ///
    /// # Errors
    /// Returns a not-ready, validation, pty, or process error.
    pub fn spawn(config: &ProotConfig, size: PtySize) -> Result<Self, LinuxError> {
        if !config.proot_path.is_file() {
            return Err(LinuxError::NotReady("PRoot is unavailable".into()));
        }
        if !config.linux_root.join("rootfs/alpine").is_dir() {
            return Err(LinuxError::NotReady(
                "the Alpine root filesystem is unavailable".into(),
            ));
        }
        let command = config.command(
            &["sh".into(), "-l".into()],
            &[],
            "/workspace",
            SessionKind::Shell,
        )?;
        let builder = portable_command(&command);
        let pty = native_pty_system();
        let pair = pty
            .openpty(size)
            .map_err(|_| LinuxError::Process("could not open Linux shell".into()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|_| LinuxError::Process("could not open shell input".into()))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| LinuxError::Process("could not open shell output".into()))?;
        let child = pair
            .slave
            .spawn_command(builder)
            .map_err(|_| LinuxError::Process("could not start Linux shell".into()))?;
        let (sender, receiver) = mpsc::sync_channel(128);
        spawn_output_reader(reader, sender);
        Ok(Self {
            master: pair.master,
            writer,
            child,
            output: Some(receiver),
        })
    }

    /// Writes raw terminal input to the guest.
    ///
    /// # Errors
    /// Returns a pty write error.
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), LinuxError> {
        self.writer
            .write_all(bytes)
            .map_err(|_| LinuxError::Process("could not write Linux shell".into()))
    }

    /// Applies a new terminal size.
    ///
    /// # Errors
    /// Returns a pty resize error.
    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), LinuxError> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| LinuxError::Process("could not resize Linux shell".into()))
    }

    /// Takes the bounded raw output receiver. The channel blocks the reader when full.
    ///
    /// # Errors
    /// This method returns a closed receiver if called more than once.
    pub fn output(&mut self) -> Receiver<Vec<u8>> {
        self.output.take().unwrap_or_else(|| {
            let (_sender, receiver) = mpsc::sync_channel(0);
            receiver
        })
    }

    /// Terminates the guest shell.
    ///
    /// # Errors
    /// Returns a process termination error.
    pub fn kill(&mut self) -> Result<(), LinuxError> {
        self.child
            .kill()
            .map_err(|_| LinuxError::Process("could not kill Linux shell".into()))
    }

    /// Waits for the shell to exit and returns its code.
    ///
    /// # Errors
    /// Returns a process wait error.
    pub fn wait(&mut self) -> Result<i32, LinuxError> {
        let status = self
            .child
            .wait()
            .map_err(|_| LinuxError::Process("could not wait for Linux shell".into()))?;
        Ok(i32::try_from(status.exit_code()).unwrap_or(1))
    }
}

impl Drop for LinuxShell {
    fn drop(&mut self) {
        drop(self.child.kill());
    }
}

fn portable_command(command: &std::process::Command) -> CommandBuilder {
    let mut argv = vec![command.get_program().to_owned()];
    argv.extend(command.get_args().map(OsString::from));
    let mut builder = CommandBuilder::from_argv(argv);
    builder.env_clear();
    for (name, value) in command.get_envs() {
        if let Some(value) = value {
            builder.env(name, value);
        }
    }
    builder.cwd(std::env::temp_dir());
    builder
}

fn spawn_output_reader(mut reader: Box<dyn Read + Send>, sender: SyncSender<Vec<u8>>) {
    thread::spawn(move || {
        let mut buffer = [0_u8; 16 * 1024];
        while read_and_send(&mut reader, &mut buffer, &sender) {}
    });
}

fn send_pty_chunk(sender: &SyncSender<Vec<u8>>, bytes: &[u8]) -> bool {
    sender.send(bytes.to_vec()).is_err()
}

fn read_and_send<R: Read>(reader: &mut R, buffer: &mut [u8], sender: &SyncSender<Vec<u8>>) -> bool {
    match reader.read(buffer) {
        Ok(0) | Err(_) => false,
        Ok(count) => !send_pty_chunk(sender, &buffer[..count]),
    }
}
