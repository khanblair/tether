// Prototype shell: gallery, interactive view, flows, and display settings.
(() => {
  const DEVICES = {
    "phone-s": { label: "Phone S", kind: "phone", w: 360, h: 740 },
    "phone-l": { label: "Phone L", kind: "phone", w: 412, h: 915 },
    "tablet-s": { label: "Tablet S", kind: "tablet", w: 600, h: 960 },
    "tablet-l": { label: "Tablet L", kind: "tablet", w: 900, h: 1280 },
  };
  const FLOWS = [
    { id: "first", title: "First run", note: "Four setup screens. Set Network to Offline to see the offline checks.", steps: ["welcome", "profile", "add-key", "open-project", "github-device", "clone", "clone-progress", "project-home"] },
    { id: "agent", title: "Agent run to pull request", note: "Press Send, then Allow.", steps: ["session-idle", "session-live", "diff", "quality", "commit", "sheet-push", "pr-draft", "pr-done"] },
    { id: "bg", title: "Approve from the background", steps: ["session-running", "notif-shade", "notif-lock", "session-approval", "sheet-always", "sheet-deny"] },
    { id: "undo", title: "Undo and rewind", steps: ["session-diffcard", "sheet-msg", "sheet-rewind", "diff"] },
    { id: "know", title: "Skills, memory, vault", steps: ["skills", "skill-approve", "skill-install", "memory", "session-search", "vault", "vault-note", "vault-map", "vault-lint", "sheet-vault-sync"] },
    { id: "cfg", title: "Settings and cost", note: "Appearance changes the whole prototype.", steps: ["settings", "set-profile", "set-providers", "set-permissions", "set-appearance", "cost"] },
    { id: "feedback", title: "Feedback and notifications", note: "Tap Undo on a snackbar. Switch the theme to check the inverse colors.", steps: ["snack-archive", "snack-commit", "snack-hunk", "snack-rewind", "toast-copied", "banners", "banner-info", "set-notifications", "set-notifications-off", "notif-shade", "notif-more", "notif-lock", "notif-lock-private"] },
    { id: "err", title: "When things go wrong", steps: ["state-thinking", "state-ratelimit", "state-offline", "state-budget", "state-interrupted", "state-key", "state-compaction", "session-bypass"] },
  ];

  const CFG = { view: "gallery", theme: "system", device: "phone-l", land: false, fs: 100, ds: 100, offline: false, reduce: false, area: "all", cur: "welcome", flow: null };
  try { Object.assign(CFG, JSON.parse(localStorage.getItem("tether-proto") || "{}")); } catch (e) { /* storage unavailable */ }
  let hist = [];
  const save = () => { try { localStorage.setItem("tether-proto", JSON.stringify(CFG)); } catch (e) { /* ignore */ } };
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];

  // ---------- geometry ----------
  const widthClass = (w) => (w < 600 ? "compact" : w < 840 ? "medium" : w < 1200 ? "expanded" : "large");
  function geometry() {
    const d = DEVICES[CFG.device] || DEVICES["phone-l"];
    const W = CFG.land ? d.h : d.w, H = CFG.land ? d.w : d.h;
    const ds = CFG.ds / 100;
    const dpW = Math.round(W / ds), dpH = Math.round(H / ds);
    return { d, W, H, ds, dpW, dpH, cls: widthClass(dpW), kind: d.kind, land: CFG.land };
  }
  const setCtx = (g) => Object.assign(CTX, { offline: CFG.offline, pane: false, w: g.dpW, h: g.dpH, cls: g.cls, land: g.land, kind: g.kind });

  function frameHtml(f, g) {
    setCtx(g);
    const baseF = f.overlay ? FR(f.overlay) : f;
    let html = adaptive(baseF) || baseF.render();
    if (f.overlay) html += f.overlayHtml();
    return html;
  }
  function phoneHtml(f, g, scale) {
    return `<div class="phone ${g.kind === "tablet" ? "tablet" : ""} ${g.cls !== "compact" ? "wide" : ""} ${g.land ? "land" : ""} ${g.kind}-kind" data-frame="${f.id}" style="width:${g.dpW}px;height:${g.dpH}px;transform:scale(${scale * g.ds})">${frameHtml(f, g)}</div>`;
  }

  // font scale: scale every text element, like Android's font size setting
  function applyFontScale(root) {
    const k = CFG.fs / 100;
    if (k === 1 || !root) return;
    const work = [];
    for (const el of root.querySelectorAll("*")) {
      if (el.namespaceURI !== "http://www.w3.org/1999/xhtml") continue;
      let own = false;
      for (const n of el.childNodes) if (n.nodeType === 3 && n.textContent.trim()) { own = true; break; }
      if (!own) continue;
      const cs = getComputedStyle(el);
      const fs = parseFloat(cs.fontSize);
      let lh = parseFloat(cs.lineHeight);
      if (Number.isNaN(lh)) lh = fs * 1.3;
      work.push([el, fs * k, lh * k]);
    }
    for (const [el, a, b] of work) { el.style.fontSize = a + "px"; el.style.lineHeight = b + "px"; }
  }
  function decorate(root) {
    $$("[data-cfg]", root).forEach((b) => {
      const [key, val] = b.dataset.cfg.split(":");
      b.classList.toggle("sel", String(CFG[key]) === val);
    });
  }
  const afterMount = (el) => { decorate(el); applyFontScale(el); };

  // ---------- controls ----------
  function syncControls() {
    document.documentElement.dataset.theme = CFG.theme === "system" ? "" : CFG.theme;
    if (CFG.theme === "system") delete document.documentElement.dataset.theme;
    document.body.classList.toggle("reduce-motion", CFG.reduce);
    $$("[data-set]").forEach((b) => { const [k, v] = b.dataset.set.split(":"); b.setAttribute("aria-pressed", String(CFG[k]) === v); });
    $$("[data-view]").forEach((b) => b.setAttribute("aria-pressed", CFG.view === b.dataset.view));
    $$("[data-select]").forEach((s) => { s.value = String(CFG[s.dataset.select]); });
    const g = geometry();
    $("#readout").textContent = `${g.d.label}${g.land ? " landscape" : ""} · ${g.dpW} × ${g.dpH} dp · ${g.cls}${g.kind === "tablet" && g.cls !== "compact" && g.cls !== "medium" ? " · panes" : ""}`;
  }
  function setCfg(k, v) {
    CFG[k] = v === "true" ? true : v === "false" ? false : /^\d+$/.test(v) ? Number(v) : v;
    save(); render();
  }

  // ---------- gallery ----------
  const frameAreas = () => AREAS.filter((a) => FRAMES.some((f) => f.area === a.id));
  function renderGallery() {
    $("#areas").hidden = false;
    $("#areas").innerHTML = [{ id: "all", title: "All screens" }, ...frameAreas()].map((a) => `<button data-area="${a.id}" aria-pressed="${CFG.area === a.id}">${a.title}</button>`).join("");
    const g = geometry();
    const target = g.kind === "tablet" ? (g.land ? 520 : 400) : g.land ? 440 : 300;
    const gs = target / g.W, cw = Math.round(g.W * gs), ch = Math.round(g.H * gs);
    const list = frameAreas().filter((a) => CFG.area === "all" || CFG.area === a.id);
    $("#view").innerHTML = `<div class="gallery">${list.map((a) => `<section id="a-${a.id}"><h2 class="area-head">${a.title}</h2><p class="area-why">${a.why}</p>
      <div class="frames" style="grid-template-columns:repeat(auto-fill,minmax(${cw}px,1fr))">${FRAMES.filter((f) => f.area === a.id).map((f) => `<div class="fcell"><div class="fview ${g.kind}" data-open="${f.id}" data-lazy="${f.id}" style="width:${cw}px;height:${ch}px" role="button" tabindex="0" aria-label="Open ${esc(f.title)} in the interactive view"></div>
        <div class="fcap" style="width:${cw}px"><h3>${f.title}</h3><p class="why">${f.why}</p><details><summary>Contents and configuration</summary><h4>On the screen</h4><ul>${(f.has || []).map((x) => `<li>${x}</li>`).join("")}</ul><h4>Configuration</h4><ul>${(f.config || []).map((x) => `<li>${x}</li>`).join("")}</ul></details></div></div>`).join("")}</div></section>`).join("")}</div>`;
    const io = new IntersectionObserver((es) => es.forEach((e) => {
      if (!e.isIntersecting) return;
      const el = e.target; io.unobserve(el);
      const f = FR(el.dataset.lazy);
      el.innerHTML = phoneHtml(f, g, gs);
      afterMount(el);
    }), { rootMargin: "600px" });
    $$("[data-lazy]").forEach((el) => io.observe(el));
  }

  // ---------- interactive ----------
  const flowOf = () => FLOWS.find((f) => f.id === CFG.flow);
  function railHtml() {
    const fl = flowOf();
    return `<h2>Flows</h2>${FLOWS.map((f) => `<button data-flow="${f.id}" aria-current="${CFG.flow === f.id}">${f.title}</button>${CFG.flow === f.id ? f.steps.map((s, i) => `<button class="flow-step" data-frame="${s}" aria-current="${CFG.cur === s}">${i + 1}. ${FR(s).title}</button>`).join("") : ""}`).join("")}
    <h2>All screens</h2>${frameAreas().map((a) => `<h2 style="margin-top:10px">${a.title}</h2>${FRAMES.filter((f) => f.area === a.id).map((f) => `<button data-frame="${f.id}" aria-current="${CFG.cur === f.id}">${f.title}</button>`).join("")}`).join("")}`;
  }
  function notesHtml(f) {
    const a = AREAS.find((x) => x.id === f.area);
    const g = geometry();
    const lay = { compact: "Single pane.", medium: "Single pane with wider margins.", expanded: "Two panes: transcript and detail.", large: "Three panes: sessions, transcript, detail." }[g.cls];
    return `<div class="card2"><span class="pill">${a ? a.title : ""}</span><h3 style="margin-top:8px">${f.title}</h3><p><b>Why:</b> ${f.why}</p>
    <h2>On the screen</h2><ul>${(f.has || []).map((x) => `<li>${x}</li>`).join("")}</ul><h2>Configuration</h2><ul>${(f.config || []).map((x) => `<li>${x}</li>`).join("")}</ul>
    <h2>Layout</h2><p>${g.dpW} × ${g.dpH} dp, ${g.cls}. ${g.kind === "tablet" ? lay : g.land ? "Landscape on a phone: transcript only, composer collapses to one line." : lay} Text ${CFG.fs}%, display ${CFG.ds}%.</p></div>`;
  }
  function mountInteractive() {
    $("#areas").hidden = true;
    $("#view").innerHTML = `<div class="interactive"><aside class="rail" id="rail" aria-label="Flows and screens"></aside><section><div class="flowbar" id="flowbar"></div><div class="stage" id="stage"></div></section><aside class="notes" id="notes" aria-label="Screen notes"></aside></div>`;
    $("#rail").innerHTML = railHtml();
    updateStage();
  }
  function updateStage() {
    const f = FR(CFG.cur) || FRAMES[0];
    const g = geometry();
    const stageW = ($("#stage") && $("#stage").clientWidth) || 700;
    const availH = Math.max(480, innerHeight - 210);
    const scale = Math.min(1, availH / (g.H + 24), (stageW - 8) / (g.W + 24));
    const fl = flowOf();
    const idx = fl ? fl.steps.indexOf(CFG.cur) : -1;
    $("#flowbar").innerHTML = `${hist.length ? `<button class="btn outline sm" data-nav="back">${I("arrow-left", "s")}Back</button>` : ""}${fl ? `<b>${fl.title}</b>${idx >= 0 ? `<span>Step ${idx + 1} of ${fl.steps.length}</span><button class="btn outline sm" data-nav="prev" ${idx <= 0 ? "disabled" : ""}>Previous</button><button class="btn tonal sm" data-nav="next" ${idx < 0 || idx >= fl.steps.length - 1 ? "disabled" : ""}>Next step</button>` : `<span>Outside this flow</span>`}${fl.note ? `<span class="c3">${fl.note}</span>` : ""}` : `<span class="c3">Tap anything in the phone. Pick a flow on the left for a guided path.</span>`}`;
    $("#stage").innerHTML = `<div class="stage-inner" style="width:${Math.round(g.W * scale)}px;height:${Math.round(g.H * scale)}px">${phoneHtml(f, g, scale)}</div>`;
    afterMount($("#stage .phone"));
    $("#notes").innerHTML = notesHtml(f);
    mountSnack();
    $$("#rail [data-frame]").forEach((b) => b.setAttribute("aria-current", String(b.dataset.frame === CFG.cur)));
    $$("#rail [data-flow]").forEach((b) => b.setAttribute("aria-current", String(b.dataset.flow === CFG.flow)));
  }

  function go(id, push = true) {
    if (!FR(id)) return;
    if (push && CFG.cur !== id) hist.push(CFG.cur);
    CFG.cur = id; save();
    try { history.replaceState(null, "", "#" + id); } catch (e) { /* file origin */ }
    if (CFG.view !== "interactive") { CFG.view = "interactive"; render(); return; }
    if (flowOf() && !flowOf().steps.includes(id)) { /* stay in the flow bar, mark outside */ }
    $("#rail").innerHTML = railHtml();
    updateStage();
  }
  function back() { const p = hist.pop(); if (p) go(p, false); }
  function backTo(id) { const i = hist.lastIndexOf(id); if (i >= 0) hist.length = i; go(id, false); }

  function render() {
    syncControls();
    if (CFG.view === "gallery") renderGallery(); else mountInteractive();
  }

  // ---------- snackbars and toasts ----------
  let snackSpec = null, snackTimer;
  const parseSnack = (s) => (s.startsWith("toast:") ? { toast: true, text: s.slice(6) } : { text: s.split("|")[0], action: s.split("|")[1] || "" });
  function mountSnack() {
    const ph = $(".stage .phone");
    if (!ph) return;
    const old = $(".snack-host", ph);
    if (old) old.replaceWith();
    if (!snackSpec) return;
    const screenEl = $(".screen", ph);
    const kids = screenEl ? [...screenEl.children].filter((k) => !k.classList.contains("gesture")) : [];
    const last = kids[kids.length - 1];
    const bottom = last && !last.classList.contains("body") && !last.classList.contains("pane-grid") ? ph.offsetHeight - last.offsetTop + 8 : 40;
    const host = document.createElement("div");
    host.className = "snack-host";
    host.innerHTML = snackSpec.toast ? toast(esc(snackSpec.text), bottom) : snack(esc(snackSpec.text), snackSpec.action ? esc(snackSpec.action) : "", bottom);
    ph.appendChild(host);
    applyFontScale(host);
  }
  function showSnack(spec) {
    snackSpec = spec; clearTimeout(snackTimer); mountSnack();
    snackTimer = setTimeout(() => { snackSpec = null; mountSnack(); }, spec.toast ? 2200 : 4500);
  }

  // live demo patch (called by s-live.js while a turn plays)
  window.livePatch = () => {
    const ph = $(".stage .phone");
    const body = ph && $("#live-body", ph);
    if (!body) return;
    const p = liveParts();
    $("#live-sl", ph).innerHTML = p.sl; body.innerHTML = p.body; $("#live-bottom", ph).innerHTML = p.bottom;
    ["#live-sl", "#live-body", "#live-bottom"].forEach((s) => afterMount($(s, ph)));
    body.scrollTop = body.scrollHeight;
  };

  // ---------- events ----------
  document.addEventListener("click", (e) => {
    const t = e.target;
    const view = t.closest("[data-view]"); if (view) { CFG.view = view.dataset.view; save(); render(); return; }
    const set = t.closest("[data-set]"); if (set) { const [k, v] = set.dataset.set.split(":"); setCfg(k, v); return; }
    const area = t.closest("[data-area]"); if (area) { CFG.area = area.dataset.area; save(); render(); window.scrollTo({ top: 0 }); return; }
    const open = t.closest("[data-open]"); if (open) { CFG.flow = CFG.flow; go(open.dataset.open); return; }
    const flow = t.closest("[data-flow]"); if (flow) { CFG.flow = CFG.flow === flow.dataset.flow ? null : flow.dataset.flow; hist = []; save(); if (CFG.flow) go(flowOf().steps[0], false); else { $("#rail").innerHTML = railHtml(); updateStage(); } return; }
    const fr = t.closest("#rail [data-frame]"); if (fr) { hist = []; go(fr.dataset.frame, false); return; }
    const nv = t.closest("[data-nav]"); if (nv) {
      const fl = flowOf(); const i = fl ? fl.steps.indexOf(CFG.cur) : -1;
      if (nv.dataset.nav === "back") back();
      if (nv.dataset.nav === "next" && i >= 0) go(fl.steps[i + 1]);
      if (nv.dataset.nav === "prev" && i > 0) go(fl.steps[i - 1], false);
      return;
    }
    const ph = t.closest(".stage .phone"); if (!ph) return;
    const cfg = t.closest("[data-cfg]"); if (cfg) { const [k, v] = cfg.dataset.cfg.split(":"); setCfg(k, v); return; }
    if (t.closest("[data-snack-action]")) { showSnack({ toast: true, text: "Undone" }); return; }
    const el = t.closest("[data-go],[data-back],[data-act],[data-snack]"); if (!el) return;
    if (el.dataset.snack) showSnack(parseSnack(el.dataset.snack));
    const act = el.dataset.act, goId = el.dataset.go;
    if (act && CFG.cur === "session-live" && window.LIVEACT[act]) { LIVEACT[act](); if (!goId) { livePatch(); return; } }
    if (el.hasAttribute("data-back")) { back(); return; }
    if (goId) {
      const isBack = el.getAttribute("aria-label") === "Back";
      if (goId === "session-live" && act === "send") { go(goId); LIVEACT.send(); return; }
      if (isBack) backTo(goId); else go(goId);
    }
  });
  document.addEventListener("change", (e) => { const s = e.target.closest("[data-select]"); if (s) setCfg(s.dataset.select, s.value); });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && CFG.view === "interactive") back();
    const o = e.target.closest && e.target.closest("[data-open]"); if (o && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); go(o.dataset.open); }
  });
  let rz; addEventListener("resize", () => { clearTimeout(rz); rz = setTimeout(() => { if (CFG.view === "interactive") updateStage(); }, 150); });

  const h = location.hash.slice(1); if (h && FR(h)) { CFG.cur = h; CFG.view = "interactive"; }
  if (!FR(CFG.cur)) CFG.cur = "welcome";
  render();
})();
