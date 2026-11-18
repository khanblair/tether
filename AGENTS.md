# Tether agent guide

Tether is an Android coding-agent harness: a Rust core in `core/` and a Kotlin and Compose app in `android/`. The docs in `docs/` are the spec, and code and docs merge together.

## Start here

1. Read `docs/progress-tracker.md` for the current phase and open tasks, then `docs/project-overview.md`.
2. For a task ID such as P1-04, read its row in `docs/build-plan.md` (the "done when" line and dependencies) and the matching sections of `docs/architecture.md`.
3. Before adding or using a library, read `docs/library-docs.md`. For UI work, read `docs/ui-rules.md`, `docs/ui-tokens.md`, and `docs/ui-registry.md`.
4. To find where a file belongs, read the "Present now" tree in `docs/project-structure.md`.

## One task, one issue, one pull request

- Take one build-plan task per GitHub issue. The issue names the task ID, the files you may touch, and the "done when" line.
- Start the pull request description with `Closes #<issue number>`, so GitHub closes the issue when the pull request merges into `main`. Work with no issue says `No issue: <reason>`. The `issue-link` check enforces this.
- Work on a branch and open a pull request into `main`. Branch names, commit format, and merging follow `docs/code-standards.md` section 10.
- Stay inside the issue's file list. A change outside it needs its own issue.
- Land shared contracts first (types, traits, schema) so parallel tasks do not collide.

## Commits

- Keep each commit to 15 files or fewer, grouped by purpose. `tools/check-commit-size.sh` fails a pull request that breaks this.
- Write commit messages in the format of `docs/code-standards.md` section 10, with no em dashes and no emoji.
- Commit and push only from the main session, and only after Blair gives the go-ahead. Subagents, reviewers, and background agents leave git alone.

## Done means

- The task's "done when" line is true, shown by a test or a run.
- The checks behind `just check` pass; CI runs the same ones.
- Docs that describe the changed behavior, structure, library, or UI value change in the same pull request, and `docs/progress-tracker.md` gets the task status and log lines.
- Generated files are regenerated (`just bindings`, `just tokens`) and committed.
- Secrets never appear in code, logs, tests, or docs; name the environment variable instead.
- Android UI changes are checked by Blair in the emulator. Say that in the pull request and run build, lint, and tests yourself.

## Conventions no tool enforces

- Write docs, UI strings, comments, and pull request text by the writing rules in `docs/project-overview.md`: short sentences, sentence case, numbers with units. `slop-lint` checks the banned words.
- Take every color, size, and duration from `tokens/tokens.json` through the generated values. User-facing text lives in `strings.xml`.
- The accent color means "your move": primary actions, the approval bar, focus, and selection only.
- Every tool that changes files, runs code, or touches the network shows its exact target before it acts.

## Review rules

When reviewing a pull request:

- Check the issue's "done when" line first, then the file list.
- Flag any doc that no longer matches the code.
- Flag secrets, cleartext network use, and file access outside the workspace path guard.
- Flag a commit that touches more than 15 files.
- Skip style that rustfmt, clippy, ktlint, or detekt already enforce.
