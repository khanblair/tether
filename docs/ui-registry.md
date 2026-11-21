# UI registry

Status: draft 0.2, 18 Nov 2026. Every UI component in the app, one row each. Check here before building a component; update the row when you build or change one.

## How to read an entry

| Column | Meaning |
|---|---|
| Name | Composable name, PascalCase |
| Purpose | What it is for, in one line |
| Props | Main parameters (state in, lambdas out). `modifier` is implied for all |
| States | Visual states it must render, each covered by a screenshot test |
| Phase | Build plan phase in which it is first needed |
| Status | Planned, In progress, Built, Changed (since last screenshot update), Deprecated |

All components start as **Planned**. Screenshot tests start with the first feature screens (P1). Until then, **Built** means the component exists and was checked by hand in the light and dark themes.

## Where components live

- Shared components: `android/designsystem/src/main/kotlin/app/tether/design/components/`
- Feature-only components: `android/feature-*/.../components/`
- A feature component used by a second feature moves to `designsystem` and its row moves to the matching group here.
- Screenshot tests: same module, `src/test/kotlin/.../screenshots/`, one test per state listed.

## Foundations

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `TetherTheme` | Applies tokens, color scheme, typography. Material dynamic color is off | `mode` (`ThemeMode`: System, Light, Dark) | light, dark, system | P0 | Built |
| `TetherSurface` | Tonal surface wrapper | `tone` (bg, surface, sunken, raised), `elevation` | each tone × light, dark | P0 | Planned |
| `TetherText` | Text bound to type tokens | `text`, `style` token, `color` token, `maxLines` | default, truncated | P0 | Planned |
| `MonoText` | Inline code, paths, commands | `text`, `size` (mono, mono-s, mono-label) | default, truncated middle for paths | P0 | Planned |
| `TetherIcon` | Lucide icon with token sizing; the icons live in `TetherIcons`: circle-check, circle-x, circle-dot, circle-pause, chevron-left, chevron-right, zap, trash-2, eye, eye-off, key-round, menu, search, bell, settings, folder, folder-open, git-branch, package, plus, brain, loader, x, arrow-left, house, sun, moon, monitor, chart-column, activity, chevron-down | `icon`, `contentDescription`, `tint`, `size` | default, tinted | P0 | Built |
| `FocusRing` | Keyboard focus indicator modifier | `shape` | unfocused, focused | P0 | Planned |

## Buttons and inputs

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `PrimaryButton` | The one accent action per screen | `label`, `onClick`, `enabled`, `loading` | enabled, pressed, disabled, loading, focused | P0 | Built |
| `SecondaryButton` | Neutral filled action | same as primary | same as primary | P0 | Planned |
| `TetherTextButton` | Low-emphasis action | `label`, `onClick`, `tone` (`TextButtonTone`: neutral, danger), `icon`, `enabled` | enabled, disabled, danger | P0 | Built |
| `TetherIconButton` | Icon-only action, 48 dp target | `icon`, `contentDescription`, `onClick`, `enabled`, `tint` | enabled, pressed, disabled | P1 | Built |
| `TetherSearchField` | Single-line search with a leading icon and a clear button | `value`, `onValueChange`, `placeholder`, `clearLabel` | empty, filled, focused | P1 | Built |
| `TetherTextField` | Single-line input | `value`, `onValueChange`, `label`, `placeholder`, `error` | empty, filled, focused, error, disabled | P1 | Planned |
| `SecretField` | Masked key input with paste and reveal. Reveal asks the caller for a biometric check | `value`, `onValueChange`, `label`, `revealed`, `onToggleReveal`, `onPaste`, `hint`, `error` | empty, masked, revealed, error | P1 | Built |
| `ToggleRow` | Setting with a switch | `title`, `description`, `checked`, `onCheckedChange` | on, off, disabled | P1 | Planned |
| `SelectRow` | Setting that opens a picker | `title`, `value`, `onClick` | default, disabled | P1 | Planned |
| `SegmentedControl` | Two to four exclusive options | `options`, `selected`, `onSelect` | each selected, disabled | P1 | Planned |
| `Chip` | Compact selectable label | `label`, `icon`, `selected`, `onClick` | default, selected, disabled | P1 | Planned |

## Navigation and structure

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `TetherTopBar` | Top app bar | `title`, `subtitle`, `navigationIcon`, `actions` | default (scrolled shadow is planned) | P0 | Built |
| `HomeDrawer` | Header with the theme toggle, destinations (Home, Projects, Needs attention, Usage and cost, Runs), session search and sessions by project, then New session, Settings, Skills and memory. Lives in the app module | `sessions`, `query`, `currentRoute`, `attentionCount`, `themeMode`, `onToggleTheme`, `onDestination`, `onOpenSession`, `onNewSession`, `onOpenSettings`, `onOpenSkills` | with sessions, no sessions, search with no match, each destination selected | P1 | Built |
| `DrawerItem` | Drawer destination row; the current one is `tonal` | `label`, `icon`, `selected`, `onClick`, `badge` | default, selected, pressed, with badge | P1 | Built |
| `ThemeToggleButton` | Icon button that shows the theme choice (monitor, sun, moon) and cycles System, Light, Dark | `mode`, `contentDescription`, `onClick` | system, light, dark | P1 | Built |
| `NotificationBell` | Bell in the top bar; an accent dot when something needs the user | `hasAttention`, `contentDescription`, `onClick` | quiet, attention | P1 | Built |
| `SectionTitle` | Small heading above a list group | `text` | default | P1 | Built |
| `ListGroup` | Bordered container for stacked rows | `content` | default | P1 | Built |
| `ProjectRow` | Project: folder icon, name, branch, summary, optional badge | `name`, `branch`, `summary`, `onClick`, `badge` | default, pressed, with badge | P1 | Built |
| `SessionRow` | Session: status mark, title, subtitle | `title`, `subtitle`, `state` (`SessionState`), `onClick` | each state | P1 | Built |
| `ListRow` | Generic row: icon, title, subtitle, trailing | `title`, `subtitle`, `leading`, `trailing`, `onClick`, `enabled` | default, pressed, disabled | P1 | Built |
| `TetherSheet` | Modal bottom sheet with a title | `title`, `onDismiss`, `content` | default | P1 | Built |
| `ModelSheet` | Pick the session's model from the saved lists of providers with a key. App module | `models`, `current`, `onPick`, `enabled` | list, empty, disabled while a turn runs | P1 | Built |
| `ModeSheet` | Pick one of five permission modes; Bypass asks for a biometric check. App module | `current`, `onPick`, `enabled` | each mode selected, bypass confirm, disabled while a turn runs | P1 | Built |
| `SessionMenuSheet` | Rename and Rewind for a session. App module | `onRename`, `onRewind` | default | P1 | Built |
| `RewindSheet` | Checkpoints with three restore scopes and a confirmation for file scopes. App module | `checkpoints`, `scope`, `onRewind` | list, empty, confirm, error | P2 | Built |
| `TurnFooter` | The blocks under the transcript: rejected key, rate limit, other failure, stopped, and the thinking label. App module | `problem`, `onRetry`, `onOpenProviders`, `onDismiss` | each problem, stopped, thinking | P1 | Built |
| `MessageActionsSheet` | Long press on a message opens it; Copy only for now. App module | `onCopy` | default | P1 | Built |
| `OfflineBanner` | Banner under the status line while the phone has no network. App module | `visible` | hidden, visible | P1 | Built |
| `SearchScreen` | Message search with scope chips and the match in bold. App module | `query`, `scope`, `hits` | idle, loading, hits, nothing found | P1 | Built |
| `TetherDialog` | Confirmation dialog | `title`, `body`, `confirm`, `dismiss`, `destructive` | default, destructive | P1 | Planned |
| `TwoPaneScaffold` | Expanded and large layouts | `primary`, `detail`, `list` | compact, medium, expanded, large | P10 | Planned |

## Transcript

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `Transcript` | Lazy persisted message list with prose and fenced code | `messages`, `listState` | empty, load failure, long history | P1 | Built; frame timing unverified |
| `UserMessage` | User's message, inset | `text`, `attachments`, `onLongPress` | default, with images, rewound (dimmed) | P1 | Planned |
| `AgentText` | Streamed Markdown text from the agent | `blocks` (parsed in core), `streaming` | complete, streaming with caret | P1 | Planned |
| `CodeBlock` | Highlighted code with copy | `code`, `language`, `spans` | default, long (scrolls), copied | P1 | Planned |
| `ThinkingBlock` | Collapsed model reasoning | `text`, `durationMs` | collapsed, expanded, streaming | P1 | Planned |
| `ToolCallCard` | One tool call with status and output | `call: ToolCallView`, `onExpand`, `onRunElsewhere` | proposed, awaiting approval, running, succeeded, failed, timed out, denied, blocked | P2 | Planned |
| `ToolGroupCard` | Collapsed group of read-only calls | `calls`, `summary` | collapsed, expanded | P2 | Planned |
| `DiffCard` | Summary of an edit batch | `summary: DiffSummary`, `quality`, `onOpen`, `onReview` | clean, warnings, blocking | P2 | Planned |
| `PlanCard` | Plan from Plan mode | `plan`, `onRun`, `onEdit`, `onKeepPlanning` | default, editing | P3 | Planned |
| `ChecklistPin` | Pinned todo list | `items`, `current`, `expanded` | collapsed, expanded, all done | P3 | Planned |
| `AskUserCard` | Agent's question with options | `question`, `options`, `onAnswer` | open, answered | P3 | Planned |
| `RunCard` | Command run with live output | `run: RunView`, `onOpenConsole` | running, succeeded, failed, killed at limit | P5 | Planned |
| `CiRunCard` | CI run status in the transcript | `run: CiRunView`, `onOpenGitHub` | queued, in progress, succeeded, failed, cancelled | P6 | Planned |
| `QualityReportCard` | Findings from the quality gate | `findings`, `onOpenFinding` | none, info, warnings, blocking | P8 | Planned |
| `ReviewCard` | Second-model review results | `review`, `onOpenFinding` | loading, findings, no findings | P8 | Planned |
| `PrCard` | Pull request status | `pr: PrView`, `onRefresh`, `onOpen` | open with checks pending, passing, failing, merged | P4 | Planned |
| `SystemNote` | Compaction, rewind, mode change markers | `kind`, `text` | compaction, rewind, mode change, model change | P2 | Planned |
| `ErrorBlock` | Inline error with action | `title`, `body`, `action` | retryable, needs settings, budget reached, interrupted | P1 | Planned |
| `JumpToLatest` | Floating button when scrolled up | `newCount`, `onClick` | hidden, visible with count | P1 | Planned |

## Composer

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `Composer` | Message input and controls | `state: ComposerState`, `onSend`, `onStop`, `onAttach` | empty, typing, multiline at max, turn running (Stop), disabled offline | P1 | Planned |
| `ModeChip` | Shows and changes permission mode | `mode`, `onClick` | plan, ask, accept edits, auto, bypass | P3 | Planned |
| `ModelChip` | Shows and changes model | `model`, `onClick` | default, model without tools (warning) | P1 | Planned |
| `AttachmentRow` | Attached images and files | `attachments`, `onRemove` | none, one, many | P1 | Planned |
| `CommandSheet` | `/` command list with filter | `commands`, `query`, `onPick` | full, filtered, no match | P3 | Planned |
| `MentionSheet` | `@` file picker with filter | `files`, `query`, `onPick` | full, filtered, no match, loading | P3 | Planned |
| `VoiceInputButton` | Speech to text | `listening`, `onToggle` | idle, listening, unavailable | P1 | Planned |
| `SendStopButton` | Send or interrupt | `running`, `enabled`, `onSend`, `onStop` | send enabled, send disabled, stop | P1 | Planned |

## Approvals and modes

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `ApprovalBar` | Pending approval above the composer | `request: ApprovalView`, `onAllow`, `onDeny`, `onAlways` | command, edit, network, push, expanded detail | P3 | Planned |
| `AlwaysAllowMenu` | Shows the rule that will be created | `rules`, `onPick` | default | P3 | Planned |
| `DenyReasonSheet` | Optional reason for denial | `onSubmit` | empty, filled | P3 | Planned |
| `ModePicker` | Sheet listing the five modes with one-line descriptions | `current`, `onPick` | each mode selected | P3 | Planned |
| `BypassConfirm` | Biometric-gated confirmation | `onConfirmed` | prompt, failed | P3 | Planned |

## Status and meters

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `StatusLine` | Model, mode, context, cost | `status: StatusView`, `onSegment` | normal, bypass, offline, near context limit | P1 | Planned |
| `ContextMeter` | Context use with breakdown link | `percent`, `onClick` | under 60%, 60 to 80%, over 80% | P1 | Planned |
| `CostMeter` | Session cost with budget | `cost`, `budget` | no budget, under budget, near, reached | P1 | Planned |
| `OfflineBanner` | Offline notice | `visible` | hidden, visible | P9 | Planned |
| `ProgressLine` | Thin indeterminate progress | `active` | idle, active | P1 | Planned |
| `RunningIndicator` | Small running icon | `label` | animated, reduced motion static | P1 | Planned |
| `StatusBadge` | Pill with a label and an optional icon | `label`, `tone` (`BadgeTone`: neutral, tonal, success, warning, danger), `icon` | each tone | P1 | Built |
| `DashboardCard` | Card on Home: optional title and action, then content | `title`, `action`, `content` | with title, without title, with action | P1 | Built |
| `StatRow` | Three stats side by side with a value, a label, and a plain comparison | `items` (`StatItem`) | one, three | P1 | Built |
| `UsageBarChart` | Bars of tokens per day with axis labels and a text description. Bars only | `values`, `labels`, `contentDescription` | all zero, one day, seven days | P1 | Built |
| `ShareRow` | A label, its share in numbers, and a thin bar | `label`, `share`, `fraction`, `color` | 0%, 62%, 100% | P1 | Built |
| `DataTable` | The table behind a chart, two columns | `headers`, `rows` | default | P1 | Built |
| `StatusMark` | Session state icon: running (turning), done, waiting, failed, idle. Decorative: the row text carries the meaning | `state` (`SessionState`) | each state, reduced motion static | P1 | Built |

## Diff and files

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `DiffView` | Full diff for a batch or the working tree | `files`, `layout` (unified, split), `wrap` | unified, split, wrapped, unwrapped, empty | P2 | Planned |
| `DiffFileChips` | File list with counts | `files`, `onPick` | default, many (scrolls) | P2 | Planned |
| `DiffHunk` | One hunk with Keep and Undo | `hunk`, `onKeep`, `onUndo` | pending, kept, undone | P2 | Planned |
| `DiffLine` | One line with gutter and marks | `line`, `wordSpans` | context, added, removed, with finding | P2 | Planned |
| `InlineFinding` | Quality finding under a line | `finding` | info, warn, block | P8 | Planned |
| `CheckpointList` | Rewind targets | `checkpoints`, `onRestore` | list, empty | P2 | Planned |
| `RestoreOptionsSheet` | Files, conversation, or both | `onPick` | default | P2 | Planned |
| `FileTree` | Lazy project tree | `nodes`, `onOpen` | loading, full, ignored shown | P2 | Planned |
| `FileViewer` | Read-only highlighted file | `content`, `spans`, `onAsk` | default, large file, binary file notice | P2 | Planned |

## Runs and environments

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `RunConsole` | Full-screen run output | `run`, `onKill`, `onRunElsewhere` | running, finished, killed | P5 | Planned |
| `ExecutorBadge` | Sandbox, Linux, or CI label | `executor` | sandbox, linux, ci | P5 | Planned |
| `RuntimePackRow` | Sandbox runtime pack with install state | `pack`, `onInstall` | not installed, downloading, ready, failed | P5 | Planned |
| `CiRunsList` | Recent CI runs | `runs`, `onOpen` | loading, list, empty | P6 | Planned |
| `LinuxSetupCard` | Linux mode setup and status | `status`, `onInstall`, `onRepair`, `onRemove` | not installed, each setup step, ready, broken, unavailable in `play` | P7 | Planned |
| `TerminalView` | Linux shell rendering | `screen` (from core `vt100`), `onInput` | default, network off badge | P7 | Planned |
| `TerminalKeyRow` | Extra keys above the keyboard | `onKey`, `modifiers` | default, Ctrl held | P7 | Planned |

## Settings and providers

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `ProviderRow` | Provider with key status: icon, name, subtitle, chevron | `name`, `subtitle`, `status` (`ProviderRowStatus`), `statusDescription`, `onClick` | unset, ok, bad key, network error, blocked | P1 | Built |
| `TestConnectionButton` | Outlined full-width button that starts a key test | `state` (`TestState`), `label`, `onTest`, `enabled` | idle, testing, ok, failed, disabled | P1 | Built |
| `ModelPicker` | Searchable model list with badges | `models`, `selected`, `onPick` | loading, list, filtered, no tools model disabled | P1 | Planned |
| `CapabilityBadges` | Tools, images, reasoning badges | `caps` | each combination | P1 | Planned |
| `GitHubAccountCard` | GitHub sign-in state | `state`, `onSignIn`, `onSignOut` | signed out, device code shown, signed in | P4 | Planned |
| `DeviceCodePanel` | OAuth device code with copy and open | `code`, `url`, `onOpen` | waiting, expired | P4 | Planned |
| `McpServerRow` | MCP server with tools | `server`, `onToggleTool` | connected, error, disabled | P9 | Planned |
| `BudgetEditor` | Session and daily budgets | `budgets`, `onChange` | none, set | P9 | Planned |
| `CostChart` | Bar chart of daily cost | `series`, `summary` | data, empty | P9 | Planned |
| `BatteryGuidance` | OEM-specific battery steps | `oem`, `onOpenSettings` | per OEM, generic | P9 | Planned |

## Feedback

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `EmptyState` | One sentence and one action | `text`, `action` | default | P1 | Planned |
| `SkeletonRow` | Loading placeholder row | `lines` | 1 line, 2 lines | P1 | Planned |
| `TetherSnackbarHost` | Snackbars (with an action) and toasts (without one) on the inverse surface of the app theme, announced politely | `hostState` | snackbar with action, toast, light, dark | P1 | Built |
| `InlineBanner` | Persistent notice in a screen, announced politely to TalkBack | `text`, `tone` (`BannerTone`: info, success, warning, danger), `title`, `actionLabel`, `onAction` | info, success, warning, danger | P1 | Built |

## Onboarding

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `OnboardingPage` | One onboarding step | `title`, `body`, `primary`, `onSkip` | default | P10 | Planned |
| `ProviderQuickAdd` | Add the first key | `providers`, `onAdd` | empty, one added | P10 | Planned |
| `ProjectQuickStart` | Clone, new, or import | `onPick` | default, GitHub not signed in | P10 | Planned |

## Icons

The full icon list and its meanings are in ui-tokens.md section 13. Icons are added to `designsystem/Icons.kt` only when a component in this registry uses them. Do not add icons for decoration.

## Rules for changing entries

1. A new component needs a row here in the same PR, with all states listed.
2. Removing a state or prop updates the row and deletes its screenshot test.
3. From P1 on, set status to **Built** only when every listed state has a passing screenshot test in light, dark, and 200% font scale. The P0 components above are Built on a hand check and get their screenshot tests in P1.
4. Set **Changed** when a merged PR alters a Built component's look; set back to **Built** when goldens are updated.
5. Deprecated components keep their row until no code uses them, then the row is deleted.
6. Add a line to the doc changes log in progress-tracker.md.
