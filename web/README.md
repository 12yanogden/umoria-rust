# Umoria web site

Content-only Astro site for GitHub Pages, vendored from the [Entropic](https://github.com/CuB3y0nd/entropic) textmode theme (MIT © CuB3y0nd — see `LICENSE`).

Configured for project Pages at `https://12yanogden.github.io/umoria-rust/` (`site` + `base` in `astro.config.mjs`).

## Requirements

- Node `>=22.12.0`
- [Bun](https://bun.sh/) (lockfile: `bun.lock` or `bun.lockb`)

## Scripts

```bash
bun install
bun run dev
bun run build
bun run preview
bun run check
bun run docs:generate-stubs   # regenerate docs stubs from docs-catalog.ts
bun run test:docs-stubs       # stub completeness / sources / outline checks
```

## Layout

- `src/config/` — site, volumes, appearance, effects
- `src/content/philes/` — `.phile` articles by volume (`volume-0`…`volume-7` per section map)
- `src/content/docs-catalog.ts` — article catalog (source of truth for stubs)
- `src/pages/docs/` — public `/docs/` and `/docs/<slug>/` routes
- `src/layouts/`, `src/modules/textmode/` — Entropic textmode stack
- `CONTENT_CONVENTIONS.md` — phile/volume/sources contract

Docs stubs are outlines only; regenerate with `bun run docs:generate-stubs` when the catalog changes.

## GitHub Pages

Maintainer note for enabling and verifying the production deploy. For deeper Astro details, see [Astro’s GitHub Pages guide](https://docs.astro.build/en/guides/deploy/github/).

1. **Pages settings:** In the GitHub repo, open **Settings → Pages → Build and deployment** and set **Source: GitHub Actions** (not “Deploy from a branch”).
2. **Workflow:** `.github/workflows/deploy.yml` runs on `push` to `main` and on `workflow_dispatch` (Actions → Run workflow).
3. **Build root:** The site is built from `web/` via `withastro/action` (local preview/build: `bun install` / `bun run build` in this directory). Package manager is auto-detected from the bun lockfile.
4. **Production URL:** `https://12yanogden.github.io/umoria-rust/` — Astro `base` is `/umoria-rust` (see `astro.config.mjs`).
5. **Verify:** After enablement, push to `main` or run the workflow manually; confirm the `deploy` job succeeds in Actions, then open the Pages URL above.
6. **Out of scope:** Custom domains are not configured by this pipeline unless added later.
