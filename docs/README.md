# Tether docs

Tether is an Android app that runs a full AI coding harness on the phone, with no PC: pick a model from five providers, work in a project workspace stored on the device, run code in an on-phone sandbox or Linux environment, and ship through GitHub.

## What each doc answers

| Doc | Question it answers |
|---|---|
| [project-overview.md](project-overview.md) | What is Tether, what does v1 include, and what is already decided? |
| [product-scope.md](product-scope.md) | What does every feature do, how does it work, and why does it exist? |
| [architecture.md](architecture.md) | How is the system built, and how do its parts talk to each other? |
| [project-structure.md](project-structure.md) | Where does every folder and file live, and what is it for? |
| [build-plan.md](build-plan.md) | What do we build, in what order, and how do we know a task is done? |
| [code-standards.md](code-standards.md) | How do we write code, tests, and text in this repo? |
| [development.md](development.md) | How do I set up, run, test, build, and release Tether? |
| [library-docs.md](library-docs.md) | Which libraries are approved, and how do we use them? |
| [ui-tokens.md](ui-tokens.md) | What are the exact colors, type, spacing, and motion values? |
| [ui-rules.md](ui-rules.md) | How should each screen look, read, and behave? |
| [ui-registry.md](ui-registry.md) | Which UI components exist, and what state is each one in? |
| [mobile.md](mobile.md) | How does the Android app work as a platform, and what changes for iOS? |
| [progress-tracker.md](progress-tracker.md) | Where is the project right now, and what changed recently? |

## Reading order

For people:

1. project-overview.md
2. product-scope.md
3. architecture.md
4. mobile.md
5. ui-rules.md, then ui-tokens.md and ui-registry.md
6. code-standards.md and development.md
7. build-plan.md and progress-tracker.md

For AI coding agents, see the reading order in [project-overview.md](project-overview.md#reading-order-for-ai-agents). Agents read progress-tracker.md at the start of every session and update it at the end.

## Reading online

The same files are published as a site on GitHub Pages: [khanblair.github.io/tether](https://khanblair.github.io/tether/). `tools/site/build.ts` builds it from this folder, so the site cannot differ from the docs.

## Keeping docs true

1. A change that alters behavior, structure, a decision, a library, or a UI value updates the matching doc in the same pull request. Code and docs merge together or not at all.
2. If code and a doc disagree, the doc is the bug until someone decides otherwise. Record the decision in the decisions log in progress-tracker.md, then fix whichever side was wrong.
3. Every doc edit adds one line to the doc changes log in progress-tracker.md.
4. New docs need a row in the table above. This folder stays at 14 files unless the decisions log records a reason to add one. Skills, memory, and the vault live inside product-scope.md and architecture.md for that reason.
5. Docs follow the writing rules in project-overview.md. The `slop-lint` check in CI fails a pull request that breaks them, and the site build fails on a broken link between docs.
