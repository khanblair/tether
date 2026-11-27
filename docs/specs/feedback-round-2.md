# Feedback round 2: scope and validity

Status: scope, written 2026-11-26 from Blair's second emulator test. Blair's decisions of 2026-11-26 are recorded in section 12 and in the sections they affect. No code is changed by this document. Every fact below was checked in the code on 2026-11-26. "Not run" means the cause is read from code and was not reproduced on a device.

## 1. Summary

| # | Blair's point | Valid | Size | Notes |
|---|---|---|---|---|
| 1 | Sent message appears after the reply | Yes | S | A regression from the Bypass work. Same cause as 2. Fix first |
| 2 | Thinking dropdown appears only after the reply | Yes | S | Same cause as 1 |
| 3 | Copy, Edit, Regenerate icons too large | Yes | S | 20 dp glyphs in 48 dp buttons |
| 4 | Theme control looks wrong | Yes | S | Differs from the prototype and from the other components |
| 5 | Interface size, text size, accent color in Settings | Yes | M | The prototype has the sizes, not the colors. Decided: see sections 5 and 12 |
| 6 | Raw Markdown in update notes | Yes | S | About screen draws notes as plain text |
| 7 | A workspace folder, set at onboarding and in Settings | Yes | L | Decided: private by default, shared folder in the `full` flavor after a spike. Paths shown in Settings and onboarding |
| 8 | Settings rows say "Not available yet" | Yes, seen in the screenshots | S | Four of five labels are stale |
| 9 | GitHub and Linux mode | Later | L | Tracks in sections 10 and 11. Linux mode needs wiring, not a rewrite |

## 2. Items 1 and 2: the transcript freezes while a turn runs

What Blair sees: the message just sent does not show until the reply is done, and the Thinking row appears only after the reply. The "Thinking" label with seconds is live.

Cause, from code (not run):
- After every core event the transcript reloads a snapshot (`CoreEvents` in `session/TranscriptSnapshot.kt`). The snapshot calls `settings.bypassActive(sessionId)` (added 2026-11-26 15:54, commit 9b56f84).
- `bypass_active` is sent through the session actor's command queue (`tether-ffi/src/approvals.rs:183`, `tether-engine/src/session/handle.rs:185`). The actor handles one command at a time (`session.rs:134`, `session/dispatch.rs`), and a turn runs inside the `Send` command. So `bypass_active` waits until the whole turn ends. Each reload waits with it.
- The engine itself is fine. It saves the user message at the start of the turn and saves each streamed batch of thinking and text (`tool_loop/stream.rs:64-111`, events `ThinkingDelta` and `TextDelta`). The test session shows the user message saved 14 seconds before the first assistant batch.

Fix: `SessionHandle` reads the shared mode lock (`tools.mode`, an `Arc<Mutex<PermissionMode>>`) directly, with no actor round trip. Add a Rust test where a slow provider is mid-turn and `bypass_active` returns at once. Add a rule to the session notes: calls on the snapshot path must never wait for the actor. After the fix the Thinking row appears with the first saved thinking batch. Optional polish: show the collapsed Thinking row from the start of the turn, in place of the footer label, so there is no gap before the first token.

## 3. Item 3: action icons

`session/MessageActionRow.kt`: `IconButton` at `TetherSize.touchMin` (48 dp) with a 20 dp glyph (`iconS`). Each icon takes a 48 dp slot.
Decided: 16 dp glyph in a 36 dp button, 4 dp gap. This needs one new size token (`icon-xs` 16) in `tokens/tokens.json`, then `just tokens`.
`docs/ui-rules.md:246` says every touch target is at least 48 dp, chips included. A 36 dp target is an exception for the secondary action row under a message. The same change writes the exception into `docs/ui-rules.md`, limited to that row.

## 4. Item 4: theme control

Today: the theme choice is inline on the main Settings list as three Material `FilterChip`s in a `Column`, stacked and left-aligned (`shell/SettingsScreen.kt:252-273`). The prototype has an Appearance screen, reached from a row "Appearance, System theme · text 100%" (`tools/prototype/s-settings.js:21, 86-88`), with horizontal chips. The app has no such screen.
The design system already has `SegmentedControl` (used in Cost and Clone), and picker sheets with a check row (`selectedTrailing`).
Proposed: build the Appearance screen (section 5), use `SegmentedControl` for System, Light, Dark, and remove the inline control. The drawer toggle stays.

## 5. Item 5: interface size, text size, accent color

Prototype, already specified (`s-settings.js:86-88`): a Theme row, a Text size row (85, 100, 115, 130, 150, 200%), a Display size row (90, 100, 115, 130%), a live preview card, and Reduce motion and Haptics toggles. It also says "the palette is fixed". Accent choice is therefore a change Blair is making on purpose; record it as a decision.

Facts:
- No scaling exists in production code (`LocalDensity` and `fontScale` are only read, or used in previews). Typography is in `sp`, space and sizes in `dp`, and only 17 lines hard-code a `dp` outside tokens. So one override at the root scales the whole app.
- Colors are static objects in `tokens/tokens.json`, generated to `Tokens.kt` by `just tokens`. `TetherTheme` exposes them through a private `LocalPalette` (`TetherTheme.kt:125`), so a runtime override is possible with a small change there.
- The brand is a two-color pair that swaps by theme: accent plum `#64165D` with tonal mint `#A7E8D7` in light, and the reverse in dark.
- The generator has a contrast gate (`tokens/generate.ts --check`).

Scaling, proposed:
- Apply `Density(density × interface scale, fontScale = text scale)` at the root in `MainActivity`, from two new preferences read at start.
- Range, decided. Blair suggested 60 to 125%, and below about 85% two rules break: touch targets under 48 dp (`ui-rules.md:246`) and text under 12 sp (`ui-tokens.md:170`). So: interface 80 to 125% in steps (80, 90, 100, 110, 125), and text 80 to 150% (80, 90, 100, 115, 130, 150) plus "Follow system" for text.
- Not scaled by this: the WebView preview, the system bars and keyboard.

Colors, decided: one accent color that Blair picks, not a pair.
- The accent is the main color of buttons, selected states, focus, and highlights (today plum `#64165D` in light, mint `#A7E8D7` in dark). Blair picks any color, such as yellow, red, or blue. The app derives everything else from that one color for light and dark: the text on the accent (dark on yellow, white on blue), the soft background (`accent-soft`), the tonal fill and its text (`tonal`, `on-tonal`), and the ink color for focus rings (`accent-ink`). The existing second accent (`accent-2`) is derived from the same hue.
- Picker: five swatches (Plum, the current default, then Blue, Green, Yellow, Red) and a custom picker (hue and saturation sliders and a hex field). Plum keeps its hand-tuned values, so the default look does not change.
- Readability fix: whatever Blair picks, the app moves the lightness and chooses black or white text until each pair passes the contrast gate (`tokens/generate.ts` has the ratios; the same rule runs in the app). The preview shows the adjusted color, so there is no surprise.
- Live preview at the top of the screen: a primary button, a tonal button, a selected row, a chip, and the status line, shown in light and in dark. A Reset to Plum action.
- Dark-mode version: the same hue at a lighter tone, with its own checks.
- Suits the app best: calm, medium chroma. The accent means "your move" (`AGENTS.md`), so it stays on primary actions, the approval bar, focus, and selection.

Where: a new Appearance screen (route `settings/appearance`) with a live preview, as in the prototype. Persist in a preferences store next to `ThemeStore`. Tests: pure functions for the effective density and for the palette contrast, plus previews at 80%, 100%, 125% and 200% system scale.

## 6. Item 6: Markdown in update notes

`updates/AboutScreen.kt:273-288`: `NotesBox` draws the GitHub release body as plain `Text`. The body is Markdown (`core/crates/tether-engine/src/updates/release.rs`, cut at 8,000 characters).
Fix: use `MarkdownBlocks` (internal to the app module, so reachable) inside the same scroll box, with the Preview button off. Check that an unclosed code fence from the 8,000 character cut shows as text, not as an error. Links stay plain text, as in chat. Tables are not supported. I read "here when updating" as the update notes on About and updates. Correct me if you meant another screen.

## 7. Item 7: a workspace folder

What exists:
- One fixed root, `filesDir/workspaces`, set in `di/AppModule.kt:69`. The database keeps only a relative project path (`projects.path`, schema comment "Relative to files/workspaces/"). Rust runtimes take the root as a string; a session takes it when it opens.
- No migration or re-root code. If the root changed, every existing project would resolve to a missing folder.
- Checkpoint blobs are derived from the root (`sessions/open.rs:156`, `root.parent()/blobs`). A shared folder root would put blobs next to it. This must become an explicit blob folder.
- No storage permission is declared. There is no use of `OpenDocumentTree` in the code. `docs/mobile.md:75` says SAF folders are copied by the Kotlin layer, because the Rust core cannot open content URIs.
- Docs promise private storage (`product-scope.md:60-69`), and `docs/mobile.md:99` says "no broad storage permissions".
- Onboarding is `Welcome, AddKey, OpenProject` (`OnboardingViewModel.kt:14-18`). A new step is an enum entry, a `when` branch, and a title.

Options:
1. Shared folder with All files access (`MANAGE_EXTERNAL_STORAGE`), `full` flavor only. Real paths, visible to file managers, a PC over USB, and other apps. Costs: a special permission screen, the Play policy (the `play` flavor stays private), slower git on shared storage, and shared storage has no symlinks or exec bits. Git checkouts with symlinks, Node installs, and Linux mode exec bits can break there. Other apps can change files, which weakens "the workspace is the boundary".
2. Choose another private location. Private internal storage or the app's own external folder (`Android/data/...`). No permission and no policy risk, but other apps cannot browse it on Android 11 and later.
3. Keep private storage and rely on the existing zip import and export.

Decided: private storage is the default, and a shared folder is offered. Blair wants to see exactly where Tether keeps projects and data, in Settings and during setup, and to be able to change the projects folder.
- A `Workspace location` setting. Private is the default. A shared folder such as `/sdcard/Tether` is offered only in the `full` flavor, behind the All files access permission, and only after the spike below shows it works.
- Each project records its own root (migration 0016, `projects.root` nullable, null means the default private root). Existing projects stay where they are and nothing moves by itself. A later "Move project" action does the copy.
- The checkpoint blob folder becomes explicit instead of derived from the root.
- Settings gets a "Where your data lives" view, read-only except the projects folder: projects, database, secrets, checkpoint blobs, memory, skills, vault, and the crash file (`files/crash/last.txt`), each with its path and size. The existing Data screen is the natural home.
- Onboarding gets a skippable step, "Where should Tether work?", between AddKey and OpenProject. It shows the private path and the same list, and offers Choose a folder (in `full`).
- Docs to change with it: `docs/product-scope.md:60-69` (private only), `docs/mobile.md:99` (no broad storage permission) and `:75`, `docs/architecture.md:251` and `:335`, and the data-extraction rules if the shared folder is excluded from backup.

Spike first (half a day), before any shared-folder code ships: on the emulator, create a repo under `/sdcard/Tether` through the core, commit, rewind a checkpoint, run a sandbox command, and open a project there. Report what fails. If git or checkpoints fail there, the shared folder option stays off, and the Settings and onboarding paths view still ships.

## 8. Item 8: stale "Not available yet" labels

`SettingsScreen.kt:212-238` uses one string, `settings_area_not_available`, with no condition, for GitHub, MCP servers, Skills and memory, Quality gate, and Budgets. Real screens exist for GitHub, MCP, Skills and memory, and Budgets. Only Quality gate is not built (phase P8). So four labels are wrong, and the GitHub one may be what made GitHub look unavailable.
Fix: each row shows its real state ("Signed in as ...", "No servers", "3 servers", "Month limit $20"), and only Quality gate says "Not available yet".

## 9. Order

1. Items 1 and 2 (a regression I introduced; small and urgent).
2. Items 3, 6 and 8 (small UI fixes).
3. Items 4 and 5 (Appearance screen, sizes, then colors).
4. Item 7 (spike first).
5. New session screen and the Runs entry (saved, decided).
6. GitHub track, then the Linux mode spike.

## 10. Later track: GitHub

- The sign-in check needs the classic `repo` scope header, so fine-grained tokens are rejected (`tether-ffi/src/github.rs:188`). `docs/product-scope.md:233` says they work. Decided: accept both. Sign-in checks the token by calling the API for the account and a repository call instead of reading the scope list, and Settings shows what the token can reach.
- Scopes are held in memory only and are lost on restart.
- On the test emulator no GitHub token or identity is saved; find out why.
- Sign in with GitHub needs a client ID that nothing supplies.
- The agent cannot init a repo, create a GitHub repository, set a remote, or work with issues. Plan: `git_init`, a create-repo tool with approval, saving the remote, then the existing push and PR tools; later issues and merge.

## 11. Later track: Linux mode and dev servers

See `docs/specs/preview.md` section 4. Correction, 2026-11-27: an earlier version of this document said Linux mode was not built. The Rust library is built (`tether-linux`, about 3,900 lines with tests, and the FFI). What is missing is the app wiring: PRoot and talloc binaries, a signed rootfs manifest with its key and hosting, the call to `configure_executors`, the `/linux` screen, background processes for dev servers, port discovery, and the localhost preview. The PRoot feasibility spike still comes first.

## 12. Decisions made by Blair, 2026-11-26

1. Item 3: 16 dp glyph in a 36 dp button, with the touch rule exception written for that row.
2. Item 5, sizes: interface 80 to 125%, text 80 to 150%, plus Follow system for text.
3. Item 5, color: one accent color Blair picks (five swatches plus a custom picker), everything else derived, with automatic readability fix and a live preview.
4. Item 7: private storage by default, a shared folder in the `full` flavor after a spike. Paths shown in Settings and onboarding, and the projects folder changeable.
5. Item 6: the update notes on About and updates.
6. GitHub: accept classic and fine-grained tokens.

Nothing here has been built. The next step is Blair's go for items 1 and 2.
