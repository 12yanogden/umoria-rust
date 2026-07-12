/**
 * TDD tests for locations + combat catalog fragment (phase_3.2.2).
 * Run: bun run test  (from web/)
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { dungeonCombatCatalogFragment } from "../dungeon-combat.ts";

const EXPECTED_LOCATIONS_SLUGS = [
  "locations/city",
  "locations/stores",
  "locations/haggling",
  "locations/underground",
  "locations/traps"
] as const;

const EXPECTED_COMBAT_SLUGS = [
  "combat/monsters",
  "combat/monster-attacks",
  "combat/hit-probability",
  "combat/damage",
  "combat/bashing",
  "combat/armor-class"
] as const;

const EXPECTED_SLUGS = [...EXPECTED_LOCATIONS_SLUGS, ...EXPECTED_COMBAT_SLUGS] as const;

const LOCATIONS_ORDERS = [10, 20, 30, 40, 50] as const;
const COMBAT_ORDERS = [10, 20, 30, 40, 50, 60] as const;

const MMSPOILERS_ANCHORS = [
  "https://beej.us/moria/mmspoilers/dungeon.html#city",
  "https://beej.us/moria/mmspoilers/dungeon.html#stores",
  "https://beej.us/moria/mmspoilers/dungeon.html#haggling",
  "https://beej.us/moria/mmspoilers/dungeon.html#underground",
  "https://beej.us/moria/mmspoilers/dungeon.html#traps",
  "https://beej.us/moria/mmspoilers/combat.html#mdescriptions",
  "https://beej.us/moria/mmspoilers/combat.html#mattacks",
  "https://beej.us/moria/mmspoilers/combat.html#hitprob",
  "https://beej.us/moria/mmspoilers/combat.html#damagecalc",
  "https://beej.us/moria/mmspoilers/combat.html#bashing",
  "https://beej.us/moria/mmspoilers/combat.html#accalc"
] as const;

describe("dungeonCombatCatalogFragment", () => {
  it("1. fragment exports dungeonCombatCatalogFragment", () => {
    assert.ok(Array.isArray(dungeonCombatCatalogFragment));
  });

  it("2. count === 11 (5 locations + 6 combat)", () => {
    assert.equal(dungeonCombatCatalogFragment.length, 11);
  });

  it("3. unique slugs within fragment", () => {
    const slugs = dungeonCombatCatalogFragment.map((e) => e.slug);
    assert.equal(new Set(slugs).size, slugs.length);
    assert.deepEqual(slugs, [...EXPECTED_SLUGS]);
  });

  it("4. section filter — 5 locations, 6 combat", () => {
    const locations = dungeonCombatCatalogFragment.filter((e) => e.section === "locations");
    const combat = dungeonCombatCatalogFragment.filter((e) => e.section === "combat");
    assert.equal(locations.length, 5);
    assert.equal(combat.length, 6);
  });

  it("5. slug prefixes match section ids", () => {
    for (const entry of dungeonCombatCatalogFragment) {
      assert.ok(entry.slug.startsWith(`${entry.section}/`), `slug ${entry.slug} must start with ${entry.section}/`);
    }
  });

  it("6. order — locations 10–50, combat 10–60; unique per section", () => {
    const locations = dungeonCombatCatalogFragment.filter((e) => e.section === "locations");
    const combat = dungeonCombatCatalogFragment.filter((e) => e.section === "combat");
    assert.deepEqual(
      locations.map((e) => e.order),
      [...LOCATIONS_ORDERS]
    );
    assert.deepEqual(
      combat.map((e) => e.order),
      [...COMBAT_ORDERS]
    );
    assert.equal(new Set(locations.map((e) => e.order)).size, locations.length);
    assert.equal(new Set(combat.map((e) => e.order)).size, combat.length);
  });

  it("7. anchor coverage — all 11 mmspoilers hrefs present", () => {
    const hrefs = dungeonCombatCatalogFragment.flatMap((e) => e.sources.map((s) => s.href));
    for (const anchor of MMSPOILERS_ANCHORS) {
      assert.ok(hrefs.includes(anchor), `expected sources to include ${anchor}`);
    }
  });

  it("8. required fields on every entry", () => {
    for (const entry of dungeonCombatCatalogFragment) {
      assert.ok(entry.title.trim().length > 0, `empty title: ${entry.slug}`);
      assert.ok(entry.summary.trim().length > 0, `empty summary: ${entry.slug}`);
      assert.ok(Array.isArray(entry.relatedSlugs), `relatedSlugs: ${entry.slug}`);
      assert.ok(entry.sources.length >= 1, `no sources: ${entry.slug}`);
      for (const src of entry.sources) {
        assert.ok(src.label.trim().length > 0, `empty source label: ${entry.slug}`);
        assert.ok(src.href.trim().length > 0, `empty source href: ${entry.slug}`);
      }
    }
  });

  it("9. split sanity — combat/monsters summary does not claim full inline table", () => {
    const monsters = dungeonCombatCatalogFragment.find((e) => e.slug === "combat/monsters");
    assert.ok(monsters, "combat/monsters entry missing");
    const summary = monsters.summary.toLowerCase();
    assert.ok(
      /stub|link|reference|source|pointer|table/.test(summary),
      "monsters summary should note stub/link/reference to source table"
    );
    assert.equal(
      /full monster (flag|ability|stat)|inline the full|paste the full/i.test(monsters.summary),
      false,
      "monsters summary must not claim to inline the full monster table"
    );
  });
});
