// Area 3: the session screen, its states, and the pickers and sheets that belong to it.
area("session", "Session", "The transcript is the product. Every other screen feeds it (composer, pickers) or explains something in it (diff, files, runs). Model, mode, context, and cost stay visible in the status line.");
area("sheets", "Session sheets and pickers", "Everything opened from the composer, status line, or a message. Sheets keep the session visible behind them so the user never loses their place.");
area("states", "Session states and errors", "Every failure says what happened and what to do, in the transcript, not in a toast that vanishes.");

const U = (t) => `<div class="msg-user t-body">${t}</div>`;
const A = (t, caret) => `<div class="msg-agent">${t}${caret ? '<span class="caret"></span>' : ""}</div>`;
const reads = () => toolCard({ icon: "search", verb: "Read 4 files, searched 2 times", status: "ok" });
const editCard = () => toolCard({ icon: "file-pen", verb: "Edited", target: "src/auth/session.py", status: "ok", dur: "0.1 s", body: `<div class="files"><div class="f"><span class="ell">src/auth/session.py</span><span><span class="add">+1</span> <span class="del">−1</span></span></div></div>` });
const pytestOut = `<div class="out"><span class="badge">Sandbox</span>\ntests/test_login.py ..F.                  [ 80%]\n================= FAILURES =================\n_____ test_token_expires_at_boundary _____\n<span class="more">… 12 lines hidden · Show all</span></div>`;
const runRunning = () => toolCard({ icon: "terminal", verb: "Running", target: "pytest tests/test_login.py -q", status: "run", body: pytestOut });
const checklist = () => `<div class="checklist"><div class="row" style="justify-content:space-between"><b class="t-title-s">Plan · 2 of 4</b><span class="c3">${I("chevron-up", "s")}</span></div>
  <div class="it c3">${I("circle-check", "s")}<s>Find the expiry check</s></div><div class="it c3">${I("circle-check", "s")}<s>Fix the boundary comparison</s></div>
  <div class="it">${I("loader", "s spin")}Run the login tests</div><div class="it c3">${I("circle-dot", "s")}Add a regression test</div></div>`;

const sessionScreen = (o) => screen(
  appbar({ menu: "sessions", title: o.title || "Fix login timeout", sub: "acme-api · main", actions: [{ icon: "file-diff", label: "Review changes", go: "diff" }, { icon: "ellipsis-vertical", label: "Session menu", go: "sheet-session-menu" }] }) +
  statusline(o.sl || {}) + (o.banner || "") + (o.checklist || "") +
  `<div class="body" style="position:relative">${o.transcript}</div>${o.jump ? `<div class="jump" role="button">${I("arrow-down", "s")}2 new</div>` : ""}` + (o.bottom || ""),
);

const T_BASE = U("The login test fails when a token expires at exactly the boundary second. Find the bug and fix it.") + A("Looking at the expiry check first.") + reads() +
  A("<code>is_expired</code> compares with <code>&lt;</code>, so a token is still valid at its exact expiry second. The test expects it to expire then.") + editCard();

frame({
  id: "session-idle", area: "session", title: "New session",
  why: "Open on the work. Show what the agent already knows, so the user can trust it or fix it before typing.",
  has: ["Context loaded: instructions, memory snapshot, skills index", "Composer with the prompt typed and Send ready", "Status line already shows model, mode, context, cost"],
  config: ["Model and mode default from the project", "Instructions: TETHER.md, then AGENTS.md, then CLAUDE.md", "Memory snapshot is frozen at session start"],
  render: () => sessionScreen({ title: "New session", sl: { ctx: 3, cost: "0.00" }, transcript: `<div class="transcript"><div class="card col gap12"><b class="t-title-s">Loaded for this session</b>
    <div class="row"><span class="c2">${I("file-text")}</span><div class="grow"><b class="t-body">TETHER.md</b><div class="t-body-s c3">12 lines · test command: pytest -q</div></div></div>
    <div class="row"><span class="c2">${I("brain")}</span><div class="grow"><b class="t-body">Memory</b><div class="t-body-s c3">6 entries · applies from this session on</div></div></div>
    <div class="row"><span class="c2">${I("book-open")}</span><div class="grow"><b class="t-body">Skills</b><div class="t-body-s c3">14 available, loaded when a task needs them</div></div></div></div>
    <div class="t-body-s c3 center">Last session in this project: Fix login timeout · yesterday</div></div>`, bottom: composer({ text: "Fix the failing login test", sendGo: "session-live" }) }),
});

frame({
  id: "session-running", area: "session", title: "Session, running",
  why: "Watch the agent work without it shouting. Only the newest running card shows a running indicator; nothing else animates.",
  has: ["Inset user message, full-width agent text, no chat bubbles", "Read-only tool calls grouped: Read 4 files, searched 2 times", "Edit card with file and +1 −1; run card with live output and executor badge", "Pinned checklist; Stop replaces Send", "Jump to latest with new-item count"],
  config: ["Tool cards collapse output to 12 + 12 lines, Show all opens a viewer", "Back leaves the screen; the turn keeps running in the foreground service"],
  render: () => sessionScreen({ checklist: checklist(), transcript: `<div class="transcript">${T_BASE}${runRunning()}${A("Waiting for the test run to finish", true)}</div>`, jump: true, bottom: composer({ running: true }) }),
});

frame({
  id: "session-approval", area: "session", title: "Approval needed",
  why: "The signature element. The exact command, the executor, and one clear Allow under the thumb. Accent is used here and almost nowhere else.",
  has: ["Approval bar with a 4 dp accent stripe", "Exact command in mono, up to 3 lines, expandable", "Deny (text), Always allow (shows the rule), Allow (primary)", "The composer stays usable to deny with a reason"],
  config: ["Permission mode decides which calls ask", "Always allow creates a rule per project, per executor, per pattern", "Notification shows the same Allow and Deny when the app is in the background"],
  render: () => sessionScreen({ checklist: checklist(), transcript: `<div class="transcript">${T_BASE}${toolCard({ icon: "terminal", verb: "Run", target: "pip install -e . && pytest -q", status: "wait" })}</div>`, bottom: approval({ title: "Run in Linux mode", command: "pip install -e . &amp;&amp; pytest -q", badge: badge("Linux mode", "tonal"), note: "Needs a package install, so the sandbox cannot run it. Network is on for this command.", allowGo: "session-diffcard", denyGo: "sheet-deny" }) + composer({}) }),
});

frame({
  id: "session-plan", area: "session", title: "Plan mode",
  why: "Review the approach before anything changes. Plan mode reads and proposes; it cannot edit or run.",
  has: ["Plan card: numbered steps, files to touch, commands to run, open questions", "Run this plan (primary), Edit plan, Keep planning", "Mode chip reads Plan"],
  config: ["Run this plan switches to the project's default mode", "The plan becomes the pinned checklist"],
  render: () => sessionScreen({ sl: { mode: "Plan", ctx: 12, cost: "0.04" }, transcript: `<div class="transcript">${U("Add rate limiting to /v1/search. Plan first.")}${A("I read the router and the middleware stack. Here is the plan.")}<div class="card col gap12"><div class="row"><span class="c2">${I("list-checks")}</span><b class="t-title-s">Plan · 4 steps</b></div>
    <ol class="t-body" style="margin:0;padding-left:20px"><li>Add a token bucket in <code class="inline-code">src/limits.py</code></li><li>Wrap <code class="inline-code">search_handler</code> with the limiter</li><li>Return 429 with <code class="inline-code">Retry-After</code></li><li>Add three tests in <code class="inline-code">tests/test_limits.py</code></li></ol>
    <div class="t-body-s c2"><b>Runs:</b> <span class="mono">pytest -q</span></div><div class="t-body-s c2"><b>Open question:</b> Limit per API key or per IP? I assumed per key.</div>
    <div class="row" style="flex-wrap:wrap">${btn("Run this plan", { go: "session-running" })}${btn("Edit plan", { kind: "text" })}${btn("Keep planning", { kind: "plain" })}</div></div></div>`, bottom: composer({ mode: "Plan" }) }),
});

frame({
  id: "session-ask", area: "session", title: "Agent asks a question",
  why: "When the agent needs a decision it asks with tappable options instead of guessing or a wall of text.",
  has: ["Question with 2 to 4 options", "Other opens the composer", "Answer is recorded as a user message"],
  config: ["The agent pauses the turn until answered; the notification shows the same options"],
  render: () => sessionScreen({ transcript: `<div class="transcript">${T_BASE}<div class="card col gap12"><div class="row"><span class="c2">${I("message-circle-question")}</span><b class="t-title-s">Which should change?</b></div>
    <div class="t-body c2">The test asserts expiry at <code class="inline-code">t</code>, but the API docs say a token is valid through <code class="inline-code">t</code>.</div>
    ${btn("Fix the code to match the test", { kind: "outline", full: true })}${btn("Fix the test to match the docs", { kind: "outline", full: true })}${btn("Other", { kind: "text" })}</div></div>`, bottom: composer({}) }),
});

frame({
  id: "session-diffcard", area: "session", title: "Turn finished",
  why: "Close the loop: what changed, what the quality gate found, and the next move.",
  has: ["Diff card: Edited 3 files +42 −7, quality result, Review", "Message actions: Copy, Retry", "Next-step chips: Commit, Open PR"],
  config: ["Quality gate runs on each edit batch (smells and slop)", "Block-level findings show a red label and stop Commit in Auto mode"],
  render: () => sessionScreen({ sl: { cost: "0.38", ctx: 41 }, transcript: `<div class="transcript">${T_BASE}${toolCard({ icon: "terminal", verb: "Ran", target: "pytest -q", status: "ok", dur: "6.2 s", body: `<div class="out"><span class="badge">Linux mode</span>\n12 passed in 4.8s</div>` })}
    ${A("The boundary test passes now, and I added <code>test_token_expires_at_boundary</code> so it stays fixed.")}
    <div class="tool"><button class="hd"><span class="c2">${I("file-diff")}</span><span class="v">Edited 3 files</span><span class="tg"></span><span class="st"><span class="add">+42</span> <span class="del">−7</span></span></button>
    <div class="files"><div class="f"><span class="ell">src/auth/session.py</span><span><span class="add">+1</span> <span class="del">−1</span></span></div><div class="f"><span class="ell">tests/test_login.py</span><span class="add">+38</span></div><div class="f"><span class="ell">CHANGELOG.md</span><span><span class="add">+3</span> <span class="del">−6</span></span></div></div>
    <div class="ft"><span class="row cwarn t-body-s">${I("triangle-alert", "s")}2 warnings</span>${btn("Review", { kind: "text", sm: true, go: "diff" })}</div></div>
    <div class="row c3"><button class="ibtn" aria-label="Copy" style="width:40px;height:40px">${I("copy", "s")}</button><button class="ibtn" aria-label="Retry" style="width:40px;height:40px">${I("refresh-cw", "s")}</button></div>
    <div class="chips">${chip("Commit", { icon: "git-commit-horizontal", go: "commit" })}${chip("Open PR", { icon: "git-pull-request", go: "pr-draft" })}${chip("Review with another model", { icon: "eye", go: "review" })}</div></div>`, bottom: composer({}) }),
});

frame({
  id: "session-bypass", area: "session", title: "Bypass mode",
  why: "Never hide risk. Bypass turns the whole status line red, so it cannot be missed. Pushes still ask.",
  has: ["Red status line starting with the word Bypass", "Red outlined mode chip", "Approval bar only for pushes"],
  config: ["Turned on per session, never a default", "Needs biometric or device credential each time"],
  render: () => sessionScreen({ sl: { bypass: true, cost: "1.12", ctx: 58 }, transcript: `<div class="transcript">${T_BASE}${toolCard({ icon: "terminal", verb: "Ran", target: "pip install -e . && pytest -q", status: "ok", dur: "9.1 s" })}${toolCard({ icon: "upload", verb: "Push", target: "origin fix/login-timeout", status: "wait" })}</div>`, bottom: approval({ icon: "upload", title: "Push to origin", command: "git push origin fix/login-timeout", note: "Pushes always ask, even in Bypass.", allowGo: "pr-done" }) + composer({ mode: "Bypass" }) }),
});

// ----- states and errors -----
const stateBlock = (kind) => ({
  rate: `<div class="card" style="background:var(--tt-color-warning-soft)"><div class="row"><span class="cwarn">${I("clock")}</span><div class="grow"><b class="t-title-s cwarn">Anthropic rate limit</b><div class="t-body-s cwarn">Retrying in 20 s.</div></div>${btn("Retry now", { kind: "text", sm: true })}</div></div>`,
  budget: `<div class="card" style="background:var(--tt-color-warning-soft)"><div class="col gap12"><div class="row"><span class="cwarn">${I("circle-dollar-sign")}</span><div class="grow"><b class="t-title-s cwarn">Session budget of $2.00 reached</b><div class="t-body-s cwarn">The agent paused after the last tool finished.</div></div></div><div class="row">${btn("Continue", { sm: true })}${btn("Raise budget", { kind: "text", sm: true, go: "set-budgets" })}</div></div></div>`,
  interrupted: `<div class="card"><div class="col gap12"><div class="row"><span class="c2">${I("circle-pause")}</span><div class="grow"><b class="t-title-s">Stopped while running <span class="mono">npm test</span></b><div class="t-body-s c2">Android ended the app to free memory. Your files are safe.</div></div></div><div class="row">${btn("Resume", { sm: true })}${btn("Discard", { kind: "plain", sm: true })}</div></div></div>`,
  key: `<div class="card" style="background:var(--tt-color-danger-soft)"><div class="col gap12"><div class="row"><span class="cdanger">${I("circle-x")}</span><div class="grow"><b class="t-title-s cdanger">Key rejected by Gemini</b><div class="t-body-s cdanger">Check the key in Settings. The turn did not start.</div></div></div><div class="row">${btn("Open provider settings", { sm: true, go: "set-provider-detail" })}</div></div></div>`,
  compact: `<div class="row c3 t-caption" style="justify-content:center;gap:10px"><hr class="hr grow">${I("layers", "s")}<span>Context compacted at 80%. 41k tokens summarized.</span><hr class="hr grow"></div>`,
  thinking: `<div class="row c2">${I("loader", "s spin")}<span class="t-body">Thinking</span><span class="t-body-s c3">· 7 s</span></div><div class="t-body-s c3">Still thinking. Large contexts can take up to 20 s.</div>`,
}[kind]);

const stateFrame = (id, title, why, kind, opts = {}) => frame({
  id, area: "states", title, why, has: opts.has || [], config: opts.config || [],
  render: () => sessionScreen({ sl: opts.sl, banner: opts.banner, transcript: `<div class="transcript">${opts.base === false ? "" : T_BASE}${stateBlock(kind)}</div>`, bottom: composer({ disabled: opts.disabled }) }),
});
stateFrame("state-thinking", "Waiting for the first token", "First pixel under 400 ms: the shell, a Thinking label, and Stop appear at once. After 5 s the elapsed time shows.", "thinking", { has: ["Thinking label with indicator", "Elapsed seconds after 5 s", "Stop available"], config: ["None"], base: false });
stateFrame("state-ratelimit", "Rate limited", "Name the provider, say when it retries, offer Retry now.", "rate", { has: ["Provider name and wait time", "Retry now"], config: ["Backoff is automatic: 5 s, 20 s, 60 s"] });
stateFrame("state-offline", "Offline", "The turn continues when the phone reconnects. A banner under the status line says so.", "interrupted", { banner: `<div class="banner">${I("wifi-off", "s")}Offline. The turn will continue when you reconnect.</div>`, has: ["Banner under the status line", "Inline interruption block with Resume"], config: ["Resume is automatic on reconnect; manual button stays"], });
stateFrame("state-budget", "Budget reached", "Money is a first-class limit. The agent pauses; the user decides.", "budget", { sl: { cost: "2.00", ctx: 63 }, has: ["Named limit and amount", "Continue and Raise budget"], config: ["Limits per session, day, and month in Settings > Budgets"] });
stateFrame("state-interrupted", "Interrupted by the system", "Android can kill a background app. Say what stopped and offer Resume or Discard.", "interrupted", { has: ["What was running", "Resume and Discard"], config: ["State is saved per event, so resume continues the same tool call"] });
stateFrame("state-key", "Key rejected", "Errors that need settings include a button to the right settings screen.", "key", { has: ["Provider named", "Open provider settings"], config: ["None"] });
stateFrame("state-compaction", "Context compacted", "At 80% context the older messages become a summary, with a visible marker so nothing disappears silently.", "compact", { sl: { ctx: 29 }, has: ["Marker in the transcript", "Context meter drops"], config: ["Threshold 80% (changeable)", "/compact [focus] does this on demand"] });

// ----- sheets -----
const over = (id, title, why, base, html, has, config) => frame({ id, area: "sheets", title, why, overlay: base, overlayHtml: html, has, config });

// Drawer: header with the theme toggle, the destinations, then sessions. The toggle shows the current theme and names the next one.
const drawerTheme = { system: ["monitor", "System", "Light"], light: ["sun", "Light", "Dark"], dark: ["moon", "Dark", "System"] };
const drawerHead = () => {
  const [icon, now, next] = drawerTheme[(window.CFG && CFG.theme) || "system"] || drawerTheme.system;
  return `<div class="row" style="padding:8px 8px 8px 16px"><img src="logo.svg" width="24" height="24" alt=""><b class="t-title-m grow">Tether</b><button class="ibtn" data-act="theme" aria-label="Theme: ${now}. Switch to ${next}.">${I(icon, "l")}</button></div>`;
};
const drawerDest = (icon, title, go, o = {}) => `<button class="lrow" style="min-height:48px;padding:8px 12px;margin:0 8px;width:calc(100% - 16px);border-radius:var(--tt-radius-sm);${o.on ? "background:var(--tt-color-tonal);color:var(--tt-color-on-tonal);" : ""}"${o.on ? ' aria-current="page"' : ""}${nav({ go })}><span class="ico" style="${o.on ? "color:var(--tt-color-on-tonal);" : ""}">${I(icon)}</span><span class="tx"><b>${title}</b></span>${o.count ? `<span class="t-body-s c2">${o.count}</span>` : ""}</button>`;

over("sessions", "Projects and sessions drawer", "The drawer is the only navigation. There is no bottom bar; the composer owns the bottom edge.", "home",
  () => `<div class="scrim"></div><div class="drawer"><div style="height:32px"></div>${drawerHead()}
  <div class="body"><div style="padding:4px 0 8px">${drawerDest("house", "Home", "home", { on: true })}${drawerDest("folder", "Projects", "projects")}${drawerDest("bell", "Needs attention", "attention", { count: "2" })}${drawerDest("chart-column", "Usage and cost", "cost")}${drawerDest("activity", "Runs", "runs")}</div>
  <div style="height:1px;background:var(--tt-color-line);margin:0 16px 8px"></div>
  <div class="pad" style="margin:4px 0 8px">${field("", "", { icon: "search", ph: "Search sessions" })}</div>
  <div class="sec-title" style="margin-top:4px">acme-api</div>
  ${[["Fix login timeout", "run", "Running"], ["Add rate limit to /v1/search", "ok", "PR #42"], ["Migrate to Pydantic v2", "wait", "Needs approval"]].map(([t, s, sub]) => `<button class="lrow" style="min-height:52px"${nav({ go: "session-running" })}><span class="ico">${statusDot(s)}</span><span class="tx"><b>${t}</b><span>${sub}</span></span></button>`).join("")}
  <div class="sec-title">tether</div><button class="lrow" style="min-height:52px"><span class="ico c3">${I("circle-check", "s")}</span><span class="tx"><b>Add UniFFI event path</b><span>Merged · 2 h ago</span></span></button>
  </div>
  <div class="pad col gap12" style="padding-bottom:8px">${btn("New session", { icon: "plus", full: true, go: "session-idle" })}<div class="row">${btn("Settings", { kind: "plain", icon: "settings", go: "settings" })}${btn("Skills and memory", { kind: "plain", icon: "brain", go: "skills" })}</div></div></div>`,
  ["Destinations: Home, Projects, Needs attention with its count, Usage and cost, Runs", "Theme toggle in the header: System, Light, Dark", "Sessions grouped by project with status icons", "Search across session titles and text", "New session and Settings at the thumb"], ["Archive a session by swipe, with an Undo snackbar, or from the row menu"]);

over("sheet-model", "Model picker", "Models differ in price, context, and tools. Show all three before the user picks.", "session-running",
  () => sheet("Model", `<div class="col gap12">${field("", "", { icon: "search", ph: "Search models" })}
  ${[["Anthropic", [["Sonnet 5.5", "200k", "$3 · $15", true], ["Opus 5.5", "200k", "$15 · $75"], ["Haiku 5.5", "200k", "$1 · $5"]]], ["OpenAI", [["GPT-5", "400k", "$1.25 · $10"], ["GPT-5 mini", "400k", "$0.25 · $2"]]], ["Gemini", [["Gemini 2.5 Pro", "1M", "$1.25 · $10"]]], ["OpenRouter", [["qwen3-coder", "256k", "$0.30 · $1.20"]]]].map(([p, ms]) => `<div><div class="sec-title" style="margin:8px 0 6px">${p}</div>${group(ms.map(([n, c, pr, sel]) => `<button class="lrow" style="min-height:60px"><span class="tx"><b>${n}</b><span>${c} context · ${pr} per M tokens</span><span class="row" style="margin-top:4px;gap:4px">${badge("Tools")}${badge("Vision")}${n.includes("Opus") || n.includes("Sonnet") ? badge("Reasoning") : ""}</span></span>${sel ? `<span class="cacc">${I("circle-check")}</span>` : ""}</button>`))}</div>`).join("")}
  <div class="row" style="justify-content:space-between"><span class="t-body">Use as the default for acme-api</span>${toggle(true)}</div></div>`, { h: "86%" }),
  ["Search, grouped by provider", "Context window, price per million tokens (in · out), capability badges", "Per-project default toggle"], ["Provider keys decide which models appear", "Switching mid-session keeps the transcript; the next turn uses the new model"]);

over("sheet-mode", "Permission mode", "Five modes, each described in plain words, so nobody turns on more freedom than they meant to.", "session-running",
  () => sheet("Permission mode", `<div class="col gap12">${[["Plan", "Reads and proposes. Changes nothing.", 0], ["Ask", "Asks before every edit and every command.", 0], ["Accept edits", "Edits files without asking. Asks before commands.", 1], ["Auto", "Runs allowed commands without asking and asks for anything new. The quality gate blocks bad commits.", 0], ["Bypass", "Asks for nothing except pushes. Red status line. Needs biometric.", 0]].map(([t, d, s]) => `<button class="opt ${s ? "sel" : ""}"${nav({ go: t === "Bypass" ? "sheet-bypass" : null, back: t !== "Bypass" })}><span class="rad"></span><span class="grow"><b class="t-title-s ${t === "Bypass" ? "cdanger" : ""}">${t}</b><span class="t-body-s c2" style="display:block">${d}</span></span></button>`).join("")}<div class="t-body-s c3">Applies to this session. The project default is Accept edits.</div></div>`, { h: "80%" }),
  ["Plan, Ask, Accept edits, Auto, Bypass with one-line explanations", "Selected mode highlighted; Bypass in red"], ["Default per project", "Bypass confirm: biometric or device credential"]);

over("sheet-bypass", "Confirm Bypass", "Irreversible-feeling choices get a deliberate confirmation that states the consequence.", "session-running",
  () => sheet("Turn on Bypass for this session?", `<div class="col gap16"><div class="card" style="background:var(--tt-color-danger-soft)"><div class="t-body cdanger">The agent will edit files and run commands without asking. It still asks before pushing. Every action is logged in the transcript.</div></div>${btn("Confirm with fingerprint", { full: true, icon: "fingerprint", kind: "tonal", go: "session-bypass" })}${btn("Cancel", { kind: "plain", full: true, back: true })}</div>`, { close: false }),
  ["Plain statement of what changes", "Biometric confirm", "Cancel"], ["Off again at session end"]);

over("sheet-context", "Context", "Running out of context mid-task is a common failure. Show what fills it and offer to fix it.", "session-running",
  () => sheet("Context", `<div class="col gap16"><div class="row" style="justify-content:space-between"><span class="t-title-l">34%</span><span class="t-body-s c3">68k of 200k tokens</span></div>
  <div class="row" style="height:12px;gap:2px;border-radius:6px;overflow:hidden"><i style="flex:6;background:var(--tt-chart-series-1)"></i><i style="flex:2;background:var(--tt-chart-series-2)"></i><i style="flex:9;background:var(--tt-chart-series-3)"></i><i style="flex:34;background:var(--tt-chart-series-4)"></i><i style="flex:51;background:var(--tt-color-surface-sunken)"></i></div>
  ${group([["System prompt", "4.1k"], ["Instructions and skills index", "2.6k"], ["Tool definitions", "5.8k"], ["Files read", "22k"], ["Conversation", "33.5k"]].map(([a, b]) => `<div class="lrow" style="min-height:44px"><span class="tx"><b class="t-body">${a}</b></span><span class="mono t-mono-s c2">${b}</span></div>`))}
  <div class="row" style="justify-content:space-between"><span class="t-body">Compact automatically at 80%</span>${toggle(true)}</div>${btn("Compact now", { kind: "outline", full: true, icon: "layers" })}</div>`),
  ["Percentage and tokens", "Breakdown list (the bar repeats it with color)", "Auto-compaction toggle and Compact now"], ["Threshold (default 80%)", "Focus text for /compact"]);

over("sheet-always", "Always allow", "Show the exact rule that will be created. No surprises.", "session-approval",
  () => sheet("Always allow", `<div class="col gap12">${opt("Allow this command pattern in acme-api", "Linux mode · asks again for anything else", true, `<div class="inline-code mono t-mono-s mt8" style="display:block">run: pip install -e . &amp;&amp; pytest *</div>`)}${opt("Allow pytest anywhere", "Any project, any executor", false)}${opt("Allow every command in Linux mode this session", "Until the session ends", false)}${btn("Create rule and allow", { full: true, go: "session-diffcard" })}<div class="t-body-s c3">Rules can be removed in Settings > Permissions.</div></div>`),
  ["Three scopes with the literal rule", "Create rule and allow"], ["Stored per project in .tether/config.toml"]);

over("sheet-deny", "Deny with a reason", "A denial teaches the agent. Quick reasons are one tap.", "session-approval",
  () => sheet("Deny and tell the agent why", `<div class="col gap12"><div class="chips">${["Wrong directory", "Too risky", "Use the sandbox instead", "Not now"].map((c) => chip(c)).join("")}</div>${field("", "", { ph: "Or write a reason", focus: true })}${btn("Deny", { full: true, kind: "tonal", back: true })}</div>`),
  ["Quick reason chips", "Free text"], ["None"]);

over("sheet-msg", "Message actions", "Long-press is never the only way: the same actions exist in the message overflow.", "session-diffcard",
  () => sheet("Message", group([row({ icon: "copy", title: "Copy", chev: false, back: true, snack: "toast:Copied" }), row({ icon: "undo-2", title: "Rewind to here", sub: "Restore files, conversation, or both", go: "sheet-rewind", chev: false }), row({ icon: "pencil", title: "Edit and resend", sub: "User messages only", chev: false })])),
  ["Copy", "Rewind to here", "Edit and resend"], ["Rewind opens the checkpoint sheet"]);

over("sheet-plus", "Attach and insert", "One plus button for everything the composer can take.", "session-idle",
  () => sheet("Add to message", group([row({ icon: "image", title: "Photo", sub: "From your gallery", chev: false }), row({ icon: "paperclip", title: "File", sub: "From this phone", chev: false }), row({ icon: "at-sign", title: "Mention a file", sub: "Insert @path from this project", go: "sheet-mention", chev: false }), row({ icon: "square-slash", title: "Commands", sub: "/compact, /plan, /review and more", go: "sheet-slash", chev: false })])),
  ["Photo, file, mention, commands"], ["Images only enabled for models with vision"]);

over("sheet-slash", "Commands", "Type / for a filtered list above the keyboard. Same names as desktop harnesses, so habits transfer.", "session-idle",
  () => sheet("Commands", `<div class="col gap12">${field("", "/", { focus: true, mono: true })}${group([["/compact", "Summarize older context now"], ["/plan", "Switch to Plan mode"], ["/diff", "Open uncommitted changes"], ["/rewind", "Open the checkpoint list"], ["/commit", "Write a message and commit"], ["/review", "Second-model review"], ["/skills", "List and open skills"], ["/memory", "Show memory files"], ["/vault", "Open the vault"]].map(([c, d]) => `<button class="lrow" style="min-height:48px"><span class="tx"><b class="mono t-mono-label">${c}</b><span>${d}</span></span></button>`))}<div class="sec-title" style="margin:0">Custom, from .tether/commands</div>${group([`<button class="lrow" style="min-height:48px"><span class="tx"><b class="mono t-mono-label">/release-notes</b><span>Draft release notes from merged PRs</span></span></button>`])}</div>`, { h: "80%" }),
  ["Filtered by what is typed", "Built-in and custom commands"], ["Custom commands are Markdown files in .tether/commands; $ARGUMENTS is replaced"]);

over("sheet-mention", "Mention a file", "A mention attaches the file's content to the message.", "session-idle",
  () => sheet("Mention a file", `<div class="col gap12">${field("", "@auth", { focus: true, mono: true })}${group([["src/auth/session.py", "Recent"], ["src/auth/tokens.py", ""], ["tests/test_login.py", "Recent"], ["docs/auth.md", ""]].map(([p, b]) => `<button class="lrow" style="min-height:48px"><span class="ico">${I("file-text")}</span><span class="tx"><b class="mono t-mono-label">${p}</b></span>${b ? badge(b) : ""}</button>`))}</div>`),
  ["Fuzzy search over the project index", "Recent files first"], ["Ignored files are hidden"]);

over("sheet-voice", "Voice input", "Dictation fills the composer, editable before sending. Nothing is sent automatically.", "session-idle",
  () => sheet("Listening", `<div class="col gap16 center"><div class="row" style="justify-content:center;height:48px;gap:4px">${[10, 22, 34, 18, 40, 26, 14, 30, 20, 8].map((h) => `<i style="width:5px;height:${h}px;border-radius:3px;background:var(--tt-color-accent-ink)"></i>`).join("")}</div><div class="t-body-l">Fix the failing login test and add a regression test</div><div class="row" style="justify-content:center">${btn("Cancel", { kind: "plain", back: true })}${btn("Insert text", { back: true })}</div></div>`),
  ["Live transcript", "Insert text or Cancel"], ["Uses the Android speech recognizer; permission asked on first use"]);

over("sheet-session-menu", "Session menu", "Management actions live in one menu, with the destructive one last and named.", "session-running",
  () => sheet("Session", group([row({ icon: "pencil", title: "Rename", chev: false }), row({ icon: "share-2", title: "Export as Markdown", chev: false }), row({ icon: "history", title: "Checkpoints", sub: "3 edit batches", go: "sheet-rewind", chev: false }), row({ icon: "archive", title: "Archive", chev: false, back: true, snack: "Session archived|Undo" }), row({ icon: "trash-2", title: "Delete this session", danger: true, chev: false })])),
  ["Rename, export, checkpoints, archive, delete"], ["Delete asks for confirmation that names the session"]);
