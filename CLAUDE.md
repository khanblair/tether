@AGENTS.md

## Claude Code

- Delegate mechanical Compose work (translating a prototype screen into Compose with token values) to Haiku subagents, and run reviews on Sonnet subagents in a fresh context.
- Run at most 4 subagents at the same time.
- Subagents are read-only to git: they never commit, push, or open pull requests.
- Subagents run only the checks for the files they changed (for example `cd core && cargo test -p tether-types` or `cd android && ./gradlew :designsystem:ktlintCheck`). The main session runs `just check` before a push, and CI runs the full set.
- Leave visual checks to Blair; take screenshots only when asked.
