// Adaptive layouts: tablets and wide windows get panes instead of one stretched column.
window.FR = (id) => FRAMES.find((f) => f.id === id);

const famOf = (id) => {
  if (/^(session-|state-)/.test(id)) return "session";
  if (id === "settings" || /^set-/.test(id)) return "settings";
  if (id === "projects" || id === "project-home") return "projects";
  return null;
};

const pane = (f) => {
  CTX.pane = true;
  const html = f.render();
  CTX.pane = false;
  return html;
};

const railHtml = () => `<div class="screen pane" style="background:var(--tt-color-surface)">
  <div class="rail-head"><img src="logo.svg" width="24" height="24" alt=""><b class="t-title-m">Tether</b></div>
  <div class="pad" style="margin:4px 0 8px">${field("", "", { icon: "search", ph: "Search sessions" })}</div>
  <div class="body"><div class="sec-title" style="margin-top:4px">acme-api</div>
  ${[["Fix login timeout", "run", "Running"], ["Add rate limit to /v1/search", "ok", "PR #42"], ["Migrate to Pydantic v2", "wait", "Needs approval"]].map(([t, s, sub], i) => `<button class="lrow" style="min-height:52px${i === 0 ? ";background:var(--tt-color-accent-soft)" : ""}"${nav({ go: "session-running" })}><span class="ico">${statusDot(s)}</span><span class="tx"><b>${t}</b><span>${sub}</span></span></button>`).join("")}
  <div class="sec-title">tether</div><button class="lrow" style="min-height:52px"><span class="ico c3">${I("circle-check", "s")}</span><span class="tx"><b>Add UniFFI event path</b><span>Merged · 2 h ago</span></span></button></div>
  <div class="pad col gap12" style="padding-bottom:8px">${btn("New session", { icon: "plus", full: true, go: "session-idle" })}</div></div>`;

window.adaptive = (f) => {
  if (CTX.kind !== "tablet" || CTX.cls === "compact" || CTX.cls === "medium") return null;
  const id = f.overlay || f.id;
  const fm = famOf(id);
  if (!fm) return null;
  const base = FR(id);
  let cols = [];
  if (fm === "session") cols = [CTX.cls === "large" ? ["p-rail", railHtml()] : null, ["p-main", pane(base)], ["p-detail", pane(FR("diff"))]];
  if (fm === "settings") cols = [["p-list", pane(FR("settings"))], ["p-main", pane(id === "settings" ? FR("set-providers") : base)]];
  if (fm === "projects") cols = [["p-list", pane(FR("projects"))], ["p-main", pane(FR("project-home"))]];
  const strip = CTX.offline ? `<div class="offline-strip">${I("wifi-off", "s")}Offline</div>` : "";
  return `<div class="screen"><div class="sb"><span>9:41</span><span class="r">${CTX.offline ? I("wifi-off") : I("wifi")}${I("battery-low")}</span></div>${strip}<div class="pane-grid" style="flex:1;min-height:0">${cols.filter(Boolean).map(([c, h]) => `<div class="p ${c}">${h}</div>`).join("")}</div><div class="gesture"><i></i></div></div>`;
};
