@AGENTS.md

## Claude Code

- Delegate mechanical Compose work (translating a prototype screen into Compose with token values) to Haiku subagents, and run reviews on Sonnet subagents in a fresh context.
- Run at most 4 subagents at the same time.
- Subagents are read-only to git: they never commit, push, or open pull requests.
- Subagents run only the checks for the files they changed (for example `cargo test -p tether-types` or `./gradlew :designsystem:ktlintCheck`). The main session runs `just check` before a push, and CI runs the full set.
- Run build, lint, and tests after UI changes and hand the visual check to Blair. Take no screenshots except when asked to do so.
