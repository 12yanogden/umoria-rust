import { withBase } from "../paths";
import type { PhileEntry, PhileRoute } from "./model";

const volumePathPattern = /^volume-(\d+)\/(.+?)(?:\.phile)?$/;

export function routeForPhile(entry: PhileEntry): PhileRoute {
  const match = entry.id.match(volumePathPattern);

  if (!match) {
    throw new Error(`Invalid phile path "${entry.id}". Expected content/philes/volume-<number>/**/*.phile.`);
  }

  const volume = Number(match[1]);
  const pathWithoutVolume = match[2];
  // Prefer explicit frontmatter slug (catalog slugs may be multi-segment).
  // Otherwise use the path under volume-<N>/ (minus .phile already stripped by pattern).
  const slug = entry.data.slug ?? pathWithoutVolume.toLowerCase();

  if (!slug) {
    throw new Error(`Unable to derive slug for "${entry.id}".`);
  }

  return {
    volume,
    slug,
    href: withBase(`/volume/${volume}/${slug}/`),
    volumeHref: withBase(`/volume/${volume}/`),
    sourcePath: entry.id
  };
}
