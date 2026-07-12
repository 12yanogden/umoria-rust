import { volumeConfig } from "../../config";
import type { Volume } from "./model";

export function volumeTitle(volume: Volume): string {
  return volumeConfig(volume.number).title;
}

export function volumeListLabel(volume: Volume): string {
  return `${volumeConfig(volume.number).listLabel} (${volume.philes.length})`;
}
