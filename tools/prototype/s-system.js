// Area: system surfaces. The shade and lock screen follow the theme through the same tokens as the app.
area("system", "System surfaces", "Things that happen outside the app: notifications that approve a run from the lock screen, battery guidance before the first long run, app lock, and the share sheet. The notification shade and lock screen follow the light or dark theme.");

// acts: [label, primary, go, snack]
const notif = (o) => `<div class="notif"${o.go ? ` data-go="${o.go}"` : ""}><div class="hd"><img src="logo.svg" alt="">Tether · ${o.proj} · ${o.when}</div><div class="ti">${o.title}</div>${o.text ? `<div class="tx">${o.text}</div>` : ""}${o.cmd ? `<div class="cmdn">${o.cmd}</div>` : ""}${o.acts ? `<div class="acts">${o.acts.map(([l, p, go, sn]) => `<button class="${p ? "p" : ""}"${nav({ go, snack: sn })}>${l}</button>`).join("")}</div>` : ""}</div>`;
const sbar = () => `<div class="sbar"><span>9:41</span><span class="row" style="gap:6px">${CTX.offline ? I("wifi-off", "s") : I("wifi", "s")}${I("battery-low", "s")}</span></div>`;

frame({
  id: "notif-shade", area: "system", title: "Notifications: approval and status",
  why: "Long runs continue in the background, so the notification is a second screen: an approval with the exact command, live status, and a result.",
  has: ["Approval with the command and Allow and Deny", "Foreground run status with cost and Stop", "Finished turn with Review and Commit", "Tap a card to open the session"],
  config: ["Notification permission is asked the first time a turn runs in the background, not during setup", "Allow and Deny do exactly what the in-app approval bar does", "Channels and importance are set in Settings > Notifications"],
  render: () => `<div class="screen"><div class="shade">${sbar()}<div style="height:6px"></div>
    ${notif({ proj: "acme-api", when: "now", title: "Run in Linux mode?", cmd: "pip install -e . &amp;&amp; pytest -q", go: "session-approval", acts: [["Deny", false, "sheet-deny"], ["Allow", true, "session-running", "toast:Allowed. The run continues."]] })}
    ${notif({ proj: "acme-api", when: "3 min", title: "Fix login timeout", text: "Running pytest · $0.21 · 34% context", go: "session-running", acts: [["Stop", false, "state-interrupted"], ["Open", false, "session-running"]] })}
    ${notif({ proj: "tether", when: "yesterday", title: "Turn finished", text: "3 files changed, +42 −7. 2 quality warnings.", go: "session-diffcard", acts: [["Review", false, "diff"], ["Commit", true, "commit"]] })}</div></div>`,
});

frame({
  id: "notif-more", area: "system", title: "Notifications: needs attention",
  why: "Anything that stops a run gets its own notification with the one action that unblocks it. Several at once collapse into a group.",
  has: ["Budget reached: Raise budget, Stop", "Error with the provider named: Open settings", "Interrupted by the system: Resume, Discard", "Group summary when three or more sessions need attention"],
  config: ["Budget and error notifications use the high-importance channel", "Each notification links to the exact screen that fixes it", "Quiet hours never mute approvals unless you turn that on"],
  render: () => `<div class="screen"><div class="shade">${sbar()}<div style="height:6px"></div>
    <div class="notif" style="padding:10px 16px"><div class="grp"><b class="t-title-s">3 sessions need attention</b><span class="c2">${I("chevron-up", "s")}</span></div></div>
    ${notif({ proj: "acme-api", when: "now", title: "Session budget of $2.00 reached", text: "The agent paused after the last tool finished.", go: "state-budget", acts: [["Stop", false, "session-running"], ["Raise budget", true, "set-budgets"]] })}
    ${notif({ proj: "dotfiles", when: "4 min", title: "Key rejected by Gemini", text: "The turn did not start. Check the key in Settings.", go: "state-key", acts: [["Open settings", true, "set-provider-detail"]] })}
    ${notif({ proj: "blog", when: "12 min", title: "Stopped while running npm test", text: "Android ended the app to free memory. Your files are safe.", go: "state-interrupted", acts: [["Discard", false, "projects"], ["Resume", true, "session-running", "toast:Resuming npm test"]] })}</div></div>`,
});

frame({
  id: "notif-lock", area: "system", title: "Approve from the lock screen",
  why: "The user can keep a long run moving without opening the app. Only Allow and Deny appear here.",
  has: ["Clock and one approval notification", "Allow and Deny only"],
  config: ["Bypass and Always allow are not offered here", "The system may require unlock to act", "Command text can be hidden on the lock screen (Settings > Notifications)"],
  render: () => `<div class="screen"><div class="shade lock">${sbar()}<div class="clock">9:41</div><div class="center c2" style="margin-bottom:16px">Wednesday, 18 November</div>${notif({ proj: "acme-api", when: "now", title: "Run in Linux mode?", cmd: "pip install -e . &amp;&amp; pytest -q", go: "session-approval", acts: [["Deny", false, "sheet-deny"], ["Allow", true, "session-running", "toast:Allowed. The run continues."]] })}</div></div>`,
});

frame({
  id: "notif-lock-private", area: "system", title: "Lock screen with content hidden",
  why: "Commands can contain paths and tokens. With the privacy setting on, the lock screen shows that an approval is waiting but not what it is.",
  has: ["Generic title, no command text", "Allow is not offered; the user must unlock to see what they approve"],
  config: ["Setting: Show command text on the lock screen (off by default)", "Redaction applies to notifications, not to the app"],
  render: () => `<div class="screen"><div class="shade lock">${sbar()}<div class="clock">9:41</div><div class="center c2" style="margin-bottom:16px">Wednesday, 18 November</div>${notif({ proj: "Tether", when: "now", title: "Approval needed", text: "Content hidden. Unlock to see the command.", go: "applock", acts: [["Unlock to review", true, "applock"]] })}</div></div>`,
});

frame({
  id: "sheet-battery", area: "system", title: "Keep long runs alive", overlay: "session-running",
  why: "Android makers kill background apps. Ask for the exemption in context, right before the first long run, with steps for this phone.",
  has: ["What happens and why", "Steps for the detected phone maker", "Open battery settings; Not now"],
  config: ["Shown once before the first long run, never at install", "Texts exist per maker: Samsung, Xiaomi, Pixel, others"],
  overlayHtml: () => sheet("Keep long runs alive", `<div class="col gap16"><p class="t-body c2" style="margin:0">Your phone may stop Tether when the screen is off. A run longer than a few minutes could end early.</p>
    <div class="card col gap12"><b class="t-title-s">On Samsung</b><div class="row" style="align-items:flex-start"><b>1</b><span class="t-body">Open Battery and device care, then Battery.</span></div><div class="row" style="align-items:flex-start"><b>2</b><span class="t-body">Choose Background usage limits, then Never sleeping apps.</span></div><div class="row" style="align-items:flex-start"><b>3</b><span class="t-body">Add Tether.</span></div></div>
    ${btn("Open battery settings", { full: true, icon: "external-link", back: true })}${btn("Not now", { kind: "plain", full: true, back: true })}</div>`),
});

frame({
  id: "applock", area: "system", title: "App lock",
  why: "Keys and code are valuable. An optional lock asks for the fingerprint after time in the background.",
  has: ["Logo and one action", "Biometric prompt, with device credential fallback"],
  config: ["Lock after 5 minutes away (changeable)", "Notifications stay readable but never show command text while locked"],
  render: () => screen(body(`<div class="col" style="height:100%;align-items:center;justify-content:center;gap:20px;padding:24px"><div style="width:84px;height:84px;border-radius:22px;background:var(--tt-color-accent-soft);display:flex;align-items:center;justify-content:center"><img src="logo.svg" width="54" height="54" alt="Tether"></div><div class="center"><div class="t-title-m">Tether is locked</div><div class="t-body c2 mt4">Use your fingerprint to continue.</div></div>${btn("Unlock", { icon: "fingerprint", kind: "tonal", go: "projects" })}</div>`)),
});

frame({
  id: "share", area: "system", title: "Share to Tether",
  why: "Start work from anywhere: share an error, a URL, or a file from another app and pick the project.",
  has: ["The shared content preview", "Project list", "Start session (primary)"],
  config: ["Text becomes the first message; files are attached", "Needs no extra permission"],
  render: () => screen(appbar({ back: true, title: "Share to Tether" }) + body(`<div class="pad col gap16"><div class="card col gap4"><div class="t-body-s c3">Shared text</div><div class="t-mono-s">TypeError: 'NoneType' object is not subscriptable<br>  File "src/auth/session.py", line 47</div></div>
    <div><b class="t-title-s">Project</b>${group([["acme-api", true], ["tether", false], ["dotfiles", false]].map(([n, s]) => `<button class="lrow" style="min-height:52px"><span class="ico">${I("folder")}</span><span class="tx"><b>${n}</b></span>${s ? `<span class="cacc">${I("circle-check")}</span>` : ""}</button>`))}</div></div>`) + `<div class="pad" style="padding-bottom:8px">${btn("Start session in acme-api", { full: true, go: "session-idle" })}</div>`),
});
