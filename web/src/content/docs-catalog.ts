/**
 * Merged docs catalog (phase_3.4) — runtime source of truth for phase_4.
 *
 * Assembles docs-nav sections + catalog-fragments.
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
    bySlug.set(entry.slug, entry);
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
