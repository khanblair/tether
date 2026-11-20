// UI helpers: every function returns an HTML string. Class names are defined in proto.css.
window.FRAMES = [];
window.AREAS = [];

window.I = (n, c = "") => `<svg class="i ${c}" viewBox="0 0 24 24" aria-hidden="true">${(window.ICONS && ICONS[n]) || ""}</svg>`;
window.esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

// navigation attributes
const nav = (o = {}) =>
  (o.go ? ` data-go="${o.go}"` : "") + (o.act ? ` data-act="${o.act}"` : "") + (o.back ? " data-back" : "") + (o.sheet ? ` data-go="${o.sheet}"` : "") + (o.snack ? ` data-snack="${String(o.snack).replace(/"/g, "&quot;")}"` : "");
window.nav = nav;

// Render context set by app.js before each frame is built.
window.CTX = { offline: false, pane: false, w: 412, h: 915, cls: "compact", land: false, kind: "phone" };

window.screen = (inner, o = {}) => {
  const strip = CTX.offline && !o.noStrip ? `<div class="offline-strip">${I("wifi-off", "s")}Offline</div>` : "";
  if (CTX.pane) return `<div class="screen pane">${inner}</div>`;
  return `<div class="screen">
  <div class="sb"><span>${o.time || "9:41"}</span><span class="r">${CTX.offline ? I("wifi-off") : I("wifi")}${I("battery-low")}</span></div>${strip}
  ${inner}
  ${o.noGesture ? "" : '<div class="gesture"><i></i></div>'}
</div>`;
};

window.appbar = (o) => {
  const lead = o.back
    ? `<button class="ibtn" aria-label="Back"${nav({ go: o.back === true ? null : o.back, back: o.back === true })}>${I("arrow-left", "l")}</button>`
    : o.menu
    ? `<button class="ibtn" aria-label="Open projects and sessions"${nav({ go: o.menu })}>${I("menu", "l")}</button>`
    : "";
  const acts = (o.actions || [])
    .map((a) => `<button class="ibtn ${a.accent ? "accent" : ""}" aria-label="${a.label}"${nav(a)}>${I(a.icon, "l")}${a.dot ? '<span class="dot" aria-hidden="true"></span>' : ""}</button>`)
    .join("");
  return `<div class="appbar">${lead}<div class="ttl"><b>${o.title}</b>${o.sub ? `<span>${o.sub}</span>` : ""}</div>${acts}</div>`;
};

window.body = (inner, cls = "") => `<div class="body ${cls}">${inner}</div>`;

window.btn = (label, o = {}) =>
  `<button class="btn ${o.kind || "primary"} ${o.full ? "full" : ""} ${o.sm ? "sm" : ""}"${o.disabled ? " disabled" : ""}${nav(o)}>${o.icon ? I(o.icon, "s") : ""}${label}${o.trail ? I(o.trail, "s") : ""}</button>`;

window.chip = (label, o = {}) =>
  `<button class="chip ${o.sel ? "sel" : ""} ${o.kind || ""}"${nav(o)}>${o.sel && !o.noCheck ? I("check", "s") : o.icon ? I(o.icon, "s") : ""}${label}${o.trail ? I(o.trail, "s") : ""}</button>`;

window.badge = (t, kind = "", icon = "") => `<span class="badge ${kind}">${icon ? I(icon) : ""}${t}</span>`;

window.row = (o) =>
  `<button class="lrow ${o.danger ? "danger" : ""}"${nav(o)}>${o.icon ? `<span class="ico">${I(o.icon)}</span>` : ""}<span class="tx"><b>${o.title}</b>${o.sub ? `<span>${o.sub}</span>` : ""}</span>${o.right || ""}${o.chev === false ? "" : `<span class="c3">${I("chevron-right")}</span>`}</button>`;

window.group = (rows) => `<div class="group">${rows.join("")}</div>`;
window.secTitle = (t) => `<div class="sec-title">${t}</div>`;
window.toggle = (on) => `<span class="toggle ${on ? "on" : ""}" role="switch" aria-checked="${!!on}"></span>`;
window.meter = (v, max, warn) => `<div class="meter ${warn ? "warn" : ""}"><i style="width:${Math.min(100, (v / max) * 100)}%"></i></div>`;

window.field = (label, value, o = {}) =>
  `<div class="field"><label>${label}</label><div class="input ${o.focus ? "focus" : ""} ${o.err ? "err" : ""}">${o.icon ? `<span class="c3">${I(o.icon)}</span>` : ""}<span class="grow ${o.mono ? "mono t-mono" : ""} ${value ? "" : "c3"}">${value || o.ph || ""}</span>${o.trail || ""}</div>${o.hint ? `<div class="hint ${o.err ? "cdanger" : ""}">${o.hint}</div>` : ""}</div>`;

window.opt = (title, sub, sel, extra = "") => `<button class="opt ${sel ? "sel" : ""}"><span class="rad"></span><span class="grow"><b class="t-title-s">${title}</b>${sub ? `<span class="t-body-s c2" style="display:block">${sub}</span>` : ""}${extra}</span></button>`;

window.statusline = (o = {}) => {
  const seg = (label, go) => `<button${nav({ go })}>${label}</button>`;
  const s = '<span class="sep">·</span>';
  if (o.bypass) {
    return `<div class="statusline bypass">${seg("Bypass", "sheet-mode")}${s}${seg(o.model || "Sonnet 5.5", "sheet-model")}${s}${seg((o.ctx || 34) + "%", "sheet-context")}${s}${seg("$" + (o.cost || "0.21"), "cost")}</div>`;
  }
  return `<div class="statusline">${seg(o.model || "Sonnet 5.5", "sheet-model")}${s}${seg(o.mode || "Accept edits", "sheet-mode")}${s}${seg((o.ctx ?? 34) + "%", "sheet-context")}${s}${seg("$" + (o.cost ?? "0.21"), "cost")}</div>`;
};

window.toolCard = (o) => {
  const st = {
    run: `<span class="st">${I("loader", "s spin")}Running</span>`,
    ok: `<span class="st cok">${I("circle-check", "s")}Done${o.dur ? " · " + o.dur : ""}</span>`,
    err: `<span class="st cdanger">${I("circle-x", "s")}Failed${o.dur ? " · " + o.dur : ""}</span>`,
    wait: `<span class="st cacc">${I("circle-pause", "s")}Needs approval</span>`,
  }[o.status || "ok"];
  return `<div class="tool"><button class="hd" aria-expanded="${!!o.body}"><span class="c2">${I(o.icon || "file-text")}</span><span class="v">${o.verb}</span><span class="tg">${o.target || ""}</span>${st}<span class="c3">${I(o.body ? "chevron-up" : "chevron-down", "s")}</span></button>${o.body || ""}${o.foot || ""}</div>`;
};

window.approval = (o) => `<div class="approval" role="alert">
  <div class="row"><span class="c2">${I(o.icon || "terminal")}</span><b class="t-title-s">${o.title}</b>${o.badge ? `<span style="margin-left:auto">${o.badge}</span>` : ""}</div>
  <div class="cmd">${o.command}</div>
  ${o.note ? `<div class="t-body-s c3 mb8">${o.note}</div>` : ""}
  <div class="acts"><button class="btn plain"${nav({ act: o.deny || "deny", go: o.denyGo })}>Deny</button><button class="btn text"${nav({ go: o.alwaysGo || "sheet-always" })}>Always allow${I("chevron-down", "s")}</button><span class="grow"></span><button class="btn primary"${nav({ act: o.allow || "allow", go: o.allowGo })}>Allow</button></div>
</div>`;

window.composer = (o = {}) => {
  const left = `<button class="ibtn" aria-label="Attach, commands, and mentions"${nav({ go: "sheet-plus" })} style="width:40px;height:40px">${I("plus")}</button>`;
  const mode = o.mode === "Auto" ? `<button class="chip accent"${nav({ go: "sheet-mode" })}>${I("zap", "s")}Auto</button>` : o.mode === "Bypass" ? `<button class="chip danger"${nav({ go: "sheet-mode" })}>${I("shield-alert", "s")}Bypass</button>` : `<button class="chip"${nav({ go: "sheet-mode" })}>${I("shield", "s")}${o.mode || "Accept edits"}</button>`;
  const model = `<button class="chip"${nav({ go: "sheet-model" })}>${o.model || "Sonnet 5.5"}${I("chevron-down", "s")}</button>`;
  const send = o.running
    ? `<button class="send stop" aria-label="Stop"${nav({ act: "stop" })}>${I("square", "s")}</button>`
    : `<button class="send" aria-label="Send"${o.disabled ? " disabled" : ""}${nav({ act: o.sendAct || "send", go: o.sendGo })}>${I("arrow-up")}</button>`;
  return `<div class="composer">
    ${o.attach || ""}
    <div class="field-line">${o.text ? `<span class="grow">${o.text}</span>` : `<span class="grow ph">${o.ph || "Message, / for commands, @ for files"}</span>`}</div>
    <div class="bar">${left}${mode}${model}<span class="grow"></span><button class="ibtn" aria-label="Voice input"${nav({ go: "sheet-voice" })} style="width:40px;height:40px">${I("mic")}</button>${send}</div>
  </div>`;
};

window.sheet = (title, inner, o = {}) => `<div class="scrim"></div><div class="sheet" ${o.h ? `style="height:${o.h}"` : ""} role="dialog" aria-label="${esc(title)}"><div class="grab"></div><div class="sh-title"><b class="t-title-m">${title}</b>${o.close === false ? "" : `<button class="ibtn" aria-label="Close" data-back>${I("x")}</button>`}</div><div class="sh-body">${inner}</div>${o.foot ? `<div class="sh-foot">${o.foot}</div>` : ""}</div>`;

// Snackbar: for undoable actions. Toast: a small confirmation that cannot be undone. Errors use neither.
window.snack = (text, action, bottom) => `<div class="snack" role="status" aria-live="polite"${bottom ? ` style="bottom:${bottom}px"` : ""}><span>${text}</span>${action ? `<button data-snack-action>${action}</button>` : ""}</div>`;
window.toast = (text, bottom) => `<div class="toast" role="status" aria-live="polite"${bottom ? ` style="bottom:${bottom}px"` : ""}>${text}</div>`;

window.skel = (n = 4) => Array.from({ length: n }, () => `<div class="lrow"><div class="skel" style="width:36px;height:36px;border-radius:8px"></div><div class="tx"><div class="skel" style="width:60%"></div><div class="skel mt8" style="width:40%;height:11px"></div></div></div>`).join("");

// diff block: lines = [kind, oldNo, newNo, html] where kind is "c" (context), "a", or "d"
window.diffFile = (o) => {
  const lines = o.lines
    .map((l) => (l.find ? `<div class="finding ${l.find.block ? "block" : ""}">${I(l.find.block ? "circle-x" : "triangle-alert", "s")}<span>${l.find.text}</span></div>` : `<div class="ln ${l.k === "a" ? "a" : l.k === "d" ? "d" : ""}"><span class="g">${l.o ?? ""}</span><span class="g">${l.n ?? ""}</span><span class="m">${l.k === "a" ? "+" : l.k === "d" ? "−" : ""}</span><span>${l.t}</span></div>`))
    .join("");
  return `<div class="diff"><div class="fh"><span class="ell">${o.path}</span><span><span class="add">+${o.add}</span> <span class="del">−${o.del}</span></span></div>
  <div class="hh"><span class="ell">${o.hunk}</span><span class="row">${btn("Undo", { kind: "plain", sm: true, snack: "Hunk undone|Undo" })}${btn("Keep", { kind: "text", sm: true, snack: "toast:Hunk kept" })}</span></div>${lines}</div>`;
};

window.statusDot = (kind) => ({ ok: `<span class="cok">${I("circle-check", "s")}</span>`, err: `<span class="cdanger">${I("circle-x", "s")}</span>`, run: `<span class="c2">${I("loader", "s spin")}</span>`, wait: `<span class="cwarn">${I("circle-pause", "s")}</span>` }[kind]);

window.frame = (f) => { FRAMES.push(f); return f; };
window.area = (id, title, why) => AREAS.push({ id, title, why });
