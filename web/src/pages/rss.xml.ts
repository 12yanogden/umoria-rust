import type { APIRoute } from "astro";
import { siteConfig } from "../config";
import { getAllPhiles } from "../modules/philes/repository";
import { requireSite, xmlHeaders } from "../modules/seo/http";
import { renderRss } from "../modules/seo/xml";

export const GET: APIRoute = async ({ site }) => {
  const philes = await getAllPhiles();

  return new Response(
    renderRss({
      site: requireSite(site, "RSS"),
      title: siteConfig.name,
      description: siteConfig.description,
      philes
    }),
    {
      headers: xmlHeaders("application/xml")
    }
  );
};
