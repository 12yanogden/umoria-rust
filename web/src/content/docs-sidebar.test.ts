/**
 * Phase 4.4 — docs sidebar / volume nav wiring.
 * Catalog-driven checks + dist/ HTML assertions (run after `bun run build`).
 * Run: bun run test  (from web/)
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import { docsCatalog } from "./docs-catalog.ts";
import { DOCS_SECTION_TO_VOLUME } from "./docs-section-volume-map.ts";
import { getDocsNavTree } from "./docs-nav-tree.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "../..");
const distRoot = join(webRoot, "dist");
const BASE = "/umoria-rust";
const BASE_SEGMENT = "umoria-rust";

/** Representative stub for HTML assertions (getting-started section). */
const SAMPLE_SLUG = "getting-started/install";

function findDistHtml(relCandidates: string[]): { path: string; html: string } | null {
  for (const rel of relCandidates) {
    const full = join(distRoot, rel);
    if (existsSync(full)) {
      return { path: full, html: readFileSync(full, "utf8") };
    }
  }
  return null;
}

function requireStubPage(slug: string): { path: string; html: string } {
  assert.ok(existsSync(distRoot), "dist/ missing — run bun run build first");
  const page = findDistHtml([
    join("docs", slug, "index.html"),
    join(BASE_SEGMENT, "docs", slug, "index.html")
  ]);
  assert.ok(page, `missing dist page for /docs/${slug}/`);
  return page;
}

function extractSidebar(html: string): string {
  const match = html.match(/<nav\b[^>]*data-docs-sidebar[^>]*>([\s\S]*?)<\/nav>/i);
  assert.ok(match, "expected <nav data-docs-sidebar> on docs stub page");
  return match[0];
}

function extractPrimaryNav(html: string): string {
  const match = html.match(/<nav\b[^>]*data-site-nav[^>]*>([\s\S]*?)<\/nav>/i);
  assert.ok(match, "expected <nav data-site-nav> primary nav");
  return match[0];
}

function sidebarLinks(sidebarHtml: string): Array<{
  href: string;
  label: string;
  attrs: string;
  ariaCurrent: boolean;
}> {
  const links: Array<{ href: string; label: string; attrs: string; ariaCurrent: boolean }> = [];
  const re = /<a\b([^>]*)>([\s\S]*?)<\/a>/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(sidebarHtml)) !== null) {
    const attrs = m[1];
    const hrefMatch = attrs.match(/\bhref\s*=\s*"([^"]*)"/i);
    const label = m[2]
      .replace(/<[^>]+>/g, "")
      .replace(/\s+/g, " ")
      .trim()
      .replace(/^\[|\]$/g, "");
    links.push({
      href: hrefMatch?.[1] ?? "",
      label,
      attrs,
      ariaCurrent: /\baria-current\s*=\s*"page"/i.test(attrs)
    });
  }
  return links;
}

function stripBase(href: string): string {
  if (href.startsWith(`${BASE}/`)) return href.slice(BASE.length);
  if (href === BASE) return "/";
  return href;
}

function normalizePath(path: string): string {
  let p = path.startsWith("/") ? path : `/${path}`;
  if (p.length > 1 && p.endsWith("/")) p = p.slice(0, -1);
  return p || "/";
}

describe("docs sidebar / volume nav (phase_4.4)", () => {
  it("1. getDocsNavTree lists every catalog section and member stub (no extras)", () => {
    const tree = getDocsNavTree();
    assert.equal(tree.length, docsCatalog.sections.length, "one group per DocsNavSection");

    const sectionIds = new Set(docsCatalog.sections.map((s) => s.id));
    const catalogSlugs = new Set(docsCatalog.articles.map((a) => a.slug));
    const seenSlugs = new Set<string>();

    for (let i = 0; i < docsCatalog.sections.length; i++) {
      const section = docsCatalog.sections[i];
      const group = tree[i];
      assert.equal(group.sectionId, section.id);
      assert.equal(group.label, section.label);
      assert.equal(
        group.volume,
        DOCS_SECTION_TO_VOLUME[section.id],
        `group ${section.id} must consume DOCS_SECTION_TO_VOLUME`
      );

      const expected = docsCatalog.articles
        .filter((a) => a.section === section.id)
        .sort((a, b) => a.order - b.order);

      assert.equal(group.articles.length, expected.length, `section ${section.id} article count`);
      for (let j = 0; j < expected.length; j++) {
        assert.equal(group.articles[j].slug, expected[j].slug);
        assert.equal(group.articles[j].title, expected[j].title);
        assert.equal(group.articles[j].href, `/docs/${expected[j].slug}/`);
        seenSlugs.add(group.articles[j].slug);
      }
    }

    assert.equal(seenSlugs.size, catalogSlugs.size);
    for (const slug of seenSlugs) {
      assert.ok(catalogSlugs.has(slug), `nav invents slug absent from catalog: ${slug}`);
    }
    for (const id of tree.map((g) => g.sectionId)) {
      assert.ok(sectionIds.has(id), `nav invents section absent from catalog: ${id}`);
    }
  });

  it("2. built stub sidebar lists all section labels and article titles/slugs", () => {
    const { html } = requireStubPage(SAMPLE_SLUG);
    const sidebar = extractSidebar(html);

    for (const section of docsCatalog.sections) {
      assert.ok(
        sidebar.includes(section.label),
        `sidebar missing section label "${section.label}"`
      );
    }

    for (const article of docsCatalog.articles) {
      const hrefNeedle = `${BASE}/docs/${article.slug}/`;
      assert.ok(
        sidebar.includes(hrefNeedle) || sidebar.includes(article.title),
        `sidebar missing article ${article.slug} (title or base-safe href)`
      );
    }

    // No invented volume public URLs as docs links
    assert.ok(
      !/<a\b[^>]*href="[^"]*\/volume\/\d+\//i.test(sidebar),
      "sidebar must not use Entropic /volume/<N>/ as public docs URLs"
    );
  });

  it("3. every sidebar href is base-safe and resolves to an existing stub route", () => {
    const { html } = requireStubPage(SAMPLE_SLUG);
    const links = sidebarLinks(extractSidebar(html));
    assert.ok(links.length >= docsCatalog.articles.length, "expected ≥1 link per catalog article");

    const articleLinks = links.filter((l) => {
      const path = normalizePath(stripBase(l.href));
      return path.startsWith("/docs/") && path !== "/docs";
    });

    for (const link of articleLinks) {
      assert.ok(
        link.href.startsWith(`${BASE}/`),
        `href ${link.href} must be prefixed with ${BASE}`
      );
      assert.ok(
        !/^\/docs\//.test(link.href),
        `bare root-absolute path ${link.href} breaks under Pages base`
      );

      const sitePath = normalizePath(stripBase(link.href));
      // /docs/<slug> → dist docs/<slug>/index.html
      const match = sitePath.match(/^\/docs\/(.+)$/);
      assert.ok(match, `unexpected sidebar href path: ${link.href} → ${sitePath}`);
      const slug = match[1];
      const page = findDistHtml([
        join("docs", slug, "index.html"),
        join(BASE_SEGMENT, "docs", slug, "index.html")
      ]);
      assert.ok(page, `broken sidebar link: ${link.href} (no dist for /docs/${slug}/)`);
    }
  });

  it("4. active article has aria-current=page; siblings do not", () => {
    const { html } = requireStubPage(SAMPLE_SLUG);
    const links = sidebarLinks(extractSidebar(html));
    const active = links.filter((l) => l.ariaCurrent);
    assert.equal(active.length, 1, "exactly one aria-current=page in sidebar");
    assert.ok(
      normalizePath(stripBase(active[0].href)) === normalizePath(`/docs/${SAMPLE_SLUG}`),
      `active href should be sample slug, got ${active[0].href}`
    );

    const siblings = links.filter(
      (l) =>
        l !== active[0] &&
        stripBase(l.href).startsWith("/docs/") &&
        stripBase(l.href) !== "/docs" &&
        stripBase(l.href) !== "/docs/"
    );
    for (const sib of siblings) {
      assert.equal(sib.ariaCurrent, false, `sibling ${sib.href} must not be aria-current`);
    }
  });

  it("5. global primary nav (Home / Downloads / Docs) still present on stub pages", () => {
    const { html } = requireStubPage(SAMPLE_SLUG);
    const primary = extractPrimaryNav(html);
    const sidebar = extractSidebar(html);
    assert.ok(primary.includes("Home"));
    assert.ok(primary.includes("Downloads"));
    assert.ok(primary.includes("Docs"));
    // Sidebar is additional chrome — distinct region
    assert.notEqual(primary, sidebar);
  });

  it("6. ownership boundary: docs index is not redefined as a second full listing route", () => {
    // Optional Docs index link in sidebar is fine; must not invent a new index route.
    const { html } = requireStubPage(SAMPLE_SLUG);
    const links = sidebarLinks(extractSidebar(html));
    for (const link of links) {
      const path = normalizePath(stripBase(link.href));
      assert.ok(
        path === "/docs" || path.startsWith("/docs/"),
        `sidebar link must stay under /docs/, got ${link.href}`
      );
      assert.ok(!path.startsWith("/docs-index"), "must not invent /docs-index route");
      assert.ok(!path.startsWith("/documentation"), "must not invent alternate docs root");
    }

    // Docs index page itself remains phase_4.3's surface (stub OK if 4.3 not landed).
    const indexPage = findDistHtml([
      "docs/index.html",
      "docs.html",
      join(BASE_SEGMENT, "docs/index.html"),
      join(BASE_SEGMENT, "docs.html")
    ]);
    assert.ok(indexPage, "docs index route must still exist");
  });
});
