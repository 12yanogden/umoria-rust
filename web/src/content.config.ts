import { defineCollection } from "astro:content";
import { phileLoader } from "./modules/philes/loader";
import { phileSchema } from "./modules/philes/schema";

const philes = defineCollection({
  loader: phileLoader(),
  schema: phileSchema
});

export const collections = { philes };
