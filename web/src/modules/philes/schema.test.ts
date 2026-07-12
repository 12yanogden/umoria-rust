/**
 * Phase 4.1 — phile frontmatter `sources` schema (DocsSourceRef-compatible).
 * Run: bun run test  (from web/)
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import type { DocsSourceRef } from "../../content/docs-catalog-types.ts";
import { phileSchema } from "./schema.ts";

const baseFields = {
  title: "Test article",
  date: new Date("2026-01-01"),
  author: "Umoria"
};

describe("phileSchema sources (phase_4.1)", () => {
  it("1. valid sources — non-empty DocsSourceRef[] parses", () => {
    const result = phileSchema.safeParse({
      ...baseFields,
      sources: [{ label: "Beej classes.txt", href: "https://beej.us/moria/classes.txt" }]
    });
    assert.equal(result.success, true, `expected success, got ${JSON.stringify(result.error?.issues)}`);
    if (result.success) {
      assert.equal(result.data.sources.length, 1);
      assert.equal(result.data.sources[0]?.label, "Beej classes.txt");
      assert.equal(result.data.sources[0]?.href, "https://beej.us/moria/classes.txt");
    }
  });

  it("2. mirrors DocsSourceRef — sample catalog shape validates without renaming", () => {
    const fromCatalog: DocsSourceRef = {
      label: "README · Building",
      href: "README.md#building"
    };
    const result = phileSchema.safeParse({
      ...baseFields,
      sources: [fromCatalog]
    });
    assert.equal(result.success, true, `expected DocsSourceRef shape to validate: ${JSON.stringify(result.error?.issues)}`);
    if (result.success) {
      const src = result.data.sources[0];
      assert.ok(src && "label" in src && "href" in src);
      assert.equal(Object.keys(src).sort().join(","), "href,label");
    }
  });

  it("3. missing sources fails — error mentions sources", () => {
    const result = phileSchema.safeParse({ ...baseFields });
    assert.equal(result.success, false, "omitted sources must fail");
    const msg = JSON.stringify(result.error?.issues ?? []);
    assert.match(msg, /sources/i, `error should mention sources, got: ${msg}`);
  });

  it("4. empty sources fails — [] not allowed for docs philes", () => {
    const result = phileSchema.safeParse({
      ...baseFields,
      sources: []
    });
    assert.equal(result.success, false, "sources: [] must fail");
    const msg = JSON.stringify(result.error?.issues ?? []);
    assert.match(msg, /sources/i, `error should mention sources, got: ${msg}`);
  });

  it("5. invalid entry fails — missing label/href or non-string", () => {
    const missingLabel = phileSchema.safeParse({
      ...baseFields,
      sources: [{ href: "https://example.com" }]
    });
    assert.equal(missingLabel.success, false, "missing label must fail");

    const missingHref = phileSchema.safeParse({
      ...baseFields,
      sources: [{ label: "Example" }]
    });
    assert.equal(missingHref.success, false, "missing href must fail");

    const badTypes = phileSchema.safeParse({
      ...baseFields,
      sources: [{ label: 1, href: true }]
    });
    assert.equal(badTypes.success, false, "non-string label/href must fail");
  });
});
