/**
 * Shared stub ↔ authored-article contract (see CONTENT_CONVENTIONS.md).
 * Used by docs-stubs tests and scripts/generate-docs-stubs.ts.
 */

/** Generated outline-only stub body marker. */
export const DOCS_STUB_MARKER = "<!-- docs-stub: outline only; full prose out of scope -->";

/** Authored full-article marker; generator must never overwrite these files. */
export const DOCS_FULL_ARTICLE_MARKER = "<!-- FULL_ARTICLE -->";

/**
 * True when a catalog phile body is authored / populated and must be skipped
 * by `docs:generate-stubs` and exempted from outline stub-scale checks.
 *
 * Authored if:
 * - body contains `<!-- FULL_ARTICLE -->`, or
 * - body lacks the docs-stub outline marker (populated page).
 */
export function isAuthoredDocsBody(body: string): boolean {
  if (body.includes(DOCS_FULL_ARTICLE_MARKER)) return true;
  return !body.includes(DOCS_STUB_MARKER);
}

/** True when the body is still a regeneratable outline stub. */
export function isOutlineStubBody(body: string): boolean {
  return !isAuthoredDocsBody(body);
}
