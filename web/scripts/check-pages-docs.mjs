/**
 * Phase 5.3 — maintainer GitHub Pages enablement documentation checks.
 * Asserts the chosen maintainer note documents Pages Source, workflow, build
 * root, production URL / base, and verification steps.
 *
 * Chosen path: web/README.md (lightest touch; already describes the Astro site).
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const __dirname = dirname(fileURLToPath(import.meta.url));
const webRoot = join(__dirname, "..");
const docsPath = join(webRoot, "README.md");

/** Same fixtures as phase_5.2 check-pages-alignment.mjs (do not import — that registers tests). */
const EXPECTED_PRODUCTION_URL = "https://12yanogden.github.io/umoria-rust/";
const EXPECTED_BASE = "/umoria-rust";

/** Required section heading for the Pages enablement note. */
export const PAGES_DOCS_HEADING = "## GitHub Pages";

function loadDocs() {
  assert.ok(existsSync(docsPath), `expected Pages enablement doc at ${docsPath}`);
  return readFileSync(docsPath, "utf8");
}

test("1. Doc file exists with required Pages heading", () => {
  assert.ok(existsSync(docsPath), `expected ${docsPath}`);
  const body = loadDocs();
  assert.ok(
    body.includes(PAGES_DOCS_HEADING),
    `expected heading ${JSON.stringify(PAGES_DOCS_HEADING)} in ${docsPath}`
  );
});

test("2. Source: GitHub Actions documented", () => {
  const body = loadDocs();
  assert.match(
    body,
    /Source:\s*GitHub Actions/i,
    "doc must document Pages Source: GitHub Actions"
  );
});

test("3. Workflow path documented", () => {
  const body = loadDocs();
  assert.match(body, /\.github\/workflows\//, "doc must mention .github/workflows/");
  assert.match(body, /deploy\.yml/, "doc must name the deploy workflow file");
  assert.match(body, /workflow_dispatch/, "doc must mention workflow_dispatch trigger");
  assert.match(body, /push/i, "doc must mention push trigger");
  assert.match(body, /\bmain\b/, "doc must mention main branch");
});

test("4. Build path documented", () => {
  const body = loadDocs();
  assert.match(body, /\bweb\//, "doc must name web/ as the Astro / build root");
  assert.match(body, /withastro\/action/, "doc must mention withastro/action");
});

test("5. Base / production URL documented", () => {
  const body = loadDocs();
  assert.ok(
    body.includes(EXPECTED_BASE),
    `doc must include base ${EXPECTED_BASE}`
  );
  assert.ok(
    body.includes("github.io/umoria-rust") || body.includes(EXPECTED_PRODUCTION_URL),
    `doc must include production URL fragment or ${EXPECTED_PRODUCTION_URL}`
  );
});

test("6. Verification steps present", () => {
  const body = loadDocs();
  assert.match(body, /workflow/i, "verification must mention workflow");
  assert.match(body, /\bdeploy\b/i, "verification must mention deploy job");
  assert.ok(
    body.includes(EXPECTED_PRODUCTION_URL) || body.includes("github.io/umoria-rust"),
    "verification must reference the Pages URL"
  );
});
