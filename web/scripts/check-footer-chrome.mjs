/**
 * Phase 2.4 — build-output assertions for site-wide footer slots.
 * Run after `bun run build` (expects `dist/` under the Astro base path).
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "..");
const distRoot = join(webRoot, "dist");
const BASE = "/umoria-rust";
const REPO_URL = "https://github.com/12yanogden/umoria-rust";
const REPO_URL_SLASH = `${REPO_URL}/`;

const PAGE_CANDIDATES = {
  home: ["index.html", join(BASE.slice(1), "index.html")],
  downloads: [
    "downloads/index.html",
    "downloads.html",
    join(BASE.slice(1), "downloads/index.html"),
    join(BASE.slice(1), "downloads.html")
  ],
  docs: [
    "docs/index.html",
    "docs.html",
    join(BASE.slice(1), "docs/index.html"),
    join(BASE.slice(1), "docs.html")
  ]
};

function findHtml(relCandidates) {
  for (const rel of relCandidates) {
    const full = join(distRoot, rel);
    if (existsSync(full)) {
      return { path: full, html: readFileSync(full, "utf8") };
    }
  }
  return null;
}

function requirePage(label, relCandidates) {
  const page = findHtml(relCandidates);
  assert.ok(page, `${label}: expected one of ${relCandidates.join(", ")} under dist/`);
  return page;
}

function extractFooter(html) {
  const match = html.match(/<footer\b[^>]*data-site-footer[^>]*>([\s\S]*?)<\/footer>/i);
  assert.ok(match, "expected <footer data-site-footer> landmark");
  return { outer: match[0], inner: match[1] };
}

function footerLinks(footerHtml) {
  const links = [];
  const re = /<a\b([^>]*)>([\s\S]*?)<\/a>/gi;
  let m;
  while ((m = re.exec(footerHtml)) !== null) {
    const attrs = m[1];
    const hrefMatch = attrs.match(/\bhref\s*=\s*"([^"]*)"/i);
    const label = m[2].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim();
    links.push({ href: hrefMatch?.[1] ?? "", label, attrs });
  }
  return links;
}

function plainText(html) {
  return html
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

test("dist/ exists after build", () => {
  assert.ok(existsSync(distRoot), "dist/ missing — run bun run build first");
  assert.ok(readdirSync(distRoot).length > 0, "dist/ is empty");
});

test("1. Footer present on home", () => {
  const page = requirePage("home", PAGE_CANDIDATES.home);
  const footer = extractFooter(page.html);
  assert.ok(footer.inner.trim().length > 0, "home footer must contain site chrome content");
  assert.ok(/<main\b/i.test(page.html), "footer should sit with main content on home");
});

test("2. Footer present on downloads", () => {
  const page = requirePage("downloads", PAGE_CANDIDATES.downloads);
  const footer = extractFooter(page.html);
  assert.ok(footer.inner.trim().length > 0, "downloads footer must contain site chrome content");
});

test("3. Footer present on docs stub", () => {
  const page = requirePage("docs", PAGE_CANDIDATES.docs);
  const footer = extractFooter(page.html);
  assert.ok(footer.inner.trim().length > 0, "docs footer must contain site chrome content");
});

test("4. License / attribution hook", () => {
  const page = requirePage("home", PAGE_CANDIDATES.home);
  const { outer, inner } = extractFooter(page.html);
  const text = plainText(inner);

  assert.ok(
    /GPL-3\.0-or-later|GNU General Public License/i.test(text) ||
      /GPL-3\.0-or-later|GNU General Public License/i.test(outer),
    "footer must identify GPL-3.0-or-later (or GNU General Public License)"
  );

  const links = footerLinks(outer);
  const hrefs = links.map((l) => l.href);
  const blobOrPath = (name) =>
    hrefs.some(
      (h) =>
        h.includes(`github.com/12yanogden/umoria-rust`) && h.includes(name)
    ) || /LICENSE|AUTHORS/.test(inner);

  assert.ok(
    hrefs.some((h) => h.includes("LICENSE")) || /\bLICENSE\b/.test(inner),
    "footer must pointer/link to LICENSE"
  );
  assert.ok(
    hrefs.some((h) => h.includes("AUTHORS")) || /\bAUTHORS\b/.test(inner),
    "footer must pointer/link to AUTHORS"
  );
  assert.ok(
    blobOrPath("LICENSE") && blobOrPath("AUTHORS"),
    "LICENSE/AUTHORS pointers should use github.com/12yanogden/umoria-rust blob URLs (or equivalent explicit paths)"
  );
});

test("5. Repo link", () => {
  const page = requirePage("home", PAGE_CANDIDATES.home);
  const { outer } = extractFooter(page.html);
  const links = footerLinks(outer);
  const repo = links.find(
    (l) => l.href === REPO_URL || l.href === REPO_URL_SLASH
  );
  assert.ok(repo, `footer must include a[href] to ${REPO_URL}`);
});

test("6. About / history blurb hook", () => {
  const page = requirePage("home", PAGE_CANDIDATES.home);
  const { outer, inner } = extractFooter(page.html);
  const aboutSlot =
    outer.match(/data-footer-about[^>]*>([\s\S]*?)<\//i)?.[1] ?? inner;
  const text = plainText(aboutSlot);

  assert.ok(text.length > 20, "about/history stub must be non-empty");
  assert.ok(
    /moria|umoria|koeneke|wilson|rust\s+port/i.test(text),
    "about stub must mention Moria/Umoria lineage (Koeneke, Wilson, and/or Rust port)"
  );
  // Not a long multi-section history article
  assert.ok(text.length < 500, "about stub must stay short (not a history essay)");
  assert.ok(
    !/<h[1-6]\b/i.test(aboutSlot) && (text.match(/\.\s+/g) ?? []).length <= 3,
    "about stub must not be a multi-section history article"
  );
});

test("7. Single layout ownership", () => {
  const layoutPath = join(webRoot, "src/layouts/textmode/TextmodeLayout.astro");
  const footerComponent = join(webRoot, "src/components/SiteFooter.astro");
  assert.ok(existsSync(layoutPath), "shared TextmodeLayout must exist");
  assert.ok(existsSync(footerComponent), "shared SiteFooter.astro component must own footer slots");

  const layoutSrc = readFileSync(layoutPath, "utf8");
  assert.ok(
    /SiteFooter|data-site-footer/i.test(layoutSrc),
    "TextmodeLayout must wire the shared footer"
  );
  assert.ok(
    /SiteFooter/.test(layoutSrc),
    "TextmodeLayout must import/render SiteFooter (not inline a divergent fork)"
  );

  const pageSources = [
    join(webRoot, "src/pages/index.astro"),
    join(webRoot, "src/pages/downloads.astro"),
    join(webRoot, "src/pages/docs/index.astro")
  ];
  for (const srcPath of pageSources) {
    const src = readFileSync(srcPath, "utf8");
    assert.ok(
      !/data-site-footer|SiteFooter|<footer\b/i.test(src),
      `${srcPath} must not embed a page-local footer duplicate`
    );
  }

  const footerSrc = readFileSync(footerComponent, "utf8");
  assert.ok(/GPL-3\.0-or-later|GNU General Public License/i.test(footerSrc));
  assert.ok(/LICENSE/.test(footerSrc) && /AUTHORS/.test(footerSrc));
  assert.ok(footerSrc.includes(REPO_URL) || footerSrc.includes("12yanogden/umoria-rust"));
  assert.ok(/moria|umoria|koeneke|wilson|rust/i.test(footerSrc));
});

test("8. No ownership bleed", () => {
  const page = requirePage("home", PAGE_CANDIDATES.home);
  const { outer, inner } = extractFooter(page.html);
  const text = plainText(inner);

  assert.ok(
    !/(spoiler|beej\.us|mmspoilers|classes\.txt|items\.txt|spells\.txt|article catalog)/i.test(
      text
    ),
    "footer must not enumerate docs article titles/slugs from a catalog"
  );
  assert.ok(!/data-site-nav/i.test(outer), "footer must not replace primary nav");
  assert.ok(/data-site-nav/i.test(page.html), "primary nav must still be present");

  // Phase 2.4 owned footer chrome only. Pages deploy workflow is phase_5.1
  // (`.github/workflows/deploy.yml`); do not assert its absence here.
});

test("9. Build green is verified separately via bun run build", () => {
  // Placeholder so the suite documents the build-green requirement;
  // the executor runs `bun run build` from web/ after implementation.
  assert.ok(true);
});
