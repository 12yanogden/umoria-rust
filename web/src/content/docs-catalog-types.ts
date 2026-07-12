/**
 * Shared docs catalog schema types (owned by phase_3 / materialized in phase_3.4).
 * Fragments import from here to avoid a cycle with docs-catalog.ts.
 * Public re-exports also live on docs-catalog.ts for phase_4 consumers.
 */

import type { DocsNavSectionId } from "./docs-nav.ts";

export type { DocsNavSectionId };

/** Short label + absolute URL or repo-relative path. */
export interface DocsSourceRef {
  label: string;
  href: string;
}

export interface DocsCatalogEntry {
  /** URL slug under /docs/, e.g. "character/races" */
  slug: string;
  title: string;
  /** One-line purpose for stub/outline agents — not full prose */
  summary: string;
  section: DocsNavSectionId;
  /** Order within section (ascending) */
  order: number;
  sources: DocsSourceRef[];
  /** Other catalog slugs this article should link to */
  relatedSlugs: string[];
  /** Optional: articles that should exist before this one in reading order */
  dependsOnSlugs?: string[];
}
