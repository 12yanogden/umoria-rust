/**
 * Authoritative DocsNavSectionId → Entropic volume number map (phase_4.2).
 * phase_4.4 must import this — do not invent an alternate mapping.
 */
import type { DocsNavSectionId } from "./docs-nav.ts";

/** Section id → volume index (storage under `philes/volume-<N>/`). */
export const DOCS_SECTION_TO_VOLUME: Readonly<Record<DocsNavSectionId, number>> = {
  "getting-started": 0,
  character: 1,
  dungeon: 2,
  combat: 3,
  items: 4,
  spells: 5,
  wizard: 6,
  reference: 7
};

/** Volume titles matching `DOCS_NAV_SECTIONS` labels (phase_4 map). */
export const DOCS_VOLUME_LABELS: Readonly<Record<number, string>> = {
  0: "Getting Started",
  1: "Character",
  2: "Dungeon",
  3: "Combat",
  4: "Items",
  5: "Spells",
  6: "Wizard Mode",
  7: "Reference"
};

/** Relative path from `web/` for a catalog stub phile. */
export function stubRelPathForSlug(slug: string, section: DocsNavSectionId): string {
  const volume = DOCS_SECTION_TO_VOLUME[section];
  if (volume === undefined) {
    throw new Error(`No volume mapping for section "${section}" (slug: ${slug})`);
  }
  return `src/content/philes/volume-${volume}/${slug}.phile`;
}

export function volumeForSection(section: DocsNavSectionId): number {
  return DOCS_SECTION_TO_VOLUME[section];
}
