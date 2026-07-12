/**
 * Merged docs catalog (phase_3.4) — runtime source of truth for phase_4.
 *
 * Assembles docs-nav sections + catalog-fragments, completes cross-links.
 * No article prose.
 */

import { characterCatalogFragment } from "./catalog-fragments/character.ts";
import { dungeonCombatCatalogFragment } from "./catalog-fragments/dungeon-combat.ts";
import { gettingStartedCatalog } from "./catalog-fragments/getting-started.ts";
import { itemsCatalogFragment } from "./catalog-fragments/items.ts";
import { spellsWizardReferenceCatalogFragment } from "./catalog-fragments/spells-wizard-reference.ts";
import type { DocsCatalogEntry, DocsSourceRef } from "./docs-catalog-types.ts";
import { DOCS_NAV_SECTIONS, type DocsNavSection, type DocsNavSectionId } from "./docs-nav.ts";

export type { DocsCatalogEntry, DocsNavSection, DocsNavSectionId, DocsSourceRef };

export interface DocsCatalog {
  sections: DocsNavSection[];
  articles: DocsCatalogEntry[];
}

/** Required dependsOnSlugs overlays (phase_3.4) — merged into fragment values. */
const DEPENDS_ON_OVERLAY: Readonly<Record<string, readonly string[]>> = {
  "getting-started/playing": ["getting-started/install"],
  "character/classes": ["character/attributes", "character/races"],
  "character/experience": ["character/classes"],
  "character/social-class-humanoids": ["character/social-class", "character/classes"],
  "character/social-class-elves": ["character/social-class", "character/classes"],
  "character/social-class-smallfolk": ["character/social-class", "character/classes"],
  "character/social-class-dwarves-trolls": ["character/social-class", "character/classes"],
  "dungeon/stores": ["dungeon/city"],
  "dungeon/haggling": ["dungeon/stores"],
  "dungeon/underground": ["dungeon/city"],
  "combat/monster-attacks": ["combat/monsters"],
  "combat/damage": ["combat/hit-probability"],
  "combat/armor-class": ["combat/damage"],
  "items/weapons": ["items/overview"],
  "items/armor": ["items/overview"],
  "items/special-properties": ["items/weapons", "items/armor"],
  "items/weapon-artifacts": ["items/weapons", "items/special-properties"],
  "items/armor-artifacts": ["items/armor", "items/special-properties"],
  "items/books": ["spells/system"],
  "spells/mana": ["spells/system"],
  "spells/failure": ["spells/system", "spells/mana"],
  "spells/mage": ["spells/system", "spells/mana", "character/classes"],
  "spells/priest": ["spells/system", "spells/mana", "character/classes"],
  "wizard/commands": ["wizard/overview"],
  "wizard/items": ["wizard/overview", "items/overview"]
};

/** Required relatedSlugs overlays (phase_3.4) — merged into fragment values. */
const RELATED_OVERLAY: Readonly<Record<string, readonly string[]>> = {
  index: [
    "getting-started/install",
    "character/attributes",
    "dungeon/city",
    "combat/monsters",
    "items/overview",
    "spells/system",
    "wizard/overview",
    "reference/sources"
  ],
  "character/classes": ["spells/mage", "spells/priest", "character/social-class", "character/experience"],
  "character/social-class": [
    "character/social-class-humanoids",
    "character/social-class-elves",
    "character/social-class-smallfolk",
    "character/social-class-dwarves-trolls"
  ],
  "dungeon/stores": ["dungeon/haggling", "dungeon/city"],
  "dungeon/haggling": ["dungeon/stores"],
  "items/weapons": ["items/special-properties", "combat/damage", "combat/hit-probability"],
  "items/armor": ["items/special-properties", "combat/armor-class"],
  "items/special-properties": ["items/weapons", "items/armor"],
  "items/books": ["spells/mage", "spells/priest", "spells/system"],
  "spells/mana": ["spells/mage", "spells/priest", "spells/failure"],
  "spells/mage": ["spells/priest", "character/classes", "items/books"],
  "spells/priest": ["spells/mage", "character/classes", "items/books"],
  "combat/hit-probability": ["combat/damage", "combat/armor-class", "items/weapons"],
  "getting-started/differences": ["reference/versions", "getting-started/install"],
  "reference/sources": ["character/social-class", "items/overview", "spells/system", "items/special-properties"]
};

function uniquePreserveOrder(values: readonly string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const v of values) {
    if (seen.has(v)) continue;
    seen.add(v);
    out.push(v);
  }
  return out;
}

function applyCrossLinks(entry: DocsCatalogEntry): DocsCatalogEntry {
  const relatedExtra = RELATED_OVERLAY[entry.slug] ?? [];
  const dependsExtra = DEPENDS_ON_OVERLAY[entry.slug] ?? [];
  const relatedSlugs = uniquePreserveOrder([...entry.relatedSlugs, ...relatedExtra]);
  const dependsMerged = uniquePreserveOrder([...(entry.dependsOnSlugs ?? []), ...dependsExtra]);
  return {
    ...entry,
    relatedSlugs,
    ...(dependsMerged.length > 0 ? { dependsOnSlugs: dependsMerged } : {})
  };
}

function mergeArticles(): DocsCatalogEntry[] {
  const fragments: DocsCatalogEntry[] = [
    ...gettingStartedCatalog,
    ...characterCatalogFragment,
    ...dungeonCombatCatalogFragment,
    ...itemsCatalogFragment,
    ...spellsWizardReferenceCatalogFragment
  ];

  const bySlug = new Map<string, DocsCatalogEntry>();
  for (const entry of fragments) {
    if (bySlug.has(entry.slug)) {
      throw new Error(
        `Duplicate catalog slug "${entry.slug}" — prefer the fragment named for that section; do not silently overwrite`
      );
    }
    bySlug.set(entry.slug, applyCrossLinks(entry));
  }

  const sectionOrder = new Map(DOCS_NAV_SECTIONS.map((s) => [s.id, s.order]));

  return [...bySlug.values()].sort((a, b) => {
    const sa = sectionOrder.get(a.section) ?? 999;
    const sb = sectionOrder.get(b.section) ?? 999;
    if (sa !== sb) return sa - sb;
    return a.order - b.order;
  });
}

export const docsCatalog: DocsCatalog = {
  sections: DOCS_NAV_SECTIONS,
  articles: mergeArticles()
};

export function getArticleBySlug(slug: string): DocsCatalogEntry | undefined {
  return docsCatalog.articles.find((a) => a.slug === slug);
}

export function articlesInSection(section: DocsNavSectionId): DocsCatalogEntry[] {
  return docsCatalog.articles.filter((a) => a.section === section);
}

/** Re-export nav sections for phase_4 convenience. */
export { DOCS_NAV_SECTION_IDS, DOCS_NAV_SECTIONS } from "./docs-nav.ts";
