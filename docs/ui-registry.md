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
| `SegmentedControl` | Two to four exclusive options | `options`, `selected`, `onSelect`, `label` | each selected, disabled | P1 | Built |
| `Chip` | Compact selectable label | `label`, `icon`, `selected`, `onClick` | default, selected, disabled | P1 | Planned |

## Navigation and structure

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `TetherTopBar` | Top app bar | `title`, `subtitle`, `navigationIcon`, `actions` | default (scrolled shadow is planned) | P0 | Built |
| `HomeDrawer` | Header with the theme toggle, destinations (Home, Projects, Needs attention, Usage and cost, Runs), session search and sessions by project, an "N archived sessions · Show" line that lists the archived sessions with Unarchive (not in the prototype; chosen by Blair), then New session, Settings, Skills and memory. Lives in the app module | `sessions`, `archived`, `query`, `currentRoute`, `attentionCount`, `themeMode`, `onToggleTheme`, `onDestination`, `onOpenSession`, `onNewSession`, `onOpenSettings`, `onOpenSkills` | with sessions, no sessions, search with no match, each destination selected | P1 | Built |
| `DrawerItem` | Drawer destination row; the current one is `tonal` | `label`, `icon`, `selected`, `onClick`, `badge` | default, selected, pressed, with badge | P1 | Built |
| `ThemeToggleButton` | Icon button that shows the theme choice (monitor, sun, moon) and cycles System, Light, Dark | `mode`, `contentDescription`, `onClick` | system, light, dark | P1 | Built |
| `NotificationBell` | Bell in the top bar; an accent dot when something needs the user | `hasAttention`, `contentDescription`, `onClick` | quiet, attention | P1 | Built |
| `SectionTitle` | Small heading above a list group | `text` | default | P1 | Built |
| `ListGroup` | Bordered container for stacked rows | `content` | default | P1 | Built |
| `ProjectRow` | Project: folder icon, name, branch, summary, optional badge | `name`, `branch`, `summary`, `onClick`, `badge` | default, pressed, with badge | P1 | Built |
| `SessionRow` | Session: status mark, title, subtitle | `title`, `subtitle`, `state` (`SessionState`), `onClick` | each state | P1 | Built |
| `ListRow` | Generic row: icon, title, subtitle, trailing | `title`, `subtitle`, `leading`, `trailing`, `onClick`, `enabled`, `danger` | default, pressed, disabled, danger | P1 | Built |
| `TetherSheet` | Modal bottom sheet with a title. It opens fully expanded and its content rises above the keyboard | `title`, `onDismiss`, `content` | default | P1 | Built |
| `ModelSheet` | Pick the session's model from the saved lists of providers with a key, with a search field (every word must appear in the name or id) above the list and the thinking effort below it. App module | `state`, `running`, `onQueryChange`, `onChoose`, `onOpenProviders`, `onDismiss` | list, empty, no match, disabled while a turn runs | P1 | Built |
| `ModeSheet` | Pick one of five permission modes; Bypass asks for a biometric check. App module | `state`, `running`, `onChoose`, `onConfirmBypass`, `onCancelBypass`, `onDismiss` | each mode selected, bypass confirm, disabled while a turn runs | P1 | Built |
| `SessionMenuSheet` | Session menu: Rename, Export as Markdown, Checkpoints (with the edit batch count), Archive, and Delete this session in the danger color. Archive and Delete are off with a reason while a turn runs. App module | `state`, `running`, `onRename`, `onExport`, `onCheckpoints`, `onArchive`, `onDelete`, `onDismiss` | default, running, failed | P1 | Built |
| `SessionDeleteSheet` | Confirmation that names the session before it is deleted for good. App module | `state`, `onConfirm`, `onDismiss` | loading, ready, failed | P1 | Built |
| `RewindSheet` | Checkpoints with three restore scopes and a confirmation for file scopes. App module | `checkpoints`, `scope`, `onRewind` | list, empty, confirm, error | P2 | Built |
| `FilesScreen` | Lazy file tree with name and text search and an ignored-files toggle. App module | `viewModel`, `onOpenFile` | loading, tree, empty folder, folder error, truncated listing, ignored shown | P2 | Built |
| `FileViewScreen` | Read-only file text with line numbers, horizontal scroll, and Copy path. App module | `viewModel`, `path` | text, truncated, binary, error | P2 | Built |
| `TurnFooter` | The blocks under the transcript: rejected key, rate limit, other failure, stopped, and the thinking label. App module | `turn`, `providerName`, `onRetry`, `onOpenProviders`, `onDismissStopped` | each problem, stopped, thinking | P1 | Built |
| `MessageActionsSheet` | Long press on a message opens it; Copy only. The visible action row is the main path. App module | `onCopy` | default | P1 | Built |
| `OfflineBanner` | Banner under the status line while the phone has no network. App module. Code name: `NetworkBanner` | none | hidden, visible | P1 | Built |
| `SearchScreen` | Message search with scope chips and the match in bold. App module | `viewModel`, `hasProject`, `onBack`, `onOpenSession` | idle, loading, hits, nothing found | P1 | Built |
| `ImportProjectScreen` | Import a project ZIP: pick a file, name the project, import | `state`, `onChooseFile`, `onNameChange`, `onImport`, `onCancel`, `onImported` | idle, picked, importing, done, failed, name too long, too large | P2 | Built |
| `ExportProjectSheet` | Export a project as a ZIP, with an option to include ignored files | `state`, `onIncludeIgnoredChange`, `onChooseDestination`, `onTryAgain`, `onExported`, `onDismiss` | editing, exporting, failed | P2 | Built |
| `TetherDialog` | Confirmation dialog | `title`, `body`, `confirm`, `dismiss`, `destructive` | default, destructive | P1 | Planned |
| `TwoPaneScaffold` | Expanded and large layouts | `primary`, `detail`, `list` | compact, medium, expanded, large | P10 | Planned |

## Transcript

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `Transcript` | Lazy persisted message list with prose and fenced code. Code name: `TranscriptScreen` | `connection`, `sheets`, `onBack` | empty, load failure, long history | P1 | Built; frame timing unverified |
| `UserMessage` | User's message, inset | `text`, `attachments`, `onLongPress` | default, with images, rewound (dimmed) | P1 | Planned |
| `AgentText` | Streamed Markdown text from the agent | `blocks` (parsed in core), `streaming` | complete, streaming with caret | P1 | Planned |
| `CodeBlock` | Highlighted code with copy | `code`, `language`, `spans` | default, long (scrolls), copied | P1 | Planned |
| `ThinkingBlock` | The reasoning of one message as one collapsed row (brain icon, "Thinking" or "Thinking…" while it streams, chevron); tapping shows the text. State is kept per message. App module | `stateKey`, `text`, `live`, `startOpen` | collapsed, expanded, streaming | P1 | Built |
| `UserBubble` | The user's message as a bubble on the end side, at most 85% of the row, with Copy and Edit below it. App module | `message`, `onEdit`, `onCopy`, `onLongPress` | short, long, no text | P1 | Built |
| `MessageActionRow` | Icon row under a message: Copy, Edit, Regenerate. 16 dp icons in 36 dp buttons, 4 dp apart. App module | `onCopy`, `onEdit`, `onRegenerate` | user, agent, newest turn | P1 | Built |
| `EditMessageSheet` | Sheet with the message text and Send; replies after the message are removed and files stay as they are. App module | `text`, `onSend`, `onDismiss` | default, blank, unchanged | P1 | Built |
| `ToolCallCard` | One tool call with status and output | `call: ToolCallUi`, `expanded`, `onToggle`, `onRunElsewhere` | proposed, awaiting approval, running, succeeded, failed, timed out, denied, blocked | P2 | Built |
| `ToolGroupCard` | Collapsed group of read-only calls | `calls`, `expanded`, `onToggleGroup`, `onToggleCall` | collapsed, expanded | P2 | Built |
| `DiffCard` | Summary of an edit batch | `summary: DiffSummary`, `quality`, `onOpen`, `onReview` | clean, warnings, blocking | P2 | Planned |
| `PlanCard` | Plan from Plan mode | `plan`, `started`, `editing`, `onRun`, `onChooseMode` | default, editing | P3 | Built |
| `ChecklistPin` | Pinned todo list | `items`, `current`, `expanded`, `onToggleExpanded`, `onDismiss` | collapsed, expanded, all done | P3 | Built |
| `AskUserCard` | Agent's question with options | `question`, `answer`, `draft`, `onOption`, `onSend` | open, answered | P3 | Built |
| `RunCard` | Command run with live output | `call: ToolCallUi`, `expanded`, `onToggle`, `onRunElsewhere` | running, succeeded, failed, killed at limit | P5 | Built |
| `CiRunCard` | CI run status in the transcript | `run: CiRunView`, `onOpenGitHub` | queued, in progress, succeeded, failed, cancelled | P6 | Planned |
| `QualityReportCard` | Findings from the quality gate | `report`, `onOpenFinding` | none, info, warnings, blocking | P8 | Built |
| `ReviewCard` | Second-model review results | `review`, `onOpenFinding` | loading, findings, no findings | P8 | Planned |
| `PrCard` | Pull request status | `pr: PrView`, `onRefresh`, `onOpen` | open with checks pending, passing, failing, merged | P4 | Planned |
| `SystemNote` | Compaction, rewind, mode change markers | `kind` (`SystemNoteKind`) | compaction, rewind, mode change, model change | P2 | Built |
| `ErrorBlock` | Inline error with action | `title`, `body`, `action` | retryable, needs settings, budget reached, interrupted | P1 | Planned |
| `JumpToLatest` | Floating button shown whenever the last row is out of view: a round arrow button, or a pill with "N new" when rows arrived since the reader scrolled up; tapping scrolls to the newest row. App module | `pill`, `onClick` | hidden, icon, count | P1 | Built |

## Composer

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `Composer` | Message input with an inline placeholder, the bar (plus, mode chip, model chip, mic, send or stop), the staged photo row, and the `/` and `@` assist panel. App module | `modelLabel`, `mode`, `running`, `onSend`, `onStop`, `onOpenModel`, `onOpenMode`, `supportsImages`, `sendBlocked`, `assist` | empty, typing, multiline at max, turn running (Stop), Auto chip, Bypass chip, staged photo, send disabled | P1 | Built |
| `ComposerBar` | The row under the field: plus button, the two chips, mic button, and the send or stop disc. App module | `mode`, `modelLabel`, `running`, `canSend`, `onPlus`, `onOpenMode`, `onOpenModel`, `onVoice`, `onStop`, `onSend` | send, send disabled, stop | P1 | Built |
| `TetherChip` | Small pill with an optional leading icon and trailing chevron. Design system | `label`, `tone`, `leadingIcon`, `trailingChevron`, `onClick` | neutral, accent, danger | P1 | Built |
| `PlusSheet` | Add to message: Photo, Camera, File, Mention a file, Commands. Camera is added by decision and is not in the prototype. File shows a reason because the core cannot attach files yet. App module | `draft`, `supportsImages`, `onDismiss`, `onPick` | each row enabled or off with its reason | P1 | Built |
| `ListeningSheet` | Voice input bottom sheet: a 10 bar waveform, the live transcript, Cancel and Insert text. App module | `state`, `onAllow`, `onTryAgain`, `onInsert`, `onCancel` | requesting, listening, denied, unavailable, failed | P1 | Built |
| `ModeChip` | Shows and changes permission mode. Built as a `TetherChip` in the composer bar: accent for Auto, danger outline for Bypass | `mode`, `onClick` | plan, ask, accept edits, auto, bypass | P3 | Built |
| `ModelChip` | Shows and changes model. Built as a `TetherChip` with a chevron in the composer bar | `model`, `onClick` | default, model without tools (warning) | P1 | Built |
| `AttachmentRow` | Attached images and files. Built for one staged photo inside `Composer`; sending images waits for core image input (issue #207) | `attachments`, `onRemove` | none, one, many | P1 | In progress |
| `CommandSheet` | `/` command list with filter. The query comes from the view model | `commands`, `query`, `loaded`, `failed`, `onPick` | full, filtered, no match | P3 | Built |
| `MentionSheet` | `@` file picker with filter. The query comes from the view model | `files`, `loading`, `failed`, `onPick` | full, filtered, no match, loading | P3 | Built |
| `MentionChips` | Removable chips for the `@` files in the message | `paths`, `onRemove` | default | P3 | Built |
| `ComposeAssistPanel` | Command and mention assist shown over the composer | `deps`, `text`, `cursor`, `onEdit` | command list, mention list, no match, loading | P3 | Built |
| `VoiceInputButton` | Speech to text. Built as the mic icon button in `ComposerBar`, which opens `ListeningSheet` | `onVoice` | idle, sheet open | P1 | Built |
| `SendStopButton` | Send or interrupt. Built as a 40 dp disc inside `ComposerBar`: arrow-up on accent, square while running | `running`, `enabled`, `onSend`, `onStop` | send enabled, send disabled, stop | P1 | Built |
| `SessionTopBar` | Session top bar: menu icon (opens the drawer), title, project and branch subtitle, Review changes, Session menu. App module | `title`, `subtitle`, `onBack`, `onOpenDrawer`, `onReviewChanges`, `onMore` | with subtitle, title only | P1 | Built |
| `EffortSection` | Thinking effort (Model default, Low, Medium, High) at the bottom of the model sheet; shown only for models that report reasoning. Added by decision, not in the prototype. App module | `effort`, `enabled`, `onChoose` | each level, disabled while a turn runs | P3 | Built |
| `LastCrashCard` | The last crash report on About, with Copy and Clear. App module | `report`, `onCopy`, `onClear` | shown only when a report exists | P0 | Built |

## Approvals and modes

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `ApprovalBar` | Pending approval above the composer | `approval`, `busy`, `failure`, `onAllow`, `onDeny`, `onAlwaysAllow` | command, edit, network, push, expanded detail | P3 | Built |
| `AlwaysAllowMenu` | Shows the rule that will be created | `rule`, `busy`, `failure`, `onConfirm`, `onCancel` | default | P3 | Built |
| `DenyReasonSheet` | Optional reason for denial | `reason`, `busy`, `failure`, `onReasonChange`, `onSubmit`, `onCancel` | empty, filled | P3 | Built |
| `ModePicker` | Sheet listing the five modes with one-line descriptions | `current`, `onPick` | each mode selected | P3 | Planned |
| `BypassConfirm` | Biometric-gated confirmation | `onConfirmed` | prompt, failed | P3 | Planned |

## Status and meters

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `StatusLine` | Model, mode, context, cost. In Bypass: Bypass, model, context, cost on the danger fill. The context breakdown sheet behind the percentage is not built | `status`, `usage`, `modelName`, `onModelClick`, `onModeClick`, `onOpenCost` | normal, bypass, offline, near context limit | P1 | Built |
| `ContextMeter` | Context use with breakdown link | `percent`, `onOpenBreakdown` | under 60%, 60 to 80%, over 80% | P1 | Built |
| `CostMeter` | Session cost with budget | `costMicros`, `budgetMicros` | no budget, under budget, near, reached | P1 | Built |
| `CostScreen` | Cost over a chosen range, with a chart and tables | `state`, `onSelect`, `onRetry`, `onBack` | data, load failure | P9 | Built |
| `OfflineBanner` | Offline notice. Code name: `NetworkBanner` | none | hidden, visible | P9 | Built |
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
| `DiffView` | Full diff for a batch or the working tree. Unified layout only, no split view | `rows`, `wrap`, `listState` | unified, wrapped, unwrapped, empty | P2 | Built |
| `DiffFileChips` | File list with counts | `files`, `onPick` | default, many (scrolls) | P2 | Planned |
| `DiffHunk` | One hunk with Keep and Undo | `hunk`, `onKeep`, `onUndo` | pending, kept, undone | P2 | Planned |
| `DiffLine` | One line with gutter and marks | `line`, `wrap` | context, added, removed, with finding | P2 | Built |
| `InlineFinding` | Quality finding under a line | `finding` | info, warn, block | P8 | Planned |
| `CheckpointList` | Rewind targets | `checkpoints`, `onRestore` | list, empty | P2 | Planned |
| `RestoreOptionsSheet` | Files, conversation, or both | `onPick` | default | P2 | Planned |
| `FileTree` | Lazy project tree | `nodes`, `onOpen` | loading, full, ignored shown | P2 | Planned |
| `FileViewer` | Read-only highlighted file | `content`, `spans`, `onAsk` | default, large file, binary file notice | P2 | Planned |
| `GitScreen` | Git: Changes, Branches, and History tabs with refresh and a way into the diff | `state`, `onSelectTab`, `onRefresh`, `onOpenDiff`, `onBack` | changes, branches, history, loading, failed, not a repository | P4 | Built |
| `DiffScreen` | Diff for one file or all changes, staged or not, with a wrap toggle | `state`, `onSetStaged`, `onToggleWrap`, `onRetry`, `onBack` | wrapped, unwrapped, empty, failed, truncated | P4 | Built |

## Runs and environments

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `RunConsole` | Full-screen run output | `run`, `onKill`, `onRunElsewhere` | running, finished, killed | P5 | Planned |
| `ExecutorBadge` | Sandbox, Linux, or CI label | `executor` | sandbox, linux, ci | P5 | Built |
| `RuntimePackRow` | Sandbox runtime pack with install state | `pack`, `source`, `onInstall`, `onCancel` | not installed, downloading, ready, failed | P5 | Built |
| `CiRunsList` | Recent CI runs | `runs`, `onOpen` | loading, list, empty | P6 | Built |
| `LinuxSetupCard` | Linux mode setup and status | `state`, `source`, `onInstall`, `onRepair`, `onRemove` | not installed, each setup step, ready, broken, unavailable in `play` | P7 | Built |
| `TerminalView` | Linux shell rendering | `screen` (from core `vt100`), `onInput` | default, network off badge | P7 | Planned |
| `TerminalKeyRow` | Extra keys above the keyboard | `onKey`, `modifiers` | default, Ctrl held | P7 | Planned |

## Settings and providers

| Name | Purpose | Props | States | Phase | Status |
|---|---|---|---|---|---|
| `AppearanceScreen` | Live preview and controls for theme, interface size, text size, and accent color | `appearance`, `theme`, change lambdas, `onBack` | light, dark, 200% font, narrow, custom picker open | P1 | In progress |
| `AccentSwatchRow` | Built-in and custom accent color choices with selected state | `selectedId`, `customArgb`, `onSelect`, `onOpenCustom` | each swatch selected, custom selected, narrow scroll | P1 | In progress |
| `CustomAccentPicker` | Hue, saturation, and validated hex controls for a custom accent | `initialArgb`, `onAccentSelected` | valid hex, empty hex, invalid length, invalid characters, adjusted color | P1 | In progress |
| `ProviderRow` | Provider with key status: icon, name, subtitle, chevron | `name`, `subtitle`, `status` (`ProviderRowStatus`), `statusDescription`, `onClick` | unset, ok, bad key, network error, blocked | P1 | Built |
| `TestConnectionButton` | Outlined full-width button that starts a key test | `state` (`TestState`), `label`, `onTest`, `enabled` | idle, testing, ok, failed, disabled | P1 | Built |
| `ModelPicker` | Searchable model list with badges | `models`, `selected`, `onPick` | loading, list, filtered, no tools model disabled | P1 | Planned |
| `CapabilityBadges` | Tools, images, reasoning badges | `caps` | each combination | P1 | Planned |
| `GitHubAccountCard` | GitHub sign-in state | `status`, `onSignOut` | signed out, device code shown, signed in | P4 | Built |
| `DeviceCodePanel` | OAuth device code with copy and open | `grant`, `expiresAtMillis`, `onOpen` | waiting, expired | P4 | Built |
| `McpServerRow` | MCP server with an enable switch and a Test button. Tool toggles are on `McpServerDetailScreen` | `server`, `testing`, `testResult`, `onOpen`, `onToggle`, `onTest` | connected, error, disabled | P9 | Built |
| `BudgetEditor` | Session and daily budgets | `daily`, `session`, `canSave`, `saving`, `onDailyChange`, `onSessionChange`, `onSave` | none, set | P9 | Built |
| `BudgetsScreen` | Session and daily budget limits with save and remove | `state`, `onSave`, `onRetry`, `onBack` (plus the field callbacks) | default, saving, load failure | P9 | Built |
| `CostChart` | Bar chart of daily cost | `days` | data, empty | P9 | Built |
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
| `OnboardingPage` | One onboarding step | `step`, `onBack`, `content` | default | P10 | Built |
| `ProviderQuickAdd` | Add the first key | `providers`, `onAdd` | empty, one added | P10 | Planned |
| `ProjectQuickStart` | Clone, new, or import | `onPick` | default, GitHub not signed in | P10 | Planned |

## Icons

The full icon list and its meanings are in ui-tokens.md section 13. Icons are added to `designsystem/Icons.kt` only when a component in this registry uses them. The Appearance row uses `palette`. Do not add icons for decoration.

## Rules for changing entries

1. A new component needs a row here in the same PR, with all states listed.
2. Removing a state or prop updates the row and deletes its screenshot test.
3. From P1 on, set status to **Built** only when every listed state has a passing screenshot test in light, dark, and 200% font scale. The P0 components above are Built on a hand check and get their screenshot tests in P1.
4. Set **Changed** when a merged PR alters a Built component's look; set back to **Built** when goldens are updated.
5. Deprecated components keep their row until no code uses them, then the row is deleted.
6. Add a line to the doc changes log in progress-tracker.md.
