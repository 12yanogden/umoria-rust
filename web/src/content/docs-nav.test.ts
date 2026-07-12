/**
 * TDD tests for docs nav taxonomy (phase_3.1).
 * Run: bun run test  (from web/)
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  DOCS_HOME_SLUG,
  DOCS_NAV_SECTION_IDS,
  DOCS_NAV_SECTIONS,
  isDocsHomeSlug,
  isSectionArticleSlug,
  sectionIdFromSlug
} from "./docs-nav.ts";

const EXPECTED_IDS = [
  "getting-started",
  "character",
  "locations",
  "combat",
  "items",
  "spells",
  "wizard",
  "reference"
] as const;

const EXPECTED_LABELS: Record<(typeof EXPECTED_IDS)[number], string> = {
  "getting-started": "Getting Started",
  character: "Character",
  locations: "Locations",
  combat: "Combat",
  items: "Items",
  spells: "Spells",
  wizard: "Wizard Mode",
  reference: "Reference"
};

describe("docs-nav taxonomy", () => {
  it("1. required section ids present — exactly eight, no extras", () => {
    assert.equal(DOCS_NAV_SECTION_IDS.length, 8);
    assert.deepEqual([...DOCS_NAV_SECTION_IDS], [...EXPECTED_IDS]);
  });

  it("2. sections array aligned — length 8 and every id in DOCS_NAV_SECTION_IDS", () => {
    assert.equal(DOCS_NAV_SECTIONS.length, 8);
    for (const section of DOCS_NAV_SECTIONS) {
      assert.ok(
        (DOCS_NAV_SECTION_IDS as readonly string[]).includes(section.id),
        `unexpected section id: ${section.id}`
      );
    }
  });

  it("3. unique section orders", () => {
    const orders = DOCS_NAV_SECTIONS.map((s) => s.order);
    assert.equal(new Set(orders).size, orders.length);
  });

  it("4. contiguous orders 1–8", () => {
    const sorted = DOCS_NAV_SECTIONS.map((s) => s.order).sort((a, b) => a - b);
    assert.deepEqual(sorted, [1, 2, 3, 4, 5, 6, 7, 8]);
  });

  it("5. labels non-empty", () => {
    for (const section of DOCS_NAV_SECTIONS) {
      assert.ok(section.label.trim().length > 0, `empty label for ${section.id}`);
    }
  });

  it("6. descriptions non-empty", () => {
    for (const section of DOCS_NAV_SECTIONS) {
      assert.ok(section.description.trim().length > 0, `empty description for ${section.id}`);
    }
  });

  it("7. frozen labels match plan", () => {
    for (const section of DOCS_NAV_SECTIONS) {
      assert.equal(section.label, EXPECTED_LABELS[section.id], `label mismatch for ${section.id}`);
    }
  });

  it("8. docs home slug constant is index", () => {
    assert.equal(DOCS_HOME_SLUG, "index");
  });

  it("9. slug prefix rules — valid section articles", () => {
    assert.equal(isSectionArticleSlug("character/races"), true);
    assert.equal(isSectionArticleSlug("getting-started/install"), true);
    assert.equal(isSectionArticleSlug("items/weapon-artifacts"), true);
  });

  it("10. slug prefix rules — invalid", () => {
    assert.equal(isSectionArticleSlug("index"), false);
    assert.equal(isSectionArticleSlug("character"), false);
    assert.equal(isSectionArticleSlug("Character/Races"), false);
    assert.equal(isSectionArticleSlug("character/races/extra"), false);
    assert.equal(isSectionArticleSlug("character/races_underscore"), false);
    assert.equal(isSectionArticleSlug(""), false);
    assert.equal(isSectionArticleSlug("/leading"), false);
    assert.equal(isSectionArticleSlug("trailing/"), false);
  });

  it("11. home predicate", () => {
    assert.equal(isDocsHomeSlug("index"), true);
    assert.equal(isDocsHomeSlug("getting-started/install"), false);
  });

  it("12. section from slug", () => {
    assert.equal(sectionIdFromSlug("locations/traps"), "locations");
    assert.equal(sectionIdFromSlug("index"), null);
  });

  it("13. prefix set matches section ids", () => {
    for (const id of DOCS_NAV_SECTION_IDS) {
      assert.equal(isSectionArticleSlug(`${id}/smoke-test`), true, `expected ${id}/smoke-test to be valid`);
    }
  });
});
