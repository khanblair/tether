# UI tokens

Status: draft 0.2, 18 Nov 2026. Every design value lives in `tokens/tokens.json`. This doc explains them. `tokens/generate.ts` produces `Tokens.kt` and the Android color resources now, and the site CSS on demand. Swift comes with iOS in v2. No value may be hardcoded in UI code.

## 1. Design direction

**Velvet Plum.** The palette has four colors, taken from the Velvet Plum swatch:

| Name | Hex | Role |
|---|---|---|
| `mint-100` | `#C6F0E4` | Light theme: soft backgrounds behind accent content, the launcher icon background |
| `mint-200` | `#A7E8D7` | Light theme: tonal fills such as selected chips. Dark theme: the accent |
| `plum-700` | `#64165D` | Light theme: the accent. Dark theme: tonal fills |
| `plum-500` | `#8A176E` | Secondary emphasis in the light theme; its lightened form is used in the dark theme |

Every other color is derived from these four or is a status color (danger, success, warning, info) or a syntax color.

**The accent means "your move."** The accent is used only where the user needs to act or is acting: the primary button, the approval bar, the active mode chip, focus rings, and selected states. In the light theme the accent is plum. In the dark theme it is mint. The two themes swap which brand hue is the accent, so that the accent is always the strongest color on screen. Everything the agent does on its own stays neutral.

**Mint-tinted neutrals in light, plum-black neutrals in dark.** Light surfaces carry a faint mint tint instead of pure white or gray. Dark surfaces are a deep plum instead of neutral black. The tint keeps long sessions soft on the eyes and ties every surface back to the palette.

**Type does the work.** IBM Plex Sans for interface text and IBM Plex Mono for code, paths, and commands. The two share proportions, so mixed lines like "Edited `src/app.py`" sit well together. Both ship inside the app (SIL Open Font License, see `android/designsystem/FONTS-LICENSE.txt`).

**No dynamic color.** Material You wallpaper colors are switched off. The brand palette is the same on every device.

**What this avoids, and why:**

| Generic default | Why we avoid it | Tether instead |
|---|---|---|
| Blue-to-purple gradients and glow | The common AI product look; says nothing about the product | Flat tonal surfaces; the plum and mint pair carries the identity |
| Material baseline purple and default shapes | Reads as unstyled | Custom tokens on Material components |
| Inter or Roboto everywhere | Generic; Roboto Mono and Inter do not pair well | Plex Sans and Plex Mono as a family |
| Glassmorphism, blur, glowing borders | Expensive to render on mid-range phones; hurts contrast | Tonal surfaces and one soft shadow |
| Chat bubbles for agent output | Implies small talk; wastes width | Full-width transcript blocks; only user messages are inset |
| Sparkle icons for AI | Carries no meaning | Icons that name the action: file, terminal, branch |
| Color as the only status signal | Fails color-blind users | Icon plus label plus color |

## 2. Color

Names below are token names. In Kotlin they are camel case: `color.accent-ink` is `TetherTokens.light.color.accentInk` or `TetherTokens.dark.color.accentInk`. In CSS they are `--tt-color-accent-ink`. In Android XML they are `@color/tt_accent_ink`, with a night variant.

### Neutrals

| Token | Light | Dark | Use |
|---|---|---|---|
| `color.bg` | `#F3FAF7` | `#140A13` | Screen background |
| `color.surface` | `#FFFFFF` | `#1E111D` | Cards, sheets, composer |
| `color.surface-sunken` | `#E8F4EF` | `#0E060D` | Tool output, inset areas |
| `color.surface-raised` | `#FFFFFF` | `#281827` | Menus, dialogs, approval bar |
| `color.code-bg` | `#EEF6F2` | `#180D17` | Code blocks and diff gutters |
| `color.text` | `#1F1020` | `#F4EAF3` | Primary text |
| `color.text-2` | `#4A3A49` | `#CDBCCB` | Secondary text, icons |
| `color.text-3` | `#6A5868` | `#A28F9F` | Captions, metadata, placeholders |
| `color.text-disabled` | `#A99BA8` | `#6A586A` | Disabled labels (exempt from contrast rules) |
| `color.line` | `#DCEBE5` | `#33223A` | Dividers (decorative only) |
| `color.line-input` | `#8C7A8B` | `#8A748A` | Input and control borders |

### Accent

| Token | Light | Dark | Use |
|---|---|---|---|
| `color.accent` | `#64165D` | `#A7E8D7` | Fills: primary button, approval bar stripe, active chip |
| `color.on-accent` | `#FFFFFF` | `#0B2B24` | Text and icons on `color.accent` |
| `color.accent-ink` | `#64165D` | `#A7E8D7` | Accent as text, icon, focus ring, or selected state on neutral surfaces |
| `color.accent-soft` | `#C6F0E4` | `#17352F` | Background tint behind accent content |
| `color.accent-2` | `#8A176E` | `#F0A3DC` | Secondary emphasis as text or icon: section markers, links to related items. Never a fill |
| `color.on-accent-2` | `#FFFFFF` | `#3A0D36` | Text on an `accent-2` fill, where one is needed |
| `color.tonal` | `#A7E8D7` | `#64165D` | Quiet fills: selected chips, filled tonal buttons, navigation selection |
| `color.on-tonal` | `#0B3B31` | `#FFD6F4` | Text and icons on `color.tonal` |

### Status

| Token | Light | Dark | Use |
|---|---|---|---|
| `color.danger` | `#B42318` | `#F97066` | Errors, destructive actions, Bypass status line |
| `color.danger-soft` | `#FBE9E7` | `#3A1714` | Error backgrounds |
| `color.success` | `#1F6B3A` | `#6CCB8F` | Passed runs, connected status |
| `color.success-soft` | `#E6F4EA` | `#133022` | Success backgrounds |
| `color.warning` | `#8A5A00` | `#F3BE63` | Warnings, with a triangle icon and a label |
| `color.warning-soft` | `#FBEBD0` | `#3A2A10` | Warning backgrounds |
| `color.info` | `#3B6FD8` | `#8DB0F5` | Links in running text, informational notes |

A warning is not an action, so it has its own color instead of sharing the accent. `color.accent-2` (hue 315) and `color.danger` (hue 4) are far apart on the color wheel, so magenta emphasis is never read as an error. Status is still never signaled by color alone.

### Diff

| Token | Light | Dark | Use |
|---|---|---|---|
| `color.diff-add-bg` | `#E6F4EA` | `#133022` | Added line background |
| `color.diff-add-mark` | `#1F6B3A` | `#6CCB8F` | `+` glyph and gutter |
| `color.diff-del-bg` | `#FBE9E7` | `#3A1714` | Removed line background |
| `color.diff-del-mark` | `#B42318` | `#F97066` | `-` glyph and gutter |
| `color.diff-word-add` | `#C7E8D1` | `#1D4A33` | Changed words inside an added line |
| `color.diff-word-del` | `#F5CFCA` | `#5A231E` | Changed words inside a removed line |

### Syntax

| Token | Light | Dark |
|---|---|---|
| `syntax.keyword` | `#8A3FA0` | `#D7A5E8` |
| `syntax.string` | `#1F6B3A` | `#9AD9AE` |
| `syntax.number` | `#9A4A00` | `#F2B47A` |
| `syntax.comment` | `#6B5F69` | `#A99BA6` |
| `syntax.function` | `#2F5BB8` | `#9CB8F2` |
| `syntax.type` | `#7A5200` | `#E9C27A` |
| `syntax.variable` | `#1F1020` | `#F4EAF3` |
| `syntax.punctuation` | `#4A3A49` | `#CDBCCB` |

Syntax colors are the one place a second hue family appears; they never appear outside code. Every syntax color meets 4.5:1 on `code-bg`, `diff-add-bg`, and `diff-del-bg` in its theme. The lowest measured values are 5.17 (light, comment on `diff-del-bg`) and 5.38 (dark, comment on `diff-add-bg`).

## 3. Contrast rules

`node tokens/generate.ts --check` measures every pair listed in the `contrast` array of `tokens.json` and fails CI if one falls below its requirement. `node tokens/generate.ts --contrast` prints the table, including all syntax pairs. Measured ratios (WCAG 2.x):

| Pair | Light | Dark | Requirement |
|---|---|---|---|
| `text` on `bg` | 17.19 | 16.54 | 4.5 |
| `text-2` on `bg` | 9.95 | 10.76 | 4.5 |
| `text-3` on `surface` | 6.54 | 6.04 | 4.5 |
| `text-3` on `surface-sunken` | 5.80 | 6.63 | 4.5 |
| `text-3` on `surface-raised` | 6.54 | 5.57 | 4.5 |
| `on-accent` on `accent` | 11.57 | 10.95 | 4.5 |
| `accent-ink` on `surface` | 11.57 | 13.15 | 4.5 |
| `accent-ink` on `bg` | 10.93 | 14.01 | 4.5 |
| `accent-ink` on `accent-soft` | 9.36 | 9.56 | 4.5 |
| `accent-2` on `surface` | 8.65 | 9.52 | 4.5 |
| `on-accent-2` on `accent-2` | 8.65 | 8.50 | 4.5 |
| `on-tonal` on `tonal` | 9.01 | 8.91 | 4.5 |
| `danger` on `surface` | 6.57 | 6.54 | 4.5 |
| `danger` on `danger-soft` | 5.61 | 5.74 | 4.5 |
| `success` on `surface` | 6.52 | 9.17 | 4.5 |
| `warning` on `surface` | 5.93 | 10.72 | 4.5 |
| `warning` on `warning-soft` | 5.05 | 8.14 | 4.5 |
| `info` on `surface` | 4.72 | 8.38 | 4.5 |
| `line-input` on `surface` | 3.98 | 4.29 | 3.0 (controls) |
| `accent-ink` on `surface` as a focus ring | 11.57 | 13.15 | 3.0 |
| `diff-add-mark` on `diff-add-bg` | 5.74 | 7.18 | 4.5 |
| `diff-del-mark` on `diff-del-bg` | 5.61 | 5.74 | 4.5 |
| `text` on `code-bg` | 16.56 | 16.15 | 4.5 |
| `chart.series-1` to `series-4` on `surface` | 4.72 to 11.57 | 6.04 to 13.15 | 3.0 |
| `chart.axis-text` on `surface` | 6.54 | 6.04 | 4.5 |

Rules:
1. Body text and icons that carry meaning meet 4.5:1. Large text (18 sp regular or 14 sp semibold and up) meets 3:1.
2. Control boundaries and focus indicators meet 3:1 against their background.
3. `color.tonal` and `color.accent-soft` are quiet fills. In the light theme they are only 1.31:1 and 1.17:1 against `bg`, so they never carry a state alone. A selected chip also shows a check icon and a label.
4. `text-3` is the lightest text allowed on any surface. `text-disabled` is only for disabled controls.
5. A new color pair is added to the `contrast` array in `tokens.json` before it is used. The check then measures it and this table gets a row.

## 4. Typography

Fonts: IBM Plex Sans (UI), IBM Plex Mono (code). Bundled weights: Sans 400, 500, 600; Mono 400, 500. Sizes in sp so system font scaling applies.

| Token | Font | Size / line height | Weight | Letter spacing | Use |
|---|---|---|---|---|---|
| `type.display` | Sans | 28 / 34 | 600 | -0.2 | Onboarding headline only |
| `type.title-l` | Sans | 22 / 28 | 600 | -0.1 | Screen titles on large layouts |
| `type.title-m` | Sans | 18 / 24 | 600 | 0 | Top bar titles, sheet titles |
| `type.title-s` | Sans | 15 / 20 | 600 | 0 | Card titles, section headers |
| `type.body-l` | Sans | 16 / 24 | 400 | 0 | Agent text in the transcript |
| `type.body` | Sans | 15 / 22 | 400 | 0 | Default text |
| `type.body-s` | Sans | 13 / 18 | 400 | 0 | Secondary text, descriptions |
| `type.label` | Sans | 14 / 20 | 500 | 0.1 | Buttons, chips, tabs |
| `type.caption` | Sans | 12 / 16 | 400 | 0.2 | Metadata, timestamps, status line |
| `type.mono` | Mono | 14 / 20 | 400 | 0 | Code blocks, commands |
| `type.mono-s` | Mono | 12 / 18 | 400 | 0 | Diff lines, tool output, paths in captions |
| `type.mono-label` | Mono | 13 / 18 | 500 | 0 | Inline file names in tool card headers |

Rules: no font below 12 sp. Line length in the transcript caps at 72 characters on wide screens through `layout.transcript-max`.

## 5. Spacing

4 dp base scale. In Kotlin: `TetherSpace.space4`.

| Token | dp |
|---|---|
| `space.0` | 0 |
| `space.0_5` | 2 |
| `space.1` | 4 |
| `space.2` | 8 |
| `space.3` | 12 |
| `space.4` | 16 |
| `space.5` | 20 |
| `space.6` | 24 |
| `space.8` | 32 |
| `space.10` | 40 |
| `space.12` | 48 |
| `space.16` | 64 |

Defaults: screen horizontal padding `space.4`; gap between transcript blocks `space.4`; inside cards `space.3`; between a label and its control `space.1`.

## 6. Radius

| Token | dp | Use |
|---|---|---|
| `radius.none` | 0 | Diff lines, full-bleed code |
| `radius.xs` | 4 | Inline code, small badges |
| `radius.sm` | 8 | Chips, inputs, buttons |
| `radius.md` | 12 | Cards, tool call cards, user messages |
| `radius.lg` | 20 | Top corners of bottom sheets |
| `radius.full` | 999 | Avatars, circular icon buttons, the send button |

## 7. Borders and elevation

| Token | Value | Use |
|---|---|---|
| `border.hairline` | 1 dp `color.line` | Dividers between list rows |
| `border.input` | 1 dp `color.line-input` | Text fields, unselected chips |
| `border.input-focus` | 2 dp `color.accent-ink` | Focused input |
| no shadow (not a token) | none | Most surfaces; separation by tone |
| `elevation.1` | `0 1 2 rgba(31,16,32,0.06)`; dark: none, use `surface-raised` | Cards on `bg` |
| `elevation.2` | `0 4 16 rgba(31,16,32,0.10)`; dark: `0 4 16 rgba(0,0,0,0.40)` | Menus, composer when content scrolls under it |
| `elevation.3` | `0 16 40 rgba(31,16,32,0.18)`; dark: `0 16 40 rgba(0,0,0,0.55)` | Dialogs, bottom sheets |

In Kotlin, `TetherElevation.level1` to `level3` are 1, 4, and 12 dp. The shadow tints are in the CSS output.

Rule: separate surfaces by tone first, shadow second, border last. Cards in the transcript use no border.

## 8. Focus

| Token | Value |
|---|---|
| `focus.ring-width` | 2 dp |
| `focus.ring-offset` | 2 dp |
| ring color | `color.accent-ink` (uses that token; no separate focus color token) |

Every focusable element shows the ring when focused by keyboard or switch access. Touch focus does not show the ring.

## 9. Motion

| Token | Value | Use |
|---|---|---|
| `motion.instant` | 0 ms | Reduced motion replacement |
| `motion.fast` | 90 ms | Press states, chip toggles |
| `motion.base` | 150 ms | Expand and collapse, fades |
| `motion.slow` | 220 ms | Card entry, pane changes |
| `motion.sheet` | 300 ms | Bottom sheets and dialogs |
| `ease.standard` | cubic-bezier(0.2, 0, 0, 1) | Most transitions |
| `ease.exit` | cubic-bezier(0.3, 0, 1, 1) | Elements leaving |
| `ease.emphasized` | cubic-bezier(0.05, 0.7, 0.1, 1) | Sheets entering |
| `motion.stream-caret` | 1,000 ms blink period | Streaming caret |

Rules: no motion longer than 300 ms. No bouncing or spring overshoot. With the system "remove animations" setting on, all durations become `motion.instant` except progress indicators.

## 10. Layout sizes

| Token | dp | Use |
|---|---|---|
| `size.touch-min` | 48 | Minimum touch target |
| `size.icon` | 24 | Standard icon |
| `size.icon-s` | 20 | Icons in dense rows and tool card headers |
| `size.icon-xs` | 16 | Inline status icons |
| `size.top-bar` | 56 | Top app bar |
| `size.status-line` | 28 | Status line under the top bar |
| `size.composer-min` | 56 | Composer collapsed height |
| `ratio.composer-max` | 0.4 of window height | Composer expanded height |
| `size.approval-bar-min` | 72 | Approval bar height |
| `size.tool-card-header` | 44 | Tool card collapsed height |
| `ratio.sheet-max` | 0.9 of window height | Bottom sheet maximum |
| `layout.transcript-max` | 720 | Max transcript width on wide screens |
| `layout.pane-list` | 320 | List pane width on expanded layouts |
| `layout.pane-detail-min` | 360 | Minimum detail pane width |

## 11. Breakpoints

Based on Android window size classes, by window width:

| Token | Width | Layout |
|---|---|---|
| `breakpoint.compact` | under 600 dp | Single pane |
| `breakpoint.medium` | 600 to 839 dp | Single pane with wider margins; diff opens as a side sheet |
| `breakpoint.expanded` | 840 to 1199 dp | Two panes: transcript and diff or files |
| `breakpoint.large` | 1200 dp and up | Three panes: sessions, transcript, diff or files |

## 12. Layers

| Token | Order | Contents |
|---|---|---|
| `layer.base` | 0 | Screen content |
| `layer.sticky` | 10 | Pinned checklist, status line |
| `layer.composer` | 20 | Composer and approval bar |
| `layer.sheet` | 30 | Bottom sheets, side sheets |
| `layer.dialog` | 40 | Dialogs |
| `layer.snackbar` | 50 | Snackbars |
| `layer.overlay` | 60 | Debug overlays (debug builds only) |

## 13. Icons

Status: planned. P0 uses text labels. The set below arrives with the first feature screens.

- Set: Lucide, converted to `ImageVector`, 24 dp grid, stroke width 1.75.
- Default color `color.text-2`; active or selected `color.accent-ink`.
- Icons for agent actions: `file-text` read, `file-pen` edit, `file-plus` write, `search` grep and glob, `terminal` run, `git-branch` branch, `git-commit` commit, `upload` push, `git-pull-request` PR, `play` CI, `globe` web, `list-checks` checklist, `message-circle-question` ask user, `plug` MCP, `shield` permission mode, `undo-2` rewind, `book-open` skill, `brain` memory, `network` vault map.
- Status icons: `circle-check` success, `circle-x` failure, `triangle-alert` warning, `loader` running (rotates; static under reduced motion), `circle-pause` waiting.
- Every icon-only button has a content description.

The launcher icon and the logo are separate from this set. The logo source is `images/logo.svg`. The Android adaptive icon is a vector foreground (`ic_launcher_foreground.xml`) on `mint-100`, plus a single color monochrome layer for themed icons on Android 13 and later.

## 14. Charts

Used only on the `/cost` screen and Linux mode disk usage.

| Token | Light | Dark |
|---|---|---|
| `chart.series-1` | `#64165D` | `#A7E8D7` |
| `chart.series-2` | `#3B6FD8` | `#8DB0F5` |
| `chart.series-3` | `#1F6B3A` | `#6CCB8F` |
| `chart.series-4` | `#6A5868` | `#A28F9F` |
| `chart.grid` | `#DCEBE5` | `#33223A` |
| `chart.axis-text` | `#6A5868` | `#A28F9F` |

Rules: bar and line charts only. Every chart has a text summary above it and a data table behind a "Show data" toggle. Series are also told apart by pattern or direct labels, not color alone. In the dark theme `series-1` (mint) and `series-3` (green) are close, so they must never appear as neighbors without a pattern or label.

## 15. Themes and generated outputs

Themes: System (default), Light, Dark. The theme follows the system setting unless the user overrides it in Settings. In P0 the override is held in memory only; persistence arrives with settings storage in P1.

`tokens.json` is the source. Generated outputs:

| Output | Path | Committed |
|---|---|---|
| Kotlin tokens | `android/designsystem/src/main/kotlin/app/tether/design/tokens/Tokens.kt` | Yes; CI checks it is current |
| Android colors, light | `android/designsystem/src/main/res/values/tokens_colors.xml` | Yes; same check |
| Android colors, dark | `android/designsystem/src/main/res/values-night/tokens_colors.xml` | Yes; same check |
| Site CSS | `site-dist/assets/tokens.css` | No; built by `tools/site/build.ts` |

The XML colors exist so the window background (`@color/tt_bg`) matches the theme before Compose draws the first frame.

The CSS form uses `--tt-` names. The shape of the output, abbreviated:

```css
:root {
  color-scheme: light dark;
  --tt-color-bg: #F3FAF7;
  --tt-color-accent: #64165D;
  --tt-color-accent-ink: #64165D;
  --tt-font-sans: "IBM Plex Sans", system-ui, sans-serif;
  --tt-space-4: 16px;
  --tt-radius-md: 12px;
  --tt-motion-base: 150ms;
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --tt-color-bg: #140A13;
    --tt-color-accent: #A7E8D7;
    --tt-color-accent-ink: #A7E8D7;
  }
}

:root[data-theme="dark"] { /* same values as the dark block above */ }

@media (prefers-reduced-motion: reduce) {
  :root { --tt-motion-base: 0ms; }
}
```

The docs site uses this CSS, so the site follows the same light and dark themes as the app.

## 16. Changing tokens

1. Edit `tokens/tokens.json`.
2. Run `just tokens`. It fails if any listed contrast pair drops below its requirement.
3. If a color changed, update the contrast table in section 3 from `node tokens/generate.ts --contrast`.
4. Update screenshot goldens and say so in the PR, once screenshot tests exist (P1-19).
5. Add a line to the doc changes log in progress-tracker.md.
