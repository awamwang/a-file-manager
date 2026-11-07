# Sigma tags capability — evaluation (AIA-59)

> Status: **eval only** · 2026-11-04  
> Sources: local sibling checkout `<项目根>/../sigma-file-manager` (user-stats, tag-selector, dashboard tagged views, list column)

## What Sigma tags are

App-local metadata, **not** NTFS Alternate Data Streams / Explorer property tags.

| Piece | Role |
|---|---|
| `ItemTag` | Catalog entry: `id`, `name`, `color` |
| `TaggedItem` | Path ↔ `tagIds[]` (+ `addedAt`, `isFile`) |
| Persistence | `user-stats` storage (`tags`, `taggedItems`) |
| Defaults | Important / Work / Personal / Archive |
| UX | Tag selector, inline rename/color/delete, list **Tags** column, dashboard tagged sections + quick-access “tagged”, view-filter by tags, project `tagIds` |

## Strengths (worth copying later)

1. **Lightweight model** — catalog + path mapping scales without FS writers.  
2. **Dashboard surface** — tags are discoverable, not only a context-menu dead-end.  
3. **List column + filter** — tags participate in browse/sort/filter, not only edit.  
4. **Reconcile path** — missing tag defs recovered from tagged items (robust against partial stores).  
5. **Multi-tag** — one path can hold several tagIds.

## Costs / risks for AFM

1. **Large surface** — selector UI, overflow, reorder merges, dashboard, column, filters, i18n — multi-slice.  
2. **No OS sync** — Explorer / Search / OneDrive tags stay separate (same class of gap as AFM favorites vs Quick Access).  
3. **Path identity** — renames/moves need the same path-rewrite hygiene Sigma already struggles to keep (history/favorites/tags).  
4. **Detach rule** — must not import Sigma tag-selector UI kit; Fluent rebuild required.

## Recommendation for A File Manager

| Horizon | Decision |
|---|---|
| **Now (post AIA-58)** | **Do not implement.** Favorites (AIA-57) cover the Day-1 “pin path” need. |
| **Next** | Optional thin slice: catalog + path map + context “Edit tags” stub → real toggle, **no** dashboard/column yet. |
| **Later** | List column + Discovery/filter integration; still app-local (document non-OS). |
| **Avoid** | Porting Sigma tag-selector / dashboard tagged views wholesale; NTFS ADS unless TZ asks. |

## Product questions (ask TZ before coding)

1. Tags vs favorites: overlapping “bookmark” UX, or tags = multi-label taxonomy only?  
2. Scope: files+dirs, or dirs-only at first?  
3. Need list-column in the first tags slice, or context + side list enough?

## Relation to recent slices

- AIA-55 System menu → OS verbs (incl. Explorer pin) remain outside AFM tag store.  
- AIA-57 Favorites → simpler single-set pins; tags would be the multi-label layer on top.
