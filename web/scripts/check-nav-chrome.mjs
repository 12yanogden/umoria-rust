/**
 * Phase 2.1 — build-output assertions for global nav / layout chrome.
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

function extractNav(html) {
  const match = html.match(/<nav\b[^>]*data-site-nav[^>]*>([\s\S]*?)<\/nav>/i);
  assert.ok(match, "expected <nav data-site-nav> region");
  return match[0];
}

function navLinks(navHtml) {
  const links = [];
  const re = /<a\b([^>]*)>([\s\S]*?)<\/a>/gi;
  let m;
  while ((m = re.exec(navHtml)) !== null) {
    const attrs = m[1];
    const hrefMatch = attrs.match(/\bhref\s*=\s*"([^"]*)"/i);
    const label = m[2].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim();
    links.push({
      href: hrefMatch?.[1] ?? "",
      label,
      attrs,
      ariaCurrent: /\baria-current\s*=\s*"page"/i.test(attrs)
    });
  }
  return links;
}

function findLink(links, label) {
  const found = links.find((l) => l.label.replace(/^\[|\]$/g, "").toLowerCase() === label.toLowerCase());
  assert.ok(found, `expected nav link labeled ${label}, got: ${links.map((l) => l.label).join(", ")}`);
  return found;
}

function assertBaseSafeHref(href, expectedSuffix) {
  assert.ok(
    href === `${BASE}${expectedSuffix}` || href === `${BASE}${expectedSuffix}`.replace(/\/$/, ""),
    `href ${href} must be base-prefixed (${BASE}${expectedSuffix}); bare root paths are not allowed`
  );
  assert.ok(!/^\/(downloads|docs)\/?$/.test(href), `bare path ${href} would break under project Pages base`);
  assert.ok(href.startsWith(`${BASE}/`) || href === BASE || href === `${BASE}/`, `href ${href} missing base ${BASE}`);
}

test("dist/ exists after build", () => {
  assert.ok(existsSync(distRoot), "dist/ missing — run bun run build first");
  const entries = readdirSync(distRoot);
  assert.ok(entries.length > 0, "dist/ is empty");
});

test("1. Primary nav present on all chrome routes", () => {
  const pages = [
    requirePage("home", ["index.html", join(BASE.slice(1), "index.html")]),
    requirePage("downloads", [
      "downloads/index.html",
      "downloads.html",
      join(BASE.slice(1), "downloads/index.html"),
      join(BASE.slice(1), "downloads.html")
    ]),
    requirePage("docs", [
      "docs/index.html",
      "docs.html",
      join(BASE.slice(1), "docs/index.html"),
      join(BASE.slice(1), "docs.html")
    ])
  ];

  for (const page of pages) {
    const nav = extractNav(page.html);
    const links = navLinks(nav);
    findLink(links, "Home");
    findLink(links, "Downloads");
    findLink(links, "Docs");
  }
});

test("2. base-safe hrefs on primary nav", () => {
  const home = requirePage("home", ["index.html", join(BASE.slice(1), "index.html")]);
  const nav = extractNav(home.html);
  const links = navLinks(nav);

  assertBaseSafeHref(findLink(links, "Home").href, "/");
  assertBaseSafeHref(findLink(links, "Downloads").href, "/downloads/");
  assertBaseSafeHref(findLink(links, "Docs").href, "/docs/");
});

test("3. Active-route affordance (aria-current=page)", () => {
  const cases = [
    {
      label: "home",
      files: ["index.html", join(BASE.slice(1), "index.html")],
      active: "Home",
      inactive: ["Downloads", "Docs"]
    },
    {
      label: "downloads",
      files: [
        "downloads/index.html",
        "downloads.html",
        join(BASE.slice(1), "downloads/index.html"),
        join(BASE.slice(1), "downloads.html")
      ],
      active: "Downloads",
      inactive: ["Home", "Docs"]
    },
    {
      label: "docs",
      files: [
        "docs/index.html",
        "docs.html",
        join(BASE.slice(1), "docs/index.html"),
        join(BASE.slice(1), "docs.html")
      ],
      active: "Docs",
      inactive: ["Home", "Downloads"]
    }
  ];

  for (const c of cases) {
    const page = requirePage(c.label, c.files);
    const links = navLinks(extractNav(page.html));
    const active = findLink(links, c.active);
    assert.equal(active.ariaCurrent, true, `${c.label}: ${c.active} should have aria-current="page"`);
    for (const name of c.inactive) {
      assert.equal(findLink(links, name).ariaCurrent, false, `${c.label}: ${name} should not be active`);
    }
  }
});

test("4. Docs nav targets docs index (not volume/phile paths)", () => {
  const home = requirePage("home", ["index.html", join(BASE.slice(1), "index.html")]);
  const docsHref = findLink(navLinks(extractNav(home.html)), "Docs").href;
  assert.ok(
    docsHref === `${BASE}/docs/` || docsHref === `${BASE}/docs`,
    `Docs href must be docs index only, got ${docsHref}`
  );
  assert.ok(!/\/volume\//.test(docsHref), "Docs nav must not point at volume/phile paths");

  const docs = requirePage("docs", [
    "docs/index.html",
    "docs.html",
    join(BASE.slice(1), "docs/index.html"),
    join(BASE.slice(1), "docs.html")
  ]);
  // phase_4.3 owns the catalog-driven DocsIndex on /docs/; nav must still land there.
  assert.ok(
    /data-docs-index|Documentation|Getting Started/i.test(docs.html),
    "docs index page should render the docs landing/index surface"
  );
});

test("5. Layout ownership boundary (nav present; footer slot owned by phase_2.4)", () => {
  const pages = [
    requirePage("home", ["index.html", join(BASE.slice(1), "index.html")]),
    requirePage("downloads", [
      "downloads/index.html",
      "downloads.html",
      join(BASE.slice(1), "downloads/index.html"),
      join(BASE.slice(1), "downloads.html")
    ]),
    requirePage("docs", [
      "docs/index.html",
      "docs.html",
      join(BASE.slice(1), "docs/index.html"),
      join(BASE.slice(1), "docs.html")
    ])
  ];

  for (const page of pages) {
    extractNav(page.html);
    const footerMatch = page.html.match(/<footer\b[^>]*data-site-footer[^>]*>([\s\S]*?)<\/footer>/i);
    assert.ok(footerMatch, "expected <footer data-site-footer> (filled by phase_2.4)");
    assert.ok(
      /data-owned-by\s*=\s*"phase_2\.4"/i.test(footerMatch[0]),
      "footer remains owned by phase_2.4"
    );
  }
});
