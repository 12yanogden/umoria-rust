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

### Regenerating stubs

When `docs-catalog.ts` changes, regenerate outline stubs (catalog wins for frontmatter + outline until articles are authored):

```bash
bun run docs:generate-stubs
```

Idempotent: re-running overwrites generated stubs consistently.

## Ownership split

- **phase_3** — which articles/sections exist (catalog taxonomy).
- **phase_4** — public `/docs/` URLs, stubs under these storage conventions, and volume label mapping.
- **phase_1 (this scaffold)** — where/how articles live; content collection / phile schema.
- **phase_4.1** — `sources` frontmatter contract + Sources component (this section’s Attribution rules).
- **phase_4.2** — stub generator, `DOCS_SECTION_TO_VOLUME`, volume labels.

## Docs entry hook

Home `siteConfig` may include a single “Docs” / “Philes” volumes section pointing at volume indexes; no per-article links on the home page.
