/**
 * Phase 2.2 — build-output assertions for splash / hero landing.
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

function requireHome() {
  const page = findHtml(["index.html", join(BASE.slice(1), "index.html")]);
  assert.ok(page, "home: expected dist/index.html or dist/umoria-rust/index.html");
  return page;
}

function extractNav(html) {
  const match = html.match(/<nav\b[^>]*data-site-nav[^>]*>([\s\S]*?)<\/nav>/i);
  assert.ok(match, "expected <nav data-site-nav> region");
  return match[0];
}

function extractHero(html) {
  // Prefer <section data-home-hero> so nested <div> closers do not truncate the region.
  const match = html.match(/<section\b[^>]*data-home-hero[^>]*>([\s\S]*?)<\/section>/i);
  assert.ok(match, "expected a <section data-home-hero> region");
  return { outer: match[0], inner: match[1] };
}

function linksIn(html) {
  const links = [];
  const re = /<a\b([^>]*)>([\s\S]*?)<\/a>/gi;
  let m;
  while ((m = re.exec(html)) !== null) {
    const attrs = m[1];
    const hrefMatch = attrs.match(/\bhref\s*=\s*"([^"]*)"/i);
    const label = m[2].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim();
    links.push({ href: hrefMatch?.[1] ?? "", label, attrs });
  }
  return links;
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

test("1. Home route exists at configured base path", () => {
  requireHome();
});

test("2. Brand-forward Umoria identity in hero (not only nav)", () => {
  const { html } = requireHome();
  const nav = extractNav(html);
  const hero = extractHero(html);

  assert.ok(/Umoria/i.test(hero.outer), "hero must contain brand string Umoria");
  assert.ok(
    /data-home-brand|ascii-hero|home-ascii/i.test(hero.outer),
    "hero must expose a prominent brand/title block (data-home-brand, ascii-hero, or home-ascii)"
  );

  // Brand signal must appear outside the global nav chrome.
  const withoutNav = html.replace(nav, "");
  assert.ok(/Umoria/i.test(withoutNav), "Umoria must appear outside <nav data-site-nav>");
});

test("3. Single headline + one support line; no card/photo/multi-CTA clutter", () => {
  const { html } = requireHome();
  const hero = extractHero(html);

  const headlines = [...hero.inner.matchAll(/data-home-headline/gi)];
  const supports = [...hero.inner.matchAll(/data-home-support/gi)];
  assert.equal(headlines.length, 1, "hero must have exactly one data-home-headline");
  assert.equal(supports.length, 1, "hero must have exactly one data-home-support");

  const headlineMatch = hero.inner.match(
    /<(?:h[1-3]|p|div|span|pre)\b[^>]*data-home-headline[^>]*>([\s\S]*?)<\/(?:h[1-3]|p|div|span|pre)>/i
  );
  const supportMatch = hero.inner.match(
    /<(?:p|div|span|pre)\b[^>]*data-home-support[^>]*>([\s\S]*?)<\/(?:p|div|span|pre)>/i
  );
  assert.ok(headlineMatch, "headline element must be present");
  assert.ok(supportMatch, "support element must be present");
  const headlineText = headlineMatch[1].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim();
  const supportText = supportMatch[1].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim();
  assert.ok(headlineText.length > 0, "headline must have text");
  assert.ok(supportText.length > 0, "support must have text");

  const ctaGroups = [...hero.inner.matchAll(/data-home-cta/gi)];
  assert.equal(ctaGroups.length, 1, "hero must have exactly one data-home-cta group");

  assert.ok(!/<img\b/i.test(hero.inner), "hero must not use a photographic <img>");
  assert.ok(
    !/\b(card-grid|cards-grid|hero-cards|stat-strip)\b/i.test(hero.inner),
    "hero must not contain a card grid / stat strip"
  );
});

test("4. CTA group — Downloads uses base-safe href", () => {
  const { html } = requireHome();
  const hero = extractHero(html);
  const ctaMatch = hero.inner.match(/<(?:div|nav|p|pre)\b[^>]*data-home-cta[^>]*>([\s\S]*?)<\/(?:div|nav|p|pre)>/i);
  assert.ok(ctaMatch, "expected data-home-cta container");
  const links = linksIn(ctaMatch[0]);
  const downloads = links.find((l) => /downloads/i.test(l.label));
  assert.ok(downloads, `expected Downloads CTA in hero, got: ${links.map((l) => l.label).join(", ")}`);
  assertBaseSafeHref(downloads.href, "/downloads/");
});

test("5. CTA group — Docs index/stub only; no article listing in hero", () => {
  const { html } = requireHome();
  const hero = extractHero(html);
  const ctaMatch = hero.inner.match(/<(?:div|nav|p|pre)\b[^>]*data-home-cta[^>]*>([\s\S]*?)<\/(?:div|nav|p|pre)>/i);
  assert.ok(ctaMatch, "expected data-home-cta container");
  const links = linksIn(ctaMatch[0]);
  const docs = links.find((l) => /^docs$/i.test(l.label.replace(/^\[|\]$/g, "").trim()) || /^docs$/i.test(l.label));
  assert.ok(docs, `expected Docs CTA in hero, got: ${links.map((l) => l.label).join(", ")}`);
  assert.ok(
    docs.href === `${BASE}/docs/` || docs.href === `${BASE}/docs`,
    `Docs CTA must be docs index only, got ${docs.href}`
  );
  assert.ok(!/\/volume\//.test(docs.href), "Docs CTA must not point at volume/phile paths");

  assert.ok(
    !/\/volume\/\d+\//i.test(hero.inner),
    "hero must not enumerate docs article/volume slugs"
  );
  assert.ok(
    !/(spoiler|beej\.us|mmspoilers|article catalog)/i.test(hero.inner),
    "hero must not preview docs article catalog content"
  );
});

test("6. Layout chrome preserved — shared nav, no duplicate site-wide nav", () => {
  const { html } = requireHome();
  const navMatches = [...html.matchAll(/<nav\b[^>]*data-site-nav/gi)];
  assert.equal(navMatches.length, 1, "exactly one primary site nav (phase_2.1) must be present");
  extractNav(html);
  extractHero(html);

  // Hero CTAs must not invent a second data-site-nav.
  const hero = extractHero(html);
  assert.ok(!/data-site-nav/i.test(hero.inner), "hero must not nest a competing data-site-nav");
});
