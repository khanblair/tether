// Renders the PNG brand assets from images/logo.svg.
//
// Usage: node tools/gen-icons.ts
// Output: images/logo.png and images/export/*.png. Android launcher icons are vector
// drawables under android/app/src/main/res and are edited by hand when the logo changes.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Resvg } from "@resvg/resvg-js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tokens = JSON.parse(readFileSync(resolve(root, "tokens/tokens.json"), "utf8"));
const logoSvg = readFileSync(resolve(root, "images/logo.svg"), "utf8");
const outDir = resolve(root, "images/export");
const fontsDir = resolve(root, "android/designsystem/src/main/res/font");

const mint100: string = tokens.palette["mint-100"];
const plum700: string = tokens.palette["plum-700"];
const bgLight: string = tokens.color.bg.light;
const ink: string = tokens.color.text.light;

// The mark inside logo.svg is 108 x 193 units, centered at (144, 144) in a 288 box.
const MARK_HEIGHT = 193;
const MARK_CENTER = 144;

const defs = logoSvg.match(/<defs>[\s\S]*?<\/defs>/)![0];
const markBody = logoSvg.match(/<g transform="[^"]*">[\s\S]*<\/g>/)![0];

function mark(cx: number, cy: number, height: number): string {
  const s = height / MARK_HEIGHT;
  const tx = cx - MARK_CENTER * s;
  const ty = cy - MARK_CENTER * s;
  return `<g transform="translate(${tx} ${ty}) scale(${s})">${markBody}</g>`;
}

function svg(w: number, h: number, body: string): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">${defs}${body}</svg>`;
}

function render(markup: string, width: number): Buffer {
  const resvg = new Resvg(markup, {
    fitTo: { mode: "width", value: width },
    font: {
      loadSystemFonts: false,
      fontFiles: [
        resolve(fontsDir, "ibm_plex_sans_regular.ttf"),
        resolve(fontsDir, "ibm_plex_sans_semibold.ttf"),
      ],
      defaultFontFamily: "IBM Plex Sans",
    },
  });
  return resvg.render().asPng();
}

function write(path: string, png: Buffer): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, png);
  console.log(`wrote ${path.replace(root + "/", "")} (${png.length} bytes)`);
}

// Transparent logo for README and docs.
write(resolve(root, "images/logo.png"), render(svg(288, 288, mark(144, 144, 193)), 1024));

// Square icons: mint background, mark sized like the Android adaptive icon safe zone.
function square(size: number, markRatio: number): Buffer {
  const body =
    `<rect width="${size}" height="${size}" fill="${mint100}"/>` +
    mark(size / 2, size / 2, size * markRatio);
  return render(svg(size, size, body), size);
}

const adaptiveRatio = (0.31 * MARK_HEIGHT) / 108;
write(resolve(outDir, "play-store-icon-512.png"), square(512, adaptiveRatio));
write(resolve(outDir, "icon-512.png"), square(512, adaptiveRatio));
write(resolve(outDir, "icon-192.png"), square(192, adaptiveRatio));
write(resolve(outDir, "apple-touch-icon-180.png"), square(180, 0.62));

// Favicon: mark only, transparent.
write(resolve(outDir, "favicon-32.png"), render(svg(288, 288, mark(144, 144, 232)), 32));

// Social card, 2:1 for the GitHub social preview and about 1.9:1 for Open Graph.
function card(w: number, h: number): Buffer {
  const markH = h * 0.62;
  const body =
    `<defs><linearGradient id="cardbg" x1="0" y1="0" x2="1" y2="1">` +
    `<stop offset="0" stop-color="${bgLight}"/><stop offset="1" stop-color="${mint100}"/>` +
    `</linearGradient></defs>` +
    `<rect width="${w}" height="${h}" fill="url(#cardbg)"/>` +
    mark(w * 0.2, h / 2, markH) +
    `<text x="${w * 0.34}" y="${h * 0.48}" font-family="IBM Plex Sans" font-weight="600" ` +
    `font-size="${h * 0.19}" fill="${plum700}">Tether</text>` +
    `<text x="${w * 0.34}" y="${h * 0.62}" font-family="IBM Plex Sans" font-weight="400" ` +
    `font-size="${h * 0.05}" fill="${ink}">A coding agent harness that runs on your phone.</text>` +
    `<text x="${w * 0.34}" y="${h * 0.71}" font-family="IBM Plex Sans" font-weight="400" ` +
    `font-size="${h * 0.04}" fill="${ink}">No PC. No server. Your own keys.</text>`;
  return render(svg(w, h, body), w);
}

write(resolve(outDir, "social-preview-1280x640.png"), card(1280, 640));
write(resolve(outDir, "og-image-1200x630.png"), card(1200, 630));
