# 对外功能回归基线：两层门禁、不做 e2e

为「对外功能不因其他改动而失效」确立统一基线机制。结论：原子行为级基线 = **静态命令/契约门禁（`scripts/check-*.mjs`）** + **可执行行为测试（后端 cargo `#[test]` + 前端纯逻辑 `*-checks.ts`）**，全部接进 `npm test`；**不引入跑应用的 e2e**。收录判据从「前端是否 `invoke`」改为「**是否处在以用户可见结果收尾的链路上**」。

**Status**: accepted（首批落地见「实现」段 W1/W2/W3；其余编号待圈做）

## 决策

1. **三层分类法**（每条对外行为都按此标注覆盖）：
   - **STATIC-GATE**：`scripts/check-*.mjs` 对源码做符号/字符串存在性断言，经 `package.json` 的 `test:*` 入聚合 `test`（`&&` 链、首失败即停）。
   - **FE-UNIT**：`scripts/*-checks.ts` 纯逻辑行为单元，`node --experimental-strip-types` 执行，由某 `.mjs` `spawn`；只 import 不依赖 vue 运行时的纯模块。
   - **BE-CARGO**：`src-tauri/src/**` 的 `#[cfg(test)]`，在临时目录/内存**真实执行**该行为，经 `test:rust`（全 crate `cargo test`，已在链里）跑。
2. **收录按链路，不按调用面**：处在任一条用户可见结果链路上的命令/facet 都要有基线，**含无前端 `invoke` 的内部 facet**。例：managed 产物 feed（`set_feed`/`get_feed`）驱动「最终下载/安装」→ 纳入；`extension_host_install_pack_from_feed` 同理。
3. **行为执行落点**：
   - 真实数据搬运/落盘 → **BE-CARGO**（临时目录或纯内存计算），随 `test:rust`。WG `rename_work_group_in_list` / `delete_work_group_in_list` 即纯列表函数，直接 `#[test]`。
   - 前端编排若是纯逻辑、或经**行为保持的提取**变为纯模块 → **FE-UNIT**；同时 `.mjs` 钉「模块/委托存在」+「前后端命令名严格一致」。例：`folderPathsForWorkGroup` 从 `NavigatorContextMenu.vue` 抽到 `src/components/navigator/workGroupPaths.ts`。
   - 融合 reactivity/`invoke`、无法 headless 执行的 FE 编排（如 promote 的 `snapshot→invoke→restoreWorkGroupOpenState→router.push→select`）→ **诚实维持 grep（正/负向断言）**（例 `check-wg-promote-jump.mjs`），其行为正确性由后端 cargo 契约守，不谎称 FE 行为测试。
4. **不做 e2e**：现有 160+ 门禁全用本范式；e2e 需另一套昂贵基础设施、拖慢且污染回归链；行为级已由 BE-CARGO 承担；AFM 是原生窗口 + 系统对话框桌面应用，浏览器 e2e 覆盖不到。
5. **前后端命令名 parity 是硬钉合点**：`.mjs` 须断言 `invoke("X")` 与 `#[tauri::command] pub fn X` 同名，防改名导致的静默失联（archive 那轮即靠此）。
6. **`#[ignore]` 的测试不算覆盖**：被忽略的真实下载测试（`managed_dependency/provision.rs:286`）不进基线；要守「下载/安装」链须改成非 ignore 的本地 feed/mock 执行测试。

## 不做什么

- 不引入 vitest / 任何前端测试框架；沿用 `*-checks.ts` + `--experimental-strip-types` 现有范式。
- 不把 `vue-tsc` / `tsc` 纳入 `test` 链（类型检查仍只在 `build`）。
- 不为纯装饰/视觉项造行为测试（符号门足矣）。
- 不自动排除「无 invoke」的内部 facet——一律按链路判定。

## 实现

- **首批已落地**（提交 `59bff45`，本轮 W1/W2/W3 工作组·项目对外链）：
  - W1 在资源管理器打开：`open_with` 测试桩从 `let _ = path` 丢弃改为记录 `EXPLORER_LAST_PATH` 并断言转发真实路径（cargo）；新增 `check-work-groups-open-explorer.mjs` 钉 WG 侧 `open_path_default` 接线（Projects 侧原由 `check-open-project-folder.mjs` 覆盖）。
  - W2 多选→加入工作组：抽 `workGroupPaths.ts` 纯 reducer + `work-group-paths-checks.ts` 行为单元 + `check-work-group-paths.mjs`。
  - W3 WG rename/delete：`work_groups/sync.rs` 补 2 个 `#[test]`（trim/空/未知 id、DEFAULT 不可删、删 active→迁移默认、删非 active→不变）。
  - 新增 2 个 `test:*` 入 `npm test` 链（128→130）。
- **待落地（后续按编号圈做）**：H4/H5/H6 下载·安装链、H1/H2/H3 前端纯逻辑对话框、H7 knowledge 命令面、H8 三个未入聚合链的门禁接线（`test:perf-probe` / `test:delete-keep-selection` / `test:provider-torrent-open`）、可选 W4 reorder 纯助手 FE-UNIT。
- **台账**：全量「原子行为×层×缺口」清单为 `afm-atomic-behavior-inventory.md`（暂存协作工作区；是否迁入仓内 `docs/` 待定）。

## 与其他 ADR 的关系

- 不改变 ADR 0020/0022（managed 产物 feed / provision）与 ADR 0014（Feature Pack 安装）的**产品语义**；只规定「如何守住它们对外链路」的基线层级要求。
