/**
 * Authoritative DocsNavSectionId → Entropic volume number map (phase_4.2).
 * phase_4.4 must import this — do not invent an alternate mapping.
 */
import { DOCS_NAV_SECTIONS, type DocsNavSectionId } from "./docs-nav.ts";

/** Section id → volume index (storage under `philes/volume-<N>/`). */
export const DOCS_SECTION_TO_VOLUME: Readonly<Record<DocsNavSectionId, number>> = {
  "getting-started": 0,
  character: 1,
  locations: 2,
  combat: 3,
  items: 4,
  spells: 5,
  wizard: 6,
  reference: 7
};

/** Volume titles derived from `DOCS_NAV_SECTIONS` labels via section→volume map. */
export const DOCS_VOLUME_LABELS: Readonly<Record<number, string>> = Object.fromEntries(
  DOCS_NAV_SECTIONS.map((section) => [DOCS_SECTION_TO_VOLUME[section.id], section.label])
);

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
