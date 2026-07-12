/**
 * TDD tests for character catalog fragment (phase_3.2.1).
 * Run: bun run test  (from web/)
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { characterCatalogFragment } from "../character.ts";

const EXPECTED_SLUGS = [
  "character/attributes",
  "character/races",
  "character/classes",
  "character/experience",
  "character/social-class"
] as const;

const EXPECTED_ORDERS = [10, 20, 30, 40, 50] as const;

const MMSPOILERS_ANCHORS = [
  "https://beej.us/moria/mmspoilers/character.html#attributes",
  "https://beej.us/moria/mmspoilers/character.html#races",
  "https://beej.us/moria/mmspoilers/character.html#classes",
  "https://beej.us/moria/mmspoilers/character.html#experience"
] as const;

const CLASSES_TXT = "https://beej.us/moria/classes.txt";

describe("characterCatalogFragment", () => {
  it("1. fragment exports characterCatalogFragment", () => {
    assert.ok(Array.isArray(characterCatalogFragment));
  });

  it("2. count === 5", () => {
    assert.equal(characterCatalogFragment.length, 5);
  });

  it("3. unique slugs within fragment", () => {
    const slugs = characterCatalogFragment.map((e) => e.slug);
    assert.equal(new Set(slugs).size, slugs.length);
    assert.deepEqual(slugs, [...EXPECTED_SLUGS]);
  });

  it("4. section consistency — section === 'character' and slug prefix character/", () => {
    for (const entry of characterCatalogFragment) {
      assert.equal(entry.section, "character", `section for ${entry.slug}`);
      assert.ok(entry.slug.startsWith("character/"), `slug prefix for ${entry.slug}`);
    }
  });

  it("5. order monotonic 10..50, unique within section", () => {
    const orders = characterCatalogFragment.map((e) => e.order);
    assert.deepEqual(orders, [...EXPECTED_ORDERS]);
    assert.equal(new Set(orders).size, orders.length);
  });

  it("6. required fields — non-empty title, summary, ≥1 sources with label+href", () => {
    for (const entry of characterCatalogFragment) {
      assert.ok(entry.title.trim().length > 0, `empty title: ${entry.slug}`);
      assert.ok(entry.summary.trim().length > 0, `empty summary: ${entry.slug}`);
      assert.ok(entry.sources.length >= 1, `no sources: ${entry.slug}`);
      for (const src of entry.sources) {
        assert.ok(src.label.trim().length > 0, `empty source label: ${entry.slug}`);
        assert.ok(src.href.trim().length > 0, `empty source href: ${entry.slug}`);
      }
    }
  });

  it("7. mmspoilers anchor coverage — four character.html anchors exactly once", () => {
    const hrefs = characterCatalogFragment.flatMap((e) => e.sources.map((s) => s.href));
    for (const anchor of MMSPOILERS_ANCHORS) {
      const count = hrefs.filter((h) => h === anchor).length;
      assert.equal(count, 1, `expected exactly one citation of ${anchor}, got ${count}`);
    }
  });

  it("8. classes.txt coverage — href present; labels reference §1–§6", () => {
    const classesSources = characterCatalogFragment.flatMap((e) => e.sources.filter((s) => s.href === CLASSES_TXT));
    assert.ok(classesSources.length >= 1, "expected ≥1 classes.txt href");

    const labelsJoined = classesSources.map((s) => s.label).join(" ");
    for (const section of ["§1", "§2", "§3", "§4", "§5", "§6"]) {
      assert.ok(labelsJoined.includes(section), `expected classes.txt source labels to reference ${section}`);
    }
    assert.ok(/introduction/i.test(labelsJoined), "expected classes.txt intro labeled among sources");
  });

  it("9. no prose dump — summary ≤ 200 chars", () => {
    for (const entry of characterCatalogFragment) {
      assert.ok(entry.summary.length <= 200, `summary too long (${entry.summary.length}): ${entry.slug}`);
    }
  });
});
