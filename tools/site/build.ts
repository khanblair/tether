// Builds the GitHub Pages site into site-dist/ from docs/*.md, tokens, and images.
//
// Usage: node tools/site/build.ts
// The landing page pulls its facts from the docs, so the site cannot drift from them.
// The build fails on a broken link between pages or to a heading that does not exist.

import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { Marked } from "marked";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const docsDir = resolve(root, "docs");
const out = resolve(root, "site-dist");
const repoUrl = "https://github.com/khanblair/tether";
const siteUrl = "https://khanblair.github.io/tether/";

// ---- markdown ----

function slug(text: string): string {
  return text
    .toLowerCase()
    .replace(/<[^>]+>/g, "")
    .replace(/[`*_]/g, "")
    .replace(/[^\p{L}\p{N}\s-]/gu, "")
    .trim()
    .replace(/\s/g, "-");
}

const marked = new Marked({
  gfm: true,
  renderer: {
    heading({ tokens, depth }) {
      const html = this.parser.parseInline(tokens);
      return `<h${depth} id="${slug(html)}">${html}</h${depth}>\n`;
    },
  },
});

function render(md: string): string {
  return marked
    .parse(md, { async: false })
    .replace(/<table>/g, '<div class="tablewrap"><table>')
    .replace(/<\/table>/g, "</table></div>");
}

/** Returns the body of the section whose heading matches, up to the next heading of the same or higher level. */
function section(md: string, heading: RegExp): string {
  const lines = md.split("\n");
  const start = lines.findIndex((l) => /^#{1,6} /.test(l) && heading.test(l));
  if (start === -1) throw new Error(`Section not found: ${heading}`);
  const depth = lines[start].match(/^#+/)![0].length;
  let end = lines.length;
  for (let i = start + 1; i < lines.length; i++) {
    const m = lines[i].match(/^(#+) /);
    if (m && m[1].length <= depth) {
      end = i;
      break;
    }
  }
  return lines.slice(start + 1, end).join("\n").trim();
}

// ---- pages ----

type Doc = { name: string; title: string; md: string };

function loadDocs(): Doc[] {
  return readdirSync(docsDir)
    .filter((f) => f.endsWith(".md"))
    .sort()
    .map((file) => {
      const md = readFileSync(resolve(docsDir, file), "utf8");
      const title = md.match(/^# (.+)$/m)?.[1] ?? file;
      return { name: file.replace(/\.md$/, ""), title, md };
    });
}

function fixLinks(html: string, fromDocs: boolean): string {
  return html
    .replace(/href="([^":#]+)\.md(#[^"]*)?"/g, (_m, p: string, hash = "") => {
      return `href="${fromDocs ? "" : "docs/"}${p.replace(/^\.\//, "")}.html${hash}"`;
    })
    .replace(/href="\.\.\/([^"]+)"/g, (_m, p: string) => `href="${repoUrl}/blob/main/${p}"`);
}

function shell(opts: { title: string; root: string; body: string; nav: string; current?: string }): string {
  const { title, body, nav } = opts;
  const r = opts.root;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title}</title>
<meta name="description" content="Tether is an Android coding agent harness that runs entirely on the phone.">
<meta property="og:title" content="${title}">
<meta property="og:description" content="A coding agent harness that runs on your phone. No PC. No server. Your own keys.">
<meta property="og:image" content="${siteUrl}assets/og-image.png">
<link rel="icon" href="${r}assets/logo.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="${r}assets/apple-touch-icon.png">
<link rel="stylesheet" href="${r}assets/tokens.css">
<link rel="stylesheet" href="${r}assets/site.css">
<script>
try { var t = localStorage.getItem("tether-theme"); if (t === "light" || t === "dark") document.documentElement.dataset.theme = t; } catch (e) {}
</script>
</head>
<body>
<header class="topbar"><div class="wrap">
  <a class="brand" href="${r}index.html"><img src="${r}assets/logo.svg" alt="" width="28" height="28">Tether</a>
  <nav class="topnav" aria-label="Main">${nav}<button class="theme-btn" type="button" id="theme-btn" aria-label="Change theme">Theme</button></nav>
</div></header>
${body}
<footer><div class="wrap">Tether is a work in progress. <a href="${repoUrl}">Source on GitHub</a>. No license has been chosen yet, so all rights are reserved.</div></footer>
<script>
(function () {
  var btn = document.getElementById("theme-btn"), order = ["system", "light", "dark"];
  function current() { try { return localStorage.getItem("tether-theme") || "system"; } catch (e) { return "system"; } }
  function apply(v) {
    if (v === "system") { delete document.documentElement.dataset.theme; } else { document.documentElement.dataset.theme = v; }
    btn.textContent = "Theme: " + v;
    try { localStorage.setItem("tether-theme", v); } catch (e) {}
  }
  btn.textContent = "Theme: " + current();
  btn.addEventListener("click", function () { apply(order[(order.indexOf(current()) + 1) % 3]); });
})();
</script>
</body>
</html>
`;
}

function landing(docs: Doc[]): string {
  const byName = (n: string) => docs.find((d) => d.name === n)!;
  const phase = section(byName("progress-tracker").md, /Current phase/);
  const tools = section(byName("development").md, /Required tools/);
  const tree = section(byName("project-structure").md, /^## Top level/);
  const goals = section(byName("project-overview").md, /^## Goals/);
  const index = section(byName("README").md, /What each doc answers/);
  const device = section(byName("project-overview").md, /Success criteria/).match(/Reference device:.*/)?.[0] ?? "";

  const body = `
<main>
<div class="wrap">
  <div class="hero">
    <div>
      <h1>Tether</h1>
      <p>A coding agent harness that runs entirely on an Android phone. No PC. No server. Your own model keys.</p>
      <div class="actions" style="margin-top:20px">
        <a class="btn primary" href="docs/project-overview.html">Read the overview</a>
        <a class="btn" href="${repoUrl}">View on GitHub</a>
      </div>
    </div>
    <img src="assets/logo.svg" alt="Tether logo">
  </div>

  <section class="block" id="status"><h2>Status</h2>
    <div class="card prose"><span class="status">In progress</span>${render(phase)}</div>
  </section>

  <section class="block" id="goals"><h2>What Tether does</h2><div class="prose">${render(goals)}</div></section>

  <section class="block" id="requirements"><h2>Requirements</h2>
    <div class="grid two">
      <div class="card prose"><h3>To use Tether</h3>
        <ul>
          <li>Android 10 (API 29) or newer, on arm64 or x86_64.</li>
          <li>An API key for at least one provider: Anthropic, OpenAI, Google Gemini, DeepSeek, or OpenRouter.</li>
          <li>A GitHub account, to clone, push, and run CI builds.</li>
        </ul>
        <p class="pill-note">${device.replace(/`/g, "")}</p>
      </div>
      <div class="card prose"><h3>To build Tether</h3>${render(tools)}</div>
    </div>
  </section>

  <section class="block" id="layout"><h2>Project layout</h2><div class="prose">${render(tree)}
    <p>Every file and folder is described in <a href="docs/project-structure.html">project-structure</a>.</p></div>
  </section>

  <section class="block" id="docs"><h2>Docs</h2><div class="prose">${render(index)}</div></section>
</div>
</main>`;
  const nav = `<a href="#status">Status</a><a href="#requirements">Requirements</a><a href="#layout">Layout</a><a href="docs/README.html">Docs</a>`;
  return fixLinks(shell({ title: "Tether, an AI coding agent on your phone", root: "", body, nav }), false);
}

function docPage(doc: Doc, docs: Doc[]): string {
  const links = docs
    .map((d) => `<li><a href="${d.name}.html"${d.name === doc.name ? ' aria-current="page"' : ""}>${d.name === "README" ? "Index" : d.name}</a></li>`)
    .join("");
  const body = `
<main><div class="wrap layout">
  <aside class="sidebar" aria-label="Docs"><h2>Docs</h2><ul>${links}</ul></aside>
  <article class="prose">${fixLinks(render(doc.md), true)}
    <p class="pill-note"><a href="${repoUrl}/blob/main/docs/${doc.name}.md">Edit this page on GitHub</a></p>
  </article>
</div></main>`;
  const nav = `<a href="../index.html#status">Status</a><a href="../index.html#requirements">Requirements</a><a href="../index.html#layout">Layout</a>`;
  return shell({ title: `${doc.title} | Tether`, root: "../", body, nav });
}

// ---- link check ----

function checkLinks(pages: Map<string, string>): void {
  const ids = new Map<string, Set<string>>();
  for (const [path, html] of pages) {
    ids.set(path, new Set([...html.matchAll(/ id="([^"]+)"/g)].map((m) => m[1])));
  }
  const broken: string[] = [];
  for (const [path, html] of pages) {
    for (const m of html.matchAll(/href="([^":]*?\.html)(#[^"]*)?"/g)) {
      const target = resolve("/" + dirname(path), m[1]).slice(1);
      if (!ids.has(target)) broken.push(`${path}: missing page ${m[1]}`);
      else if (m[2] && !ids.get(target)!.has(m[2].slice(1))) broken.push(`${path}: missing heading ${m[1]}${m[2]}`);
    }
  }
  if (broken.length) {
    console.error("Broken links:\n  " + broken.join("\n  "));
    process.exit(1);
  }
}

// ---- main ----

const docs = loadDocs();
const pages = new Map<string, string>();
pages.set("index.html", landing(docs));
for (const doc of docs) pages.set(`docs/${doc.name}.html`, docPage(doc, docs));
checkLinks(pages);

rmSync(out, { recursive: true, force: true });
mkdirSync(resolve(out, "docs"), { recursive: true });
mkdirSync(resolve(out, "assets/fonts"), { recursive: true });
for (const [path, html] of pages) writeFileSync(resolve(out, path), html);

execFileSync("node", [resolve(root, "tokens/generate.ts"), "--css", resolve(out, "assets/tokens.css")], { stdio: "inherit" });
copyFileSync(resolve(here, "site.css"), resolve(out, "assets/site.css"));
copyFileSync(resolve(root, "images/logo.svg"), resolve(out, "assets/logo.svg"));
copyFileSync(resolve(root, "images/export/og-image-1200x630.png"), resolve(out, "assets/og-image.png"));
copyFileSync(resolve(root, "images/export/apple-touch-icon-180.png"), resolve(out, "assets/apple-touch-icon.png"));
const fonts = resolve(root, "android/designsystem/src/main/res/font");
for (const f of readdirSync(fonts)) copyFileSync(resolve(fonts, f), resolve(out, "assets/fonts", f));
copyFileSync(resolve(root, "android/designsystem/FONTS-LICENSE.txt"), resolve(out, "assets/fonts/OFL.txt"));
writeFileSync(resolve(out, ".nojekyll"), "");
console.log(`Built ${pages.size} pages into ${out}`);
