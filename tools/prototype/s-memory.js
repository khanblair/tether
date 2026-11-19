// Area 6: skills, memory, and the vault.
area("memory", "Skills, memory, and vault", "How the agent gets better at your projects without a server: skills it loads on demand, two small memory files, and a vault of linked notes in Obsidian's format. All of it is plain files on this phone, and the user can read and edit every one.");

const seg = (items, sel) => `<div class="seg" style="display:flex;width:100%">${items.map((t) => `<button style="flex:1" aria-pressed="${t === sel}">${t}</button>`).join("")}</div>`;

const skillRow = (name, desc, scope, state, used, go) => `<button class="lrow" style="min-height:72px;align-items:flex-start;padding:12px 14px"${nav({ go: go || "skill-detail" })}><span class="tx"><b class="mono t-mono-label">${name}</b><span class="t-body-s c2" style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden">${desc}</span><span class="row" style="margin-top:6px;gap:6px">${badge(scope, "tonal")}${state === "Stale" ? badge("Stale", "warn") : ""}<span class="t-caption c3">${used}</span></span></span></button>`;

frame({
  id: "skills", area: "memory", title: "Skills",
  why: "Reusable know-how the agent loads only when a task needs it, so the prompt stays small. Anything the agent wrote waits for approval.",
  has: ["Approval banner for agent-written skills", "Search and scope filters: project, user, bundled", "Rows: name, description, scope, state, last used", "Install from GitHub or zip"],
  config: ["Format: the open Agent Skills standard (SKILL.md)", "Precedence: project, then user, then bundled", "Stale after 14 days unused, archived after 30; never deleted"],
  render: () => screen(appbar({ back: true, title: "Skills", sub: "14 available · 2 in this project", actions: [{ icon: "download", label: "Install a skill", go: "skill-install" }] }) + body(`<div class="pad col gap12">
    <button class="card row" style="background:var(--tt-color-accent-soft)"${nav({ go: "skill-approve" })}><span class="cacc">${I("circle-pause", "l")}</span><span class="grow"><b class="t-title-s">1 skill waiting for approval</b><div class="t-body-s c2">The agent wrote <span class="mono">handle-flaky-tests</span> after you corrected it.</div></span><span class="c3">${I("chevron-right")}</span></button>
    ${field("", "", { icon: "search", ph: "Search skills" })}
    <div class="chips scroll">${chip("All", { sel: true })}${chip("Project")}${chip("User")}${chip("Bundled")}</div>
    ${group([skillRow("fix-failing-tests", "Find and fix failing tests in a Python project. Use when a test run fails or a test is flaky.", "Project", "Active", "Used 2 h ago"), skillRow("write-commit-message", "Write a short imperative commit message with a body that explains why.", "User", "Active", "Used yesterday"), skillRow("release-notes", "Draft release notes from merged pull requests since the last tag.", "User", "Stale", "Unused 16 days"), skillRow("review-pr", "Review a pull request for correctness, tests, and risk.", "Bundled", "Active", "Used 3 days ago")])}
    <div class="t-body-s c3 center">2 archived skills · Show</div></div>`)),
});

frame({
  id: "skill-detail", area: "memory", title: "Skill detail",
  why: "A skill is a folder of Markdown. Show the real file so the user can trust or edit it.",
  has: ["Name, description, scope, last used", "SKILL.md preview with its frontmatter", "Reference files the agent can load on demand", "Enabled and Pinned toggles; Delete named and last"],
  config: ["Pinned skills are never archived by upkeep", "Tether-only settings use a tether- key prefix inside metadata, so the skill stays valid for other agents"],
  render: () => screen(appbar({ back: "skills", title: "fix-failing-tests", sub: "Project skill · .tether/skills", actions: [{ icon: "pencil", label: "Edit" }] }) + body(`<div class="pad col gap16">
    <div class="surface" style="padding:4px 0">${row({ title: "Enabled", chev: false, right: toggle(true) })}<div class="hr"></div>${row({ title: "Pinned", sub: "Exempt from archiving", chev: false, right: toggle(false) })}</div>
    <div class="code" style="padding:12px"><div class="t-mono-s c3">SKILL.md</div><div class="t-mono-s" style="white-space:pre-wrap;margin-top:6px"><span class="cm">---</span>\n<span class="kw">name:</span> fix-failing-tests\n<span class="kw">description:</span> Find and fix failing tests in a\n  Python project. Use when a test run fails.\n<span class="kw">metadata:</span>\n  tether-requires-tools: run,git\n<span class="cm">---</span>\n\n<b># Fix failing tests</b>\n1. Run the failing test alone first.\n2. Read the test, then the code it covers.\n3. Change the code unless the test is wrong.\n<span class="cacc">… 24 more lines</span></div></div>
    <div><b class="t-title-s">Reference files</b>${group([row({ icon: "file-text", title: "references/pytest-tips.md", sub: "Loaded only when needed", chev: false }), row({ icon: "terminal", title: "scripts/run-tests.sh", sub: "Runs through the executor, with approval", chev: false })])}</div>
    ${btn("Delete this skill", { kind: "plain", full: true, icon: "trash-2" })}</div>`)),
});

frame({
  id: "skill-approve", area: "memory", title: "Approve an agent-written skill",
  why: "Show before doing. The agent proposes; the user reads the exact file before it exists.",
  has: ["What triggered it (a correction, a repeated workflow)", "The new file as a diff", "Approve, Edit, Reject"],
  config: ["Approval is on by default (Settings > Skills and memory)", "Content rule: lessons, not logs. A size warning shows past 24,000 characters"],
  render: () => screen(appbar({ back: "skills", title: "New skill", sub: "Written by the agent · not saved yet" }) + body(`<div class="pad col gap12">
    <div class="card"><div class="t-body-s c2">You corrected the agent twice about flaky tests, so it wrote a skill. Nothing is saved until you approve.</div></div>
    <div class="diff"><div class="fh"><span class="ell">.tether/skills/handle-flaky-tests/SKILL.md</span><span class="add">+14</span></div>
    ${[`---`, `name: handle-flaky-tests`, `description: Triage a flaky test. Use when a test passes`, `  and fails without code changes.`, `---`, ``, `# Handle flaky tests`, `1. Rerun the test 5 times before changing code.`, `2. Look for time, order, and shared state.`, `3. Never add a blanket retry.`].map((t, i) => `<div class="ln a"><span class="g"></span><span class="g">${i + 1}</span><span class="m">+</span><span>${esc(t)}</span></div>`).join("")}</div></div>`) +
    `<div class="pad row" style="padding-bottom:8px">${btn("Reject", { kind: "plain", go: "skills" })}${btn("Edit", { kind: "text" })}<span class="grow"></span>${btn("Approve and save", { go: "skills", snack: "Skill saved" })}</div>`),
});

frame({
  id: "skill-install", area: "memory", title: "Install a skill",
  why: "Skills can come from anywhere, so installing is a review step, not a download.",
  has: ["GitHub URL or zip", "Quarantine: the files are listed before anything is saved", "Static scan with a result per check", "Install is a separate, explicit action"],
  config: ["Scans for instructions to read secrets or send data elsewhere, shell downloads, hidden Unicode", "Scripts run only inside the executors, under the permission mode"],
  render: () => screen(appbar({ back: "skills", title: "Install a skill", sub: "Quarantined, not installed" }) + body(`<div class="pad col gap16">
    ${field("Source", "github.com/acme/skills/pdf-extract", { mono: true, icon: "link", focus: true })}
    <div class="card col gap4"><b class="t-title-s">Scan result</b>${checkRow(true, "SKILL.md is valid", "Name matches the folder")}${checkRow(true, "No secret or network instructions")}${checkRow(false, "1 script needs review", "scripts/extract.sh runs a download with curl")}</div>
    ${group([row({ icon: "file-text", title: "SKILL.md", sub: "2.1 KB", chev: false }), row({ icon: "terminal", title: "scripts/extract.sh", sub: "0.4 KB · flagged", right: badge("Review", "warn"), chev: false })])}</div>`) +
    `<div class="pad col gap12" style="padding-bottom:8px">${btn("Install skill", { full: true, go: "skills" })}${btn("Discard", { kind: "plain", full: true, go: "skills" })}</div>`),
});

frame({
  id: "memory", area: "memory", title: "Memory",
  why: "Two small files carry facts across sessions. Small caps force the agent to keep only what matters, and the user can read and edit it all.",
  has: ["MEMORY.md and USER.md with size meters", "Entries with edit and remove", "A note that changes apply from the next session", "Search past sessions"],
  config: ["Caps: 2,200 characters (MEMORY.md), 1,375 (USER.md)", "A write over the cap fails and the agent must trim", "Writes are scanned for prompt injection and secrets"],
  render: () => screen(appbar({ back: true, title: "Memory", sub: "Snapshot taken at session start", actions: [{ icon: "plus", label: "Add entry" }] }) + body(`<div class="pad col gap16">
    ${seg(["MEMORY.md", "USER.md"], "MEMORY.md")}
    <div class="col gap4"><div class="row" style="justify-content:space-between"><span class="t-body-s c2">Agent notes</span><span class="t-body-s c2 mono">1,420 of 2,200</span></div>${meter(1420, 2200)}</div>
    ${group(["Project acme-api: run tests with pytest -q. Python 3.12.", "Prefers small commits: one logical change each.", "CI tooling job fails on banned words. Run slop-lint before pushing.", "Do not touch migrations/ without asking."].map((t) => `<div class="lrow" style="align-items:flex-start;padding:12px 14px"><span class="tx"><b class="t-body" style="font-weight:400">${t}</b></span><button class="ibtn" style="width:40px;height:40px" aria-label="Edit entry">${I("pencil", "s")}</button><button class="ibtn" style="width:40px;height:40px" aria-label="Remove entry">${I("trash-2", "s")}</button></div>`))}
    <div class="card row"><span class="c2">${I("info")}</span><span class="t-body-s c2">Changes apply from the next session. This session uses the snapshot taken at its start, which keeps the cached prompt cheap.</span></div>
    ${group([row({ icon: "search", title: "Search past sessions", sub: "Full text across every message", go: "session-search" })])}</div>`)),
});

frame({
  id: "session-search", area: "memory", title: "Search past sessions",
  why: "Memory has a size limit; history does not. Search finds the message, not a summary of it.",
  has: ["Search across every session", "Hits show the real message with the match highlighted", "Scope: this project or all"],
  config: ["SQLite full-text search on the device; no model call, no cost"],
  render: () => screen(appbar({ back: "memory", title: "Search" }) + body(`<div class="pad col gap12">${field("", "boundary", { icon: "search", focus: true })}<div class="chips">${chip("This project", { sel: true })}${chip("All projects")}</div>
    ${group([["Fix login timeout", "yesterday", "…token expires at exactly the <mark class='wa' style='background:var(--tt-color-diff-word-add);color:inherit'>boundary</mark> second…"], ["Migrate to Pydantic v2", "2 days ago", "…validators run at the model <mark class='wa' style='background:var(--tt-color-diff-word-add);color:inherit'>boundary</mark>, not inside…"], ["Add rate limit to /v1/search", "5 days ago", "…tokens refill at the window <mark class='wa' style='background:var(--tt-color-diff-word-add);color:inherit'>boundary</mark> so bursts…"]].map(([t, d, s]) => `<button class="lrow" style="align-items:flex-start;padding:12px 14px"${nav({ go: "session-running" })}><span class="tx"><b>${t}</b><span>${d}</span><span class="t-body-s c2" style="margin-top:4px">${s}</span></span></button>`))}</div>`)),
});

const wl = (t) => `<span class="cacc">[[${t}]]</span>`;
const noteRow = (t, d, tg) => `<button class="lrow" style="min-height:64px;align-items:flex-start;padding:12px 14px"${nav({ go: "vault-note" })}><span class="tx"><b>${t}</b><span class="ell">${d}</span></span>${tg ? badge(tg) : ""}</button>`;

frame({
  id: "vault", area: "memory", title: "Vault",
  why: "Long-term notes that outgrow two small files. It is a folder of Markdown in Obsidian's format, so it also opens in Obsidian.",
  has: ["Notes grouped by type with their one-line description", "Entry rows for index.md and log.md", "Sync status and Notes, Map, Lint views", "Search by text, tag, and link"],
  config: ["Lives in app storage; export and import as zip, or sync a shared folder", "Every note needs a description; unlinked notes show in Lint"],
  render: () => screen(appbar({ back: true, title: "Vault", sub: "212 notes · synced 3 min ago", actions: [{ icon: "search", label: "Search vault" }, { icon: "cloud", label: "Sync and export", go: "sheet-vault-sync" }] }) + body(`<div class="pad col gap12">
    <div class="seg" style="display:flex;width:100%"><button style="flex:1" aria-pressed="true">Notes</button><button style="flex:1" aria-pressed="false" ${nav({ go: "vault-map" })}>Map</button><button style="flex:1" aria-pressed="false" ${nav({ go: "vault-lint" })}>Lint</button></div>
    ${group([row({ icon: "list-checks", title: "index.md", sub: "Catalog of every note, one line each", chev: false }), row({ icon: "scroll-text", title: "log.md", sub: "Append-only record, newest last", chev: false })])}
    <div><div class="sec-title" style="margin:8px 0 6px">Decisions</div>${group([noteRow("Use token bucket for rate limits", "Chose a token bucket over a fixed window to allow short bursts.", "decision"), noteRow("Auth session design", "How sessions, tokens, and refresh interact in acme-api.", "reference")])}</div>
    <div><div class="sec-title" style="margin:8px 0 6px">Conventions</div>${group([noteRow("Commit style", "Small commits, imperative subject, body explains why.", "convention")])}</div>
    <div><div class="sec-title" style="margin:8px 0 6px">Bugs</div>${group([noteRow("Expiry off by one", "Token valid at exact expiry second; fixed in PR #43.", "bug")])}</div></div>`)),
});

frame({
  id: "vault-note", area: "memory", title: "Vault note",
  why: "Notes are linked. Backlinks show what leans on this note, and missing links are visible instead of silent.",
  has: ["Properties: type, tags, updated, description", "Body with wikilinks in the accent color", "Backlinks and outlinks", "Unresolved link marked as missing", "Ask the agent about this note"],
  config: ["Frontmatter keys: description (required), type, tags, aliases, created, updated", "Links: [[Note]], [[Note#Heading]], [[Note|alias]]"],
  render: () => screen(appbar({ back: "vault", title: "Expiry off by one", sub: "bug · updated 2 days ago", actions: [{ icon: "pencil", label: "Edit" }] }) + body(`<div class="pad col gap16">
    <div class="card col gap4"><div class="row" style="flex-wrap:wrap">${badge("bug", "tonal")}${badge("auth")}${badge("tests")}</div><div class="t-body-s c2" style="margin-top:6px">Token valid at exact expiry second; fixed in PR #43.</div></div>
    <div class="t-body">The check in ${wl("Auth session design")} used <code class="inline-code">&lt;</code>, so a token lived one second too long. The regression test is described in ${wl("Commit style#Tests")}. A refresh policy is still open: ${wl("Token rotation")}.</div>
    <div class="banner" style="margin:0">${I("link", "s")}<span>Token rotation does not exist yet. Create it or remove the link.</span></div>
    <div><b class="t-title-s">Linked from (3)</b>${group([row({ icon: "file-text", title: "Auth session design", chev: false }), row({ icon: "file-text", title: "acme-api map", sub: "Map note", chev: false }), row({ icon: "scroll-text", title: "log.md", chev: false })])}</div></div>`) + `<div class="pad" style="padding-bottom:8px">${btn("Ask about this note", { full: true, icon: "at-sign", kind: "tonal", go: "session-idle" })}</div>`),
});

const mapNode = (d, t, n, kind) => `<button class="lrow" style="min-height:44px;padding-left:${14 + d * 22}px"${nav({ go: "vault-note" })}><span class="ico">${I(kind || "file-text", "s")}</span><span class="tx"><b class="t-body">${t}</b></span><span class="badge">${I("link")}${n}</span></button>`;
frame({
  id: "vault-map", area: "memory", title: "Vault map",
  why: "The agent navigates the vault through map notes instead of scanning everything. The same structure is shown as a list, not a hairball graph.",
  has: ["Map notes as roots, children indented", "Link count per note", "Depth chips: 1 or 2 hops", "Tap a note to open it"],
  config: ["Map notes have type: moc and only group links", "Map depth and token budget shape what the agent reads"],
  render: () => screen(appbar({ back: "vault", title: "Map", sub: "acme-api map · 12 notes" }) + body(`<div class="pad col gap12"><div class="seg" style="display:flex;width:100%"><button style="flex:1" aria-pressed="false" ${nav({ go: "vault" })}>Notes</button><button style="flex:1" aria-pressed="true">Map</button><button style="flex:1" aria-pressed="false" ${nav({ go: "vault-lint" })}>Lint</button></div>
    <div class="chips">${chip("1 hop")}${chip("2 hops", { sel: true })}</div>
    ${group([mapNode(0, "acme-api map", 12, "network"), mapNode(1, "Decisions", 3, "folder"), mapNode(2, "Use token bucket for rate limits", 4), mapNode(2, "Single database per service", 2), mapNode(1, "Conventions", 2, "folder"), mapNode(2, "Commit style", 5), mapNode(1, "Bugs", 2, "folder"), mapNode(2, "Expiry off by one", 3)])}</div>`)),
});

frame({
  id: "vault-lint", area: "memory", title: "Vault lint",
  why: "A vault rots when notes are orphaned or links break. Lint lists them with a one-tap fix.",
  has: ["Orphan notes (nothing links to them)", "Unresolved links", "Notes missing a description", "Ask the agent to fix"],
  config: ["Runs on demand with /vault lint and after large edits"],
  render: () => screen(appbar({ back: "vault", title: "Lint", sub: "3 issues" }) + body(`<div class="pad col gap16"><div class="seg" style="display:flex;width:100%"><button style="flex:1" aria-pressed="false" ${nav({ go: "vault" })}>Notes</button><button style="flex:1" aria-pressed="false" ${nav({ go: "vault-map" })}>Map</button><button style="flex:1" aria-pressed="true">Lint</button></div>
    <div><b class="t-title-s">Orphans (1)</b>${group([row({ icon: "file-text", title: "Old deploy checklist", sub: "No notes link here", chev: false })])}</div>
    <div><b class="t-title-s">Unresolved links (1)</b>${group([row({ icon: "link", title: "Token rotation", sub: "Linked from Expiry off by one", chev: false })])}</div>
    <div><b class="t-title-s">Missing description (1)</b>${group([row({ icon: "file-text", title: "Scratch ideas", chev: false })])}</div>
    ${btn("Ask the agent to fix these", { full: true, kind: "tonal", go: "session-idle" })}</div>`)),
});

frame({
  id: "sheet-vault-sync", area: "memory", title: "Vault sync and export", overlay: "vault",
  why: "The vault lives in app storage, so it needs a safe way in and out. A shared folder lets Obsidian for Android open the same notes.",
  has: ["Shared folder picker with last sync time", "Sync now; sync after each turn that wrote notes", "Export and import as zip", "Conflicts keep both copies"],
  config: ["Android shares folders as content URIs, so the app copies changed files both ways by hash and time", "The agent only ever sees the private copy"],
  overlayHtml: () => sheet("Sync and export", `<div class="col gap16">${group([row({ icon: "folder", title: "Shared folder", sub: "Documents/Obsidian/Tether · synced 3 min ago", chev: false })])}
    <div class="row" style="justify-content:space-between"><span class="t-body">Sync after each turn that wrote notes</span>${toggle(true)}</div>${btn("Sync now", { full: true, icon: "refresh-cw", snack: "Vault synced. 4 notes updated." })}
    <div class="row">${btn("Export zip", { kind: "outline", icon: "upload" })}${btn("Import zip", { kind: "outline", icon: "download" })}</div><div class="t-body-s c3">If both sides changed the same note, Tether keeps both copies and tells you.</div></div>`),
});
