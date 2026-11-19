# Tether agent guide

Tether is an Android coding-agent harness: a Rust core in `core/` and a Kotlin and Compose app in `android/`. The docs in `docs/` are the spec, and code and docs merge together.

## Start here

1. Read `docs/progress-tracker.md` for the current phase and open tasks, then `docs/project-overview.md`.
2. Read `docs/code-standards.md` section 13 (rules for AI agents) before you change code.
3. For a task ID such as P1-04, read its row in `docs/build-plan.md` (the "done when" line and dependencies) and the matching sections of `docs/architecture.md`.
4. Before adding or using a library, read `docs/library-docs.md`. For UI work, read `docs/ui-rules.md`, `docs/ui-tokens.md`, and `docs/ui-registry.md`.
5. To find where a file belongs, read the "Present now" tree in `docs/project-structure.md`.

## One task, one issue, one pull request

- Take one build-plan task per GitHub issue. The issue links the task row, which holds the "done when" line and the dependencies.
- Start the pull request description with `Closes #<issue number>`, so GitHub closes the issue when the pull request merges into `main`. Work with no issue says `No issue: <reason>`. The `issue-link` check enforces this.
- Work on a branch and open a pull request into `main`. Branch names, commit format, and merging follow `docs/code-standards.md` section 10.
- Change only what the task needs, plus the docs, tracker rows, and generated files that must change with it. Work that no task covers gets its own issue.
- Land shared contracts first (types, traits, schema) so parallel tasks do not collide.

## Who writes and who reviews

- Codex writes the core Rust tasks from the issue and pushes to the pull request branch. Claude Code writes the Android UI, using Haiku subagents for the mechanical translation of screens, and reviews what Codex wrote.
- Request `@codex review` on every pull request. A pull request is reviewed by the other model than the one that wrote it (decision D28).

## Commits

- Keep each commit to 15 files or fewer, grouped by purpose. `just commits` checks it, and CI runs the same check.
- Write commit messages in the format of `docs/code-standards.md` section 10, with the task ID.
- Commit and push only after Blair gives the go-ahead. Reviewers and subagents may read git (`git diff`, `git log`) and never change it.

## Done means

- The task's "done when" line is true, shown by a test or a run.
- `just check` and `just commits` pass. CI also runs `cargo deny`, both Android assemble tasks, the docs site build, and the `issue-link` check.
- Docs that describe the changed behavior, structure, library, or UI value change in the same pull request. `docs/progress-tracker.md` gets the task status and rows in the session, file changes, and doc changes logs. The pull request number exists only after the pull request opens, so add it in a follow-up commit.
- Generated files are regenerated (`just bindings`, `just tokens`) and committed.
- Secrets never appear in code, logs, tests, or docs; name the environment variable instead.
- Blair checks Android UI changes in the emulator. Say that in the pull request, and run `just check` and `cd android && ./gradlew assembleFullDebug` yourself.

## Conventions no tool enforces

- Write docs, UI strings, comments, commit messages, and pull request text by the writing rules in `docs/project-overview.md`: short sentences, sentence case, numbers with units. CI runs `slop-lint` on docs, `README.md`, and `strings.xml` only; run `just slop <path>` on other files you write.
- Take every color, size, and duration from `tokens/tokens.json` through the generated values. User-facing text lives in `strings.xml`.
- The accent color means "your move": primary actions, the approval bar, focus, and selection only.
- Each tool the app runs that changes files, runs code, or touches the network shows its exact target before it acts.

## Review rules

When reviewing a pull request:

- Check the issue's task row and its "done when" line first, then that the diff stays inside the task.
- Flag any doc that no longer matches the code.
- Flag secrets, cleartext network use, and file access outside the workspace path guard.
- Follow the review process and checklist in `docs/code-standards.md` sections 10 and 11.
- Skip style that rustfmt, clippy, ktlint, or detekt already enforce.
