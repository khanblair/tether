# UI rules

Status: draft 0.1, 18 Nov 2026. Values come from ui-tokens.md; components from ui-registry.md.

## 1. Core rules

1. **The transcript is the product.** Every screen either feeds the transcript (composer, pickers) or explains something in it (diff, files, runs).
2. **The accent means your move.** Only actions the user must take, or is taking, use the accent. The agent's own activity stays neutral. The accent is plum in the light theme and mint in the dark theme (see ui-tokens.md).
3. **Show before doing.** Any action that changes files, runs code, or touches the network shows its exact target before it happens: the path, the command, the URL.
4. **Never hide cost or risk.** Model, mode, context, and cost are always visible in the status line during a session. Bypass mode turns the status line red.
5. **One primary action per screen.** Secondary actions go in overflow menus or as text buttons.
6. **Everything by thumb.** Primary actions sit in the bottom half of the screen. Nothing needed often lives only in the top bar.
7. **No gesture-only features.** Every swipe or long press has a visible equivalent.
8. **Familiar words.** Use the same terms as desktop harnesses where the meaning is the same: session, plan mode, accept edits, rewind, compact.

## 2. Layout

Compact (phone portrait), from top to bottom:

```
┌──────────────────────────────┐
│ Top bar: ≡  Project / session  ⋮ │  56 dp
│ Status line: model · mode · ctx · $│  28 dp
│ Pinned checklist (if any)      │  collapsible
│                                │
│ Transcript (scrolls)           │
│                                │
│ Approval bar (when pending)    │  72 dp+
│ Composer                       │  56 dp to 40% height
└──────────────────────────────┘
```

- The navigation drawer (≡) is the only navigation: Home, Projects, Needs attention, Usage and cost, Runs, then sessions. There is no bottom navigation bar; the composer owns the bottom edge.
- Back is an arrow icon (`arrow-left`) whose content description is "Back". Never the word "Back" as a text button.
- The system action bar is off. Every screen draws its own top bar.
- Screen padding is `space.4` horizontally. The transcript caps at `layout.transcript-max` wide.
- The keyboard pushes the composer and approval bar up; the transcript stays scrolled to the newest item if it was already at the bottom.
- Every screen, sheet, and the drawer keeps its text fields above the keyboard. A sheet opens fully expanded and lifts its content by the keyboard height. A form that can be taller than the space above the keyboard scrolls.

## 3. Rules per view

### Onboarding
- Four screens maximum before the user reaches a project. Each has one primary button and a Skip text button.
- Never ask for unrelated permissions such as notifications or battery during onboarding. A shared
  folder choice can open All files access when the person selects it.
- The workspace step shows the private projects path and a short list of local data paths. Private
  storage is the default. In the `full` flavor on Android 11 and later, Choose a folder opens the shared folder controls.

### Home
- The app opens here. Top bar: menu button (opens the drawer), the title "Home", and the bell. One scrolling column of cards, `space.4` apart. A card is `surface`, `radius.md`, a hairline `line` border, and no shadow.
- The cards, top to bottom:
  1. **Summary line.** One sentence about now: "2 things need you. 1 session is running." or "Nothing needs you right now."
  2. **Needs you now.** Up to three Needs attention items, then the sessions running now. **See all** opens Needs attention. The card is hidden when both lists are empty.
  3. **Today.** One card with three stats side by side: sessions, tokens, and files changed. Each has a plain secondary line that compares with yesterday. Hidden until there is any usage.
  4. **Usage, last 7 days.** A sentence first ("212k tokens this week, 18% more than last week."), then a bar chart of tokens per day, then **By model**: each model with its share in numbers and a thin bar. A **Show data** button opens the table behind the chart. Cost is not shown until prices are wired (P1-14).
  5. **Recent projects.** The three most recently active projects, and **All projects**.
  6. **Providers.** "2 of 5 providers have a key" with one row per provider ("Key saved" or "No key yet"). It counts saved keys and says nothing about whether a key works. With no key at all, the card says sessions need a key and shows one primary button, **Add a key**.
- Charts are bars with `chart` colors, a text summary, and a data table. No pie or donut charts. Each chart has a content description with the same sentence.
- Every build shows only real data, never sample numbers: with no usage the Usage card says "No usage yet. It appears here after your first session."
- The cards are not a grid of equal tiles: each has its own shape and one clear action.

### Drawer
- Header: the logo, "Tether", and the theme toggle on the right.
- Destinations, current one highlighted with `tonal`: **Home**, **Projects**, **Needs attention** (with the count when it is not zero), **Usage and cost**, **Runs**.
- Then a session search and the sessions grouped by project. This part scrolls together with the destinations.
- Pinned at the bottom: **New session**, **Settings**, **Skills and memory**.
- Choosing a destination clears the screens above Home, so Back from any of them goes Home. The drawer opens by swipe only on Home and Projects.

### Theme toggle
- One icon button in the drawer header. It cycles System, Light, Dark, and System again. The icon shows the current choice (monitor, sun, moon). The content description names it: "Theme: System. Switch to Light."
- The choice is saved and applies at once. Settings > Appearance shows the same choice.

### Project list
- Reached from the drawer. Its top bar has a menu button, the title, and Search.
- Rows show project name, branch, uncommitted change count, last activity. Sort by last activity.
- The primary action is **New project**, which opens a sheet: Clone from GitHub, New empty, Import zip.
- Empty state explains what a project is in one sentence and offers the same three actions.
- Search filters the list by project name in place; it does not open another screen.

### Needs attention
- The bell in the Home top bar opens it, and so does the drawer link. A dot on the bell shows when anything needs the user, and the dot clears when the list is empty.
- It lists, newest first, in three groups: **Waiting for you** (a session needs approval, with the session and project named), **Failed** (a run or session stopped with an error, with the reason in one line), and **Updates** (a newer app version is out).
- Each row opens what it names: the session, or About for an update. There is no "mark as read": an item leaves the list when its cause is gone.
- Empty state: "Nothing needs you right now." One sentence, no illustration.
- It is not a notification history. System notifications (approvals in the background, run results) are separate and stay in the Android shade.

### Project home
- Shows repo and branch, uncommitted changes (tap opens the diff), default executor, and recent sessions.
- Primary action: **New session**. The composer is visible at the bottom so the user can start typing at once.

### New empty project
- A full screen opened from New project > New empty project. One field, **Project name** (1 to 80 characters, no control characters), a line under it ("Stored on this phone, in a private folder."), and a primary **Create** button that stays disabled until the trimmed name is not empty.
- A name that is too long or has a control character shows the reason in `danger` under the field as the user types. Create opens the new project's home. A failure to create the folder shows an inline block with **Try again** and creates nothing.

### Starting a session
- Every way to start a session creates it and opens the real session screen, so there is one composer with its model and mode sheets, slash commands, and mentions. These are the drawer's **New session**, **New session** on a project home, and `/new`. The drawer starts in the only project, or opens a project picker first when there are several, or opens New project when there are none. The empty transcript of a new session shows what it loads: the instructions file, memory entries, skills, and the last session. A session left with no message is deleted when its screen closes.
- **New session** on a project home takes the first provider that has a saved key and the first model in that provider's saved list, creates the session, and opens it. The composer row shows the model name. The session opens without tools until approvals ship (P3).
- No provider has a key: no session is created. An inline block says "No provider key yet. Add one to start a session." with **Add a key**, which opens Providers.
- A key is saved but its model list is empty: no session is created. The block says "Test the connection in Providers so the model list is saved." with **Open Providers**.

### Session
- The transcript reads like a chat, with no "You" or "Agent" labels: the side says who wrote it (decision D39, which replaces the prototype's inset user message).
- User messages: a bubble aligned to the end, at most 85% of the row, `surface-sunken` background, `radius.lg`, plain text. The accent color is never used for it.
- Agent text: full width, aligned to the start, no background and no bubble, `type.body-m`, with Markdown: bold, italic, strikethrough, inline code, headings (levels 3 to 6 share one style), nested lists, quotes, horizontal rules, pipe tables, and fenced code. A table has a soft hairline border and a sunken header row, columns as wide as their content up to 40% of the transcript width, wrapping cells, column alignment from the delimiter row, and sideways scrolling when it is wider than the screen. A link shows its text only.
- Reasoning is one collapsed row, "Thinking" with a chevron, also while the reply streams. Tapping it shows the text. The state is kept per message.
- Under a user bubble: Copy and Edit. Under the last message of each finished agent turn: Copy, and Regenerate on the newest turn only. Edit and Regenerate are off while a turn runs. Edit rewinds the conversation to that message and sends the new text; files are not restored.
- Tool calls, diffs, runs, plans, and reports are cards: full width, `surface-sunken` in light mode or `surface` in dark mode, `radius.md`, no border.
- Consecutive read-only tool calls (read, list, glob, grep) group into one collapsed card: "Read 6 files, searched 2 times." Tapping expands the list.
- The newest running card shows a running indicator in its header. Nothing else animates.
- A "Jump to latest" button appears whenever the last row is out of view, in any session. It is a round arrow button, and it becomes a pill with the count of new items when rows arrived since the reader scrolled up.
- A running local server appears in a compact bar above the composer. The bar names its port and offers Preview and Stop. It disappears when the process exits or stops. Server preview shows the exact loopback URL, keeps other hosts blocked until network is enabled, and never restarts a stopped process.

### Diff view
- Unified diff on compact and medium widths; side-by-side only on expanded and wider.
- File list on top as chips with `+n −m` counts; tapping a chip scrolls to that file.
- Per-hunk **Keep** and **Undo** as text buttons at the hunk's top right. Per-file actions in an overflow menu.
- Quality findings appear inline under the line they refer to, with severity icon and label.
- Line wrapping on by default; toggle in the top bar.

### Files
- Tree view with lazy expansion; ignored files dimmed and hidden by default.
- Viewer is read-only with syntax highlighting. **Ask about this file** inserts an `@` mention into the composer and returns to the session.

### Runs
- Run console shows command (mono), executor badge, live output, exit code, duration.
- **Run elsewhere** offers the other available executors.
- CI runs list shows workflow, branch, status, duration, and a link to the run on GitHub.
- The Runs screen lists the history of the selected project. A local row shows the command (mono) as its title, a line with the status and the time (`Running · 3 min ago`, `Stopped · 21 min ago`, or the exit code and duration), and the session name on one line. The executor badge sits at the end, then one icon button.
- A background command shows a spinner and a **Stop** icon only while its process runs in an open session. When the process is gone (the app was restarted, or the session screen was closed, which ends its processes), the row shows a Stopped mark and no spinner. Stop ends the process and the row updates.
- A local row that is not running shows a **Restart** icon. It opens the session that ran the command with "Run this command again: `<command>`" in the message box. The agent runs it only after you press Send and approve it, like any command. CI rows have no buttons.

### Linux shell
- Full-screen terminal with a key row above the keyboard: Esc, Tab, Ctrl, Alt, arrows, `|`, `/`, `-`, `~`. Ctrl and Alt stay armed for one key, and the next key counts whether it comes from the row or from the Android keyboard (Ctrl then C sends the interrupt).
- The top bar shows the project folder and a network on/off badge.
- The emulator supports shell text, ANSI colours, cursor movement, scrollback, and alternate-screen programs.
- It does not provide mouse reporting, mouse selection, or full terminfo compatibility.
- Long press copies the visible terminal rows. The shell has no horizontal scrolling.

### Settings
- Grouped list matching product-scope.md F19. Each group is one screen.
- Appearance replaces the inline theme control on the main Settings list. It contains the live
  preview, theme, interface size, text size, accent color, custom color picker, and reset actions.
- The accent picker is not in the prototype; added by decision D41.
- Data shows the path and size of projects, the database, secrets, checkpoints, memory, skills,
  vault, and the crash report. The projects row opens Workspace folder.
- Workspace folder shows the effective projects folder and whether it is private or custom. It
  keeps the text field inside the scrolling settings page.
- Destructive actions (delete project, remove Linux mode, clear data) are at the bottom of their group, in `color.danger`, and need confirmation that names the thing being deleted.

### Cost
- Summary first in a sentence ("This session: $0.84, 212k tokens"), chart second, data table behind "Show data".
- The Cost screen counts usage, not only spend. The three totals (today, 7 days, 30 days) show the cost and the tokens in and out. Free models cost $0 but their tokens are counted, so the chart and the by-project and by-model tables show them: each table row reads "cost · tokens in, tokens out". When the range has tokens and no spend, the chart shows tokens by day and a line says that free models cost nothing. "No usage yet." appears only when there are no tokens and no spend.

## 4. Key element anatomy

### Tool call card

```
┌──────────────────────────────────────────┐
│ [icon] Verb  target (mono)      status ▾ │  header, 44 dp
├──────────────────────────────────────────┤
│ Output (mono-s, surface-sunken)          │  collapsed to 12 + 12 lines
│ … 140 lines hidden · Show all            │
└──────────────────────────────────────────┘
```

- Verb in past tense when done ("Edited", "Ran"), present progressive while running ("Running"), imperative when awaiting approval ("Run").
- Status: running indicator, check, cross, or pause icon, each with a text label for TalkBack.

### Approval bar

```
┌▌─────────────────────────────────────────┐
│▌ Run in Linux mode                       │  title-s
│▌ npm install && npm test   (mono)        │  exact action, up to 3 lines, expandable
│▌ [Deny]   [Always allow ▾]   [ Allow ]   │  Allow is the accent primary button
└▌─────────────────────────────────────────┘
 ▲ 4 dp accent stripe
```

- Allow sits on the right, under the thumb. Deny is a text button; long-press or its arrow adds a reason.
- "Always allow" opens a menu showing the exact rule that will be created.
- While an approval is pending, the composer stays usable for typing a reply that denies with that reason.

### Composer

```
┌──────────────────────────────────────────┐
│ [+]  Message, / for commands, @ for files │
│      [mode chip] [model chip]    [mic] [send] │
└──────────────────────────────────────────┘
```

- Send becomes Stop (square icon) while a turn runs.
- The plus button opens attachments, commands, and file mention.
- Mode chip shows the mode name; accent fill only for Auto, red outline and label for Bypass.

### Status line

`Sonnet · Accept edits · 34% · $0.21` in `type.caption`, `text-3`. Tapping any part opens its picker or detail. In Bypass: `danger-soft` background and the word "Bypass" first.

### Diff card

Header: "Edited 3 files  +42 −7". Body: file list with counts. Footer: quality result ("2 warnings" with icon) and **Review** button. Block-level findings show a red label: "1 blocking issue".

### Plan card

Numbered steps, files to touch, commands to run, open questions. Footer: **Run this plan** (primary), **Edit plan**, **Keep planning**.

## 5. Interactions

| Interaction | Behavior |
|---|---|
| Tap tool card header | Expand or collapse |
| Long-press any message | Menu: Copy, Rewind to here, Edit and resend (user messages only) |
| Swipe a session row left | Archive, with Undo snackbar; also in the row's overflow menu |
| Pull down on the transcript top | Load older messages |
| Type `/` or `@` at word start | Opens command or file sheet above the keyboard |
| Tap status line segment | Opens model picker, mode picker, context breakdown, or cost |
| Back during a running turn | Leaves the screen; the turn keeps running in the service |
| Stop button | Interrupts after the current tool finishes, or at once if streaming text |
| Notification Allow / Deny | Same as the approval bar |
| Hardware keyboard | Enter sends, Shift+Enter new line, Ctrl+K command sheet, Esc interrupts |

Haptics: a light tick on Allow, Deny, and send. No haptics on streaming or automatic events.

Critical flows that must have UI tests (code-standards.md section 7): send and stream, stop, approve, deny with reason, always allow, plan to run, diff hunk keep and undo, rewind each option, model switch mid-session, Bypass on and off.

## 6. Feedback states

| State | Display |
|---|---|
| Loading a list | Skeleton rows matching the real layout; never a full-screen spinner |
| Waiting for first token | "Thinking" label with a running indicator in the transcript, plus elapsed seconds after 5 s |
| Streaming | Text appears in place with a caret at the end |
| Tool running | Card header running indicator; live output for runs |
| Awaiting approval | Approval bar; notification if backgrounded |
| Empty | One sentence of what goes here and one action to fill it |
| Error, retryable | Inline block in `danger-soft` with what happened, what to do, and **Retry** |
| Error, needs settings | Same block with a button to the right settings screen |
| Offline | Banner under the status line: "Offline. The turn will continue when you reconnect." |
| Rate limited | Inline: "Anthropic rate limit. Retrying in 20 s." with **Retry now** |
| Budget reached | Inline block: "Session budget of $2.00 reached." with **Continue** and **Raise budget** |
| Interrupted by system | Block: "Stopped while running `npm test`." with **Resume** and **Discard** |
| Success after action | Snackbar for undoable actions only; otherwise the state change is the feedback |
| Short confirmation | Toast, such as "Copied". No action, gone after 2 s |

A snackbar has an action. It uses the inverse surface: `text` as the background, `bg` as the text, `tonal` for the action. It sits above the bottom edge. A toast has no action. It is a pill on `surface-raised` with a hairline `line-input` border and `text` as the text, so it follows the theme instead of inverting it. It sits at the top, below the status bar. Both follow the app theme (System, Light, Dark), not the device's, sit on `layer.snackbar`, and are announced politely to TalkBack. The app never calls the platform `Toast`, because it ignores the app theme.

## 7. Writing rules

- Follow the writing rules and slop list in project-overview.md.
- Sentence case everywhere.
- Buttons are verbs: "Allow", "Run this plan", "Clone". Never "OK" or "Submit".
- Errors say what happened and what to do: "Key rejected by OpenAI. Check the key in Settings." Never "Something went wrong."
- Name things exactly: the provider, the file, the command, the branch.
- Numbers with units and no false precision: "$0.21", "34%", "12 s".
- No exclamation marks, no emoji, no "Oops".
- Placeholders show an example of real input: "Fix the failing login test".
- Agent-facing system prompt text follows the same rules; the agent's tone in replies is set to be direct and brief.

## 8. Accessibility

- [ ] All text meets the contrast rules in ui-tokens.md section 3.
- [ ] Touch targets at least 48 × 48 dp, including chips and hunk buttons.
  Exception: the Copy, Edit, and Regenerate row under a chat message uses 36 dp targets, because the actions are secondary and sit beside each other.
- [ ] Every icon-only control has a content description that names the action.
- [ ] Streaming text is announced politely by TalkBack once per completed paragraph, not per delta.
- [ ] Tool card state changes are announced: "Edit to app.py finished."
- [ ] Approval bar receives TalkBack focus when it appears and announces the action.
- [ ] Diff lines expose "added" or "removed" to TalkBack; `+` and `−` glyphs are always visible.
- [ ] Layouts work at 200% font scale without clipping; the status line wraps to two lines if needed.
- [ ] Reduced motion setting removes all non-essential animation.
- [ ] Full keyboard and switch access, with a visible focus ring.
- [ ] Color is never the only signal: every status has an icon and a label.
- [ ] Headings marked as headings for TalkBack navigation in settings and long cards.

## 9. Performance

- Transcript uses `LazyColumn` with stable keys and `contentType` per block kind.
- Text deltas update one `AnnotatedString` per block; batching happens in the core (32 ms).
- Code highlighting spans come from the core once per block and are cached.
- Long tool output renders only the visible collapsed lines; "Show all" opens a dedicated viewer for over 2,000 lines.
- Images in the transcript load as thumbnails at display size.
- No shadows or blur on scrolling content.
- Baseline profiles cover app start, opening a session, and scrolling the transcript.

## 10. Responsive layouts

| Width | Session layout | Diff and files |
|---|---|---|
| Compact | Single pane | Full-screen route |
| Medium | Single pane, wider margins | Side sheet from the right, 60% width |
| Expanded | Transcript left, detail right | Detail pane shows diff, file, or run |
| Large | Sessions list, transcript, detail | Same as expanded |

Rotation and foldable posture changes keep the session state and scroll position. In horizontal orientation on a phone, the composer collapses to one line until focused.

<!-- slop-lint:off -->

## 11. Do and don't

| Do | Don't |
|---|---|
| "Edited `src/auth.py`  +12 −3" | "I've made some changes to improve your authentication!" |
| Accent only on Allow, primary buttons, active mode, focus | Accent headers, decorative accent icons |
| Show the full command in the approval bar | "The agent wants to run a command" |
| "Key rejected by Gemini. Check the key in Settings." | "Oops! Something went wrong." |
| Collapse six reads into "Read 6 files" | Six separate cards for six reads |
| Red status line in Bypass | A small icon that is easy to miss |
| Skeleton rows while loading the session list | A centered spinner on an empty screen |
| "Run this plan" | "Let's go!" |
| Text summary above each chart | A chart with no explanation |

## 12. AI slop checklist for visuals and copy

Check every new screen and string against this list before merge.

| Tell | Tether's rule |
|---|---|
| Purple or rainbow gradients | No gradients anywhere |
| Sparkle or magic-wand icons for AI features | Icons name the action (file, terminal, branch) |
| Glass, blur, glowing borders | Flat tonal surfaces only |
| Every card with a border and a shadow | Tone separates; borders only on inputs |
| Oversized rounded corners on everything | Radius scale in ui-tokens.md; cards 12 dp |
| Generic stock illustrations or 3D blobs in empty states | One sentence and one action; no illustration |
| Emoji in headings, buttons, or lists | None |
| Chat bubbles for agent output | Full-width blocks; only user messages are inset |
| "Welcome back!" greetings and cheerful filler | Open on the work: last session and composer |
| Vague buttons ("Get started", "Explore", "Learn more") | Verbs naming the result ("Clone repo", "Add key") |
| Banned words from project-overview.md | `slop-lint` fails the build |
| Fake numbers in mockups or empty states | Real or clearly sample data, labeled as sample |
| Centered everything | Left-aligned text; center only single-line empty states |
| Animated typing effects for non-streamed text | Text appears at once unless it is actually streaming |
| Inconsistent icon styles mixed from several sets | Lucide only |

<!-- slop-lint:on -->
