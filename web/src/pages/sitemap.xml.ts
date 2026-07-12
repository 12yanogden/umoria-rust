import type { APIRoute } from "astro";
import { getAllPhiles } from "../modules/philes/repository";
import { requireSite, xmlHeaders } from "../modules/seo/http";
import { renderSitemap, sitemapEntries } from "../modules/seo/xml";
import { getAllVolumes } from "../modules/volumes/repository";

export const GET: APIRoute = async ({ site }) => {
  const philes = await getAllPhiles();
  const volumes = await getAllVolumes(philes);

  return new Response(renderSitemap(requireSite(site, "Sitemap"), sitemapEntries(volumes, philes)), {
    headers: xmlHeaders("application/xml")
  });
};
