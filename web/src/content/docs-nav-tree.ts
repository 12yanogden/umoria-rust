/**
 * Catalog → docs sidebar / volume nav tree (phase_4.4).
 * Consumes phase_3 catalog + phase_4.2 DOCS_SECTION_TO_VOLUME — do not redefine either.
 */
import { docsCatalog } from "./docs-catalog.ts";
import type { DocsNavSectionId } from "./docs-nav.ts";
import { DOCS_SECTION_TO_VOLUME } from "./docs-section-volume-map.ts";

export type DocsNavTreeArticle = {
  slug: string;
  title: string;
  /** Site-root path (no Astro base): `/docs/<slug>/` */
  href: string;
  order: number;
};

export type DocsNavTreeGroup = {
  sectionId: DocsNavSectionId;
  label: string;
  order: number;
  /** Entropic volume index from DOCS_SECTION_TO_VOLUME */
  volume: number;
  articles: DocsNavTreeArticle[];
};

/** Ordered nav tree: one group per DocsNavSection, articles by catalog order. */
export function getDocsNavTree(): DocsNavTreeGroup[] {
  const sections = [...docsCatalog.sections].sort((a, b) => a.order - b.order);

  return sections.map((section) => {
    const volume = DOCS_SECTION_TO_VOLUME[section.id];
    if (volume === undefined) {
      throw new Error(`DOCS_SECTION_TO_VOLUME missing section "${section.id}"`);
    }

    const articles = docsCatalog.articles
      .filter((a) => a.section === section.id)
      .sort((a, b) => a.order - b.order)
      .map((a) => ({
        slug: a.slug,
        title: a.title,
        href: `/docs/${a.slug}/`,
        order: a.order
      }));

    return {
      sectionId: section.id,
      label: section.label,
      order: section.order,
      volume,
      articles
    };
  });
}
