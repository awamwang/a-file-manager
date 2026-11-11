# Performance ledger

开发约定（特性性能分析 / 基线 / 非阻塞 UI）：[docs/dev-performance-conventions.md](./docs/dev-performance-conventions.md)。

Follows the performance-optimization skill: measure → identify → fix → verify → keep/revert.

| Idea | Baseline → Result | Verdict | Why |
|------|-------------------|---------|-----|
| Backend `list_directory` on 5000 files | median **12.3ms** (debug) | kept as-is | Not the bottleneck |
| Mount all 5000 list rows (plain DOM probe) | median **137–170ms** create+layout | identified | Full listing v-for was the user-facing cost |
| Windowed Navigator list (`listVirtualWindow`, ≥80 entries) | 5000 rows **137ms** → ~40-row window **0.8ms** (DOM probe) | **kept** | ~170× less mount work; scroll spacers keep total height |
| Group-by-type flattened virt items + variable-height window | grouped path used to mount all rows; now same ≥80 window | **kept** | `listVirtItems` + `computeVariableListVirtualWindow`; icons/tiles use `list--virt-flat` column flow while windowed |
| `sortedEntries` identity cache + `list_sort` / `list_window` probes | re-sort every computed tick → cache hit | **kept** | `afm.perfProbe=1` logs `[AFM_perf] list_sort` / `list_window` |
| Soft near-transparent icon slot → final image | Fluent glyph → shell/thumb morph | **kept** | `shell-icon-slot` until one-shot resolve; no shell→thumb two-phase paint |
| Icon/tiles list virtualization | large dirs thumbs squashed to thin strips | **disabled** | CSS grid window offsets unreliable; list/details still windowed |
| Batch-parallel `IShellItemImageFactory` (4 workers) | bench faster; live SageThumbs slower/flaky | **reverted** | Shell thumbnail handlers need serial COM |
| Fix `guide/index.ts` dynamic import path | build failed → resolves | **kept** | Unblocks production `vite build` (correctness, not a micro-opt) |

## Budgets / guards

- `npm run test:list-virtual` — window math（含变高/分组展平）+ sort 缓存 + Navigator wiring
- Existing: `test:ui-perf`, `test:shell-icon-*`, `test:startup-async`, `test:list-placeholders`

## How to re-measure listing

```powershell
$env:AFM_PERF_DIR = "<path-to-large-dir>"
cargo test --manifest-path src-tauri/Cargo.toml --lib navigator::list::tests::bench_list_large_directory -- --nocapture
```
