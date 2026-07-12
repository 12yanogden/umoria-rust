import type { DocsSourceRef } from "../../content/docs-catalog-types.ts";
import { withBase } from "../paths.ts";

/** Defensive empty-state copy when Sources is invoked with `[]`. */
export const SOURCES_EMPTY_COPY = "Sources: (none listed)";

/** Same GitHub blob base as SiteFooter — for catalog repo-relative paths. */
const REPO_BLOB_BASE = "https://github.com/12yanogden/umoria-rust/blob/main";

/**
 * Resolve a source href for rendering:
 * - absolute `http(s):` URLs → as-is
 * - site-root paths starting with `/` → `withBase` (phase_1)
 * - otherwise treat as repo-relative → GitHub blob URL (same scheme as SiteFooter)
 */
export function resolveSourceHref(href: string): string {
  if (/^https?:\/\//i.test(href)) {
    return href;
  }
  if (href.startsWith("/")) {
    return withBase(href);
  }
  const clean = href.replace(/^\.\//, "");
  return `${REPO_BLOB_BASE}/${clean}`;
}

function escapeHtml(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

/** HTML for the Sources attribution block (used by Sources.astro / layout). */
export function renderSourcesHtml(sources: DocsSourceRef[]): string {
  if (sources.length === 0) {
    return `<section class="phile-sources" data-phile-sources data-empty="true"><p class="phile-sources-empty">${SOURCES_EMPTY_COPY}</p></section>`;
  }

  const items = sources
    .map((src) => {
      const href = escapeHtml(resolveSourceHref(src.href));
      const label = escapeHtml(src.label);
      return `<li><a href="${href}">${label}</a></li>`;
    })
    .join("");

  return `<section class="phile-sources" data-phile-sources><p class="phile-sources-label">Sources</p><ul class="phile-sources-list">${items}</ul></section>`;
}
