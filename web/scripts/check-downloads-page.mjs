/**
 * Phase 2.3 — build-output assertions for Downloads page IA.
 * Run after `bun run build` (expects `dist/` under the Astro base path).
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "..");
const distRoot = join(webRoot, "dist");
const BASE = "/umoria-rust";
const RELEASES_HREF_RE = /https:\/\/github\.com\/12yanogden\/umoria-rust\/releases\/?/;

const DOWNLOADS_CANDIDATES = [
  "downloads/index.html",
  "downloads.html",
  join(BASE.slice(1), "downloads/index.html"),
  join(BASE.slice(1), "downloads.html")
];

function findHtml(relCandidates) {
  for (const rel of relCandidates) {
    const full = join(distRoot, rel);
    if (existsSync(full)) {
      return { path: full, html: readFileSync(full, "utf8") };
    }
  }
  return null;
}

function requireDownloadsPage() {
  const page = findHtml(DOWNLOADS_CANDIDATES);
  assert.ok(page, `Downloads route: expected one of ${DOWNLOADS_CANDIDATES.join(", ")} under dist/`);
  return page;
}

/** Strip script/style noise; keep main document text for copy assertions. */
function pageBody(html) {
  return html
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, " ")
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, " ")
    .replace(/<!--[\s\S]*?-->/g, " ");
}

function headingText(html, level) {
  const re = new RegExp(`<h${level}\\b[^>]*>([\\s\\S]*?)<\\/h${level}>`, "gi");
  const texts = [];
  let m;
  while ((m = re.exec(html)) !== null) {
    texts.push(m[1].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim());
  }
  return texts;
}

function sectionAfterHeading(html, headingPattern) {
  const re = /<h([2-6])\b[^>]*>([\s\S]*?)<\/h\1>/gi;
  let m;
  while ((m = re.exec(html)) !== null) {
    const text = m[2].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim();
    if (headingPattern.test(text)) {
      const start = m.index + m[0].length;
      const rest = html.slice(start);
      const next = rest.search(/<h[1-6]\b/i);
      return next === -1 ? rest : rest.slice(0, next);
    }
  }
  return null;
}

test("1. Downloads route exists in dist/", () => {
  assert.ok(existsSync(distRoot), "dist/ missing — run bun run build first");
  requireDownloadsPage();
});

test("2. Page title / H1 contains Download(s)", () => {
  const { html } = requireDownloadsPage();
  const h1s = headingText(html, 1);
  assert.ok(h1s.length > 0, "expected an h1 on Downloads page");
  assert.ok(
    h1s.some((t) => /downloads?/i.test(t)),
    `h1 should contain Download/Downloads, got: ${h1s.join(" | ")}`
  );
});

test("3. Build-from-source section with README-aligned stub facts", () => {
  const { html } = requireDownloadsPage();
  const body = pageBody(html);
  const section = sectionAfterHeading(body, /build from source|from source|building/i);
  assert.ok(section, "expected an h2+ section for building from source");

  assert.ok(/cargo build --release/.test(section), "source section must mention cargo build --release");
  assert.ok(/target\/release\/umoria/.test(section), "source section must mention binary path target/release/umoria");
  assert.ok(/ncurses/i.test(section) || /pkg-config/i.test(section), "source section must mention ncurses and/or pkg-config");
  assert.ok(
    /brew install ncurses pkg-config/.test(section),
    "source section must include macOS brew dep line"
  );
  assert.ok(
    /apt-get install libncurses-dev pkg-config/.test(section),
    "source section must include Linux apt dep line"
  );
});

test("4. Release artifacts section with GitHub Releases link", () => {
  const { html } = requireDownloadsPage();
  const body = pageBody(html);
  const section = sectionAfterHeading(body, /release|binar(y|ies)|artifacts?/i);
  assert.ok(section, "expected an h2+ section for release artifacts / binaries");

  const hrefMatch = section.match(/\bhref\s*=\s*"([^"]+)"/i);
  assert.ok(hrefMatch, "release section must contain a link");
  assert.ok(
    RELEASES_HREF_RE.test(hrefMatch[1]),
    `release link href must be GitHub Releases URL, got ${hrefMatch[1]}`
  );
});

test("5. Platform emphasis names macOS and Linux", () => {
  const { html } = requireDownloadsPage();
  const body = pageBody(html);
  assert.ok(/\bmacOS\b/.test(body), "Downloads page must name macOS");
  assert.ok(/\bLinux\b/.test(body), "Downloads page must name Linux");
});

test("6. Windows is not a first-class supported platform", () => {
  const { html } = requireDownloadsPage();
  const body = pageBody(html);

  const primaryWindows =
    /supported[:\s]+[^.]*\bwindows\b/i.test(body) ||
    /primary[:\s]+[^.]*\bwindows\b/i.test(body) ||
    /\bwindows\b[^.]{0,40}\b(install|download|build)\b/i.test(body) ||
    /<(?:h[2-6]|li|dt|strong)[^>]*>\s*windows\s*</i.test(body);

  const cautionOk = /windows.{0,80}(not a proven|not proven|not (a )?primary)/i.test(body);

  assert.ok(
    !primaryWindows || cautionOk,
    "Windows must not be presented as a primary supported download/build target"
  );

  if (/\bwindows\b/i.test(body) && !cautionOk) {
    assert.fail("Windows mentioned without a cautionary 'not proven' note — inventing a Windows path fails");
  }
});
