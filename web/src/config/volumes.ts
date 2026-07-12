import { DOCS_VOLUME_LABELS } from "../content/docs-section-volume-map";

export type VolumePhileSort = {
  by: "date" | "order";
  direction: "asc" | "desc";
};

export type VolumeConfig = {
  title: string;
  subtitle?: string;
  listLabel: string;
  postscript?: string[];
  entryPrefix?: string;
  entryLabel?: "index" | "year";
  reverseEntryNumbers?: boolean;
  phileSort?: VolumePhileSort;
};

export const defaultVolumeConfig = (number: number): VolumeConfig => ({
  title: `Docs Volume ${number}`,
  listLabel: `Volume ${number}`,
  phileSort: {
    by: "date",
    direction: "desc"
  },
  postscript: ["  ──[ EOF ]──────────────────────────────────────────────────────────────────//───"]
});

/**
 * Re-export section→volume map for phase_4.4 (authoritative assignment in
 * `docs-section-volume-map.ts`).
 */
export { DOCS_SECTION_TO_VOLUME, DOCS_VOLUME_LABELS } from "../content/docs-section-volume-map";

const eof = ["  ──[ EOF ]──────────────────────────────────────────────────────────────────//───"];

function sectionVolumeConfig(volume: number): VolumeConfig {
  const title = DOCS_VOLUME_LABELS[volume] ?? `Docs Volume ${volume}`;
  return {
    title,
    listLabel: title,
    phileSort: {
      by: "order",
      direction: "asc"
    },
    postscript: eof
  };
}

/** Volume labels match catalog section labels (phase_4 section→volume map). */
export const volumeConfigs = new Map<number, VolumeConfig>([
  [0, sectionVolumeConfig(0)],
  [1, sectionVolumeConfig(1)],
  [2, sectionVolumeConfig(2)],
  [3, sectionVolumeConfig(3)],
  [4, sectionVolumeConfig(4)],
  [5, sectionVolumeConfig(5)],
  [6, sectionVolumeConfig(6)],
  [7, sectionVolumeConfig(7)]
]);

export function volumeConfig(number: number): VolumeConfig {
  return volumeConfigs.get(number) ?? defaultVolumeConfig(number);
}
