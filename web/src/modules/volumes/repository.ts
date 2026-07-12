import { withBase } from "../paths";
import type { Phile } from "../philes/model";
import { getAllPhiles, getPhilesByVolume } from "../philes/repository";
import type { Volume } from "./model";

export async function getAllVolumes(philes?: Phile[]): Promise<Volume[]> {
  const allPhiles = philes ?? (await getAllPhiles());
  const philesByVolume = new Map<number, Phile[]>();

  for (const phile of allPhiles) {
    const volumePhiles = philesByVolume.get(phile.route.volume);

    if (volumePhiles) {
      volumePhiles.push(phile);
    } else {
      philesByVolume.set(phile.route.volume, [phile]);
    }
  }

  return [...philesByVolume.entries()]
    .sort(([left], [right]) => compareVolumes(left, right))
    .map(([number, volumePhiles]) => ({
      number,
      href: withBase(`/volume/${number}/`),
      philes: volumePhiles
    }));
}

function compareVolumes(left: number, right: number): number {
  return left - right;
}

export async function getVolume(number: number): Promise<Volume | undefined> {
  const philes = await getPhilesByVolume(number);

  if (philes.length === 0) {
    return undefined;
  }

  return {
    number,
    href: withBase(`/volume/${number}/`),
    philes
  };
}
