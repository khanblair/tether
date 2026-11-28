use super::run_support::guest_cwd;
use super::{RunContext, RunTools};
use crate::tool_loop::ToolOutput;
use tether_linux::RunCommand;

impl RunTools {
    pub(super) async fn run_background(
        &self,
        command: String,
        cwd: String,
        context: &RunContext<'_>,
    ) -> Result<ToolOutput, String> {
        let manager = self
            .background
            .as_ref()
            .ok_or("background processes are unavailable")?
            .clone();
        let network = (self.network_allowed)(&context.session_id);
        let runner = if network {
            self.linux_on.as_ref().or(self.linux_off.as_ref())
        } else {
            self.linux_off.as_ref().or(self.linux_on.as_ref())
        }
        .ok_or("Linux mode is not set up")?
        .clone();
        let request = tether_linux::RunRequest {
            command: RunCommand::Shell(command.clone()),
            cwd: guest_cwd(&cwd),
            extra_env: Vec::new(),
            timeout: None,
            // OutputRing is the only limit on retained output.
            // A per-run cap would stop log forwarding once reached.
            output_cap_bytes: usize::MAX,
        };
        let result =
            tokio::task::spawn_blocking(move || manager.start(runner, request, command, cwd))
                .await
                .map_err(|_| "background process thread stopped")??;
        let port = result.port.map_or_else(
            || "port: not detected".into(),
            |port| format!("port: {port}\nurl: http://127.0.0.1:{port}/"),
        );
        Ok(ToolOutput {
            text: format!(
                "process: {}\n{}\noutput:\n{}",
                result.id, port, result.first_output
            ),
        })
    }
}
