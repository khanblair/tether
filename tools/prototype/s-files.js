// Area 5: files and runs.
area("files", "Files and runs", "These screens explain what the agent touched and ran. They are read-only on purpose: changes go through the agent, so every edit has a checkpoint.");

const tree = (d, name, open, kids) => `<div><button class="lrow" style="min-height:44px;padding-left:${14 + d * 18}px"><span class="ico">${I(open ? "chevron-down" : "chevron-right", "s")}</span><span class="ico">${I(open ? "folder-open" : "folder")}</span><span class="tx"><b class="mono t-mono-label">${name}</b></span></button>${kids || ""}</div>`;
const leaf = (d, name, extra, go) => `<button class="lrow" style="min-height:44px;padding-left:${14 + d * 18 + 22}px"${nav({ go: go || "file-view" })}><span class="ico">${I("file-text", "s")}</span><span class="tx"><b class="mono t-mono-label">${name}</b></span>${extra || ""}</button>`;

frame({
  id: "files", area: "files", title: "Files",
  why: "See the project the way the agent sees it. Ignored files are dimmed and hidden by default.",
  has: ["Lazy tree, folders expand on tap", "Modified files marked with a badge", "Search", "Ignored files hidden (toggle)"],
  config: ["Ignore rules: .gitignore plus Tether defaults", "Show ignored files toggle in the overflow menu"],
  render: () => screen(appbar({ back: true, title: "Files", sub: "acme-api", actions: [{ icon: "search", label: "Search files" }, { icon: "ellipsis-vertical", label: "More" }] }) + body(`<div class="pad" style="padding-bottom:12px">${group([
    tree(0, "src", true, tree(1, "auth", true, leaf(2, "session.py", badge("Modified", "tonal")) + leaf(2, "tokens.py")) + leaf(1, "limits.py")),
    tree(0, "tests", true, leaf(1, "test_login.py", badge("Modified", "tonal"))),
    tree(0, "docs", false), leaf(0, "TETHER.md"), leaf(0, "pyproject.toml"), leaf(0, "CHANGELOG.md", badge("Modified", "tonal")),
  ])}</div>`)),
});

const pyLines = [
  [38, `${kw("from")} .tokens ${kw("import")} Token`], [39, ""], [40, `<span class="cm"># A token is valid through its expiry second.</span>`],
  [41, `${kw("def")} <span class="fn">is_expired</span>(token: <span class="ty">Token</span>, now: <span class="ty">int</span>) -> <span class="ty">bool</span>:`],
  [42, `    <span class="cm">"""A token is expired once its expiry second is reached."""</span>`], [43, `    ${kw("return")} token.expires_at &lt;= now`], [44, ""],
  [45, `${kw("def")} <span class="fn">refresh_session</span>(session, now):`], [46, `    ${kw("if")} <span class="fn">is_expired</span>(session.token, now):`], [47, `        session.token = <span class="fn">issue_token</span>(session.user)`],
];
frame({
  id: "file-view", area: "files", title: "File viewer",
  why: "Read the code the agent is talking about without leaving the app. Editing happens through the agent.",
  has: ["Read-only, syntax highlighted", "Line numbers, horizontal scroll", "Ask about this file inserts an @ mention and returns to the session"],
  config: ["Syntax colors come from the syntax tokens, checked for contrast on code backgrounds"],
  render: () => screen(appbar({ back: "files", title: "session.py", sub: "src/auth · 112 lines", actions: [{ icon: "copy", label: "Copy path", snack: "toast:Path copied" }] }) + body(`<div class="pad"><div class="code">${pyLines.map(([n, t]) => `<div class="ln"><span class="g">${n}</span><span>${t}</span></div>`).join("")}</div></div>`) +
    `<div class="pad" style="padding-bottom:8px">${btn("Ask about this file", { full: true, icon: "at-sign", kind: "tonal", go: "session-idle" })}</div>`),
});

const runRow = (go, st, cmd, ex, meta) => `<button class="lrow" style="min-height:64px"${nav({ go })}><span class="ico">${statusDot(st)}</span><span class="tx"><b class="mono t-mono-label ell">${cmd}</b><span>${meta}</span></span>${badge(ex, ex === "CI" ? "tonal" : "")}</button>`;
frame({
  id: "runs", area: "files", title: "Runs",
  why: "One list for everything that executed: sandbox, Linux mode, and CI. The executor badge says where it ran.",
  has: ["Command, executor badge, status, duration", "CI runs with workflow and branch", "Filter by executor"],
  config: ["History kept per project; output over 256 KB is stored as a file, not in memory"],
  render: () => screen(appbar({ back: true, title: "Runs", sub: "acme-api", actions: [{ icon: "list-filter", label: "Filter" }] }) + `<div class="pad" style="padding-bottom:8px"><div class="chips scroll">${chip("All", { sel: true })}${chip("Sandbox")}${chip("Linux mode")}${chip("CI")}</div></div>` + body(`<div class="pad">${group([
    runRow("run-console", "run", "pytest tests/test_login.py -q", "Sandbox", "Running · 3 s"),
    runRow("run-console", "ok", "pip install -e . && pytest -q", "Linux mode", "12 passed · 6.2 s · 4 min ago"),
    runRow("ci-run", "run", "tether.yml · tether/fix-login-timeout", "CI", "android job running · 3 m"),
    runRow("ci-run", "err", "tether.yml · main", "CI", "tooling failed · yesterday"),
    runRow("run-console", "ok", "python -c \"import sys; print(sys.version)\"", "Sandbox", "0.1 s · yesterday"),
  ])}</div>`)),
});

frame({
  id: "run-console", area: "files", title: "Run console",
  why: "Live output with the command, executor, exit code, and a way to run it somewhere else.",
  has: ["Exact command in mono", "Executor badge and routing reason", "Live output, auto-scroll that pauses when you scroll up", "Run elsewhere"],
  config: ["Router picks Sandbox, then Linux mode, then CI, and states why", "Time and output limits per executor in Settings > Executors"],
  render: () => screen(appbar({ back: true, title: "Run", sub: "Sandbox · running 3 s", actions: [{ icon: "square", label: "Stop" }] }) + body(`<div class="pad col gap12">
    <div class="card col gap4"><div class="row">${badge("Sandbox", "")}<span class="t-body-s c3">Chosen because pytest runs on WebAssembly Python</span></div><div class="mono t-mono-label" style="margin-top:6px">pytest tests/test_login.py -q</div></div>
    <div class="term" style="border-radius:12px;min-height:260px;flex:none"><span class="d">$ pytest tests/test_login.py -q</span>\ncollected 12 items\n\ntests/test_login.py <span class="p">....</span>F<span class="p">...</span>\n\n<span class="d">running…</span></div></div>`) +
    `<div class="pad row" style="padding-bottom:8px">${btn("Run elsewhere", { kind: "outline", go: "sheet-elsewhere" })}<span class="grow"></span>${btn("Stop", { kind: "tonal", icon: "square", sm: true })}</div>`),
});

frame({
  id: "sheet-elsewhere", area: "files", title: "Run elsewhere", overlay: "run-console",
  why: "When the sandbox cannot do the job, offer the next executor with the cost of switching stated.",
  has: ["Available executors with what each can do", "Unavailable ones say why"],
  config: ["Linux mode needs the root filesystem; CI needs a GitHub repo and counts Actions minutes"],
  overlayHtml: () => sheet("Run elsewhere", `<div class="col gap12">${opt("Linux mode", "Full Alpine userland. Network on. Starts in about 2 s.", false)}${opt("GitHub Actions", "Runs in the cloud. Uses about 1 minute of Actions time.", false)}${opt("Sandbox", "Current. Fastest, no network.", true)}${btn("Run in Linux mode", { full: true, kind: "tonal", back: true })}</div>`),
});

frame({
  id: "ci-run", area: "files", title: "CI run",
  why: "Heavy builds run in GitHub Actions. Show status, trim the log to the failing step, and hand the failure back to the agent.",
  has: ["Workflow, branch, commit", "Jobs with status icon and label", "Failure summary with the failing step", "Ask the agent to fix; Open on GitHub"],
  config: ["Tether adds .github/workflows/tether.yml only after approval", "Polling backs off to save battery"],
  render: () => screen(appbar({ back: true, title: "CI run", sub: "tether.yml · #218", actions: [{ icon: "external-link", label: "Open on GitHub" }] }) + body(`<div class="pad col gap16">
    <div class="card col gap4"><div class="row">${badge("Failed", "err", "circle-x")}<span class="t-body-s c3">tooling · step 4 of 7</span></div><div class="t-body" style="margin-top:6px">slop-lint found 1 banned word in docs/architecture.md:335.</div></div>
    <div>${group([["rust", "ok"], ["bindings", "ok"], ["tooling", "err"], ["android", "run"]].map(([n, s]) => `<div class="lrow" style="min-height:48px"><span class="ico">${statusDot(s)}</span><span class="tx"><b>${n}</b></span><span class="c3">${I("chevron-right")}</span></div>`))}</div>
    <div class="term" style="border-radius:12px;flex:none;min-height:110px"><span class="d">tooling › Banned words and slop patterns</span>\ndocs/architecture.md:335: banned-word:realm\nslop-lint: 1 finding(s)\n<span class="d">Process completed with exit code 1.</span></div></div>`) +
    `<div class="pad col gap12" style="padding-bottom:8px">${btn("Ask the agent to fix", { full: true, go: "session-idle" })}</div>`),
});

const keys = ["Esc", "Tab", "Ctrl", "Alt", "←", "↓", "↑", "→", "|", "/", "-", "~"];
frame({
  id: "shell", area: "files", title: "Linux shell",
  why: "A real terminal for when the agent or the user needs one. Phone keyboards lack Esc, Tab, and Ctrl, so a key row sits above the keyboard.",
  has: ["Full-screen terminal in the project folder", "Key row: Esc Tab Ctrl Alt arrows | / - ~", "Network on or off badge in the top bar"],
  config: ["Network toggle per shell", "Font size follows the monospace scale", "Linux mode is in the full build; the play build may not ship it"],
  render: () => screen(appbar({ back: true, title: "Linux shell", sub: "~/acme-api", actions: [{ icon: "wifi", label: "Network on", accent: true }, { icon: "ellipsis-vertical", label: "More" }] }) +
    `<div class="term"><span class="p">acme-api $</span> ls\npyproject.toml  src  tests  TETHER.md\n<span class="p">acme-api $</span> pytest -q\n............                      [100%]\n12 passed in 4.81s\n<span class="p">acme-api $</span> git status -s\n M src/auth/session.py\n M tests/test_login.py\n<span class="p">acme-api $</span> <span class="caret"></span></div><div class="keyrow">${keys.map((k) => `<button>${k}</button>`).join("")}</div>`),
});

frame({
  id: "linux-setup", area: "files", title: "Linux mode setup",
  why: "Linux mode is a download, so say how big it is, what is in it, and how to remove it. Offline, setup waits instead of failing.",
  has: ["Size, version, and package list", "Download progress with Wi-Fi only option", "Remove Linux mode (in Settings)"],
  config: ["Packages: bash, git, nodejs, npm, python3, build-base", "Root filesystem is verified against a signed manifest"],
  render: () => screen(appbar({ back: "set-executors", title: "Linux mode", sub: "Alpine 3.22 · arm64" }) + body(`<div class="pad col gap16">
    <p class="t-body c2" style="margin:0">Linux mode runs a real Linux userland inside Tether, so the agent can use npm, pip, and git as on a computer. It needs a one-time download.</p>
    <div class="card col gap12"><div class="row" style="justify-content:space-between"><b class="t-title-s">Root filesystem</b><span class="t-body-s c3">${CTX.offline ? "Waiting for network" : "64% of 212 MB"}</span></div>${meter(CTX.offline ? 0 : 64, 100)}<div class="row">${I(CTX.offline ? "wifi-off" : "loader", "s " + (CTX.offline ? "" : "spin"))}<span class="t-body-s c2">${CTX.offline ? "Offline. The download starts when you reconnect." : "Downloading and verifying"}</span></div></div>
    ${group([row({ icon: "package", title: "Packages", sub: "bash git nodejs npm python3 py3-pip build-base", chev: false }), row({ icon: "database", title: "Disk after setup", sub: "About 620 MB", chev: false })])}
    <div class="row" style="justify-content:space-between"><span class="t-body">Download on Wi-Fi only</span>${toggle(true)}</div></div>`) + `<div class="pad" style="padding-bottom:8px">${btn("Install Linux mode", { full: true, disabled: CTX.offline })}</div>`),
});
