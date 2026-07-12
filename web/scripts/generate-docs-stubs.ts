/**
 * Regenerate docs stub philes from docs-catalog.ts (phase_4.2).
 * Catalog wins for frontmatter + outline on remaining outline stubs.
 * Authored articles (FULL_ARTICLE marker and/or no stub marker) are skipped.
 *
 * Usage (from web/): bun run docs:generate-stubs
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { docsCatalog, type DocsCatalogEntry } from "../src/content/docs-catalog.ts";
import {
  DOCS_STUB_MARKER,
  isAuthoredDocsBody
} from "../src/content/docs-stub-contract.ts";
import { stubRelPathForSlug } from "../src/content/docs-section-volume-map.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "..");

/** Stable stub date for generated content. */
const STUB_DATE = "2026-07-11";
const STUB_AUTHOR = "Umoria";

function yamlQuote(value: string): string {
  return JSON.stringify(value);
}

function renderSourcesYaml(sources: DocsCatalogEntry["sources"]): string {
  return sources
    .map((s) => `  - label: ${yamlQuote(s.label)}\n    href: ${yamlQuote(s.href)}`)
    .join("\n");
}

function renderBody(entry: DocsCatalogEntry): string {
  const lines: string[] = [
    "",
    DOCS_STUB_MARKER,
    "",
    "## Outline",
    "",
    `- ${entry.summary}`
  ];
  if (entry.dependsOnSlugs && entry.dependsOnSlugs.length > 0) {
    lines.push(`- Depends on: ${entry.dependsOnSlugs.join(", ")}`);
  }
  if (entry.relatedSlugs.length > 0) {
    lines.push(`- Related: ${entry.relatedSlugs.join(", ")}`);
  }
  lines.push("");
  return lines.join("\n");
}

function renderStub(entry: DocsCatalogEntry): string {
  const sourcesBlock = renderSourcesYaml(entry.sources);
  return `---
title: ${yamlQuote(entry.title)}
date: ${STUB_DATE}
author: ${yamlQuote(STUB_AUTHOR)}
order: ${entry.order}
slug: ${yamlQuote(entry.slug)}
sources:
${sourcesBlock}
---
${renderBody(entry)}`;
}

function main(): void {
  let written = 0;
  let skipped = 0;
  for (const entry of docsCatalog.articles) {
    const rel = stubRelPathForSlug(entry.slug, entry.section);
    const abs = join(webRoot, rel);
    if (existsSync(abs)) {
      const existing = readFileSync(abs, "utf8");
      if (isAuthoredDocsBody(existing)) {
        skipped += 1;
        continue;
      }
    }
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, renderStub(entry), "utf8");
    written += 1;
  }
  console.log(
    `Wrote ${written} docs stubs from catalog (${docsCatalog.articles.length} articles; skipped ${skipped} authored).`
  );
}

main();
