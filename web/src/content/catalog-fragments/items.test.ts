/**
 * TDD tests for items catalog fragment (phase_3.2.3).
 * Run: bun run test  (from web/) or
 *   node --experimental-strip-types --test src/content/catalog-fragments/items.test.ts
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { itemsCatalogFragment } from "./items.ts";

/** items.txt table groups — each must appear in ≥1 source label. */
const ITEMS_TXT_SECTIONS = [
  "intro",
  "Swords",
  "Hafted",
  "Polearms",
  "Bows",
  "Missiles",
  "Soft",
  "Hard",
  "Shields",
  "Footwear",
  "Headgear",
  "Misc armor",
  "Rings",
  "Amulets",
  "Scrolls",
  "Books",
  "Wands",
  "Staffs",
  "Potions",
  "Normal Food",
  "Mushrooms",
  "Miscellaneous",
  "Shop Items"
] as const;

const MMSPOILERS_ITEM_ANCHORS = [
  "#amulets",
  "#armor",
  "#armorartifacts",
  "#diggers",
  "#food",
  "#gems",
  "#potions",
  "#rings",
  "#scrolls",
  "#staves",
  "#wands",
  "#weapons",
  "#weaponartifacts"
] as const;

const WEAPNARM_HREF = "https://beej.us/moria/weapnarm.txt";
const ITEMS_TXT_HREF = "https://beej.us/moria/items.txt";
const SUMMARY_MAX = 200;

describe("itemsCatalogFragment (phase_3.2.3)", () => {
  it("1. exports itemsCatalogFragment", () => {
    assert.ok(Array.isArray(itemsCatalogFragment));
  });

  it("2. count === 15", () => {
    assert.equal(itemsCatalogFragment.length, 15);
  });

  it("3. unique slugs, all prefixed items/", () => {
    const slugs = itemsCatalogFragment.map((e) => e.slug);
    assert.equal(new Set(slugs).size, slugs.length);
    for (const slug of slugs) {
      assert.ok(slug.startsWith("items/"), `slug must start with items/: ${slug}`);
    }
  });

  it("4. section === items for every entry", () => {
    for (const entry of itemsCatalogFragment) {
      assert.equal(entry.section, "items", `${entry.slug} section`);
    }
  });

  it("5. order 10..150 step 10, unique", () => {
    const orders = itemsCatalogFragment.map((e) => e.order).sort((a, b) => a - b);
    assert.equal(new Set(orders).size, orders.length);
    assert.deepEqual(orders, [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150]);
  });

  it("6. items.txt coverage — every table group in ≥1 source label", () => {
    const labels = itemsCatalogFragment.flatMap((e) =>
      e.sources.filter((s) => s.href === ITEMS_TXT_HREF || s.href.includes("items.txt")).map((s) => s.label)
    );
    const joined = labels.join(" | ");
    for (const section of ITEMS_TXT_SECTIONS) {
      assert.ok(
        labels.some((l) => l.includes(section)),
        `items.txt section missing from source labels: ${section}\nlabels: ${joined}`
      );
    }
  });

  it("7. weapnarm.txt cited exactly once", () => {
    const cites = itemsCatalogFragment.flatMap((e) =>
      e.sources.filter((s) => s.href === WEAPNARM_HREF).map(() => e.slug)
    );
    assert.equal(cites.length, 1, `expected exactly one weapnarm cite, got: ${cites.join(", ")}`);
    assert.equal(cites[0], "items/special-properties");
  });

  it("8. mmspoilers items.html — all 13 anchors present", () => {
    const hrefs = itemsCatalogFragment.flatMap((e) => e.sources.map((s) => s.href));
    for (const anchor of MMSPOILERS_ITEM_ANCHORS) {
      assert.ok(
        hrefs.some((h) => h.includes("mmspoilers/items.html") && h.endsWith(anchor)),
        `missing mmspoilers items.html anchor: ${anchor}`
      );
    }
  });

  it("9. no dump guard — only overview cites items.txt intro; summaries ≤200 chars", () => {
    for (const entry of itemsCatalogFragment) {
      assert.ok(
        entry.summary.length <= SUMMARY_MAX,
        `${entry.slug} summary too long (${entry.summary.length}): ${entry.summary}`
      );
      const introCite = entry.sources.some(
        (s) => (s.href === ITEMS_TXT_HREF || s.href.includes("items.txt")) && /intro/i.test(s.label)
      );
      if (introCite) {
        assert.equal(entry.slug, "items/overview", `only overview may cite items.txt intro: ${entry.slug}`);
      }
    }
  });

  it("10. required fields on every entry", () => {
    for (const entry of itemsCatalogFragment) {
      assert.ok(entry.slug.trim().length > 0, "slug");
      assert.ok(entry.title.trim().length > 0, `${entry.slug} title`);
      assert.ok(entry.summary.trim().length > 0, `${entry.slug} summary`);
      assert.equal(typeof entry.order, "number", `${entry.slug} order`);
      assert.ok(Array.isArray(entry.sources) && entry.sources.length >= 1, `${entry.slug} sources`);
      for (const src of entry.sources) {
        assert.ok(src.label.trim().length > 0, `${entry.slug} source label`);
        assert.ok(src.href.trim().length > 0, `${entry.slug} source href`);
      }
      assert.ok(Array.isArray(entry.relatedSlugs), `${entry.slug} relatedSlugs`);
    }
  });
});
