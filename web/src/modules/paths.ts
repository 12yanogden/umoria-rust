/** Prefix a site-root path with Astro `base` (e.g. `/umoria-rust/`). */
export function withBase(path: string): string {
  const base = import.meta.env.BASE_URL.replace(/\/?$/, "/");
  const clean = path.replace(/^\//, "");
  return `${base}${clean}`;
}

/** Strip Astro `base` from a pathname and normalize trailing slashes (home → `/`). */
export function stripBase(pathname: string, baseUrl: string = import.meta.env.BASE_URL): string {
  const base = baseUrl.replace(/\/$/, "") || "";
  let path = pathname;
  if (base && (path === base || path.startsWith(`${base}/`))) {
    path = path.slice(base.length) || "/";
  }
  if (path.length > 1 && path.endsWith("/")) {
    path = path.slice(0, -1);
  }
  return path || "/";
}

/** True when `pathname` matches a nav `path` after base stripping and slash normalization. */
export function isActivePath(pathname: string, navPath: string, baseUrl?: string): boolean {
  const current = stripBase(pathname, baseUrl);
  let target = navPath.startsWith("/") ? navPath : `/${navPath}`;
  if (target.length > 1 && target.endsWith("/")) {
    target = target.slice(0, -1);
  }
  return current === (target || "/");
}
