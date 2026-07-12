/**
 * Phase 4.3 — docs index lists all catalogued articles, grouped, base-safe.
 * Run: bun run test:docs-index  (from web/; expects dist/ after bun run build)
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import { docsCatalog } from "./docs-catalog.ts";
import { primaryNav } from "../config/site.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "../..");
const distRoot = join(webRoot, "dist");
const BASE = "/umoria-rust";
const BASE_SEGMENT = "umoria-rust";

/** Catalog overview slug maps to /docs/index/; landing is /docs/ — not a self-link. */
const DOCS_OVERVIEW_SLUG = "index";

function findDistHtml(relCandidates: string[]): { path: string; html: string } | null {
  for (const rel of relCandidates) {
    const full = join(distRoot, rel);
    if (existsSync(full)) {
      return { path: full, html: readFileSync(full, "utf8") };
    }
  }
  return null;
}

function requireDocsIndex(): { path: string; html: string } {
  const page = findDistHtml([
    join("docs", "index.html"),
    join(BASE_SEGMENT, "docs", "index.html")
  ]);
  assert.ok(page, "docs index missing under dist/ — run bun run build first");
  return page;
}

function expectedHrefForSlug(slug: string): string {
  return `${BASE}/docs/${slug}/`;
}

function extractHrefs(html: string): string[] {
  const hrefs: string[] = [];
  const re = /\bhref\s*=\s*"([^"]*)"/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(html)) !== null) {
    hrefs.push(m[1]);
  }
  return hrefs;
}

function listingRegion(html: string): string {
  const match = html.match(
    /<(?:section|article|nav|div)\b[^>]*(?:data-docs-index|class="[^"]*docs-index[^"]*")[^>]*>[\s\S]*$/i
  );
  if (match) return match[0];
  // Fallback: whole body after layout chrome — still useful for fail-first.
  return html;
}

/** Decode common HTML entities so catalog titles match built markup. */
function htmlIncludesText(html: string, text: string): boolean {
  if (html.includes(text)) return true;
  const escaped = text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
  return html.includes(escaped);
}

describe("docs index (phase_4.3)", () => {
  it("1. docs index lists all catalogued articles (title + slug href)", () => {
    assert.ok(docsCatalog.articles.length >= 1, "catalog must have ≥1 article");
    const { html } = requireDocsIndex();
    const region = listingRegion(html);
    const hrefs = extractHrefs(region);

    let listed = 0;
    for (const entry of docsCatalog.articles) {
      const expected = expectedHrefForSlug(entry.slug);
      const hasHref = hrefs.some(
        (h) => h === expected || h === expected.replace(/\/$/, "")
      );
      assert.ok(
        htmlIncludesText(region, entry.title),
        `docs index missing catalog title "${entry.title}" (${entry.slug})`
      );
      // Overview slug links to /docs/index/ stub (not the /docs/ landing self-link).
      if (entry.slug === DOCS_OVERVIEW_SLUG) {
        assert.ok(
          hasHref || /aria-current\s*=\s*"page"/i.test(region),
          `overview "${entry.slug}" must link to ${expected} or be marked current`
        );
      } else {
        assert.ok(hasHref, `docs index missing base-safe href ${expected} for ${entry.slug}`);
      }
      listed += 1;
    }
    assert.equal(listed, docsCatalog.articles.length);
  });

  it("2. every listed article href resolves to a built stub (base-safe)", () => {
    assert.ok(existsSync(distRoot), "dist/ missing — run bun run build first");
    const { html } = requireDocsIndex();
    const region = listingRegion(html);
    const hrefs = extractHrefs(region);

    for (const entry of docsCatalog.articles) {
      const expected = expectedHrefForSlug(entry.slug);
      const listedHref = hrefs.find(
        (h) => h === expected || h === expected.replace(/\/$/, "")
      );
      if (entry.slug === DOCS_OVERVIEW_SLUG && !listedHref) {
        // Self/current marking allowed; stub still must exist at /docs/index/.
      } else {
        assert.ok(listedHref, `missing href for ${entry.slug}`);
        assert.ok(
          listedHref.startsWith(`${BASE}/`),
          `href ${listedHref} must be base-prefixed with ${BASE}`
        );
        assert.ok(
          !/^\/docs\//.test(listedHref),
          `bare /docs/ path ${listedHref} is not base-safe`
        );
      }

      const stubPage = findDistHtml([
        join("docs", entry.slug, "index.html"),
        join(BASE_SEGMENT, "docs", entry.slug, "index.html")
      ]);
      assert.ok(stubPage, `missing dist stub for /docs/${entry.slug}/`);
    }
  });

  it("3. grouping matches catalog sections order and article section", () => {
    assert.ok(docsCatalog.sections.length >= 1, "expected non-empty sections");
    const { html } = requireDocsIndex();
    const region = listingRegion(html);

    const sections = [...docsCatalog.sections].sort((a, b) => a.order - b.order);
    let lastLabelPos = -1;
    for (const section of sections) {
      const pos = region.indexOf(section.label);
      assert.ok(pos !== -1, `missing section label "${section.label}"`);
      assert.ok(
        pos > lastLabelPos,
        `section "${section.label}" should appear after previous section in order`
      );
      lastLabelPos = pos;
    }

    // Pick an article from two different sections; each title must appear after its section label.
    const bySection = new Map<string, typeof docsCatalog.articles>();
    for (const a of docsCatalog.articles) {
      const list = bySection.get(a.section) ?? [];
      list.push(a);
      bySection.set(a.section, list);
    }
    const sectionIds = sections.map((s) => s.id).filter((id) => (bySection.get(id)?.length ?? 0) > 0);
    assert.ok(sectionIds.length >= 2, "need ≥2 populated sections for grouping check");
    const a = bySection.get(sectionIds[0])![0];
    const b = bySection.get(sectionIds[1])![0];
    const labelA = sections.find((s) => s.id === a.section)!.label;
    const labelB = sections.find((s) => s.id === b.section)!.label;
    const posLabelA = region.indexOf(labelA);
    const posLabelB = region.indexOf(labelB);
    const titleAEsc = a.title.replaceAll("&", "&amp;");
    const titleBEsc = b.title.replaceAll("&", "&amp;");
    const posTitleA = Math.max(region.indexOf(a.title), region.indexOf(titleAEsc));
    const posTitleB = Math.max(region.indexOf(b.title), region.indexOf(titleBEsc));
    assert.ok(posTitleA > posLabelA, `"${a.title}" should appear under "${labelA}"`);
    assert.ok(posTitleB > posLabelB, `"${b.title}" should appear under "${labelB}"`);
    // Cross-check: A should not sit between B's label and B's title when A’s section is earlier.
    if (posLabelA < posLabelB) {
      assert.ok(
        posTitleA < posLabelB,
        `"${a.title}" should stay under "${labelA}", before "${labelB}"`
      );
    }
  });

  it("4. reachable from phase_2 Docs nav", () => {
    const docsNav = primaryNav.find((n) => n.label === "Docs");
    assert.ok(docsNav, "primaryNav must include Docs");
    assert.equal(docsNav.path, "/docs/");

    const home = findDistHtml(["index.html", join(BASE_SEGMENT, "index.html")]);
    assert.ok(home, "home page missing under dist/");
    const navMatch = home.html.match(/<nav\b[^>]*data-site-nav[^>]*>([\s\S]*?)<\/nav>/i);
    assert.ok(navMatch, "expected site nav on home");
    const docsLink = [...navMatch[0].matchAll(/<a\b([^>]*)>([\s\S]*?)<\/a>/gi)].find((m) => {
      const label = m[2].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim().replace(/^\[|\]$/g, "");
      return label.toLowerCase() === "docs";
    });
    assert.ok(docsLink, "Docs link missing from primary nav");
    const href = docsLink[1].match(/\bhref\s*=\s*"([^"]*)"/i)?.[1] ?? "";
    assert.ok(
      href === `${BASE}/docs/` || href === `${BASE}/docs`,
      `Docs nav href must target docs index (${BASE}/docs/), got ${href}`
    );

    const { html } = requireDocsIndex();
    assert.ok(
      !/Docs stub/i.test(html) || docsCatalog.articles.every((a) => html.includes(a.title)),
      "docs index must be the listing page, not a blank stub-only page"
    );
    for (const entry of docsCatalog.articles.slice(0, 3)) {
      assert.ok(
        htmlIncludesText(html, entry.title),
        `listing page should include "${entry.title}"`
      );
    }
  });

  it("5. catalog not redefined — index source imports docs-catalog", () => {
    const indexPage = readFileSync(join(webRoot, "src/pages/docs/index.astro"), "utf8");
    const componentCandidates = [
      join(webRoot, "src/components/DocsIndex.astro"),
      join(webRoot, "src/components/docs/DocsIndex.astro")
    ];
    const sources = [indexPage];
    for (const p of componentCandidates) {
      if (existsSync(p)) sources.push(readFileSync(p, "utf8"));
    }
    const joined = sources.join("\n");
    assert.match(
      joined,
      /docs-catalog/,
      "docs index implementation must import/consume docs-catalog.ts"
    );
    // No second hard-coded inventory of many slug string literals in page/component.
    const slugLiterals = joined.match(/["'][a-z0-9-]+\/[a-z0-9-]+["']/g) ?? [];
    assert.ok(
      slugLiterals.length < 5,
      `suspected hard-coded article inventory (${slugLiterals.length} section/topic literals)`
    );
  });

  it("6. no full article bodies on the index", () => {
    const { html } = requireDocsIndex();
    const region = listingRegion(html);
    assert.doesNotMatch(
      region,
      /<!-- docs-stub: outline only/,
      "must not dump stub markdown bodies onto the index"
    );
    assert.doesNotMatch(
      region,
      /## Outline/,
      "must not embed stub outline headings for every article"
    );
    // Index should stay listing-scale, not concatenate all stub bodies.
    const MAX_LISTING_CHARS = 80_000;
    assert.ok(
      region.length < MAX_LISTING_CHARS,
      `docs index listing unexpectedly large (${region.length})`
    );
  });
});
