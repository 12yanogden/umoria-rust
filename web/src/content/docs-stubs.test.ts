/**
 * Phase 4.2 — stub completeness, sources frontmatter, outline, volume map.
 * Run: bun run test  (from web/)
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";

import { docsCatalog } from "./docs-catalog.ts";
import { DOCS_NAV_SECTIONS, type DocsNavSectionId } from "./docs-nav.ts";
import { DOCS_SECTION_TO_VOLUME, stubRelPathForSlug } from "./docs-section-volume-map.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "../..");
const philesRoot = join(webRoot, "src/content/philes");
const distRoot = join(webRoot, "dist");
const BASE_SEGMENT = "umoria-rust";

/** Non-catalog scaffold fixtures (underscore prefix) are ignored as orphans. */
function isFixturePhile(relPosix: string): boolean {
  const base = relPosix.split("/").at(-1) ?? "";
  return base.startsWith("_");
}

function listPhileFiles(dir: string, base = dir): string[] {
  if (!existsSync(dir)) return [];
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      out.push(...listPhileFiles(full, base));
    } else if (name.endsWith(".phile")) {
      out.push(relative(base, full).split("\\").join("/"));
    }
  }
  return out;
}

function parsePhileFile(absPath: string): { data: Record<string, unknown>; body: string } {
  const source = readFileSync(absPath, "utf8").replace(/\r\n/g, "\n");
  assert.ok(source.startsWith("---\n"), `expected YAML frontmatter in ${absPath}`);
  const end = source.indexOf("\n---\n", 4);
  assert.ok(end !== -1, `missing closing frontmatter delimiter in ${absPath}`);
  const data = parseYaml(source.slice(4, end)) as Record<string, unknown>;
  const body = source.slice(end + 5);
  return { data, body };
}

function findDistHtml(relCandidates: string[]): string | null {
  for (const rel of relCandidates) {
    const full = join(distRoot, rel);
    if (existsSync(full)) return full;
  }
  return null;
}

describe("docs stubs (phase_4.2)", () => {
  it("0. DOCS_SECTION_TO_VOLUME maps all nav sections to volume 0–7", () => {
    const expected: Record<DocsNavSectionId, number> = {
      "getting-started": 0,
      character: 1,
      dungeon: 2,
      combat: 3,
      items: 4,
      spells: 5,
      wizard: 6,
      reference: 7
    };
    for (const section of DOCS_NAV_SECTIONS) {
      assert.equal(
        DOCS_SECTION_TO_VOLUME[section.id],
        expected[section.id],
        `section ${section.id} volume`
      );
    }
    assert.equal(Object.keys(DOCS_SECTION_TO_VOLUME).length, 8);
  });

  it("1. stub exists for every catalog slug; no orphan catalog stubs", () => {
    assert.ok(docsCatalog.articles.length >= 1, "catalog must have ≥1 article");
    const catalogSlugs = new Set(docsCatalog.articles.map((a) => a.slug));

    for (const entry of docsCatalog.articles) {
      const rel = stubRelPathForSlug(entry.slug, entry.section);
      const abs = join(webRoot, rel);
      assert.ok(existsSync(abs), `missing stub for ${entry.slug}: expected ${rel}`);
    }

    const onDisk = listPhileFiles(philesRoot);
    for (const rel of onDisk) {
      if (isFixturePhile(rel)) continue;
      // volume-N/<catalog-slug>.phile → strip volume-N/ and .phile
      const match = rel.match(/^volume-\d+\/(.+)\.phile$/);
      assert.ok(match, `unexpected phile path shape: ${rel}`);
      const slug = match[1];
      assert.ok(
        catalogSlugs.has(slug),
        `orphan stub (not in catalog): ${rel} → slug "${slug}"`
      );
    }
  });

  it("2. every stub has sources frontmatter matching catalog hrefs", () => {
    for (const entry of docsCatalog.articles) {
      const abs = join(webRoot, stubRelPathForSlug(entry.slug, entry.section));
      const { data } = parsePhileFile(abs);
      const sources = data.sources;
      assert.ok(Array.isArray(sources) && sources.length >= 1, `${entry.slug}: sources required`);
      assert.ok(entry.sources.length >= 1, `${entry.slug}: catalog sources required`);

      const stubHrefs = new Set(
        (sources as { label?: string; href?: string }[]).map((s) => s.href).filter(Boolean)
      );
      for (const src of entry.sources) {
        assert.ok(
          stubHrefs.has(src.href),
          `${entry.slug}: stub sources missing catalog href ${src.href}`
        );
      }
      for (const s of sources as { label?: string; href?: string }[]) {
        assert.equal(typeof s.label, "string");
        assert.ok((s.label as string).length >= 1, `${entry.slug}: empty source label`);
        assert.equal(typeof s.href, "string");
        assert.ok((s.href as string).length >= 1, `${entry.slug}: empty source href`);
      }
    }
  });

  it("3. title matches catalog; body has outline bullets; stub-scale", () => {
    const MAX_BODY_CHARS = 4000;
    for (const entry of docsCatalog.articles) {
      const abs = join(webRoot, stubRelPathForSlug(entry.slug, entry.section));
      const { data, body } = parsePhileFile(abs);
      assert.equal(
        String(data.title).trim(),
        entry.title.trim(),
        `${entry.slug}: title mismatch`
      );
      assert.match(body, /^[\s\S]*^[\t ]*[-*] /m, `${entry.slug}: expected outline bullet list`);
      assert.ok(
        body.includes(entry.summary) || body.toLowerCase().includes("outline"),
        `${entry.slug}: outline should reflect summary or mark outline`
      );
      assert.ok(
        body.length <= MAX_BODY_CHARS,
        `${entry.slug}: body too long for stub (${body.length} > ${MAX_BODY_CHARS})`
      );
      assert.doesNotMatch(body, /FULL_ARTICLE/, `${entry.slug}: must not mark full article`);
    }
  });

  it("4. required phase_1 frontmatter: title, date, author", () => {
    for (const entry of docsCatalog.articles) {
      const abs = join(webRoot, stubRelPathForSlug(entry.slug, entry.section));
      const { data } = parsePhileFile(abs);
      assert.ok(data.title, `${entry.slug}: title`);
      assert.ok(data.date, `${entry.slug}: date`);
      assert.equal(data.author, "Umoria", `${entry.slug}: default author Umoria`);
    }
  });

  it("5. generator script + package.json docs:generate-stubs exist", () => {
    const pkg = JSON.parse(readFileSync(join(webRoot, "package.json"), "utf8")) as {
      scripts?: Record<string, string>;
    };
    assert.ok(pkg.scripts?.["docs:generate-stubs"], "package.json must define docs:generate-stubs");
    const scriptHint = pkg.scripts["docs:generate-stubs"];
    assert.match(scriptHint, /generate-docs-stubs/, `unexpected script: ${scriptHint}`);
    const candidates = [
      join(webRoot, "scripts/generate-docs-stubs.ts"),
      join(webRoot, "scripts/generate-docs-stubs.mjs")
    ];
    assert.ok(
      candidates.some((p) => existsSync(p)),
      "scripts/generate-docs-stubs.ts or .mjs must exist"
    );
  });

  it("7. public /docs/<slug>/ routes exist in dist after build", () => {
    assert.ok(existsSync(distRoot), "dist/ missing — run bun run build first");
    for (const entry of docsCatalog.articles) {
      const slugPath = entry.slug; // may contain /
      const page = findDistHtml([
        join("docs", slugPath, "index.html"),
        join(BASE_SEGMENT, "docs", slugPath, "index.html")
      ]);
      assert.ok(page, `missing dist page for /docs/${slugPath}/`);
    }
  });
});
