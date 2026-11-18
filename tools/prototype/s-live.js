// Live session demo: type-free scripted turn. Send, watch streaming and tool calls, Allow or Deny, then review.
window.LIVE = { items: [], phase: "idle", cost: 0, ctx: 3, gen: 0, resolve: null, text: "Fix the failing login test" };

const liveReset = () => { LIVE.gen++; Object.assign(LIVE, { items: [], phase: "idle", cost: 0, ctx: 3, resolve: null }); };
const sleep = (ms, gen) => new Promise((res) => setTimeout(() => res(gen === LIVE.gen), ms));
const patch = () => window.livePatch && window.livePatch();

async function typeInto(item, text, gen) {
  for (let i = 0; i < text.length; i += 3) {
    item.t = text.slice(0, i + 3);
    patch();
    if (!(await sleep(28, gen))) return false;
  }
  item.t = text; item.done = true; patch();
  return true;
}

async function playTurn() {
  const gen = ++LIVE.gen;
  LIVE.phase = "running"; LIVE.items = [{ k: "user", t: LIVE.text }]; LIVE.cost = 0.02; LIVE.ctx = 6;
  const think = { k: "think" }; LIVE.items.push(think); patch();
  if (!(await sleep(1500, gen))) return;
  LIVE.items.pop();
  const a1 = { k: "agent", t: "", done: false }; LIVE.items.push(a1);
  if (!(await typeInto(a1, "Looking at the expiry check first.", gen))) return;
  const rd = { k: "reads", status: "run" }; LIVE.items.push(rd); patch();
  if (!(await sleep(1100, gen))) return;
  rd.status = "ok"; LIVE.cost = 0.09; LIVE.ctx = 14;
  const a2 = { k: "agent", t: "", done: false }; LIVE.items.push(a2);
  if (!(await typeInto(a2, "is_expired compares with <, so a token is still valid at its exact expiry second. The test expects it to expire then. Fixing the comparison.", gen))) return;
  LIVE.items.push({ k: "edit" }); LIVE.cost = 0.16; LIVE.ctx = 21; patch();
  if (!(await sleep(700, gen))) return;
  LIVE.items.push({ k: "run-wait" }); LIVE.phase = "approval"; patch();
  const allowed = await new Promise((res) => { LIVE.resolve = res; });
  if (gen !== LIVE.gen) return;
  LIVE.phase = "running";
  const run = LIVE.items.find((i) => i.k === "run-wait");
  if (!allowed) {
    run.k = "run-denied"; patch();
    const a3 = { k: "agent", t: "", done: false }; LIVE.items.push(a3);
    if (!(await typeInto(a3, "Understood. I will run the tests in the sandbox instead.", gen))) return;
  }
  const target = allowed ? "pip install -e . && pytest -q" : "pytest -q";
  const where = allowed ? "Linux mode" : "Sandbox";
  const r = { k: "run", status: "run", target, where, lines: [] };
  LIVE.items.push(r); patch();
  for (const line of ["collected 12 items", "tests/test_login.py ....F...", "tests/test_login.py ....F...  [ 66%]", "12 passed in 4.8s"]) {
    if (!(await sleep(650, gen))) return;
    r.lines.push(line); patch();
  }
  r.status = "ok"; LIVE.cost = 0.31; LIVE.ctx = 36; patch();
  const a4 = { k: "agent", t: "", done: false }; LIVE.items.push(a4);
  if (!(await typeInto(a4, "The boundary test passes, and I added test_token_expires_at_boundary so it stays fixed.", gen))) return;
  LIVE.items.push({ k: "diff" }); LIVE.cost = 0.38; LIVE.ctx = 41; LIVE.phase = "done"; patch();
}

window.LIVEACT = {
  send: () => { if (LIVE.phase === "idle" || LIVE.phase === "done") playTurn(); },
  stop: () => { LIVE.gen++; LIVE.phase = "stopped"; LIVE.items.push({ k: "stopped" }); patch(); },
  allow: () => { if (LIVE.resolve) { const r = LIVE.resolve; LIVE.resolve = null; r(true); } },
  deny: () => { if (LIVE.resolve) { const r = LIVE.resolve; LIVE.resolve = null; r(false); } },
  reset: () => { liveReset(); patch(); },
};

const liveItem = (it) => {
  switch (it.k) {
    case "user": return U(esc(it.t));
    case "think": return `<div class="row c2">${I("loader", "s spin")}<span class="t-body">Thinking</span></div>`;
    case "agent": return A(esc(it.t), !it.done);
    case "reads": return toolCard({ icon: "search", verb: it.status === "run" ? "Reading files" : "Read 4 files, searched 2 times", status: it.status });
    case "edit": return editCard();
    case "run-wait": return toolCard({ icon: "terminal", verb: "Run", target: "pip install -e . && pytest -q", status: "wait" });
    case "run-denied": return toolCard({ icon: "terminal", verb: "Denied", target: "pip install -e . && pytest -q", status: "err" });
    case "run": return toolCard({ icon: "terminal", verb: it.status === "run" ? "Running" : "Ran", target: it.target, status: it.status, dur: it.status === "ok" ? "6.2 s" : "", body: `<div class="out"><span class="badge">${it.where}</span>\n${esc(it.lines.join("\n"))}</div>` });
    case "diff": return `<div class="tool"><button class="hd"><span class="c2">${I("file-diff")}</span><span class="v">Edited 3 files</span><span class="tg"></span><span class="st"><span class="add">+42</span> <span class="del">−7</span></span></button><div class="files"><div class="f"><span class="ell">src/auth/session.py</span><span><span class="add">+1</span> <span class="del">−1</span></span></div><div class="f"><span class="ell">tests/test_login.py</span><span class="add">+38</span></div></div><div class="ft"><span class="row cwarn t-body-s">${I("triangle-alert", "s")}2 warnings</span>${btn("Review", { kind: "text", sm: true, go: "diff" })}</div></div><div class="chips">${chip("Commit", { icon: "git-commit-horizontal", go: "commit" })}${chip("Open PR", { icon: "git-pull-request", go: "pr-draft" })}${btn("Run again", { kind: "plain", sm: true, act: "reset" })}</div>`;
    case "stopped": return `<div class="card"><div class="row"><span class="c2">${I("circle-pause")}</span><div class="grow"><b class="t-title-s">Stopped</b><div class="t-body-s c2">You interrupted the turn. Nothing else was changed.</div></div>${btn("Start over", { kind: "text", sm: true, act: "reset" })}</div></div>`;
  }
  return "";
};

window.liveParts = () => ({
  sl: statusline({ ctx: LIVE.ctx, cost: LIVE.cost.toFixed(2), model: "Sonnet 5.5" }),
  body: `<div class="transcript">${LIVE.items.length ? LIVE.items.map(liveItem).join("") : `<div class="card col gap12"><b class="t-title-s">Live demo</b><div class="t-body-s c2">Press Send. The agent reads files, edits one, asks to run a command, and shows the diff. Try Allow, Deny, and Stop.</div></div>`}</div>`,
  bottom: (LIVE.phase === "approval" ? approval({ title: "Run in Linux mode", command: "pip install -e . &amp;&amp; pytest -q", badge: badge("Linux mode", "tonal"), note: "Needs a package install, so the sandbox cannot run it. Network is on for this command.", allow: "allow", deny: "deny", alwaysGo: "sheet-always" }) : "") +
    composer({ running: LIVE.phase === "running" || LIVE.phase === "approval", text: LIVE.phase === "idle" || LIVE.phase === "done" || LIVE.phase === "stopped" ? esc(LIVE.text) : "", ph: "Reply, or press Stop", sendAct: "send" }),
});

frame({
  id: "session-live", area: "session", title: "Live demo (tap Send)",
  why: "The core workflow end to end: send, stream, tool calls, approval, diff. Interactive in the Interactive view; the gallery shows the start state.",
  has: ["Send starts a scripted turn", "Thinking, streaming text with caret, grouped tool cards", "Approval bar: Allow runs in Linux mode; Deny falls back to the sandbox", "Stop interrupts at any point; Run again resets"],
  config: ["Status line cost and context update as the turn progresses", "In the real app the model, mode, and budget control which steps ask"],
  render: () => { const p = liveParts(); return screen(
    appbar({ menu: "sessions", title: "Fix login timeout", sub: "acme-api · main", actions: [{ icon: "file-diff", label: "Review changes", go: "diff" }, { icon: "ellipsis-vertical", label: "Session menu", go: "sheet-session-menu" }] }) +
    `<div id="live-sl">${p.sl}</div><div class="body" id="live-body" style="position:relative">${p.body}</div><div id="live-bottom">${p.bottom}</div>`); },
});
