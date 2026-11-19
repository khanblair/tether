@AGENTS.md

## Claude Code

- Codex writes the core Rust tasks (decision D28). For one, open a draft pull request from a branch, post the task as an `@codex` comment, and review what Codex pushes: read the diff, run the checks, and break the code to see that its tests fail. Push fixes to its branch.
- Delegate mechanical Compose work (translating a prototype screen into Compose with token values) to Haiku subagents.
- Comment `@codex review` on every pull request you open. Start a Sonnet review subagent only when Blair asks for one.
- Work in the current checkout, one issue at a time, on a branch you switch to. Leave the Agent tool's `isolation` option unset.
- Run at most 4 subagents at the same time.
- Subagents are read-only to git: they never commit, push, or open pull requests.
- Subagents run only the checks for the files they changed (for example `cd core && cargo test -p tether-types` or `cd android && ./gradlew :designsystem:ktlintCheck`). The main session runs `just check` before a push, and CI runs the full set.
- Leave visual checks to Blair; take screenshots only when asked.
