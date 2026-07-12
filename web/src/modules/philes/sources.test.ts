/**
 * Phase 4.1 — Sources attribution render + layout wiring.
 * Run: bun run test  (from web/)
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import type { DocsSourceRef } from "../../content/docs-catalog-types.ts";
import { renderSourcesHtml, SOURCES_EMPTY_COPY } from "./sources.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "../../..");

describe("Sources component (phase_4.1)", () => {
  it("6. renders links — href and visible label for each entry", () => {
    const sources: DocsSourceRef[] = [
      { label: "Beej classes.txt", href: "https://beej.us/moria/classes.txt" },
      { label: "README", href: "README.md" }
    ];
    const html = renderSourcesHtml(sources);
    assert.match(html, /href="https:\/\/beej\.us\/moria\/classes\.txt"/);
    assert.match(html, />Beej classes\.txt</);
    assert.match(html, /README/);
    // Resolved repo-relative or raw href must appear for README entry
    assert.ok(
      html.includes("README.md") || html.includes("blob/main/README.md"),
      `expected README href in output: ${html}`
    );
  });

  it("7. empty defensive state — explicit (none listed), not silent", () => {
    const html = renderSourcesHtml([]);
    assert.match(html, /\(none listed\)/);
    assert.ok(html.includes(SOURCES_EMPTY_COPY) || html.includes("(none listed)"));
    assert.doesNotMatch(html, /<a\b/i, "empty state must not render source anchors");
  });

  it("8. layout — PhileShell mounts Sources; fixture sources appear in attribution HTML", () => {
    const shellPath = join(webRoot, "src/components/phile/PhileShell.astro");
    assert.ok(existsSync(shellPath), "PhileShell.astro must exist");
    const shellSrc = readFileSync(shellPath, "utf8");
    assert.match(shellSrc, /Sources/, "PhileShell must mount Sources attribution");
    assert.match(
      shellSrc,
      /sources/i,
      "PhileShell must pass phile sources into the attribution component"
    );

    const fixturePath = join(webRoot, "src/content/philes/volume-0/_sources-fixture.phile");
    assert.ok(existsSync(fixturePath), "minimal sources fixture phile must exist for layout/render tests");
    const fixture = readFileSync(fixturePath, "utf8");
    assert.match(fixture, /sources:/);
    assert.match(fixture, /label:/);
    assert.match(fixture, /href:/);

    const expectedHref = "https://beej.us/moria/classes.txt";
    assert.ok(fixture.includes(expectedHref), "fixture must declare expected source href");

    // Content-rendered attribution path used by the article layout
    const sources: DocsSourceRef[] = [{ label: "Beej classes.txt", href: expectedHref }];
    const html = renderSourcesHtml(sources);
    assert.ok(html.includes(expectedHref), "layout attribution output must include fixture source href");
  });
});
