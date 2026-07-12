/**
 * TDD tests for spells + wizard + reference catalog fragment (phase_3.2.4).
 * Run: bun run test  (from web/) or
 *   node --experimental-strip-types --test src/content/catalog-fragments/spells-wizard-reference.test.ts
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { spellsWizardReferenceCatalogFragment } from "./spells-wizard-reference.ts";

const SPELLS_TXT = "https://beej.us/moria/spells.txt";

const SPELL_ANCHORS = [
  "https://beej.us/moria/mmspoilers/spells.html#spellsystem",
  "https://beej.us/moria/mmspoilers/spells.html#mana",
  "https://beej.us/moria/mmspoilers/spells.html#failure",
  "https://beej.us/moria/mmspoilers/spells.html#magespells",
  "https://beej.us/moria/mmspoilers/spells.html#clericspells"
] as const;

const WIZARD_ANCHORS = [
  "https://beej.us/moria/mmspoilers/wizardmode.html#enteringwizard",
  "https://beej.us/moria/mmspoilers/wizardmode.html#wizardcommands",
  "https://beej.us/moria/mmspoilers/wizardmode.html#wizarditems"
] as const;

const REFERENCE_ANCHORS = [
  "https://beej.us/moria/mmspoilers/beginning.html#credits",
  "https://beej.us/moria/mmspoilers/beginning.html#usingthespoilers",
  "https://beej.us/moria/mmspoilers/beginning.html#revisionhistory",
  "https://beej.us/moria/mmspoilers/general.html#moriaversions",
  "https://beej.us/moria/mmspoilers/general.html#frequentlyaskedquestions"
] as const;

const SUMMARY_MAX = 200;

describe("spellsWizardReferenceCatalogFragment (phase_3.2.4)", () => {
  it("1. exports spellsWizardReferenceCatalogFragment", () => {
    assert.ok(Array.isArray(spellsWizardReferenceCatalogFragment));
  });

  it("2. count === 10 (5 + 3 + 2)", () => {
    assert.equal(spellsWizardReferenceCatalogFragment.length, 10);
  });

  it("3. unique slugs within fragment", () => {
    const slugs = spellsWizardReferenceCatalogFragment.map((e) => e.slug);
    assert.equal(new Set(slugs).size, slugs.length);
  });

  it("4. section counts — spells:5, wizard:3, reference:2; slug prefixes match", () => {
    const bySection = (id: string) => spellsWizardReferenceCatalogFragment.filter((e) => e.section === id);

    const spells = bySection("spells");
    const wizard = bySection("wizard");
    const reference = bySection("reference");

    assert.equal(spells.length, 5);
    assert.equal(wizard.length, 3);
    assert.equal(reference.length, 2);
    assert.equal(
      spells.length + wizard.length + reference.length,
      spellsWizardReferenceCatalogFragment.length,
      "no entries outside spells/wizard/reference"
    );

    for (const entry of spells) {
      assert.ok(entry.slug.startsWith("spells/"), `prefix: ${entry.slug}`);
    }
    for (const entry of wizard) {
      assert.ok(entry.slug.startsWith("wizard/"), `prefix: ${entry.slug}`);
    }
    for (const entry of reference) {
      assert.ok(entry.slug.startsWith("reference/"), `prefix: ${entry.slug}`);
    }
  });

  it("5. order unique per section (spells 10–50, wizard 10–30, reference 10–20)", () => {
    const ordersFor = (section: string) =>
      spellsWizardReferenceCatalogFragment
        .filter((e) => e.section === section)
        .map((e) => e.order)
        .sort((a, b) => a - b);

    const spellOrders = ordersFor("spells");
    const wizardOrders = ordersFor("wizard");
    const referenceOrders = ordersFor("reference");

    assert.equal(new Set(spellOrders).size, spellOrders.length);
    assert.equal(new Set(wizardOrders).size, wizardOrders.length);
    assert.equal(new Set(referenceOrders).size, referenceOrders.length);

    assert.deepEqual(spellOrders, [10, 20, 30, 40, 50]);
    assert.deepEqual(wizardOrders, [10, 20, 30]);
    assert.deepEqual(referenceOrders, [10, 20]);
  });

  it("6. spells.txt on all five spell entries; labels cover §1.1, §1.2, §1.3", () => {
    const spellEntries = spellsWizardReferenceCatalogFragment.filter((e) => e.section === "spells");
    assert.equal(spellEntries.length, 5);

    for (const entry of spellEntries) {
      assert.ok(
        entry.sources.some((s) => s.href === SPELLS_TXT),
        `${entry.slug} must cite ${SPELLS_TXT}`
      );
    }

    const labels = spellEntries
      .flatMap((e) => e.sources.filter((s) => s.href === SPELLS_TXT).map((s) => s.label))
      .join(" ");
    for (const section of ["§1.1", "§1.2", "§1.3"]) {
      assert.ok(labels.includes(section), `spells.txt labels must cover ${section}: ${labels}`);
    }
  });

  it("7. spell anchors — all five spells.html anchors in href set", () => {
    const hrefs = spellsWizardReferenceCatalogFragment.flatMap((e) => e.sources.map((s) => s.href));
    for (const anchor of SPELL_ANCHORS) {
      assert.ok(hrefs.includes(anchor), `missing spell anchor: ${anchor}`);
    }
  });

  it("8. wizard anchors — all three wizardmode.html anchors", () => {
    const hrefs = spellsWizardReferenceCatalogFragment.flatMap((e) => e.sources.map((s) => s.href));
    for (const anchor of WIZARD_ANCHORS) {
      assert.ok(hrefs.includes(anchor), `missing wizard anchor: ${anchor}`);
    }
  });

  it("9. reference anchors — general.html + beginning.html checklist present", () => {
    const hrefs = spellsWizardReferenceCatalogFragment.flatMap((e) => e.sources.map((s) => s.href));
    for (const anchor of REFERENCE_ANCHORS) {
      assert.ok(hrefs.includes(anchor), `missing reference anchor: ${anchor}`);
    }
  });

  it("10. required fields; summaries ≤200 chars", () => {
    for (const entry of spellsWizardReferenceCatalogFragment) {
      assert.ok(entry.slug.trim().length > 0, "slug");
      assert.ok(entry.title.trim().length > 0, `${entry.slug} title`);
      assert.ok(entry.summary.trim().length > 0, `${entry.slug} summary`);
      assert.ok(entry.summary.length <= SUMMARY_MAX, `${entry.slug} summary too long (${entry.summary.length})`);
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
