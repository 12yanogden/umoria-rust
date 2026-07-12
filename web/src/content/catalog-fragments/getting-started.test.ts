/**
 * TDD tests for getting-started catalog fragment (phase_3.3).
 * Run: bun run test  (from web/)
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import { gettingStartedCatalog } from "./getting-started.ts";

const EXPECTED_SLUGS = [
  "index",
  "getting-started/install",
  "getting-started/playing",
  "getting-started/differences",
  "getting-started/contributing"
] as const;

const SPOILER_PREFIXES = ["character/", "dungeon/", "combat/", "items/", "spells/", "wizard/", "reference/"] as const;

const DEPENDS_ON_INSTALL = [
  "getting-started/playing",
  "getting-started/differences",
  "getting-started/contributing"
] as const;

function bySlug(slug: string) {
  return gettingStartedCatalog.find((e) => e.slug === slug);
}

describe("getting-started catalog fragment", () => {
  it("1. fragment exports gettingStartedCatalog", () => {
    assert.ok(Array.isArray(gettingStartedCatalog), "gettingStartedCatalog must be an array");
  });

  it("2. exact article set — five required slugs", () => {
    assert.equal(gettingStartedCatalog.length, 5);
    const slugs = gettingStartedCatalog.map((e) => e.slug).sort();
    assert.deepEqual(slugs, [...EXPECTED_SLUGS].sort());
  });

  it("3. unique slugs (fragment-local)", () => {
    const slugs = gettingStartedCatalog.map((e) => e.slug);
    assert.equal(new Set(slugs).size, slugs.length);
  });

  it("4. required fields — title, summary, section, order, sources, relatedSlugs", () => {
    for (const entry of gettingStartedCatalog) {
      assert.ok(entry.title.trim().length > 0, `empty title for ${entry.slug}`);
      assert.ok(entry.summary.trim().length > 0, `empty summary for ${entry.slug}`);
      assert.ok(entry.section, `missing section for ${entry.slug}`);
      assert.equal(typeof entry.order, "number", `order must be numeric for ${entry.slug}`);
      assert.ok(Array.isArray(entry.sources) && entry.sources.length >= 1, `sources required for ${entry.slug}`);
      for (const src of entry.sources) {
        assert.ok(src.label.trim().length > 0, `empty source label on ${entry.slug}`);
        assert.ok(src.href.trim().length > 0, `empty source href on ${entry.slug}`);
      }
      assert.ok(Array.isArray(entry.relatedSlugs), `relatedSlugs required for ${entry.slug}`);
      assert.ok(entry.relatedSlugs.length >= 1, `≥1 relatedSlug required for ${entry.slug}`);
    }
  });

  it("5. section assignment — all getting-started", () => {
    for (const entry of gettingStartedCatalog) {
      assert.equal(entry.section, "getting-started", `section mismatch for ${entry.slug}`);
    }
  });

  it("6. order monotonic 0–4 matching inventory", () => {
    const byOrder = [...gettingStartedCatalog].sort((a, b) => a.order - b.order);
    assert.deepEqual(
      byOrder.map((e) => e.order),
      [0, 1, 2, 3, 4]
    );
    assert.equal(byOrder[0].slug, "index");
    assert.equal(byOrder[1].slug, "getting-started/install");
    assert.equal(byOrder[2].slug, "getting-started/playing");
    assert.equal(byOrder[3].slug, "getting-started/differences");
    assert.equal(byOrder[4].slug, "getting-started/contributing");
  });

  it("7. README sources cited on install and playing", () => {
    for (const slug of ["getting-started/install", "getting-started/playing"] as const) {
      const entry = bySlug(slug);
      assert.ok(entry, `missing ${slug}`);
      assert.ok(
        entry.sources.some((s) => s.href.includes("README.md")),
        `${slug} must cite README.md`
      );
    }
  });

  it("8. CHANGELOG cited for differences", () => {
    const entry = bySlug("getting-started/differences");
    assert.ok(entry, "missing getting-started/differences");
    assert.ok(
      entry.sources.some((s) => s.href.includes("CHANGELOG.md")),
      "differences must cite CHANGELOG.md"
    );
  });

  it("9. CONTRIBUTING cited", () => {
    const entry = bySlug("getting-started/contributing");
    assert.ok(entry, "missing getting-started/contributing");
    assert.ok(
      entry.sources.some((s) => s.href.includes("CONTRIBUTING.md")),
      "contributing must cite CONTRIBUTING.md"
    );
  });

  it("10. index is root slug — Documentation uses index exactly", () => {
    const doc = gettingStartedCatalog.find((e) => e.title === "Documentation");
    assert.ok(doc, "missing Documentation entry");
    assert.equal(doc.slug, "index");
    assert.ok(!gettingStartedCatalog.some((e) => e.slug === "getting-started/index"));
  });

  it("11. reading order deps — playing, differences, contributing depend on install", () => {
    for (const slug of DEPENDS_ON_INSTALL) {
      const entry = bySlug(slug);
      assert.ok(entry, `missing ${slug}`);
      assert.ok(Array.isArray(entry.dependsOnSlugs), `${slug} needs dependsOnSlugs`);
      assert.ok(
        entry.dependsOnSlugs.includes("getting-started/install"),
        `${slug} must depend on getting-started/install`
      );
    }
  });

  it("12. no spoiler article bodies — only relatedSlugs may reference spoiler prefixes", () => {
    for (const entry of gettingStartedCatalog) {
      for (const prefix of SPOILER_PREFIXES) {
        assert.ok(!entry.slug.startsWith(prefix), `spoiler entry slug not allowed: ${entry.slug}`);
      }
    }

    const fragmentPath = join(dirname(fileURLToPath(import.meta.url)), "getting-started.ts");
    const source = readFileSync(fragmentPath, "utf8");
    // Strip relatedSlugs / dependsOnSlugs string arrays so spoiler forward refs don't false-positive.
    const withoutLinkArrays = source
      .replace(/relatedSlugs\s*:\s*\[[^\]]*\]/gs, "relatedSlugs: []")
      .replace(/dependsOnSlugs\s*:\s*\[[^\]]*\]/gs, "dependsOnSlugs: []");
    for (const prefix of SPOILER_PREFIXES) {
      assert.ok(
        !withoutLinkArrays.includes(`slug: "${prefix}`) && !withoutLinkArrays.includes(`slug: '${prefix}`),
        `fragment must not author spoiler slug entries under ${prefix}`
      );
    }
  });
});
