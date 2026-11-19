@AGENTS.md

## Claude Code

- Codex writes the core Rust tasks (decision D28), always with GPT-5.6 Luna. Ask for no other Codex model unless Blair changes it. Run each task as a local Codex CLI session in the current checkout, one issue at a time and at most 4 sessions at once: write the task down first (files it may touch, design, tests, rules), then run `codex exec -m gpt-5.6-luna -s workspace-write -C <repo> -o <file> -` with the task on stdin. Tell the session to leave git alone and to run only `cargo fmt`, `cargo test`, and `cargo clippy`. Then review what it wrote: read the diff, run the checks, and break the code to see that its tests fail. You commit, push, and open the pull request.
- Use Haiku subagents for the UI: translating a prototype screen into Compose with token values, and other mechanical Compose work.
- Comment `@codex review` on every pull request you open. Start a Sonnet review subagent only when Blair asks for one.
- Work in the current checkout, one issue at a time, on a branch you switch to. Leave the Agent tool's `isolation` option unset.
- Run at most 4 subagents at the same time.
- Subagents are read-only to git: they never commit, push, or open pull requests.
- Subagents run only the checks for the files they changed (for example `cd core && cargo test -p tether-types` or `cd android && ./gradlew :designsystem:ktlintCheck`). The main session runs `just check` before a push, and CI runs the full set.
- Leave visual checks to Blair; take screenshots only when asked.
