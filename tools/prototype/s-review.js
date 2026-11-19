// Area 4: review and ship.
area("review", "Review and ship", "Reviewing a diff on a small screen is the riskiest step, so it gets the most care: per-hunk control, inline quality findings, checkpoints, and exact targets before anything is pushed.");

const kw = (t) => `<span class="kw">${t}</span>`;
const sessionHunk = [
  { k: "c", o: 41, n: 41, t: `${kw("def")} <span class="fn">is_expired</span>(token, now):` },
  { k: "c", o: 42, n: 42, t: `    <span class="cm">"""A token is expired once its expiry second is reached."""</span>` },
  { k: "d", o: 43, t: `    ${kw("return")} token.expires_at <mark class="wd">&lt;</mark> now` },
  { k: "a", n: 43, t: `    ${kw("return")} token.expires_at <mark class="wa">&lt;=</mark> now` },
  { k: "c", o: 44, n: 44, t: "" },
  { k: "c", o: 45, n: 45, t: `${kw("def")} <span class="fn">refresh_session</span>(session, now):` },
  { find: { text: "Warning: refresh_session is 58 lines, the limit is 50. Split it into smaller functions." } },
];
const testHunk = [
  { k: "a", n: 61, t: `${kw("def")} <span class="fn">test_token_expires_at_boundary</span>():` },
  { k: "a", n: 62, t: `    token = <span class="fn">make_token</span>(expires_at=<span class="nu">100</span>)` },
  { k: "a", n: 63, t: `    ${kw("assert")} <span class="fn">is_expired</span>(token, now=<span class="nu">100</span>)` },
  { k: "a", n: 64, t: `    ${kw("assert not")} <span class="fn">is_expired</span>(token, now=<span class="nu">99</span>)` },
  { find: { text: "Warning: assertions repeat the production comparison. Test the behavior with a named boundary constant." } },
];

frame({
  id: "diff", area: "review", title: "Diff view",
  why: "Review what the agent changed before it ships. Compact width uses a unified diff; wide screens add side by side.",
  has: ["File chips with +n −m; tap to jump", "Per-hunk Keep and Undo (48 dp targets)", "Word-level highlights, +/− glyphs always visible", "Quality findings inline under the line, with severity icon and label", "Wrap toggle, Commit changes at the bottom"],
  config: ["Line wrapping on by default", "Side by side only on expanded and wider windows", "Per-file actions in the overflow menu"],
  render: () => screen(appbar({ back: true, title: "Changes", sub: "3 files · +42 −7 · tether/fix-login-timeout", actions: [{ icon: "text-wrap", label: "Toggle line wrapping" }, { icon: "ellipsis-vertical", label: "More" }] }) +
    `<div class="pad" style="padding-bottom:8px"><div class="chips scroll">${chip("session.py +1 −1", { kind: "file", sel: true, noCheck: true })}${chip("test_login.py +38", { kind: "file" })}${chip("CHANGELOG.md +3 −6", { kind: "file" })}</div></div>` +
    body(`<div class="pad col gap12">${diffFile({ path: "src/auth/session.py", add: 1, del: 1, hunk: "@@ -41,7 +41,7 @@ is_expired", lines: sessionHunk })}${diffFile({ path: "tests/test_login.py", add: 38, del: 0, hunk: "@@ -58,0 +61,38 @@", lines: testHunk })}
    <div class="diff"><div class="fh"><span class="ell">CHANGELOG.md</span><span><span class="add">+3</span> <span class="del">−6</span></span></div><div class="hh"><span>Collapsed · 1 hunk</span>${btn("Expand", { kind: "text", sm: true })}</div></div></div>`) +
    `<div class="pad row" style="padding-bottom:8px">${btn("Undo all", { kind: "plain", go: "sheet-rewind" })}<span class="grow"></span>${btn("Commit changes", { go: "commit" })}</div>`),
});

frame({
  id: "sheet-rewind", area: "review", title: "Rewind", overlay: "session-diffcard",
  why: "Mistakes are cheap when undo is cheap. A checkpoint is saved before each edit batch.",
  has: ["Checkpoints with time and files touched", "Three explicit options: files and conversation, files only, conversation only", "Rewind button states exactly what will be restored"],
  config: ["Checkpoints are kept per session", "Restoring files never touches the Git history"],
  overlayHtml: () => sheet("Rewind", `<div class="col gap12">${[["Before editing session.py", "12 min ago · 1 file", false], ["After tests added", "6 min ago · 2 files", true], ["Before CHANGELOG.md", "2 min ago · 3 files", false]].map(([a, b, s]) => opt(a, b, s)).join("")}
  <div class="sec-title" style="margin:8px 0 0">Restore</div><div class="chips">${chip("Files and conversation", { sel: true })}${chip("Files only")}${chip("Conversation only")}</div>
  ${btn("Rewind files and conversation", { full: true, kind: "tonal", back: true, snack: "Rewound 2 files and 4 messages|Undo" })}<div class="t-body-s c3">This restores 2 files and removes 4 messages after the checkpoint.</div></div>`),
});

frame({
  id: "commit", area: "review", title: "Commit",
  why: "The agent writes the message; the user confirms files and branch. Pushing is a separate, explicit step.",
  has: ["Branch chip (agent branches are tether/<task>)", "Editable message draft with Regenerate", "File list with checkboxes", "Quality result and push guard note", "Commit (primary) and Commit and push"],
  config: ["Author from the profile", "Push guard: never the default branch without asking; force push asks even in Bypass"],
  render: () => screen(appbar({ back: true, title: "Commit", sub: "acme-api" }) + body(`<div class="pad col gap16">
    <div class="row">${chip("tether/fix-login-timeout", { icon: "git-branch", trail: "chevron-down" })}<span class="t-body-s c3">New branch from main</span></div>
    <div><div class="row" style="justify-content:space-between"><b class="t-title-s">Message</b>${btn("Regenerate", { kind: "text", sm: true, icon: "refresh-cw" })}</div>
    <div class="input mt8" style="align-items:flex-start;padding:10px 12px;min-height:120px"><div class="t-body">Fix token expiry at the boundary second<br><br><span class="c2">is_expired used &lt;, so a token stayed valid at its exact expiry second. Compare with &lt;= and add a regression test.</span></div></div></div>
    <div>${group(["src/auth/session.py|+1 −1", "tests/test_login.py|+38", "CHANGELOG.md|+3 −6"].map((r) => { const [p, c] = r.split("|"); return `<div class="lrow" style="min-height:48px"><span class="cacc">${I("circle-check")}</span><span class="tx"><b class="mono t-mono-label">${p}</b></span><span class="t-mono-s c2">${c}</span></div>`; }))}</div>
    <button class="card row"${nav({ go: "quality" })}><span class="cwarn">${I("triangle-alert")}</span><span class="grow"><b class="t-title-s">Quality gate: 2 warnings, none blocking</b><div class="t-body-s c2">Tap to see them. Warnings do not stop the commit.</div></span><span class="c3">${I("chevron-right")}</span></button>
    <div class="t-body-s c3">Author: Blair Akandwanaho &lt;blair@example.com&gt;</div></div>`) +
    `<div class="pad col gap12" style="padding-bottom:8px">${btn("Commit and push", { full: true, go: "sheet-push" })}${btn("Commit only", { kind: "outline", full: true, go: "session-diffcard", snack: "Committed 3 files|Undo" })}</div>`),
});

frame({
  id: "sheet-push", area: "review", title: "Confirm push", overlay: "commit",
  why: "Anything that changes a remote shows its exact target first.",
  has: ["Remote URL and branch", "Commits being pushed", "Flags: new branch, not the default branch, no force", "Push and Cancel"],
  config: ["Always asks, in every mode but Bypass; force push asks even in Bypass"],
  overlayHtml: () => sheet("Push to GitHub", `<div class="col gap16"><div class="card col gap4"><div class="t-body-s c3">Target</div><div class="mono t-mono-label">github.com/khanblair/acme-api</div><div class="mono t-mono-label">tether/fix-login-timeout</div></div>
  <div class="col gap4"><div class="row"><span class="cok">${I("circle-check", "s")}</span><span class="t-body">1 commit: Fix token expiry at the boundary second</span></div><div class="row"><span class="cok">${I("circle-check", "s")}</span><span class="t-body">New branch, not the default branch</span></div><div class="row"><span class="cok">${I("circle-check", "s")}</span><span class="t-body">No force push</span></div></div>
  <div class="row">${btn("Cancel", { kind: "plain", back: true })}<span class="grow"></span>${btn("Push", { go: "pr-draft" })}</div></div>`),
});

frame({
  id: "pr-draft", area: "review", title: "Open a pull request",
  why: "The agent drafts, the user edits. The base, title, and body are all visible before anything is posted.",
  has: ["Base and head branches", "Title and Markdown body", "Draft toggle", "Open pull request (primary), Edit with agent"],
  config: ["Template from .github/pull_request_template.md when present", "Posting asks for approval in every mode but Bypass"],
  render: () => screen(appbar({ back: true, title: "Open a pull request", sub: "khanblair/acme-api" }) + body(`<div class="pad col gap16">
    <div class="row">${chip("tether/fix-login-timeout", { icon: "git-branch" })}<span class="c3">${I("arrow-right", "s")}</span>${chip("main", { icon: "git-branch", trail: "chevron-down" })}</div>
    ${field("Title", "Fix token expiry at the boundary second", { focus: true })}
    <div class="field"><label>Description</label><div class="input" style="align-items:flex-start;padding:10px 12px;min-height:150px"><div class="t-body"><b>What changed</b><br>is_expired now uses &lt;=, so a token expires at its expiry second.<br><br><b>Tests</b><br>Added test_token_expires_at_boundary. 12 passed.</div></div></div>
    <div class="row" style="justify-content:space-between"><span class="t-body">Open as draft</span>${toggle(false)}</div></div>`) +
    `<div class="pad row" style="padding-bottom:8px">${btn("Edit with agent", { kind: "plain" })}<span class="grow"></span>${btn("Open pull request", { go: "pr-done" })}</div>`),
});

frame({
  id: "pr-done", area: "review", title: "Pull request opened",
  why: "Close the loop with a link and live CI, so the user knows if the push worked.",
  has: ["PR number and link", "CI jobs with status icon and label", "Ask agent to fix when a job fails"],
  config: ["CI polling backs off from 5 s to 30 s", "Failed logs are trimmed to the failing step"],
  render: () => screen(appbar({ back: "session-diffcard", title: "Pull request #43", sub: "khanblair/acme-api" }) + body(`<div class="pad col gap16">
    <div class="card col gap12"><div class="row"><span class="cok">${I("circle-check", "l")}</span><b class="t-title-m">Pull request opened</b></div><div class="mono t-mono-label cacc">github.com/khanblair/acme-api/pull/43</div><div class="row">${btn("Open on GitHub", { kind: "outline", sm: true, icon: "external-link" })}${btn("Copy link", { kind: "plain", sm: true, icon: "copy" })}</div></div>
    <div><b class="t-title-s">Checks</b>${group([["rust", "ok", "1 m 12 s"], ["tooling", "ok", "48 s"], ["android", "run", "Running · 3 m"], ["bindings", "ok", "55 s"]].map(([n, s, d]) => `<div class="lrow" style="min-height:48px"><span class="ico">${statusDot(s)}</span><span class="tx"><b>${n}</b></span><span class="t-body-s c3">${d}</span></div>`))}</div>
    ${btn("Ask the agent to watch CI", { kind: "tonal", full: true, icon: "eye", go: "ci-run" })}</div>`)),
});

frame({
  id: "quality", area: "review", title: "Quality gate",
  why: "Catch code smells and AI slop before they reach a commit. Findings carry a severity icon, a label, and a place.",
  has: ["Summary: 2 warnings, 0 blocking", "Grouped findings with file and line", "Fix with agent or Ignore once per finding", "What was checked"],
  config: ["Thresholds and warn or block per rule in Settings > Quality gate", "Block-level findings stop commits in Auto mode"],
  render: () => screen(appbar({ back: true, title: "Quality gate", sub: "3 files checked in 140 ms" }) + body(`<div class="pad col gap16">
    <div class="card row"><span class="cwarn">${I("triangle-alert", "l")}</span><div class="grow"><b class="t-title-m">2 warnings, 0 blocking</b><div class="t-body-s c2">You can commit. Fixing both takes about a minute.</div></div></div>
    <div><b class="t-title-s">Code smells</b>${group([`<div class="lrow" style="align-items:flex-start;padding:12px 14px"><span class="cwarn" style="margin-top:2px">${I("triangle-alert")}</span><span class="tx"><b>Long function <span class="badge warn">Warning</span></b><span class="mono t-mono-s">src/auth/session.py:45</span><span class="t-body-s c2" style="margin-top:4px">refresh_session is 58 lines. The limit is 50.</span><span class="row" style="margin-top:6px">${btn("Fix with agent", { kind: "text", sm: true })}${btn("Ignore once", { kind: "plain", sm: true })}</span></span></div>`])}</div>
    <div><b class="t-title-s">Slop</b>${group([`<div class="lrow" style="align-items:flex-start;padding:12px 14px"><span class="cwarn" style="margin-top:2px">${I("triangle-alert")}</span><span class="tx"><b>Test repeats the code under test <span class="badge warn">Warning</span></b><span class="mono t-mono-s">tests/test_login.py:63</span><span class="t-body-s c2" style="margin-top:4px">The assertion copies the production comparison. Use a named boundary constant.</span><span class="row" style="margin-top:6px">${btn("Fix with agent", { kind: "text", sm: true })}${btn("Ignore once", { kind: "plain", sm: true })}</span></span></div>`])}</div>
    <div class="t-body-s c3">Checked: length, nesting, parameters, duplicates, placeholders, restating comments, vacuous tests, banned words.</div></div>`)),
});

frame({
  id: "review", area: "review", title: "Second-model review (/review)",
  why: "A different model reads the diff cold. Findings are ranked and every one has a place.",
  has: ["Reviewer model chip", "Findings ranked by severity", "Apply selected fixes"],
  config: ["Reviewer model in Settings > Models and defaults", "Costs tokens; the estimate shows before it runs"],
  render: () => screen(appbar({ back: true, title: "Review", sub: "Opus 5.5 reviewed 3 files · $0.07" }) + body(`<div class="pad col gap12">
    ${[["Medium", "warn", "Boundary comparison changes behavior for tokens issued with zero lifetime", "src/auth/session.py:43", true], ["Low", "", "CHANGELOG.md lost 6 lines that were not related to this fix", "CHANGELOG.md:12", true], ["Low", "", "New test has no negative case for now = expires_at - 1 beyond one second", "tests/test_login.py:64", false]].map(([sv, k, t, w, sel]) => `<div class="card col gap4"><div class="row">${badge(sv, k)}<span class="mono t-mono-s c3 grow">${w}</span><span class="${sel ? "cacc" : "c3"}">${I(sel ? "circle-check" : "circle-dot")}</span></div><div class="t-body">${t}</div></div>`).join("")}</div>`) +
    `<div class="pad row" style="padding-bottom:8px"><span class="t-body-s c3">2 selected</span><span class="grow"></span>${btn("Apply selected fixes", { go: "session-diffcard" })}</div>`),
});
