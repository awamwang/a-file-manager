<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 性能基线（Phase A）

盘点与测量日期：2026-11-10。目标是建立可复跑的场景清单、分类矩阵与第一轮数字；本阶段不做算法优化。

## 探针开关

| 开关 | 作用 | 输出 |
| --- | --- | --- |
| `localStorage.setItem("afm.startupTiming", "1")` | 冷启动里程碑 | 控制台 `[AFM_startup]` |
| `localStorage.setItem("afm.perfProbe", "1")` | 操作级 span（list / icon / switch / tree） | 控制台 `[AFM_perf]` |
| `AFM_PERF=1` | Rust 侧 command 耗时 | stderr；建议同时设 `AFM_PERF_LOG` 写文件（Windows GUI 子系统常丢弃 stderr） |
| `AFM_PERF_DIR=<dir>` | 单元测试 bench 目录 | `cargo test … --nocapture` |

关闭或未设置时探针为 no-op。前端建议同时打开两个 localStorage 开关再 reload：

```js
localStorage.setItem("afm.startupTiming", "1");
localStorage.setItem("afm.perfProbe", "1");
location.reload();
```

Rust 运行时文件日志示例（PowerShell）：

```powershell
$env:AFM_PERF = "1"
$env:AFM_PERF_LOG = "G:\Projects\Tools\a-file-manager\.perf-fixtures\afm-perf.log"
```

## 固定场景（必测）

| ID | 场景 | 数据约定 | 主要看什么 |
| --- | --- | --- | --- |
| S1 | 冷启动到首屏可交互 | 默认 OpenState | `startupMark` 里程碑链 |
| S2 | 打开中等目录 | 约 500–1000 条目 | `navigator_list` + 首批 icon |
| S3 | 打开大目录 | 约 5k+ 条目 | list median、主线程卡顿、icon 追赶 |
| S4 | 图标视图滚动缩略图 | 同 S3，`preferThumbnail` | icon batch 耗时与队列积压 |
| S5 | Tab / 工作组切换 | 两路径已打开过 | `tab_switch` 到 listing 就绪 |
| S6 | 窗口回焦刷新 | 已打开目录 | focus debounce 后的 re-list |

## 卡点 → 代码路径

| 操作 | 代码入口 | span / mark 名 |
| --- | --- | --- |
| 冷启动 bootstrap | `src/main.ts`、`revealMainWindow.ts`、`AppShell.vue`、`NavigatorView.vue` | `startupMark(...)` |
| 内容区列目录 | `src/state/navigatorBrowse.ts` → `navigateTo` / `navigateSecondary` | `navigator_list` |
| Tab / WG 切换到 listing 就绪 | `switchNavigatorTab` / `switchNavigatorTabAdjacent` / `restoreWorkGroupOpenState` | `tab_switch` |
| Shell 图标 / 缩略图批次 | `src/state/shellIcons.ts` → `flushQueue` | `shell_icon_batch` |
| 侧栏树展开列目录 | `src/components/shell/FolderTreeBranch.vue` → `loadChildren` | `tree_list` |
| Rust 纯列举（无 IPC） | `src-tauri/src/navigator/list.rs` `bench_list_large_directory` | `AFM_PERF list_directory` |
| Rust JSON 序列化对照 | 同上 bench | `AFM_PERF list_serde_json` |
| Rust icon/thumb 冷热 | `shell_icons::tests::bench_shell_icon_batch` | `AFM_PERF shell_icon_batch` |

## 分类矩阵

资源类型可多选：`CPU` / `IO` / `IPC` / `主线程` / `内存`。

| 场景 | 操作 | 代码入口 | 一次/动态 | 资源类型 | 关键路径? | 假设 | 基线数字 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| S1 | 冷启动里程碑链 | `startupTiming` marks | 一次 | CPU, IPC, 主线程 | 是 | hydrate + 首屏 reveal 占主导 | 见 Run #1 |
| S2 | `navigator_list` | `navigatorBrowse.navigateTo` | 动态 | IO, IPC, CPU | 是 | 随条目数近似线性；Rust list 远小于 FE 往返 | 见 Run #1 |
| S2 | 首批 `shell_icon_batch` | `shellIcons.flushQueue` | 动态 | IPC, 内存 | 是 | 视口批大小约 40；冷路径贵、热路径近零 | 见 Run #1 |
| S3 | `navigator_list` 大目录 | 同上 | 动态 | IO, IPC, CPU, 主线程 | 是 | 全量返回；serde 已大于纯 list | 见 Run #1 |
| S3 | Rust `list_directory` | `navigator/list.rs` | 动态 | IO, CPU | 否（对照） | FE − Rust ≈ IPC+序列化+应用 | 见 Run #1 |
| S4 | 缩略图 batch | `shellIcons` preferThumbnail | 动态 | IPC, IO, 内存 | 是 | BATCH_THUMB=8；冷路径显著 | 见 Run #1 |
| S5 | `tab_switch` | `switchNavigatorTab*` | 动态 | IPC, 主线程 | 是 | 切换本身轻，墙钟≈listing | 见 Run #1 |
| S6 | focus re-list | `refreshActiveListing` | 动态 | IO, IPC | 部分 | 400ms debounce 后触发 | 见 Run #1 |
| — | `tree_list` | `FolderTreeBranch.loadChildren` | 动态 | IO, IPC | 否（延后） | afterPaint，不抢首屏 | 见 Run #1 |

## 环境与数据约定

- **机器**：填写 Run 时记录 OS、CPU、磁盘类型。
- **构建**：debug/`tauri:dev`（未优化；对比优化请另开 Run）。
- **测试目录**：
  - 中等：`.perf-fixtures/med`（约 1000 文件）
  - 大：`.perf-fixtures/large`（约 5000 文件）
- **生成夹具**（PowerShell）：

```powershell
$root = "G:\Projects\Tools\a-file-manager\.perf-fixtures"
New-Item -ItemType Directory -Force -Path "$root\med","$root\large" | Out-Null
1..1000 | ForEach-Object { Set-Content -Path "$root\med\f$_.txt" -Value $_ }
1..5000 | ForEach-Object { Set-Content -Path "$root\large\f$_.txt" -Value $_ }
```

### Rust 纯列举 / 序列化 / icon 对照

```powershell
$env:AFM_PERF_DIR = "G:\Projects\Tools\a-file-manager\.perf-fixtures\large"
cargo test --manifest-path src-tauri/Cargo.toml --lib navigator::list::tests::bench_list_large_directory -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --lib shell_icons::tests::bench_shell_icon_batch -- --nocapture
```

### 前端跑法（S1–S6）

1. 设 `AFM_PERF` / `AFM_PERF_LOG`，启动 `npm run tauri:dev`。
2. DevTools 打开两个 localStorage 开关并 reload。
3. 按场景操作：进 med/large、切图标视图滚动、切 Tab、失焦回焦。
4. 从控制台 `[AFM_perf]` / `[AFM_startup]` 与 `AFM_PERF_LOG` 汇总中位数。

### JSON 片段格式

```json
{
  "run": 1,
  "date": "2026-11-10",
  "git_sha": "<short>",
  "machine": { "os": "Windows 11", "build": "debug", "notes": "" },
  "samples": []
}
```

## Run #1

- **日期**：2026-11-10
- **构建**：debug（`cargo test` / `target/debug`）
- **夹具**：`.perf-fixtures/med` n≈1000；`.perf-fixtures/large` n≈5000
- **说明**：含 Rust bench、`AFM_PERF_LOG` 实机导航，以及 FE 探针接线。S1/S5 墙钟需在 DevTools 打开 `afm.startupTiming` / `afm.perfProbe` 后补一行即可。

| 场景 | 操作 | n | samples_ms | median_ms | p95_ms | notes |
| --- | --- | --- | --- | --- | --- | --- |
| S2 | Rust list_directory | 1000 | [1.9, 1.9, 1.9, 2.0, 2.1] | 1.9 | 2.1 | `AFM_PERF_DIR=med` |
| S2 | Rust list_serde_json | 1000 | [4.1, 4.1, 4.1, 4.2, 4.3] | 4.1 | 4.3 | 序列化已大于纯 list |
| S2 | command `navigator_list` | 1000 | [2.1] | 2.1 | 2.1 | 实机进 med（`AFM_PERF_LOG`） |
| S3 | Rust list_directory | 5000 | [9.7, 9.8, 10.3, 10.3, 10.3] | 10.3 | 10.3 | `AFM_PERF_DIR=large` |
| S3 | Rust list_serde_json | 5000 | [25.5, 29.0, 30.7, 31.5, 34.4] | 30.7 | 34.4 | IPC 载荷成本主导候选 |
| S3 | command `navigator_list` | 5000 | [17.1, 11.5] | 14.3 | 17.1 | 实机进 large + F5 |
| S2/S3 | shell_icon_batch cold | 40 | [152.9, 153.0, 153.2, 153.2, 255.2] | 153.2 | 255.2 | thumb=0 bench；清缓存 |
| S2/S3 | shell_icon_batch warm | 40 | — | 0.1 | — | 同批热路径 |
| S4 | shell_icon_batch live | 8 | 58 batches | 84 | 226 | 实机 thumb=1 size=96；min 51.4 |
| S4 | shell_icon_batch cold | 8 | [64.9, 67.6, 69.5, 70.1, 94.8] | 69.5 | 94.8 | thumb=1 size=96 bench |
| S6 | command re-list | 5000 | [11.5] | 11.5 | 11.5 | F5 / focus 路径 |
| — | tree parent lists | 3–42 | [0.3–9.9] | — | — | 侧栏展开祖先路径 |
| S1 | startup chain | — | — | — | — | 开 `afm.startupTiming` 后补测 |
| S5 | tab_switch | — | — | — | — | 开 `afm.perfProbe` 后补测 |

```json
{
  "run": 1,
  "date": "2026-11-10",
  "git_sha": "local",
  "machine": { "os": "Windows 11", "build": "debug", "notes": "bench + AFM_PERF_LOG live nav" },
  "samples": [
    { "scenario": "S2", "op": "list_directory", "n": 1000, "median": 1.9, "p95": 2.1 },
    { "scenario": "S2", "op": "list_serde_json", "n": 1000, "median": 4.1, "p95": 4.3 },
    { "scenario": "S2", "op": "navigator_list_cmd", "n": 1000, "median": 2.1 },
    { "scenario": "S3", "op": "list_directory", "n": 5000, "median": 10.3, "p95": 10.3 },
    { "scenario": "S3", "op": "list_serde_json", "n": 5000, "median": 30.7, "p95": 34.4 },
    { "scenario": "S3", "op": "navigator_list_cmd", "n": 5000, "samples_ms": [17.1, 11.5], "median": 14.3 },
    { "scenario": "S3", "op": "shell_icon_batch_cold", "n": 40, "median": 153.2, "notes": "thumb=0" },
    { "scenario": "S4", "op": "shell_icon_batch_live", "n": 8, "median": 84, "p95": 226, "notes": "thumb=1 size=96; 58 batches" },
    { "scenario": "S6", "op": "navigator_list_cmd", "n": 5000, "median": 11.5, "notes": "F5" }
  ]
}
```

## Run #2（Phase B：并行 + 类型图标缓存后）

- **日期**：2026-11-10
- **改动**：`SHELL_ICON_PARALLELISM=4` 批内并行；壳图标 `type:<ext>` 二级缓存；真 thumb 仍仅 path 缓存
- **对照**：同 `.perf-fixtures/large`，debug 构建

| 指标 | Run #1 | Run #2 | 变化 |
| --- | --- | --- | --- |
| icon 冷批 n=40 thumb=0 median | 153.2ms | **18.4ms** | −88% |
| thumb 冷批 n=8 bench median | 69.5ms | **30.9ms** | −56% |
| thumb 实机批 median（size=96） | 84ms | **~12.5ms** | −85% |
| thumb 实机首批 | 226ms p95 / 冷 | **144.6ms** 首批 | 首批仍偏冷，随后批 ~9–20ms |

```json
{
  "run": 2,
  "date": "2026-11-10",
  "git_sha": "local",
  "machine": { "os": "Windows 11", "build": "debug", "notes": "Phase B parallel+type cache" },
  "samples": [
    { "scenario": "S3", "op": "shell_icon_batch_cold", "n": 40, "median": 18.4, "notes": "thumb=0; vs R1 153.2" },
    { "scenario": "S4", "op": "shell_icon_batch_cold", "n": 8, "median": 30.9, "notes": "thumb=1 bench; vs R1 69.5" },
    { "scenario": "S4", "op": "shell_icon_batch_live", "n": 8, "median": 12.5, "max": 144.6, "notes": "thumb=1 size=96; vs R1 84" }
  ]
}
```

成功标准核对：icon 冷批 ≥40% ↓（达成）；实机 thumb median ≥25% ↓（达成）。

## 下一阶段候选（按数据排序）

1. **大目录 JSON 序列化（S3）** — serde median ≈ **30.7ms** vs list **10.3ms**；IPC 载荷/字段裁剪（图标冷路径已在 Phase B 处理）。
2. **大目录 FE 渲染 / 主线程** — Rust list/icon 批已明显下降，可感知卡顿更可能在 WebView 挂载行（虚拟列表 / 占位）。
3. **缩略图首批残留** — 实机首批仍可到 ~145ms（Shell 冷 COM）；可评估渐进「先壳图标后升级 thumb」。
4. **侧栏树祖先列举** — 进深路径多次 `navigator_list`；非关键但争用 IO/IPC。
5. **冷启动链（S1）** — 开 `startupMark` 后按里程碑拆 defer。
