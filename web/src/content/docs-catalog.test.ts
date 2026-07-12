/**
 * Validation tests for merged docs catalog (phase_3.4).
 * Run: bun run test  (from web/)
 *
 * Exception: `reference/sources` may have empty relatedSlugs if it only
 * links via `sources` — this catalog still gives it relatedSlugs ≥ 1.
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  articlesInSection,
  docsCatalog,
  getArticleBySlug,
  type DocsCatalogEntry,
  type DocsNavSectionId
} from "./docs-catalog.ts";
import {
  DOCS_NAV_SECTIONS,
  DOCS_NAV_SECTION_IDS,
  isDocsHomeSlug,
  isSectionArticleSlug
} from "./docs-nav.ts";

/** External URLs that must appear in ≥1 entry.sources[].href */
export const REQUIRED_SOURCE_URLS = [
  // Beej .txt files
  "https://beej.us/moria/classes.txt",
  "https://beej.us/moria/items.txt",
  "https://beej.us/moria/spells.txt",
  "https://beej.us/moria/weapnarm.txt",
  // mmspoilers major pages
  "https://beej.us/moria/mmspoilers/index.html",
  "https://beej.us/moria/mmspoilers/character.html",
  "https://beej.us/moria/mmspoilers/dungeon.html",
  "https://beej.us/moria/mmspoilers/combat.html",
  "https://beej.us/moria/mmspoilers/items.html",
  "https://beej.us/moria/mmspoilers/spells.html",
  "https://beej.us/moria/mmspoilers/wizardmode.html",
  "https://beej.us/moria/mmspoilers/general.html",
  "https://beej.us/moria/mmspoilers/beginning.html"
] as const;

const REQUIRED_REPO_SOURCE_SUBSTRINGS = ["README.md", "CHANGELOG.md", "CONTRIBUTING.md"] as const;

const MIN_ARTICLES = 45;
const MAX_ARTICLES = 55;

/** Required dependsOnSlugs minimums (phase_3.4). */
const REQUIRED_DEPENDS_ON: ReadonlyArray<readonly [string, readonly string[]]> = [
  ["getting-started/playing", ["getting-started/install"]],
  ["character/classes", ["character/attributes", "character/races"]],
  ["character/experience", ["character/classes"]],
  ["character/social-class", ["character/races", "character/classes"]],
  ["locations/stores", ["locations/city"]],
  ["locations/haggling", ["locations/stores"]],
  ["locations/underground", ["locations/city"]],
  ["combat/monster-attacks", ["combat/monsters"]],
  ["combat/damage", ["combat/hit-probability"]],
  ["combat/armor-class", ["combat/damage"]],
  ["items/weapons", ["items/overview"]],
  ["items/armor", ["items/overview"]],
  ["items/special-properties", ["items/weapons", "items/armor"]],
  ["items/weapon-artifacts", ["items/weapons", "items/special-properties"]],
  ["items/armor-artifacts", ["items/armor", "items/special-properties"]],
  ["items/books", ["spells/system"]],
  ["spells/mana", ["spells/system"]],
  ["spells/failure", ["spells/system", "spells/mana"]],
  ["spells/mage", ["spells/system", "spells/mana", "character/classes"]],
  ["spells/priest", ["spells/system", "spells/mana", "character/classes"]],
  ["wizard/commands", ["wizard/overview"]],
  ["wizard/items", ["wizard/overview", "items/overview"]]
];

/** Required relatedSlugs minimums (phase_3.4). */
const REQUIRED_RELATED: ReadonlyArray<readonly [string, readonly string[]]> = [
  [
    "index",
    [
      "getting-started/install",
      "character/attributes",
      "locations/city",
      "combat/monsters",
      "items/overview",
      "spells/system",
      "wizard/overview",
      "reference/sources"
    ]
  ],
  [
    "character/classes",
    ["spells/mage", "spells/priest", "character/social-class", "character/experience"]
  ],
  ["character/social-class", ["character/races", "character/classes"]],
  ["locations/stores", ["locations/haggling", "locations/city"]],
  ["locations/haggling", ["locations/stores"]],
  ["items/weapons", ["items/special-properties", "combat/damage", "combat/hit-probability"]],
  ["items/armor", ["items/special-properties", "combat/armor-class"]],
  ["items/special-properties", ["items/weapons", "items/armor"]],
  ["items/books", ["spells/mage", "spells/priest", "spells/system"]],
  ["spells/mana", ["spells/mage", "spells/priest", "spells/failure"]],
  ["spells/mage", ["spells/priest", "character/classes", "items/books"]],
  ["spells/priest", ["spells/mage", "character/classes", "items/books"]],
  ["combat/hit-probability", ["combat/damage", "combat/armor-class", "items/weapons"]],
  ["getting-started/differences", ["reference/versions", "getting-started/install"]],
  [
    "reference/sources",
    ["character/social-class", "items/overview", "spells/system", "items/special-properties"]
  ]
];

/** Normalize href for coverage: strip trailing slash; keep path; drop query. */
function normalizeHref(href: string): { base: string; fragment: string } {
  const hashIdx = href.indexOf("#");
  const withoutHash = hashIdx >= 0 ? href.slice(0, hashIdx) : href;
  const fragment = hashIdx >= 0 ? href.slice(hashIdx + 1) : "";
  const base = withoutHash.replace(/\/+$/, "");
  return { base, fragment };
}

function hrefCoversRequired(required: string, candidate: string): boolean {
  const req = normalizeHref(required);
  const got = normalizeHref(candidate);
  // Required page URL is covered if candidate base equals or starts with required base
  // (anchors on child articles still count for the page).
  return got.base === req.base || got.base.startsWith(`${req.base}`);
}

function allSourceHrefs(): string[] {
  return docsCatalog.articles.flatMap((a) => a.sources.map((s) => s.href));
}

function assertIncludesAll(haystack: readonly string[], needles: readonly string[], label: string) {
  for (const needle of needles) {
    assert.ok(
      haystack.includes(needle),
      `${label}: missing ${needle} in [${haystack.join(", ")}]`
    );
  }
}

describe("docsCatalog", () => {
  it("1. unique slugs — no empties, set size equals length", () => {
    const slugs = docsCatalog.articles.map((a) => a.slug);
    assert.ok(slugs.every((s) => s.trim().length > 0), "empty slug found");
    assert.equal(new Set(slugs).size, slugs.length, "duplicate slugs");
  });

  it("2. no empty titles or summaries", () => {
    for (const entry of docsCatalog.articles) {
      assert.ok(entry.title.trim().length > 0, `empty title: ${entry.slug}`);
      assert.ok(entry.summary.trim().length > 0, `empty summary: ${entry.slug}`);
    }
  });

  it("3. sections populated — every DocsNavSectionId has ≥1 article", () => {
    for (const id of DOCS_NAV_SECTION_IDS) {
      const count = articlesInSection(id).length;
      assert.ok(count >= 1, `section ${id} has ${count} articles`);
    }
  });

  it("4. section membership — valid section id and finite order", () => {
    const valid = new Set<string>(DOCS_NAV_SECTION_IDS);
    for (const entry of docsCatalog.articles) {
      assert.ok(valid.has(entry.section), `invalid section on ${entry.slug}: ${entry.section}`);
      assert.ok(Number.isFinite(entry.order), `non-finite order on ${entry.slug}`);
    }
  });

  it("5. nav alignment — docsCatalog.sections matches DOCS_NAV_SECTIONS", () => {
    assert.deepEqual(docsCatalog.sections, DOCS_NAV_SECTIONS);
  });

  it("6. related slug integrity — every relatedSlugs target exists", () => {
    const slugSet = new Set(docsCatalog.articles.map((a) => a.slug));
    for (const entry of docsCatalog.articles) {
      for (const rel of entry.relatedSlugs) {
        assert.ok(slugSet.has(rel), `${entry.slug} relatedSlugs → missing ${rel}`);
      }
    }
  });

  it("7. depends-on integrity — targets exist, no self-deps", () => {
    const slugSet = new Set(docsCatalog.articles.map((a) => a.slug));
    for (const entry of docsCatalog.articles) {
      for (const dep of entry.dependsOnSlugs ?? []) {
        assert.notEqual(dep, entry.slug, `${entry.slug} depends on itself`);
        assert.ok(slugSet.has(dep), `${entry.slug} dependsOnSlugs → missing ${dep}`);
      }
    }
  });

  it("8–9. Beej .txt + mmspoilers major page coverage", () => {
    const hrefs = allSourceHrefs();
    for (const required of REQUIRED_SOURCE_URLS) {
      const hit = hrefs.some((h) => hrefCoversRequired(required, h));
      assert.ok(hit, `missing source coverage for ${required}`);
    }
  });

  it("10. repo source coverage — README, CHANGELOG, CONTRIBUTING", () => {
    const hrefs = allSourceHrefs();
    for (const sub of REQUIRED_REPO_SOURCE_SUBSTRINGS) {
      assert.ok(
        hrefs.some((h) => h.includes(sub)),
        `no sources[].href contains ${sub}`
      );
    }
  });

  it("11. target article count band — 45–55 inclusive", () => {
    const n = docsCatalog.articles.length;
    assert.ok(n >= MIN_ARTICLES && n <= MAX_ARTICLES, `article count ${n} outside ${MIN_ARTICLES}–${MAX_ARTICLES}`);
  });

  it("12. docs index present — exactly one slug===index in getting-started", () => {
    const indexes = docsCatalog.articles.filter((a) => a.slug === "index");
    assert.equal(indexes.length, 1);
    assert.equal(indexes[0]?.section, "getting-started");
  });

  it("13. cross-link minimums — required relatedSlugs", () => {
    for (const [slug, required] of REQUIRED_RELATED) {
      const entry = getArticleBySlug(slug);
      assert.ok(entry, `missing article ${slug}`);
      assertIncludesAll(entry.relatedSlugs, required, `${slug} relatedSlugs`);
    }
  });

  it("14. dependency minimums — required dependsOnSlugs", () => {
    for (const [slug, required] of REQUIRED_DEPENDS_ON) {
      const entry = getArticleBySlug(slug);
      assert.ok(entry, `missing article ${slug}`);
      assertIncludesAll(entry.dependsOnSlugs ?? [], required, `${slug} dependsOnSlugs`);
    }
  });

  it("helpers: getArticleBySlug and articlesInSection", () => {
    const idx = getArticleBySlug("index");
    assert.ok(idx);
    assert.equal(idx.slug, "index");
    assert.equal(getArticleBySlug("does-not-exist"), undefined);

    const character = articlesInSection("character" satisfies DocsNavSectionId);
    assert.ok(character.length >= 1);
    assert.ok(character.every((a: DocsCatalogEntry) => a.section === "character"));
  });

  it("optional: every article has relatedSlugs.length >= 1 (reference/sources exception documented)", () => {
    for (const entry of docsCatalog.articles) {
      if (entry.slug === "reference/sources" && entry.relatedSlugs.length === 0) {
        continue;
      }
      assert.ok(
        entry.relatedSlugs.length >= 1,
        `${entry.slug} has empty relatedSlugs`
      );
    }
  });

  it("optional: slug prefix convention (index or section/topic kebab)", () => {
    // Plan regex omitted hyphens in the first segment; use docs-nav helpers instead.
    for (const entry of docsCatalog.articles) {
      assert.ok(
        isDocsHomeSlug(entry.slug) || isSectionArticleSlug(entry.slug),
        `slug convention fail: ${entry.slug}`
      );
    }
  });

  it("optional: dependsOnSlugs graph is acyclic", () => {
    const bySlug = new Map(docsCatalog.articles.map((a) => [a.slug, a]));
    const visiting = new Set<string>();
    const visited = new Set<string>();

    function visit(slug: string): void {
      if (visited.has(slug)) return;
      assert.ok(!visiting.has(slug), `cycle involving ${slug}`);
      visiting.add(slug);
      const entry = bySlug.get(slug);
      for (const dep of entry?.dependsOnSlugs ?? []) {
        visit(dep);
      }
      visiting.delete(slug);
      visited.add(slug);
    }

    for (const slug of bySlug.keys()) {
      visit(slug);
    }
  });
});
