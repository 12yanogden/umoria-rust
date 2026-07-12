# Umoria web content conventions

Contract for later phases. This scaffold does **not** define the article catalog.

## Storage (philes)

| Concern | Convention |
| --- | --- |
| Article format | `.phile` under `src/content/philes/volume-<N>/` (optional category subdirs OK) |
| Filename / slug | kebab-case; slug defaults from filename when omitted |
| Default author | `"Umoria"` for stubs |

### Required frontmatter

- `title`
- `date`
- `author`
- `sources` — non-empty array of `{ label, href }` (see [Attribution](#attribution))

### Optional frontmatter

- `order`
- `slug`
- `lang` (`en` default)
- `redacted`

### Attribution

**Locked (phase_4.1):** every docs phile declares `sources` in frontmatter as a **non-empty** array matching phase_3’s `DocsSourceRef`:

```yaml
sources:
  - label: "Beej classes.txt"
    href: "https://beej.us/moria/classes.txt"
  - label: "README · Building"
    href: "README.md#building"
```

| Rule | Detail |
| --- | --- |
| Shape | `{ label: string, href: string }[]` — same field names as `DocsSourceRef` in `src/content/docs-catalog-types.ts` (re-exported from `docs-catalog.ts`) |
| Validation | Omitted `sources` **or** `sources: []` **fails** the phile content schema |
| Rendering | `src/components/Sources.astro` (helper: `src/modules/philes/sources.ts`); mounted by `PhileShell` on every phile page |
| Empty defensive UI | If the component is ever passed `[]`, it renders `Sources: (none listed)` — production stubs must not hit this (schema rejects empty/missing) |
| Href resolution | Absolute `http(s):` URLs as-is; site-root paths starting with `/` via phase_1 `withBase`; other relative paths treated as **repo-relative** → `https://github.com/12yanogden/umoria-rust/blob/main/<path>` (same blob scheme as `SiteFooter`) |

This supersedes the earlier scaffold note of optional `sources: string[]` and the body `--[ Sources ]--` deferral. Field name remains `sources` (not a parallel key).

Non-catalog scaffold/fixture philes (e.g. `_placeholder`, `_sources-fixture`) still use the same schema when they live under `src/content/philes/` so `bun run build` stays valid; they are not catalog articles.

## Routes

| Concern | Convention |
| --- | --- |
| Entropic theme routes (storage / default) | `/volume/<N>/` index; `/volume/<N>/<slug>/` article — **theme defaults only; not the public docs URL contract** |
| Public docs URLs | Owned by **phase_4**: `/docs/` (index) and `/docs/<catalog-slug>/` (articles). Phase_4 adds Astro page shims that load philes from `volume-<N>/`. Storage path ≠ public URL. |
| Non-phile pages | Splash `/`, downloads `/downloads/`, and chrome live under `src/pages/` — not as philes |

## Volumes (phase_4.2)

Section → volume numbers and labels are authoritative in `src/content/docs-section-volume-map.ts` (`DOCS_SECTION_TO_VOLUME`, `DOCS_VOLUME_LABELS`; re-exported from `src/config/volumes.ts`). Labels match `docs-nav.ts` section labels. Section ids: `getting-started` … `reference` → `volume-0` … `volume-7`.

Stub philes: one per catalog slug at `src/content/philes/volume-<N>/<catalog-slug>.phile`. Underscore-prefixed fixtures (`_placeholder`, `_sources-fixture`) are non-catalog.

### Outline stubs vs authored articles

| Kind | Body signal | `docs:generate-stubs` | Stub gate (`docs-stubs.test.ts` case 3) |
| --- | --- | --- | --- |
| Outline stub | `<!-- docs-stub: outline only; full prose out of scope -->` | Overwrites from catalog | Outline bullets, summary/“outline”, body ≤ 4000 chars, **no** `FULL_ARTICLE` |
| Authored article | Prefer `<!-- FULL_ARTICLE -->` near the top of the body after frontmatter; also any catalog phile that **lacks** the docs-stub marker | **Skipped** (never overwritten) | Exempt from stub-scale / outline-only checks; existence, title, sources, and required frontmatter still validated |

Shared helpers: `src/content/docs-stub-contract.ts` (`DOCS_STUB_MARKER`, `DOCS_FULL_ARTICLE_MARKER`, `isAuthoredDocsBody`).

When populating a stub: remove the docs-stub marker, write full prose, and add `<!-- FULL_ARTICLE -->` so regenerates and tests treat the page as authored.

### Regenerating stubs

When `docs-catalog.ts` changes, regenerate **outline stubs only** (catalog wins for those files’ frontmatter + outline). Authored / `FULL_ARTICLE` pages are never overwritten:

```bash
bun run docs:generate-stubs
```

Idempotent for stubs: re-running overwrites remaining outline stubs consistently; skips authored pages.

## Ownership split

- **phase_3** — which articles/sections exist (catalog taxonomy).
- **phase_4** — public `/docs/` URLs, stubs under these storage conventions, and volume label mapping.
- **phase_1 (this scaffold)** — where/how articles live; content collection / phile schema.
- **phase_4.1** — `sources` frontmatter contract + Sources component (this section’s Attribution rules).
- **phase_4.2** — stub generator, `DOCS_SECTION_TO_VOLUME`, volume labels.

## Docs entry hook

Home `siteConfig` may include a single “Docs” / “Philes” volumes section pointing at volume indexes; no per-article links on the home page.
