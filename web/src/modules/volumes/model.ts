import type { Phile } from "../philes/model";

export type Volume = {
  number: number;
  href: string;
  philes: Phile[];
};
