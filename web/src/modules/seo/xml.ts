import { withBase } from "../paths";
import type { Phile } from "../philes/model";
import type { Volume } from "../volumes/model";

export type SitemapEntry = {
  href: string;
};

export function absoluteUrl(site: URL, href: string): string {
  const base = import.meta.env.BASE_URL.replace(/\/?$/, "/");
  const path = href.startsWith(base) || href === base.slice(0, -1) || !href.startsWith("/") ? href : withBase(href);
  return new URL(path, site).toString();
}

export function phileExcerpt(phile: Phile, maxLength = 240): string {
  if (phile.data.redacted) {
    return "[REDACTED]";
  }

  const text = plainText(phile.body ?? "");

  if (text.length <= maxLength) {
    return text;
  }

  return `${text.slice(0, maxLength - 1).trimEnd()}...`;
}

export function renderRss(options: { site: URL; title: string; description: string; philes: Phile[] }): string {
  const latestDate = options.philes.reduce<Date | undefined>(
    (latest, phile) => (!latest || phile.data.date > latest ? phile.data.date : latest),
    undefined
  );

  return xmlDocument(`<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>${escapeXml(options.title)}</title>
    <link>${escapeXml(absoluteUrl(options.site, "/"))}</link>
    <description>${escapeXml(options.description)}</description>
    <language>en</language>
    <lastBuildDate>${formatRssDate(latestDate)}</lastBuildDate>
    <atom:link href="${escapeXml(absoluteUrl(options.site, "/rss.xml"))}" rel="self" type="application/rss+xml" />
${rssPhiles(options.philes)
  .map((phile) => renderRssItem(options.site, phile))
  .join("\n")}
  </channel>
</rss>`);
}

export function renderSitemap(site: URL, entries: SitemapEntry[]): string {
  return xmlDocument(`<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${entries.map((entry) => renderSitemapEntry(site, entry)).join("\n")}
</urlset>`);
}

export function sitemapEntries(volumes: Volume[], philes: Phile[]): SitemapEntry[] {
  return [
    { href: "/" },
    { href: "/rss.xml" },
    ...volumes.map((volume) => ({
      href: volume.href
    })),
    ...philes.map((phile) => ({
      href: phile.route.href
    }))
  ];
}

function renderRssItem(site: URL, phile: Phile): string {
  const url = absoluteUrl(site, phile.route.href);

  return `    <item>
      <title>${escapeXml(phile.data.title)}</title>
      <link>${escapeXml(url)}</link>
      <guid isPermaLink="true">${escapeXml(url)}</guid>
      <pubDate>${formatRssDate(phile.data.date)}</pubDate>
      <author>${escapeXml(phile.data.author)}</author>
      <description>${escapeXml(phileExcerpt(phile))}</description>
    </item>`;
}

function rssPhiles(philes: Phile[]): Phile[] {
  return [...philes].sort((left, right) => right.data.date.getTime() - left.data.date.getTime());
}

function plainText(input: string): string {
  return input
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/<img\b[^>]*>/gi, " ")
    .replace(/!\[[^\]]*]\([^)]+\)/g, " ")
    .replace(/\[([^\]]+)]\([^)]+\)/g, "$1")
    .replace(/<[^>]+>/g, " ")
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/^>\s?/gm, "")
    .replace(/[*_~`]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function renderSitemapEntry(site: URL, entry: SitemapEntry): string {
  return `  <url>
    <loc>${escapeXml(absoluteUrl(site, entry.href))}</loc>
  </url>`;
}

function formatRssDate(date: Date | undefined): string {
  if (!date) {
    return "Thu, 01 Jan 1970 00:00:00 GMT";
  }

  return date.toUTCString();
}

function escapeXml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function xmlDocument(input: string): string {
  return `${input.trim()}\n`;
}
