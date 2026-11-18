// Areas 7 and 8: settings and cost.
area("settings", "Settings", "One place to see and change every rule the agent follows. Each group is one screen. Destructive actions sit at the bottom of their group, in the danger color, and name the thing they delete.");
area("cost", "Cost", "Never hide cost. The summary comes first, as a sentence; the chart second; the data table behind a toggle.");

const S = (id, title, why, sub, content, o = {}) => frame({
  id, area: o.area || "settings", title, why, has: o.has || [], config: o.config || [],
  render: () => screen(appbar({ back: o.back || "settings", title: o.barTitle || title, sub }) + body(`<div class="pad col gap16">${content()}</div>`) + (o.bottom || "")),
});
const tg = (label, sub, on) => row({ title: label, sub, chev: false, right: toggle(on) });

frame({
  id: "settings", area: "settings", title: "Settings",
  why: "Grouped list that mirrors the feature list. The local profile sits on top because it is the closest thing to an account.",
  has: ["Profile card (local, no account)", "Groups: Providers, Models, Permissions, Executors, GitHub, MCP, Skills and memory, Quality gate, Budgets, Appearance, Privacy, Data, About", "Network status shown where it matters"],
  config: ["On tablets the list stays on the left and each group opens on the right"],
  render: () => screen(appbar({ back: true, title: "Settings" }) + body(`<div class="pad col gap16">
    <button class="card row"${nav({ go: "set-profile" })} style="background:var(--tt-color-surface)"><span class="avatar">BA</span><span class="grow"><b class="t-title-s">Blair</b><div class="t-body-s c3">Local profile · blair@example.com</div></span><span class="c3">${I("chevron-right")}</span></button>
    ${group([row({ icon: "key-round", title: "Providers", sub: "Anthropic, OpenAI", go: "set-providers" }), row({ icon: "cpu", title: "Models and defaults", sub: "Sonnet 5.5 · reviewer Opus 5.5", go: "set-models" }), row({ icon: "shield", title: "Permissions", sub: "Accept edits · 3 saved rules", go: "set-permissions" })])}
    ${group([row({ icon: "terminal", title: "Executors", sub: "Sandbox, Linux mode, CI", go: "set-executors" }), row({ icon: "git-branch", title: "GitHub", sub: "khanblair · push guard on", go: "set-github" }), row({ icon: "plug", title: "MCP servers", sub: "1 connected", go: "set-mcp" })])}
    ${group([row({ icon: "brain", title: "Skills and memory", sub: "Approval on · vault synced", go: "set-skillsmem" }), row({ icon: "badge-check", title: "Quality gate", sub: "2 rules block commits", go: "set-quality" }), row({ icon: "circle-dollar-sign", title: "Budgets", sub: "$2 per session · $20 per month", go: "set-budgets" })])}
    ${group([row({ icon: "palette", title: "Appearance", sub: "System theme · text 100%", go: "set-appearance" }), row({ icon: "bell", title: "Notifications", sub: "Approvals, run status, quiet hours", go: "set-notifications" }), row({ icon: "lock", title: "Privacy", sub: "Biometric lock off", go: "set-privacy" }), row({ icon: "database", title: "Data", sub: "612 MB used", go: "set-data" }), row({ icon: "info", title: "About", sub: "Tether 0.0.1 · core 0.0.1", go: "set-about" })])}</div>`)),
});

S("set-profile", "Profile", "Tether has no account. The profile is local and sets who the agent commits as.", "Stored on this phone only", () => `
  <div class="col" style="align-items:center;gap:12px"><span class="avatar xl">BA</span><div class="swatches">${["accent", "accent-2", "tonal", "info", "success", "warning"].map((c, i) => `<i class="${i === 0 ? "sel" : ""}" style="background:var(--tt-color-${c})">${i === 0 ? I("check", "s") : ""}</i>`).join("")}</div></div>
  ${field("Display name", "Blair")}${field("Commit author name", "Blair Akandwanaho")}${field("Commit author email", "blair@example.com", { hint: "Appears in commit history and pull requests." })}
  <div class="sec-title" style="margin:0">GitHub</div>${group([row({ icon: "git-branch", title: "khanblair", sub: "Connected · repo, workflow", go: "set-github" })])}
  ${btn("Remove profile and local data", { kind: "plain", full: true, icon: "trash-2" })}`, { has: ["Avatar initials and color", "Author name and email", "GitHub connection", "Remove profile"], config: ["Nothing leaves the phone except in commits you push"] });

S("set-providers", "Providers", "Keys and status for every provider in one list.", "Keys live in the Android Keystore", () =>
  group([["Anthropic", "14 models · tested 2 min ago", "ok"], ["OpenAI", "Key rejected. Check the key.", "err"], ["Google Gemini", "No key", ""], ["DeepSeek", "No key", ""], ["OpenRouter", "No key", ""]].map(([n, s, st]) => `<button class="lrow"${nav({ go: "set-provider-detail" })}><span class="ico">${st === "ok" ? `<span class="cok">${I("circle-check")}</span>` : st === "err" ? `<span class="cdanger">${I("circle-x")}</span>` : `<span class="c3">${I("circle-dot")}</span>`}</span><span class="tx"><b>${n}</b><span>${s}</span></span><span class="c3">${I("chevron-right")}</span></button>`)),
  { has: ["Status icon and label per provider", "Tap to edit the key"], config: ["Add several; the model picker shows all their models"] });

S("set-provider-detail", "Provider: Anthropic", "Edit one key, test it, and see what it unlocks.", "Anthropic", () => `
  ${field("API key", "sk-ant-••••••••••••••••••••Qx2", { mono: true, trail: `<button class="btn text sm">${I("eye", "s")}Show</button>`, hint: "Revealing the key asks for your fingerprint." })}
  ${field("Base URL", "https://api.anthropic.com", { mono: true, hint: "Change only for a proxy." })}
  ${CTX.offline ? `<div class="banner" style="margin:0">${I("wifi-off", "s")}Offline. The model list is from your last test, 2 days ago.</div>` : `<div class="card row" style="background:var(--tt-color-success-soft)"><span class="cok">${I("circle-check")}</span><div class="grow"><b class="t-title-s cok">Key accepted</b><div class="t-body-s cok">14 models available.</div></div></div>`}
  ${btn("Test connection", { kind: "outline", full: true, icon: "zap", disabled: CTX.offline })}${btn("Remove key", { kind: "plain", full: true, icon: "trash-2" })}`,
  { back: "set-providers", has: ["Key with biometric reveal", "Base URL", "Test result, or the offline note"], config: ["Removing a key keeps your sessions"] });

S("set-models", "Models and defaults", "Pick the model for work and a different one for review, per project if needed.", "Defaults for new sessions", () => `
  ${group([row({ title: "Default model", sub: "Sonnet 5.5", icon: "cpu" }), row({ title: "Reviewer model", sub: "Opus 5.5 · used by /review", icon: "eye" }), row({ title: "Summary model", sub: "Haiku 5.5 · used for compaction", icon: "layers" })])}
  <div class="sec-title" style="margin:0">Per project</div>${group([row({ title: "acme-api", sub: "Opus 5.5", icon: "folder" }), row({ title: "tether", sub: "Default", icon: "folder" })])}`,
  { has: ["Default, reviewer, and summary models", "Per-project overrides"], config: ["A cheaper model for compaction lowers cost on long sessions"] });

S("set-permissions", "Permissions", "The rules for what asks and what does not. Saved rules are visible and removable.", "Default mode and saved rules", () => `
  ${group([row({ title: "Default mode", sub: "Accept edits", icon: "shield" }), tg("Allow Bypass mode", "Needs fingerprint each time", false)])}
  <div class="sec-title" style="margin:0">Saved rules</div>${group(["run: pytest * · acme-api · any executor", "run: pip install -e . && pytest * · acme-api · Linux mode", "web_fetch: docs.python.org · all projects"].map((t) => `<div class="lrow" style="min-height:56px"><span class="tx"><b class="mono t-mono-s" style="font-weight:500">${t}</b></span><button class="ibtn" style="width:40px;height:40px" aria-label="Remove rule">${I("trash-2", "s")}</button></div>`))}
  ${group([tg("Push guard", "Ask before every push, always", true)])}`,
  { has: ["Default mode", "Saved always-allow rules with remove", "Bypass switch, push guard"], config: ["Rules are stored in .tether/config.toml per project", "Force push asks even in Bypass"] });

S("set-executors", "Executors", "Where commands run, with limits you can see.", "Sandbox, Linux mode, CI", () => `
  <div class="sec-title" style="margin:0">Sandbox</div>${group([row({ title: "Memory limit", sub: "512 MB", icon: "gauge" }), row({ title: "Time limit", sub: "120 s per command", icon: "timer" }), row({ title: "Runtime packs", sub: "Python 3.12, QuickJS, SQLite", icon: "package" })])}
  <div class="sec-title" style="margin:0">Linux mode</div>${group([row({ title: "Alpine 3.22", sub: "Installed · 620 MB", icon: "terminal", go: "linux-setup" }), tg("Network on by default", "Per shell and per command", true)])}
  <div class="sec-title" style="margin:0">CI</div>${group([row({ title: "Workflow file", sub: ".github/workflows/tether.yml · up to date", icon: "workflow" }), row({ title: "Actions minutes", sub: "About 1 min per run", icon: "clock" })])}`,
  { has: ["Sandbox limits", "Linux mode install and network", "CI workflow file"], config: ["Default executor per project in Project home"] });

S("set-github", "GitHub", "The one account Tether touches, with its scopes in plain view.", "khanblair", () => `
  ${group([row({ icon: "git-branch", title: "khanblair", sub: "Connected via device flow", chev: false, right: badge("Active", "ok") }), row({ title: "Scopes", sub: "repo, workflow", icon: "shield-check", chev: false })])}
  ${group([tg("Push guard", "Ask before every push", true), row({ title: "Default branch", sub: "Ask before working on main", icon: "git-branch" }), row({ title: "Agent branch prefix", sub: "tether/", icon: "tag" })])}
  ${btn("Sign out of GitHub", { kind: "plain", full: true, icon: "trash-2" })}`,
  { has: ["Account and scopes", "Push guard, default branch policy, branch prefix"], config: ["Token is revoked on GitHub when you sign out"] });

S("set-mcp", "MCP servers", "Extra tools from remote servers, each with its own approval.", "Streamable HTTP", () => `
  ${group([`<button class="lrow"><span class="ico">${I("plug")}</span><span class="tx"><b>docs-search</b><span class="mono t-mono-s">https://mcp.example.com/docs</span><span>3 tools · Always ask</span></span>${badge("Connected", "ok")}</button>`])}
  ${btn("Add server", { full: true, icon: "plus", kind: "tonal" })}${CTX.offline ? `<div class="banner" style="margin:0">${I("wifi-off", "s")}Offline. Server tools are unavailable until you reconnect.</div>` : ""}`,
  { has: ["Servers with URL, tool count, approval mode", "Add server"], config: ["stdio servers run through Linux mode"] });

S("set-skillsmem", "Skills and memory", "How much freedom the agent has to write its own skills and notes.", "Approval and size limits", () => `
  ${group([tg("Ask before saving a skill", "Show the file as a diff first", true), tg("Ask before memory writes", "Otherwise shown in the transcript", false), tg("Prune unused skills", "Stale after 14 days, archived after 30", true)])}
  ${group([row({ title: "MEMORY.md limit", sub: "2,200 characters", chev: false }), row({ title: "USER.md limit", sub: "1,375 characters", chev: false })])}
  ${group([row({ icon: "cloud", title: "Vault sync", sub: "Documents/Obsidian/Tether", go: "vault" }), row({ icon: "download", title: "Install a skill", go: "skill-install" })])}`,
  { has: ["Approval switches", "Size limits", "Vault sync and install"], config: ["Skill writes ask by default; memory writes do not"] });

S("set-quality", "Quality gate", "Choose which problems only warn and which stop a commit.", "Code smells and AI slop", () => `
  ${group([row({ title: "Long function", sub: "Over 50 lines · Warn", icon: "badge-check" }), row({ title: "Deep nesting", sub: "Over 3 levels · Warn", icon: "badge-check" }), row({ title: "Placeholder code", sub: "TODO stubs, fake imports · Block", icon: "shield-alert" }), row({ title: "Vacuous tests", sub: "No assertions, skipped tests · Block", icon: "shield-alert" })])}
  ${group([tg("Run on every edit batch", "Under 300 ms for 20 files", true), tg("Check prose for banned words", "Docs and UI strings", true)])}`,
  { has: ["Rules with threshold and Warn or Block", "Run and prose switches"], config: ["Overrides saved in .tether/config.toml under [quality]"] });

S("set-budgets", "Budgets", "Set money limits before the agent can exceed them.", "Pause when a limit is reached", () => `
  ${group([row({ title: "Per session", sub: "$2.00", icon: "circle-dollar-sign" }), row({ title: "Per day", sub: "$8.00", icon: "circle-dollar-sign" }), row({ title: "Per month", sub: "$20.00 · $11.40 used", icon: "circle-dollar-sign" })])}
  <div class="col gap4"><div class="row" style="justify-content:space-between"><span class="t-body-s c2">This month</span><span class="t-body-s c2">$11.40 of $20.00</span></div>${meter(11.4, 20)}</div>
  ${group([tg("Warn at 80%", "A note in the transcript", true)])}`,
  { has: ["Session, day, and month limits", "Month meter", "Warn threshold"], config: ["At a limit the agent pauses; Continue or Raise budget resumes"] });

S("set-appearance", "Appearance", "Theme and size are system-level choices. This is the real screen: tapping a size changes the preview above and the whole app.", "Theme, text size, display size", () => `
  <div><div class="t-body-s c2 mb8" style="font-weight:500">Theme</div><div class="chips">${["System", "Light", "Dark"].map((t) => `<button class="chip" data-cfg="theme:${t.toLowerCase()}">${t}</button>`).join("")}</div></div>
  <div><div class="t-body-s c2 mb8" style="font-weight:500">Text size</div><div class="chips">${[85, 100, 115, 130, 150, 200].map((n) => `<button class="chip" data-cfg="fs:${n}">${n}%</button>`).join("")}</div><div class="hint">Follows Android's font size unless you change it here.</div></div>
  <div><div class="t-body-s c2 mb8" style="font-weight:500">Display size</div><div class="chips">${[90, 100, 115, 130].map((n) => `<button class="chip" data-cfg="ds:${n}">${n}%</button>`).join("")}</div><div class="hint">Makes everything larger or smaller, like Android display size.</div></div>
  <div class="card col gap4"><b class="t-title-s">Preview</b><div class="t-body-l">Edited <span class="mono t-mono-label">src/auth/session.py</span> <span class="add">+1</span> <span class="del">−1</span></div><div class="t-body-s c3">Sonnet 5.5 · Accept edits · 34% · $0.21</div></div>
  ${group([tg("Reduce motion", "Stops the running indicator and caret", false), tg("Haptics", "A light tick on Allow, Deny, and send", true)])}`,
  { has: ["Theme: System, Light, Dark", "Text size and display size choices that apply live", "Preview line", "Reduce motion and haptics"], config: ["Text and display size also work with 200% font scale", "Dynamic wallpaper color is off; the palette is fixed"] });

const channels = (off) => `<div class="sec-title" style="margin:0">Channels</div><div style="${off ? "opacity:.55" : ""}">${group([
  row({ icon: "shield", title: "Approvals", sub: "Allow or deny a command · Heads-up with sound", chev: false, right: toggle(!off) }),
  row({ icon: "loader", title: "Run status", sub: "Silent · Stays while a turn runs", chev: false, right: toggle(!off) }),
  row({ icon: "circle-check", title: "Finished turns", sub: "Default importance", chev: false, right: toggle(!off) }),
  row({ icon: "circle-dollar-sign", title: "Budget and errors", sub: "Heads-up · Names the fix", chev: false, right: toggle(!off) }),
])}</div>
${group([tg("Show command text on the lock screen", "Off by default. Commands can contain paths and tokens", false), tg("Vibrate on approval", "A light tick, same as in the app", true), tg("Quiet hours", "22:00 to 07:00", true), tg("Approvals still notify in quiet hours", "Turn off to hold them until morning", true)])}
${btn("Send a test notification", { kind: "outline", full: true, icon: "bell", snack: "toast:Test notification sent" })}`;
S("set-notifications", "Notifications", "Control what can interrupt you. Each channel is a real Android notification channel, so the system settings stay in step.", "Allowed", () => `
  <div class="card row" style="background:var(--tt-color-success-soft)"><span class="cok">${I("circle-check")}</span><div class="grow"><b class="t-title-s cok">Notifications are on</b><div class="t-body-s cok">Approvals can reach you while the app is closed.</div></div>${btn("System settings", { kind: "text", sm: true, icon: "external-link" })}</div>
  ${channels(false)}`,
  { has: ["Permission status with a link to system settings", "Four channels: Approvals, Run status, Finished turns, Budget and errors", "Lock screen privacy, vibration, quiet hours", "Send a test notification"], config: ["The permission is asked the first time a turn runs in the background, not at install", "Quiet hours never mute approvals unless you turn that on", "Command text is hidden on the lock screen unless you allow it"] });
S("set-notifications-off", "Notifications: off", "When the system blocks notifications, say what that costs and how to fix it.", "Blocked in system settings", () => `
  <div class="card col gap12" style="background:var(--tt-color-danger-soft)"><div class="row" style="align-items:flex-start"><span class="cdanger">${I("circle-x")}</span><div class="grow"><b class="t-title-s cdanger">Notifications are off in system settings</b><div class="t-body-s cdanger">A run that needs approval waits inside the app until you open it. Long runs will not tell you when they finish.</div></div></div>${btn("Open system settings", { sm: true, icon: "external-link" })}</div>
  ${channels(true)}`,
  { back: "set-notifications", has: ["What breaks and what to do", "Channels shown but disabled"], config: ["Re-checked every time you open this screen"] });

S("set-privacy", "Privacy", "Lock the app and hide it from screenshots.", "App lock and capture", () => `
  ${group([tg("Biometric app lock", "After 5 minutes in the background", false), tg("Block screenshots", "Also hides the app in recents", false)])}
  ${group([row({ icon: "shield-check", title: "Redaction check", sub: "Keys are removed from logs and exports", chev: false })])}`,
  { has: ["Biometric lock, screenshot blocking"], config: ["Keys are never written to logs or exports"] });

S("set-data", "Data", "Where the storage goes, and how to export or delete it.", "612 MB on this phone", () => `
  ${group([row({ icon: "folder", title: "Projects", sub: "4 · 410 MB", chev: false }), row({ icon: "terminal", title: "Linux mode", sub: "620 MB (not counted in backups)", chev: false }), row({ icon: "package", title: "Runtime packs", sub: "96 MB", chev: false }), row({ icon: "database", title: "Caches", sub: "12 MB", chev: false, right: btn("Clear", { kind: "text", sm: true }) })])}
  ${group([row({ icon: "share-2", title: "Export all sessions", sub: "Markdown zip" }), row({ icon: "trash-2", title: "Delete project acme-api", sub: "Removes the files and sessions on this phone", danger: true }), row({ icon: "trash-2", title: "Remove Linux mode", sub: "Frees 620 MB", danger: true })])}`,
  { has: ["Storage by area", "Export", "Delete actions in the danger color that name the target"], config: ["Delete asks for a confirmation that repeats the name"] });

S("set-about", "About", "Versions, licenses, and the first-run replay.", "Tether 0.0.1", () => `
  ${group([row({ title: "App version", sub: "0.0.1 (full)", chev: false }), row({ title: "Core version", sub: "0.0.1", chev: false }), row({ title: "Licenses", sub: "Open source libraries and IBM Plex fonts" }), row({ title: "Replay setup", sub: "Show the welcome screens again", go: "welcome" })])}
  <div class="t-body-s c3 center">Tether has no server. Nothing about your use is collected.</div>`,
  { has: ["Versions", "Licenses", "Replay setup"], config: ["Developer section appears in debug builds only"] });

// ---- cost ----
const days = [0.4, 0.9, 0.2, 1.4, 0.7, 0.3, 0, 0.5, 1.1, 0.8, 0.6, 1.9, 0.5, 0.84];
const bars = (() => {
  const w = 340, h = 150, bw = 16, g = (w - days.length * bw) / (days.length - 1), max = 2;
  return `<svg viewBox="0 0 ${w} ${h + 22}" role="img" aria-label="Daily cost over 14 days, highest $1.90"><line x1="0" y1="${h}" x2="${w}" y2="${h}" stroke="var(--tt-chart-grid)"/><line x1="0" y1="${h / 2}" x2="${w}" y2="${h / 2}" stroke="var(--tt-chart-grid)" stroke-dasharray="3 4"/>${days.map((v, i) => `<rect x="${i * (bw + g)}" y="${h - (v / max) * h}" width="${bw}" height="${(v / max) * h}" rx="3" fill="var(--tt-chart-series-1)"/>`).join("")}<text x="0" y="${h + 16}" font-size="11" fill="var(--tt-chart-axis-text)">Nov 5</text><text x="${w}" y="${h + 16}" font-size="11" text-anchor="end" fill="var(--tt-chart-axis-text)">Nov 18</text><text x="${w}" y="12" font-size="11" text-anchor="end" fill="var(--tt-chart-axis-text)">$2</text></svg>`;
})();
frame({
  id: "cost", area: "cost", title: "Cost",
  why: "Say the number first. A chart explains it; a table proves it.",
  has: ["One-sentence summary", "Bar chart of daily cost with axis labels", "By model, with a pattern-free legend and direct numbers", "Show data toggle for the table"],
  config: ["Period: session, day, month", "Budgets link to Settings > Budgets", "Charts are bars or lines only, with a text summary and a data table"],
  render: () => screen(appbar({ back: true, title: "Cost", sub: "acme-api" }) + body(`<div class="pad col gap16">
    <div><div class="t-title-l">This session: $0.84</div><div class="t-body c2">212k tokens in, 18k out. $11.40 of your $20 monthly budget is used.</div></div>
    <div class="chips">${chip("Session", { sel: true })}${chip("14 days")}${chip("Month")}</div>
    <div class="chart card">${bars}</div>
    <div><b class="t-title-s">By model</b>${group([["Sonnet 5.5", "$0.62", "74%"], ["Haiku 5.5", "$0.09", "11%"], ["Opus 5.5 (review)", "$0.13", "15%"]].map(([m, c, p]) => `<div class="lrow" style="min-height:52px"><span class="tx"><b>${m}</b></span><span class="t-body-s c3">${p}</span><b class="t-body" style="min-width:52px;text-align:right">${c}</b></div>`))}</div>
    ${btn("Show data", { kind: "text", icon: "chevron-down" })}</div>`)),
});
