# Performance ledger

Follows the performance-optimization skill: measure → identify → fix → verify → keep/revert.

| Idea | Baseline → Result | Verdict | Why |
|------|-------------------|---------|-----|
| Backend `list_directory` on 5000 files | median **12.3ms** (debug) | kept as-is | Not the bottleneck |
| Mount all 5000 list rows (plain DOM probe) | median **137–170ms** create+layout | identified | Full listing v-for was the user-facing cost |
| Windowed Navigator list (`listVirtualWindow`, ≥80 entries) | 5000 rows **137ms** → ~40-row window **0.8ms** (DOM probe) | **kept** | ~170× less mount work; scroll spacers keep total height |
| Fix `guide/index.ts` dynamic import path | build failed → resolves | **kept** | Unblocks production `vite build` (correctness, not a micro-opt) |

## Budgets / guards

- `npm run test:list-virtual` — window math + Navigator wiring
- Existing: `test:ui-perf`, `test:shell-icon-*`, `test:startup-async`, `test:list-placeholders`

## How to re-measure listing

```powershell
$env:AFM_PERF_DIR = "<path-to-large-dir>"
cargo test --manifest-path src-tauri/Cargo.toml --lib navigator::list::tests::bench_list_large_directory -- --nocapture
```
