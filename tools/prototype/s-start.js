// Areas 1 and 2: first run and projects.
area("first", "First run", "Four screens: welcome with a phone check, a local profile, a provider key, and a project. Goal: install to first useful session fast (success criterion S1). Every step is skippable, and every step that needs the network says so and waits when the phone is offline. No permission prompts here.");
area("projects", "Projects", "The entry point. Rows are sorted by last activity and show what needs attention (uncommitted changes). Everything important sits in the bottom half of the screen.");

const dots = (n) => `<div class="row" style="justify-content:center;gap:6px;margin:8px 0 12px" aria-label="Step ${n} of 4">${[1, 2, 3, 4].map((i) => `<i style="width:${i === n ? 20 : 6}px;height:6px;border-radius:3px;background:${i === n ? "var(--tt-color-accent-ink)" : "var(--tt-color-line-input)"}"></i>`).join("")}</div>`;
const foot = (n, primary, skipGo) => `<div class="pad" style="padding-bottom:8px">${dots(n)}${primary}${skipGo ? `<div class="center mt8">${btn("Skip", { kind: "plain", go: skipGo })}</div>` : ""}</div>`;
const checkRow = (ok, text, sub) => `<div class="check-row"><span class="${ok ? "cok" : "cwarn"}">${I(ok ? "circle-check" : "triangle-alert")}</span><div class="grow"><div class="t-body">${text}</div>${sub ? `<div class="t-body-s c3">${sub}</div>` : ""}</div></div>`;

frame({
  id: "welcome", area: "first", title: "1 of 4 · Welcome and phone check",
  why: "Say what the app is in one sentence, then check the phone can run it, including whether it is online. A failed check is better found now than mid-session.",
  has: ["Logo and one sentence", "Phone check: Android version, free storage, network", "Offline: says what still works and what waits", "Primary: Set up your profile"],
  config: ["Needs Android 10+, about 1 GB free to start", "The network check is information, not a blocker", "Shown once; Settings > About replays it"],
  render: () => screen(body(`<div class="pad col gap16" style="padding-top:12px;padding-bottom:12px">
    <div style="width:80px;height:80px;border-radius:22px;background:var(--tt-color-accent-soft);display:flex;align-items:center;justify-content:center"><img src="logo.svg" alt="Tether logo" width="52" height="52"></div>
    <div><div class="t-display">Code from your phone.</div><p class="t-body-l c2" style="margin:10px 0 0">Tether runs a coding agent on this device. Your projects and your model keys stay here.</p></div>
    <div class="card col gap4"><b class="t-title-s">Phone check</b>
      ${checkRow(true, "Android 15", "Needs Android 10 or newer")}${checkRow(true, "18.4 GB free", "About 1 GB is needed to start")}
      ${CTX.offline ? checkRow(false, "No network", "You can finish setup offline. Testing a key and cloning wait until you are online.") : checkRow(true, "Online over Wi-Fi", "Needed to test a key and to clone")}</div>
  </div>`) + foot(1, btn("Set up your profile", { full: true, go: "profile" }))),
});

frame({
  id: "profile", area: "first", title: "2 of 4 · Your profile",
  why: "Tether has no account and no server. A local profile gives the agent an author name for the commits it makes for you, and sets how the app looks.",
  has: ["Avatar with initials and a color from the palette", "Display name; commit author name and email", "Import from git config", "Theme: System, Light, Dark"],
  config: ["Stored only on this phone", "Author name and email appear in commit history and pull requests", "Skip uses a placeholder and asks again at the first commit"],
  render: () => screen(appbar({ back: "welcome", title: "Your profile", sub: "Step 2 of 4" }) + body(`<div class="pad col gap16">
    <p class="t-body c2" style="margin:0">There is no Tether account. This profile stays on this phone and is used as the author of commits the agent makes for you.</p>
    <div class="col" style="align-items:center;gap:12px"><span class="avatar xl">BA</span>
      <div class="swatches">${["accent", "accent-2", "tonal", "info", "success", "warning"].map((c, i) => `<i class="${i === 0 ? "sel" : ""}" style="background:var(--tt-color-${c})">${i === 0 ? I("check", "s") : ""}</i>`).join("")}</div></div>
    ${field("Display name", "Blair", { focus: true })}
    ${field("Commit author name", "Blair Akandwanaho")}
    ${field("Commit author email", "blair@example.com", { hint: "GitHub offers a private no-reply address in your email settings." })}
    ${btn("Import from git config", { kind: "outline", full: true, icon: "download" })}
    <div><div class="t-body-s c2 mb8" style="font-weight:500">Theme</div><div class="chips">${chip("System", { sel: true })}${chip("Light")}${chip("Dark")}</div></div>
  </div>`) + foot(2, btn("Save profile", { full: true, go: "add-key" }), "add-key")),
});

const keyScreen = (state) => {
  const off = CTX.offline && state !== "err";
  return screen(appbar({ back: "profile", title: "Add a provider key", sub: "Step 3 of 4" }) + body(`<div class="pad col gap16">
  <p class="t-body c2" style="margin:0">Any one provider is enough. The key is wrapped by the Android Keystore and leaves this phone only in requests to that provider.</p>
  <div class="chips scroll">${["Anthropic", "OpenAI", "Google Gemini", "DeepSeek", "OpenRouter"].map((p, i) => chip(p, { sel: state === "err" ? i === 1 : i === 0 })).join("")}</div>
  ${field("API key", state === "err" ? "sk-proj-••••••••••••••••3f9a" : "sk-ant-••••••••••••••••••••••Qx2", { mono: true, focus: state !== "err", err: state === "err", trail: `<button class="btn text sm">${I("copy", "s")}Paste</button>`, hint: state === "err" ? "Key rejected by OpenAI. Check the key in Settings." : "Find it in console.anthropic.com under API keys." })}
  ${off ? `<div class="card" style="background:var(--tt-color-warning-soft)"><div class="row" style="align-items:flex-start"><span class="cwarn">${I("wifi-off")}</span><div class="grow"><b class="t-title-s cwarn">Cannot test while offline</b><div class="t-body-s cwarn">Save the key now. Tether tests it the next time you are online and tells you if it fails.</div></div></div></div>`
  : state === "err" ? `<div class="card" style="background:var(--tt-color-danger-soft)"><div class="row" style="align-items:flex-start"><span class="cdanger">${I("circle-x")}</span><div class="grow"><b class="t-title-s cdanger">Test failed: 401</b><div class="t-body-s cdanger">OpenAI says the key is invalid. Paste it again or create a new key.</div></div></div></div>`
  : `<div class="card" style="background:var(--tt-color-success-soft)"><div class="row"><span class="cok">${I("circle-check")}</span><div class="grow"><b class="t-title-s cok">Key accepted</b><div class="t-body-s cok">14 models available, including Sonnet 5.5 and Opus 5.5.</div></div></div></div>`}
  ${btn("Test connection", { kind: "outline", full: true, icon: "zap", disabled: off })}
</div>`) + foot(3, btn(off ? "Save key, test later" : "Save key", { full: true, go: "open-project", disabled: state === "err", snack: off ? "Key saved. It will be tested when you are online." : "Key saved. Testing it now." }), "open-project"));
};

frame({
  id: "add-key", area: "first", title: "3 of 4 · Provider key",
  why: "Nothing works without a model key, so this is the one step that matters. Test it before leaving so the first session does not fail. Offline, saving still works and the test waits.",
  has: ["Five providers as chips", "Key field with paste", "Test connection with an inline result", "Offline: test disabled, Save key, test later"],
  config: ["Provider, API key (Keystore), base URL for OpenRouter and custom endpoints", "Result states: accepted, rejected, network error, blocked by region", "Switch Network to Offline in the viewer to see the offline variant"],
  render: () => keyScreen("ok"),
});
frame({
  id: "add-key-error", area: "first", title: "Key rejected",
  why: "Errors say what happened and what to do, with the provider's name. Save stays disabled until a key passes.",
  has: ["Error field and inline block", "Exact provider and status code", "Save disabled"], config: ["Same screen as Provider key"],
  render: () => keyScreen("err"),
});

frame({
  id: "open-project", area: "first", title: "4 of 4 · Open a project",
  why: "A session needs a project. Offer the three ways in. GitHub is the only account Tether touches, and it is optional.",
  has: ["Clone from GitHub (sign in once)", "New empty project", "Import a zip", "Offline: clone is disabled with a reason, the other two work"],
  config: ["GitHub sign-in uses the device flow or a personal access token", "After opening, a prompt suggestion is based on the repo language"],
  render: () => screen(appbar({ back: "add-key", title: "Open a project", sub: "Step 4 of 4" }) + body(`<div class="pad col gap12">
    <p class="t-body c2" style="margin:0">A project is a code repository stored on this phone. The agent can only touch files inside it.</p>
    ${group([
      CTX.offline ? `<div class="lrow" style="opacity:.6"><span class="ico">${I("git-branch")}</span><span class="tx"><b>Clone from GitHub</b><span>Needs a network connection</span></span>${badge("Offline", "warn", "wifi-off")}</div>` : row({ icon: "git-branch", title: "Clone from GitHub", sub: "Sign in once, then pick a repository", go: "github-device" }),
      row({ icon: "folder-open", title: "New empty project", sub: "Start from nothing", go: "project-home" }),
      row({ icon: "package", title: "Import a zip", sub: "Pick an archive from your files", go: "project-home" }),
    ])}
    <div class="card"><div class="row" style="align-items:flex-start"><span class="c2">${I("info")}</span><span class="t-body-s c2">GitHub sign-in is optional. Without it you can still import a zip or start empty, but the agent cannot push or open pull requests.</span></div></div>
  </div>`) + foot(4, btn("Skip for now", { kind: "plain", full: true, go: "projects-empty" }))),
});

frame({
  id: "github-device", area: "first", title: "Sign in to GitHub", overlay: "open-project",
  why: "Device flow: no password handling in the app. The code is large enough to read and copy.",
  has: ["One-time code in mono", "Open github.com/login/device", "Waiting state", "Fallback: use a token"],
  config: ["Scopes asked: repo, workflow", "Token stored in the Keystore; revocable from Settings > GitHub"],
  overlayHtml: () => sheet("Sign in to GitHub", `<div class="col gap16 center">
    <p class="t-body c2" style="margin:0">Enter this code on GitHub to connect your account.</p>
    <div class="t-display mono" style="letter-spacing:4px;background:var(--tt-color-code-bg);border-radius:12px;padding:16px">WDJB-MJHT</div>
    ${btn("Copy code and open GitHub", { full: true, icon: "external-link", go: "clone" })}
    <div class="row c2" style="justify-content:center">${I("loader", "s spin")}<span class="t-body-s">Waiting for approval on GitHub</span></div>
    ${btn("Use a personal access token instead", { kind: "text" })}</div>`),
});

const proj = (n, branch, ch, when, extra = "") => `<button class="lrow"${nav({ go: "project-home" })}><span class="ico">${I("folder")}</span><span class="tx"><b>${n}</b><span class="mono t-mono-s">${branch}</span><span>${ch}${when}</span></span>${extra}</button>`;

frame({
  id: "projects", area: "projects", title: "Projects",
  why: "Resume work in one tap. Rows show the signal that matters: branch, uncommitted changes, last activity.",
  has: ["Menu, Search, and bell in the top bar; the dot on the bell when something needs you", "Rows sorted by last activity", "Badge for uncommitted changes and running sessions", "Primary action at the bottom: New project"],
  config: ["Sort: last activity (default), name", "Archive by swipe, with the same action in the row menu", "Settings is in the drawer, not in this top bar"],
  render: () => screen(appbar({ title: "Projects", menu: "sessions", actions: [{ icon: "search", label: "Search projects" }, { icon: "bell", label: "Needs attention, 2 items", go: "attention", dot: true }] }) + body(`<div class="pad col gap12">
    ${group([
      `<button class="lrow"${nav({ go: "project-home" })}><span class="ico">${I("folder")}</span><span class="tx"><b>acme-api</b><span><span class="mono">main</span> · 3 uncommitted · 12 min ago</span></span>${badge("Running", "tonal", "loader")}</button>`,
      proj("tether", "main", "Clean · ", "2 h ago"),
      proj("dotfiles", "feat/zsh", "1 uncommitted · ", "Yesterday"),
      proj("blog", "main", "Clean · ", "3 days ago"),
    ])}
    <div class="t-body-s c3 center">4 projects · 612 MB on this phone</div>
  </div>`) + `<div class="pad" style="padding-bottom:8px">${btn("New project", { full: true, icon: "plus", go: "sheet-newproject" })}</div>`),
});

frame({
  id: "projects-empty", area: "projects", title: "Projects, empty",
  why: "Empty states explain what goes here and offer one action. No illustration.",
  has: ["One sentence on what a project is", "The same three actions as New project"],
  config: ["None"],
  render: () => screen(appbar({ title: "Projects", menu: "sessions", actions: [{ icon: "bell", label: "Needs attention", go: "attention-empty" }] }) + body(`<div class="pad col gap16" style="padding-top:24px">
    <p class="t-body-l c2" style="margin:0">A project is a code repository stored on this phone. Open one to start a session.</p>
    ${group([row({ icon: "git-branch", title: "Clone from GitHub", go: "clone" }), row({ icon: "folder-open", title: "New empty project", go: "project-home" }), row({ icon: "package", title: "Import a zip", go: "project-home" })])}
  </div>`)),
});

// Needs attention: one list of what needs the user. Items leave when their cause is gone.
const attnRow = (dot, title, sub, go) => `<button class="lrow"${nav({ go })}><span class="ico">${statusDot(dot)}</span><span class="tx"><b>${title}</b><span>${sub}</span></span><span class="c3">${I("chevron-right")}</span></button>`;

frame({
  id: "attention", area: "projects", title: "Needs attention",
  why: "One place for what needs you. Items leave when their cause is gone; there is no read state.",
  has: ["Waiting for you, Failed, and Updates groups", "Each row opens what it names", "A dot on the bell when the list is not empty"],
  config: ["Not a notification history: system notifications stay in the Android shade"],
  render: () => screen(appbar({ title: "Needs attention", back: true }) + body(`${secTitle("Waiting for you")}<div class="pad">${group([attnRow("wait", "Migrate to Pydantic v2", "acme-api · Needs approval", "session-approval")])}</div>
    ${secTitle("Failed")}<div class="pad">${group([attnRow("err", "Stopped while running npm test", "blog · 12 min ago", "state-interrupted")])}</div>
    ${secTitle("Updates")}<div class="pad">${group([row({ icon: "circle-dot", title: "Version 0.0.4 is available", sub: "You have 0.0.3", go: "set-about" })])}</div>`)),
});

frame({
  id: "attention-empty", area: "projects", title: "Needs attention, empty",
  why: "An empty list says so in one sentence.",
  has: ["One sentence in the body-l secondary style", "No illustration"],
  config: ["None"],
  render: () => screen(appbar({ title: "Needs attention", back: true }) + body(`<div class="pad" style="padding-top:24px"><p class="t-body-l c2" style="margin:0">Nothing needs you right now.</p></div>`)),
});

frame({
  id: "sheet-newproject", area: "projects", title: "New project", overlay: "projects",
  why: "One sheet, three sources. Matches the three first-run options so there is one mental model.",
  has: ["Clone from GitHub", "New empty project", "Import a zip"],
  config: ["Remembers the last source used"],
  overlayHtml: () => sheet("New project", group([row({ icon: "git-branch", title: "Clone from GitHub", sub: "Pick a repository and branch", go: "clone" }), row({ icon: "folder-open", title: "New empty project", sub: "Name it and start from nothing", go: "project-home" }), row({ icon: "package", title: "Import a zip", sub: "Unpack an archive into a new project", go: "project-home" })])),
});

frame({
  id: "clone", area: "projects", title: "Clone from GitHub",
  why: "Search beats scrolling on a phone. Branch and clone live at the thumb.",
  has: ["Search field", "Repo rows with visibility and language", "Branch picker and Clone repo at the bottom"],
  config: ["Branch (default: repo default)", "Shallow clone depth (default: full) in Advanced"],
  render: () => screen(appbar({ back: true, title: "Clone from GitHub", sub: "khanblair" }) + body(`<div class="pad col gap12">
    ${CTX.offline ? `<div class="banner" style="margin:0">${I("wifi-off", "s")}Offline. Showing repositories saved from your last visit.</div>` : ""}${field("", "", { icon: "search", ph: "Search repositories" })}
    ${group([
      `<button class="lrow" style="background:var(--tt-color-accent-soft)"><span class="ico">${I("git-branch")}</span><span class="tx"><b>khanblair/acme-api</b><span>Python · private · updated 12 min ago</span></span><span class="cacc">${I("circle-check")}</span></button>`,
      row({ icon: "git-branch", title: "khanblair/tether", sub: "Rust · public · updated 2 h ago", chev: false }),
      row({ icon: "git-branch", title: "khanblair/dotfiles", sub: "Shell · public · updated yesterday", chev: false }),
      row({ icon: "git-branch", title: "kolaborate/mobile", sub: "TypeScript · private · updated 2 days ago", chev: false }),
    ])}
  </div>`) + `<div class="pad col gap12" style="padding-bottom:8px"><div class="row">${chip("main", { icon: "git-branch", trail: "chevron-down" })}<span class="t-body-s c3">Full history, about 48 MB</span></div>${btn("Clone repo", { full: true, go: "clone-progress", disabled: CTX.offline })}${CTX.offline ? `<div class="t-body-s cwarn center">Cloning needs a connection. It will not start until you are online.</div>` : ""}</div>`),
});

frame({
  id: "clone-progress", area: "projects", title: "Cloning",
  why: "Long operations show real progress and a way out.",
  has: ["Named target", "Progress bar and counts", "Cancel, then Open project when done"],
  config: ["Cancel keeps nothing"],
  render: () => screen(appbar({ back: "clone", title: "Cloning" }) + body(`<div class="pad col gap16" style="padding-top:12px">
    <div class="card col gap12"><div class="row"><span class="c2">${I("git-branch")}</span><b class="t-title-s">khanblair/acme-api</b><span class="grow"></span><span class="mono t-mono-s c3">main</span></div>
    ${meter(62, 100)}<div class="t-body-s c2">Receiving objects 1,204 of 1,940</div><div class="t-body-s c3">8.2 MB of 13.1 MB · 11 s left</div></div>
    <div class="row">${btn("Cancel", { kind: "plain", go: "clone" })}<span class="grow"></span>${btn("Open project", { go: "project-home", disabled: true })}</div></div>`)),
});

frame({
  id: "project-home", area: "projects", title: "Project home",
  why: "Answer what changed, how code runs here, and where to continue. The composer is visible so typing starts a session at once.",
  has: ["Repo, branch, remote", "Uncommitted changes card, opens the diff", "Default executor and instruction file status", "Recent sessions with status", "Composer at the bottom"],
  config: ["Default model and mode for this project", "Default executor (Sandbox, Linux mode, CI)", "Allowed commands and quality thresholds in .tether/config.toml"],
  render: () => screen(appbar({ menu: "sessions", title: "acme-api", sub: "main · github.com/khanblair/acme-api", actions: [{ icon: "ellipsis-vertical", label: "Project menu", go: "set-executors" }] }) + body(`<div class="pad col gap16">
    <button class="card row"${nav({ go: "diff" })} style="min-height:64px"><span class="c2">${I("file-diff", "l")}</span><span class="grow"><b class="t-title-s">3 uncommitted changes</b><div class="t-body-s c2"><span class="add">+42</span> <span class="del">−7</span> in 3 files</div></span><span class="btn text sm">Review</span></button>
    ${group([
      row({ icon: "terminal", title: "Default executor", sub: "Sandbox · Python 3.12, Node via QuickJS", go: "set-executors" }),
      row({ icon: "file-text", title: "TETHER.md", sub: "Found. Loaded into every session", right: badge("12 lines", "ok") }),
      row({ icon: "book-open", title: "Skills and memory", sub: "2 project skills · 6 memory entries", go: "skills" }),
    ])}
    <div><div class="row" style="justify-content:space-between"><b class="t-title-s">Recent sessions</b>${btn("New session", { kind: "text", sm: true, go: "session-idle" })}</div>
    ${group([
      `<button class="lrow"${nav({ go: "session-running" })}><span class="ico">${I("loader", "spin")}</span><span class="tx"><b>Fix login timeout</b><span>Running pytest · $0.21</span></span></button>`,
      `<button class="lrow"${nav({ go: "session-running" })}><span class="ico cok">${I("circle-check")}</span><span class="tx"><b>Add rate limit to /v1/search</b><span>PR #42 opened · yesterday</span></span></button>`,
      `<button class="lrow"${nav({ go: "session-running" })}><span class="ico c3">${I("circle-pause")}</span><span class="tx"><b>Migrate to Pydantic v2</b><span>Waiting for approval · 2 days ago</span></span></button>`,
    ])}</div>
  </div>`) + composer({ ph: "New session in acme-api. / for commands", disabled: true, sendGo: "session-idle" })),
});
