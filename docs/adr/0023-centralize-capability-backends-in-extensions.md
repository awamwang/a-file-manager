# Centralize capability backends in the Extensions surface

Capability backends were spread across four entries that are really different facets of the
same set of services (file-gateway / knowledge / tool-runtime): install state in `/extensions`,
attach/browse in `/providers`, runtime invocation in `/tools` (injected by the
`awam.tool-runtime` pack), and backend URL/token config in `Settings → Extensions`. This ADR
folds them into a single `Extensions` page organized as a left section-nav (master-detail),
presenting each backend as one **Capability** card whose mode is either `managed` (a Feature
Pack is installed and AFM spawns the sidecar) or `config` (AFM attaches to an externally run
service). AI Add-on and Open tools stay outside — they are external-managed surfaces, not
capability backends — and are reached only via cross-links.

**Status**: accepted

**Scope decided** (grilled with the design owner):
- In: Feature Pack install + store, Providers, Tool Runtime, backend service config.
- Out (cross-link only): AI Add-on (`/ai`), Open tools (`/settings/open-tools`).
- IA: single `/extensions` page with left section-nav (same master-detail idiom as `AiView` /
  `SettingsView`); sections = Overview / Installed / Store / Providers / Tool Runtime / Service
  config.
- Tool Runtime ownership: a new pack contribution point, `extensionsSection`, replaces the
  pack's top-level side-nav + route. The pack contributes a section inside Extensions instead
  of hijacking a main-menu pillar.

**Considered Options**:
- Fold everything (incl. AI Add-on) into one extensions center — rejected: contradicts ADR
  0022's reclassification of AI Add-on / Open tools as external-managed surfaces, and bloats
  the page.
- Keep separate routes/pages, only collapse the main-menu entries into a parent group —
  rejected: install vs config vs browse still live in disconnected pages; does not deliver the
  "centralized management" goal.
- Hard-code Tool Runtime as a built-in Extensions section and drop its pack route — rejected:
  reverts the contribution system direction; third-party packs would have nowhere to mount UI.
- Keep `/tools` as a top-level route with only an entry card in Extensions — rejected: leaves
  the menu item the request is specifically trying to remove.

**Concrete changes** (for the follow-up implementation plan):
- New contribution type `extensionsSection` in the Feature Pack format
  (`docs/feature-pack-format.md`, `packs/tool-runtime/pack.json`): `{ id, labelKey, entry, order }`
  where `entry` reuses the `builtin:` component-id scheme already used by route contributions
  (`src/state/featurePack/builtinEntries.ts`). Aggregated in
  `src/state/featurePack/contributions.ts` (`getSectionExtras`) and in the Rust
  `extension_host_list_contributions` payload.
- `ExtensionsView.vue` becomes the master-detail host; `ProvidersView`, the existing pack/store
  blocks, `ExtensionsSettingsPanel`, and `ToolRuntimeView` are refactored into section panels.
- Side nav (`src/shell/sideNav.ts`): drop the `providers` top-level item. Generic
  `sideNav`/`applyFeaturePackSideNav` contributions remain supported for future packs that
  legitimately want a top-level item, but `awam.tool-runtime` no longer uses it.
- Routing (`src/router/index.ts`): `/extensions/:section`; legacy redirects `/providers →
  /extensions/providers`, `/tools → /extensions/tool-runtime`, `/settings/extensions →
  /extensions/config` (same idiom as the existing `/health → /settings/health`).
  `/settings/open-tools`, `/settings/health`, `/settings/discovery` stay.
- The `extensions` entry is removed from `SettingsView`'s section list; its panel is hosted by
  Extensions.
- Unified **Capability** card (Overview): mode chip (`managed` / `config` / `builtin` /
  `absent`) + connectivity + one primary action per mode (install / enable / upgrade / edit URL
  / attach / detach). Mode derived from the same signals already in use:
  `managed_dependency_list.mode` and `ToolRuntimeView` `urlLocked`.
- Re-point dependents of the old `/tools` and `/providers`: the command-palette
  `cmd.toolRuntimeOpen` route, and the search-panel tool source that currently keys off the
  `builtin:tool-runtime` *route* contribution (`toolRuntimeRouteContribution`,
  `src/search/sources/tools.ts`) must switch to the `extensionsSection` signal.
- i18n: add `extensions.sections.*` / capability-mode keys; keep `nav.toolRuntime` as the
  section label; drop `nav.providers`. Must satisfy the 5-locale `check-i18n` parity gate.

**Consequences**:
- Reverses two glossary entries — "工具管理运行时 … 一级侧栏独立入口" and the split that kept
  ExtensionHost as the install-only shell while tool-runtime was a separate pillar. Glossary
  updated in the same change.
- Removes two main-menu pillars (`providers`, `tools`); a cleaner pillar set, at the cost of
  one more level of navigation inside Extensions. Mitigated by deep-link redirects + the
  Overview capability cards as the "jump straight to a backend" entry.
- Moderate template churn across four view bodies; mostly moves, not logic rewrites. Backend +
  front + Rust contribution aggregation and `check-tool-runtime-pack.mjs` all need the new
  contribution surface.
