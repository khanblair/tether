// Generates Tokens.kt (and CSS for the site) from tokens/tokens.json.
//
// Usage (Node 22.18 or later runs TypeScript directly):
//   node tokens/generate.ts                 write Tokens.kt
//   node tokens/generate.ts --check         fail if Tokens.kt is stale or a contrast pair fails
//   node tokens/generate.ts --contrast      print the measured contrast table as Markdown
//   node tokens/generate.ts --css <file>    write the CSS variables to <file>

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type Themed = { light: string; dark: string };
type Theme = "light" | "dark";
type Pair = { fg: string; bg: string; min: number; note?: string };

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const tokens = JSON.parse(readFileSync(resolve(here, "tokens.json"), "utf8"));
const ktPath = resolve(
  root,
  "android/designsystem/src/main/kotlin/app/tether/design/tokens/Tokens.kt",
);

const themes: Theme[] = ["light", "dark"];

function camel(name: string): string {
  const parts = name.split("-");
  return parts[0] + parts.slice(1).map((p) => p[0].toUpperCase() + p.slice(1)).join("");
}

function pascal(name: string): string {
  const c = camel(name);
  return c[0].toUpperCase() + c.slice(1);
}

function kotlinColor(hex: string): string {
  return `Color(0xFF${hex.slice(1).toUpperCase()})`;
}

function num(n: number): string {
  return Number.isInteger(n) ? String(n) : String(n);
}

// ---- contrast (WCAG 2.x) ----

function channel(c: number): number {
  const v = c / 255;
  return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

function luminance(hex: string): number {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function ratio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

function lookup(path: string, theme: Theme): string {
  const [group, name] = path.split(".");
  const entry: Themed | undefined = tokens[group]?.[name];
  if (!entry) throw new Error(`Unknown token in contrast list: ${path}`);
  return entry[theme];
}

function measure(): { pair: Pair; theme: Theme; value: number }[] {
  const rows: { pair: Pair; theme: Theme; value: number }[] = [];
  for (const pair of tokens.contrast as Pair[]) {
    for (const theme of themes) {
      rows.push({ pair, theme, value: ratio(lookup(pair.fg, theme), lookup(pair.bg, theme)) });
    }
  }
  return rows;
}

function failures(): string[] {
  return measure()
    .filter((r) => r.value < r.pair.min)
    .map(
      (r) =>
        `${r.theme}: ${r.pair.fg} on ${r.pair.bg} is ${r.value.toFixed(2)}, needs ${r.pair.min}`,
    );
}

function contrastTable(): string {
  const byPair = new Map<string, { pair: Pair; light?: number; dark?: number }>();
  for (const r of measure()) {
    const key = `${r.pair.fg} on ${r.pair.bg} ${r.pair.min}`;
    const row = byPair.get(key) ?? { pair: r.pair };
    row[r.theme] = r.value;
    byPair.set(key, row);
  }
  const lines = ["| Pair | Light | Dark | Requirement |", "|---|---|---|---|"];
  for (const { pair, light, dark } of byPair.values()) {
    const label = `\`${pair.fg}\` on \`${pair.bg}\`${pair.note ? ` (${pair.note})` : ""}`;
    lines.push(`| ${label} | ${light!.toFixed(2)} | ${dark!.toFixed(2)} | ${pair.min} |`);
  }
  return lines.join("\n");
}

// ---- Kotlin ----

function kotlinPalette(theme: Theme): string {
  const group = (name: string, cls: string, indent: string) => {
    const entries = Object.entries(tokens[name] as Record<string, Themed>);
    const body = entries
      .map(([k, v]) => `${indent}    ${camel(k)} = ${kotlinColor(v[theme])},`)
      .join("\n");
    return `${indent}${name === "color" ? "color" : name} = ${cls}(\n${body}\n${indent}),`;
  };
  return [
    `    val ${theme}: TetherPalette = TetherPalette(`,
    group("color", "TetherColors", "        "),
    group("syntax", "TetherSyntaxColors", "        "),
    group("chart", "TetherChartColors", "        "),
    "    )",
  ].join("\n");
}

function kotlinColorClass(name: string, cls: string): string {
  const fields = Object.keys(tokens[name]).map((k) => `    val ${camel(k)}: Color,`);
  return `data class ${cls}(\n${fields.join("\n")}\n)`;
}

function kotlinDpObject(name: string, group: Record<string, number>, prefix = ""): string {
  const lines = Object.entries(group).map(([k, v]) => {
    const id = prefix ? prefix + camel(k) : camel(k);
    return `    val ${id}: Dp = ${num(v)}.dp`;
  });
  return `object ${name} {\n${lines.join("\n")}\n}`;
}

function kotlin(): string {
  const type = Object.entries(tokens.type as Record<string, any>).map(
    ([k, t]) =>
      `    val ${camel(k)} = TetherTextToken(` +
      `TetherFont.${pascal(t.font)}, ${num(t.size)}.sp, ${num(t.line)}.sp, ${t.weight}, ` +
      `${num(t.tracking)}.sp)`,
  );
  const motion = Object.entries(tokens.motion as Record<string, number>).map(
    ([k, v]) => `    const val ${camel(k)}Ms: Int = ${v}`,
  );
  const ease = Object.entries(tokens.ease as Record<string, number[]>).map(
    ([k, v]) => `    val ${camel(k)} = CubicBezierEasing(${v.map((n) => `${num(n)}f`).join(", ")})`,
  );
  const elevation = Object.entries(tokens.elevation as Record<string, any>).map(
    ([k, v]) => `    val level${k}: Dp = ${v.dp}.dp`,
  );
  const breakpoints = Object.entries(tokens.breakpoint as Record<string, number>).map(
    ([k, v]) => `    const val ${camel(k)}Dp: Int = ${v}`,
  );
  const ratios = Object.entries(tokens.ratio as Record<string, number>).map(
    ([k, v]) => `    const val ${camel(k)}: Float = ${num(v)}f`,
  );
  const layers = Object.entries(tokens.layer as Record<string, number>).map(
    ([k, v]) => `    const val ${camel(k)}: Float = ${num(v)}f`,
  );
  const sizes = { ...tokens.size, ...tokens.layout } as Record<string, number>;

  return `// Generated by tokens/generate.ts from tokens/tokens.json. Do not edit by hand.
// Run \`just tokens\` after changing the JSON.
package app.tether.design.tokens

import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

${kotlinColorClass("color", "TetherColors")}

${kotlinColorClass("syntax", "TetherSyntaxColors")}

${kotlinColorClass("chart", "TetherChartColors")}

/** All colors for one theme. */
data class TetherPalette(
    val color: TetherColors,
    val syntax: TetherSyntaxColors,
    val chart: TetherChartColors,
)

enum class TetherFont { Sans, Mono }

/** One text style from the type scale. The theme maps [font] to a bundled font family. */
data class TetherTextToken(
    val font: TetherFont,
    val size: TextUnit,
    val lineHeight: TextUnit,
    val weight: Int,
    val letterSpacing: TextUnit,
)

object TetherTokens {
${themes.map(kotlinPalette).join("\n\n")}
}

object TetherType {
${type.join("\n")}
}

${kotlinDpObject("TetherSpace", tokens.space, "space")}

${kotlinDpObject("TetherRadius", tokens.radius)}

${kotlinDpObject("TetherSize", sizes)}

${kotlinDpObject("TetherBorder", tokens.border)}

${kotlinDpObject("TetherFocus", tokens.focus)}

object TetherElevation {
${elevation.join("\n")}
}

object TetherMotion {
${motion.join("\n")}
}

object TetherEase {
${ease.join("\n")}
}

object TetherBreakpoint {
${breakpoints.join("\n")}
}

object TetherRatio {
${ratios.join("\n")}
}

object TetherLayer {
${layers.join("\n")}
}
`;
}

// ---- CSS ----

function cssVars(theme: Theme, indent: string): string[] {
  const out: string[] = [];
  for (const group of ["color", "syntax", "chart"]) {
    for (const [k, v] of Object.entries(tokens[group] as Record<string, Themed>)) {
      out.push(`${indent}--tt-${group === "color" ? "color" : group}-${k}: ${v[theme]};`);
    }
  }
  for (const [k, v] of Object.entries(tokens.elevation as Record<string, any>)) {
    out.push(`${indent}--tt-elevation-${k}: ${v[theme]};`);
  }
  return out;
}

function css(): string {
  const shared: string[] = [
    `  --tt-font-sans: "${tokens.font.sans}", system-ui, sans-serif;`,
    `  --tt-font-mono: "${tokens.font.mono}", ui-monospace, monospace;`,
  ];
  for (const [k, v] of Object.entries(tokens.space as Record<string, number>)) {
    shared.push(`  --tt-space-${k}: ${v}px;`);
  }
  for (const [k, v] of Object.entries(tokens.radius as Record<string, number>)) {
    shared.push(`  --tt-radius-${k}: ${v}px;`);
  }
  for (const [k, v] of Object.entries(tokens.motion as Record<string, number>)) {
    shared.push(`  --tt-motion-${k}: ${v}ms;`);
  }
  for (const [k, v] of Object.entries(tokens.layer as Record<string, number>)) {
    shared.push(`  --tt-layer-${k}: ${v};`);
  }
  for (const [k, v] of Object.entries(tokens.ease as Record<string, number[]>)) {
    shared.push(`  --tt-ease-${k}: cubic-bezier(${v.join(", ")});`);
  }
  return `/* Generated by tokens/generate.ts from tokens/tokens.json. Do not edit by hand. */
:root {
  color-scheme: light dark;
${cssVars("light", "  ").join("\n")}
${shared.join("\n")}
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
${cssVars("dark", "    ").join("\n")}
  }
}

:root[data-theme="dark"] {
${cssVars("dark", "  ").join("\n")}
}

@media (prefers-reduced-motion: reduce) {
  :root {
    --tt-motion-fast: 0ms;
    --tt-motion-base: 0ms;
    --tt-motion-slow: 0ms;
    --tt-motion-sheet: 0ms;
  }
}
`;
}

// ---- Android color resources ----

function androidColors(theme: Theme): string {
  const lines: string[] = [];
  for (const [k, v] of Object.entries(tokens.palette as Record<string, string>)) {
    lines.push(`    <color name="tt_palette_${k.replace(/-/g, "_")}">${v}</color>`);
  }
  for (const group of ["color", "syntax", "chart"]) {
    for (const [k, v] of Object.entries(tokens[group] as Record<string, Themed>)) {
      const name = group === "color" ? k : `${group}-${k}`;
      lines.push(`    <color name="tt_${name.replace(/-/g, "_")}">${v[theme]}</color>`);
    }
  }
  return `<?xml version="1.0" encoding="utf-8"?>
<!-- Generated by tokens/generate.ts from tokens/tokens.json. Do not edit by hand. -->
<resources>
${lines.join("\n")}
</resources>
`;
}

// ---- main ----

const resDir = resolve(root, "android/designsystem/src/main/res");
const outputs: { path: string; content: string }[] = [
  { path: ktPath, content: kotlin() },
  { path: resolve(resDir, "values/tokens_colors.xml"), content: androidColors("light") },
  { path: resolve(resDir, "values-night/tokens_colors.xml"), content: androidColors("dark") },
];

const args = process.argv.slice(2);
const bad = failures();

if (args.includes("--contrast")) {
  console.log(contrastTable());
  process.exit(bad.length ? 1 : 0);
}

if (bad.length) {
  console.error("Contrast failures:\n" + bad.map((b) => `  ${b}`).join("\n"));
  process.exit(1);
}

const cssIndex = args.indexOf("--css");
if (cssIndex !== -1) {
  const target = resolve(process.cwd(), args[cssIndex + 1]);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, css());
  console.log(`Wrote ${target}`);
  process.exit(0);
}

if (args.includes("--check")) {
  const stale = outputs.filter(({ path, content }) => {
    try {
      return readFileSync(path, "utf8") !== content;
    } catch {
      return true; // a missing file counts as stale
    }
  });
  if (stale.length) {
    console.error(
      "Stale generated files. Run `just tokens` and commit the result:\n" +
        stale.map((o) => `  ${o.path.replace(root + "/", "")}`).join("\n"),
    );
    process.exit(1);
  }
  console.log("Generated token files are current and every contrast pair passes.");
  process.exit(0);
}

for (const { path, content } of outputs) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
  console.log(`Wrote ${path.replace(root + "/", "")}`);
}
