import { z } from "astro/zod";
import type { DocsSourceRef } from "../../content/docs-catalog-types.ts";

export const requiredPhileFields = ["title", "date", "author"] as const;

/** Zod mirror of phase_3 `DocsSourceRef` (`label` + `href`). */
export const docsSourceRefSchema: z.ZodType<DocsSourceRef> = z.object({
  label: z.string().min(1),
  href: z.string().min(1)
});

export const phileSchema = z.object({
  title: z.string().min(1),
  date: z.date(),
  author: z.string().min(1),
  lang: z.enum(["en", "zh"]).default("en"),
  slug: z.string().optional(),
  order: z.number().int().nonnegative().optional(),
  redacted: z.boolean().default(false),
  /** Required non-empty; same shape as catalog `DocsSourceRef[]`. */
  sources: z.array(docsSourceRefSchema).min(1, { message: "sources must contain at least one entry" })
});
