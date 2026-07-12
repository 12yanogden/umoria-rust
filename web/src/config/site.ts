export type PrimaryNavLink = {
  label: string;
  /** Site-root path (no Astro `base`); hrefs are built with `withBase`. */
  path: string;
};

export type HomeHeroCopy = {
  brand: string;
  headline: string;
  support: string;
  downloadsLabel: string;
  docsLabel: string;
};

export type SiteConfig = {
  name: string;
  description: string;
  homeAsciiArt: string;
  homeHero: HomeHeroCopy;
  primaryNav: PrimaryNavLink[];
};

export const primaryNav: PrimaryNavLink[] = [
  { label: "Home", path: "/" },
  { label: "Downloads", path: "/downloads/" },
  { label: "Docs", path: "/docs/" }
];

export const siteConfig: SiteConfig = {
  name: "Umoria",
  description: "The Dungeons of Moria — Rust port documentation and site.",
  homeAsciiArt: `██╗   ██╗███╗   ███╗ ██████╗ ██████╗ ██╗ █████╗
██║   ██║████╗ ████║██╔═══██╗██╔══██╗██║██╔══██╗
██║   ██║██╔████╔██║██║   ██║██████╔╝██║███████║
██║   ██║██║╚██╔╝██║██║   ██║██╔══██╗██║██╔══██║
╚██████╔╝██║ ╚═╝ ██║╚██████╔╝██║  ██║██║██║  ██║
 ╚═════╝ ╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝`,
  primaryNav,
  homeHero: {
    brand: "Umoria",
    headline: "A Rust port of The Dungeons of Moria",
    support: "Classic dungeon crawling, rebuilt for modern systems.",
    downloadsLabel: "Downloads",
    docsLabel: "Docs"
  }
};
