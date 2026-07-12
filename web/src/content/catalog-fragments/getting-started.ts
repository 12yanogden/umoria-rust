/**
 * Repo-specific getting-started docs catalog fragment (phase_3.3).
 *
 * Merged by phase_3.4 into docs-catalog.ts.
 */

import type { DocsCatalogEntry } from "../docs-catalog-types.ts";

export const gettingStartedCatalog: DocsCatalogEntry[] = [
  {
    slug: "index",
    title: "Documentation",
    summary: "Overview of site docs and entry points into each nav section.",
    section: "getting-started",
    order: 0,
    sources: [
      { label: "Site IA", href: ".cursor/plans/umoria-web-site/phase_3.md" },
      { label: "README · project intro", href: "README.md" }
    ],
    relatedSlugs: [
      "getting-started/install",
      "getting-started/playing",
      "getting-started/differences",
      "getting-started/contributing",
      "character/classes",
      "dungeon/city",
      "reference/sources"
    ]
  },
  {
    slug: "getting-started/install",
    title: "Install & build",
    summary: "Build Umoria 5.7.15 from source on macOS/Linux with Rust, ncurses, and pkg-config.",
    section: "getting-started",
    order: 1,
    sources: [
      { label: "README · Platforms", href: "README.md#platforms" },
      { label: "README · Building", href: "README.md#building" },
      { label: "MSRV", href: "rust-toolchain.toml" }
    ],
    relatedSlugs: ["getting-started/playing", "getting-started/differences", "reference/versions"]
  },
  {
    slug: "getting-started/playing",
    title: "Playing",
    summary: "Run the game from the repository root, CLI flags, saves, and where to go next in the docs.",
    section: "getting-started",
    order: 2,
    sources: [
      { label: "README · Playing", href: "README.md#playing" },
      { label: "README · Testing and goldens", href: "README.md#testing-and-goldens" }
    ],
    relatedSlugs: [
      "character/classes",
      "character/races",
      "character/attributes",
      "dungeon/city",
      "combat/monsters",
      "wizard/overview"
    ],
    dependsOnSlugs: ["getting-started/install"]
  },
  {
    slug: "getting-started/differences",
    title: "This port vs classic",
    summary: "How this Rust 5.7.15 ncurses port relates to upstream C Umoria and what changed in the lineage.",
    section: "getting-started",
    order: 3,
    sources: [
      { label: "README · project lineage", href: "README.md" },
      { label: "CHANGELOG · HEAD", href: "CHANGELOG.md" },
      { label: "CHANGELOG · 5.7.15", href: "CHANGELOG.md#5715-2021-06-02" },
      { label: "Historical documents", href: "historical/" }
    ],
    relatedSlugs: ["reference/versions", "reference/sources", "getting-started/install", "character/races"],
    dependsOnSlugs: ["getting-started/install"]
  },
  {
    slug: "getting-started/contributing",
    title: "Contributing",
    summary: "Report bugs, run local checks, and open PRs without changing gameplay rules.",
    section: "getting-started",
    order: 4,
    sources: [
      { label: "CONTRIBUTING", href: "CONTRIBUTING.md" },
      { label: "Code of Conduct", href: "CODE_OF_CONDUCT.md" },
      { label: "Golden capture", href: "tools/capture/README.md" },
      {
        label: "README · contributions pointer",
        href: "README.md#code-of-conduct-and-contributions"
      }
    ],
    relatedSlugs: ["getting-started/install", "getting-started/differences"],
    dependsOnSlugs: ["getting-started/install"]
  }
];
