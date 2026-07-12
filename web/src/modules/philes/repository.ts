import { getCollection } from "astro:content";
import { type VolumePhileSort, volumeConfig } from "../../config";
import type { Phile } from "./model";
import { routeForPhile } from "./routing";

let productionPhileCache: Phile[] | undefined;

export async function getAllPhiles(): Promise<Phile[]> {
  if (import.meta.env.PROD && productionPhileCache) {
    return productionPhileCache;
  }

  const entries = await getCollection("philes");
  const philes = entries.map((entry) => ({
    ...entry,
    route: routeForPhile(entry)
  }));

  assertUniqueSlugs(philes);

  const sorted = philes.sort(comparePhiles);

  if (import.meta.env.PROD) {
    productionPhileCache = sorted;
  }

  return sorted;
}

export async function getPhilesByVolume(volume: number): Promise<Phile[]> {
  return (await getAllPhiles()).filter((phile) => phile.route.volume === volume);
}

export async function getPhileByRoute(volume: number, slug: string): Promise<Phile | undefined> {
  return (await getAllPhiles()).find((phile) => phile.route.volume === volume && phile.route.slug === slug);
}

/** Look up a phile by catalog / public docs slug (may be multi-segment). */
export async function getPhileByCatalogSlug(catalogSlug: string): Promise<Phile | undefined> {
  const philes = await getAllPhiles();
  return philes.find((phile) => phile.data.slug === catalogSlug || phile.route.slug === catalogSlug);
}

function comparePhiles(left: Phile, right: Phile): number {
  if (left.route.volume !== right.route.volume) {
    return left.route.volume - right.route.volume;
  }

  const config = volumeConfig(left.route.volume);
  const sort = config.phileSort ?? { by: "date", direction: "desc" };
  const sorted = compareByVolumeSort(left, right, sort);

  if (sorted !== 0) {
    return sorted;
  }

  return left.route.slug.localeCompare(right.route.slug);
}

function compareByVolumeSort(left: Phile, right: Phile, sort: VolumePhileSort): number {
  if (sort.by === "date") {
    return compareDates(left, right, sort.direction);
  }

  const leftHasOrder = left.data.order !== undefined;
  const rightHasOrder = right.data.order !== undefined;

  const leftOrder = left.data.order;
  const rightOrder = right.data.order;

  if (leftOrder !== undefined && rightOrder !== undefined && leftOrder !== rightOrder) {
    return compareNumbers(leftOrder, rightOrder, sort.direction);
  }

  if (leftHasOrder !== rightHasOrder) {
    return leftHasOrder ? -1 : 1;
  }

  return compareDates(left, right, "desc");
}

function compareDates(left: Phile, right: Phile, direction: VolumePhileSort["direction"]): number {
  return compareNumbers(left.data.date.getTime(), right.data.date.getTime(), direction);
}

function compareNumbers(left: number, right: number, direction: VolumePhileSort["direction"]): number {
  return direction === "asc" ? left - right : right - left;
}

function assertUniqueSlugs(philes: Phile[]): void {
  const seen = new Map<string, string>();

  for (const phile of philes) {
    const key = `${phile.route.volume}/${phile.route.slug}`;
    const existing = seen.get(key);

    if (existing) {
      throw new Error(`Duplicate phile route "${key}" in "${existing}" and "${phile.route.sourcePath}".`);
    }

    seen.set(key, phile.route.sourcePath);
  }
}
