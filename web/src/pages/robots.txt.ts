import type { APIRoute } from "astro";
import { requireSite, xmlHeaders } from "../modules/seo/http";
import { absoluteUrl } from "../modules/seo/xml";

export const GET: APIRoute = ({ site }) => {
  const siteUrl = requireSite(site, "Robots");

  return new Response(`User-agent: *\nAllow: /\nSitemap: ${absoluteUrl(siteUrl, "/sitemap.xml")}\n`, {
    headers: xmlHeaders("text/plain")
  });
};
