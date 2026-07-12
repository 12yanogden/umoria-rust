/**
 * Docs navigation taxonomy (phase_3.1).
 *
 * Slug grammar:
 * - Docs home: `index` only (maps conceptually to `/docs/`).
 * - Articles: `{sectionId}/{topic}` — exactly two kebab-case segments.
 * - Sidebar / volume groups: one per `DocsNavSection`, ordered 1–8.
 *
 * Keep types in sync with docs-catalog.ts (phase_3.4 owns the merged catalog).
 */

/** Nav section id used by docs sidebar / volume index. */
export type DocsNavSectionId =
  | "getting-started"
  | "character"
  | "dungeon"
  | "combat"
  | "items"
  | "spells"
  | "wizard"
  | "reference";

/** Sidebar / volume section row. Keep in sync with docs-catalog.ts. */
export interface DocsNavSection {
  id: DocsNavSectionId;
  label: string;
  /** Sidebar / volume order */
  order: number;
  description: string;
}

/** Sole article slug allowed without a `section/topic` prefix. */
export const DOCS_HOME_SLUG = "index" as const;

export const DOCS_NAV_SECTION_IDS: readonly DocsNavSectionId[] = [
  "getting-started",
  "character",
  "dungeon",
  "combat",
  "items",
  "spells",
  "wizard",
  "reference"
] as const;

export const DOCS_NAV_SECTIONS: DocsNavSection[] = [
  {
    id: "getting-started",
    order: 1,
    label: "Getting Started",
    description: "Install, build, play, and orient yourself to this Rust port."
  },
  {
    id: "character",
    order: 2,
    label: "Character",
    description: "Races, classes, attributes, experience, and social class."
  },
  {
    id: "dungeon",
    order: 3,
    label: "Dungeon",
    description: "The city, stores, haggling, the underground, and traps."
  },
  {
    id: "combat",
    order: 4,
    label: "Combat",
    description: "Monsters, attacks, hit probability, damage, bashing, and armor class."
  },
  {
    id: "items",
    order: 5,
    label: "Items",
    description: "Weapons, armor, consumables, artifacts, and item tables."
  },
  {
    id: "spells",
    order: 6,
    label: "Spells",
    description: "Spell system, mana, failure, and mage/priest spell lists."
  },
  {
    id: "wizard",
    order: 7,
    label: "Wizard Mode",
    description: "Entering wizard mode, commands, and wizard items."
  },
  {
    id: "reference",
    order: 8,
    label: "Reference",
    description: "Sources, attribution, version history, and external links."
  }
];

const SECTION_ID_SET = new Set<string>(DOCS_NAV_SECTION_IDS);

/** Kebab-case path segment: lowercase letters, digits, hyphen-separated. */
const KEBAB_SEGMENT = /^[a-z0-9]+(-[a-z0-9]+)*$/;

/** Predicate: docs home slug only (`index`). */
export function isDocsHomeSlug(slug: string): boolean {
  return slug === DOCS_HOME_SLUG;
}

/**
 * Valid section-prefixed article slug: `{sectionId}/{topic}`.
 * Rejects bare section ids, home slug, deeper paths, and non-kebab segments.
 */
export function isSectionArticleSlug(slug: string): boolean {
  if (!slug || slug.includes("//") || slug.startsWith("/") || slug.endsWith("/")) {
    return false;
  }
  const parts = slug.split("/");
  if (parts.length !== 2) {
    return false;
  }
  const [section, topic] = parts;
  if (!SECTION_ID_SET.has(section)) {
    return false;
  }
  if (!KEBAB_SEGMENT.test(topic)) {
    return false;
  }
  return true;
}

/** Returns section id from a valid article slug, or null for `index` / invalid. */
export function sectionIdFromSlug(slug: string): DocsNavSectionId | null {
  if (isDocsHomeSlug(slug) || !isSectionArticleSlug(slug)) {
    return null;
  }
  return slug.split("/")[0] as DocsNavSectionId;
}
