# 统一托管依赖 · Phase 2：file/knowledge pack 化与共享运行时落地计划

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development / executing-plans 逐任务执行；checkbox 跟踪。

**Goal:** 在 ADR 0022「集成方 in-tree 构建 + AFM 自有 feed + shared-first 运行时」模型下，把剩余两个托管依赖 **awam-file / awam-knowledge** 收编为 Feature Pack，并把共享运行时与发布 CI 落地，使三依赖全部走同一「安装→启用→attach-or-spawn→健康启停」闭环。

**背景:** Phase 1（`2026-11-18-managed-dependency-awam-file-r3-pilot.md`）的 Rust 底座 + IPC + 投影 + 前端已全部落地；tool-runtime 已端到端验证（feed→下载→SHA→spawn→health→store 一键安装，445 测试绿）。本计划接续其剩余部分。

**Depends on:** ADR 0012 / 0013 / 0019（被 0022 部分取代）/ 0020（被 0022 修订）/ 0022；Phase 1 计划。

**Global Constraints:**
- 进程模型不变：服务在 AFM 进程**之外**（HTTP attach-or-spawn）；进程态唯一权威 = `ManagedDependencyRecord.spawned_pid`（C-4）
- 自构建 managed 产物**跳过 versionCompat 门**；门仅对 config-mode 外部实例（警告不硬阻断）
- 运行时 precedence：`runtime: self` 用自带；`shared(python|node)` 用 AFM 共享运行时；**自带则用自带，否则用共享**
- 三仓 MIT license：捆绑产物内保留 NOTICE/THIRD-PYNOTICE 署名
- pack id 稳定性：file 用 `awam.file-gateway`（projection 映射已预留）；knowledge 定 `awam.knowledge`
- 提交信息中文；注释中文

---

## Task P2-1: awam-file pack 化（首选闭环）

**Files:** Create `packs/file-gateway/pack.json` + `packs/file-gateway/feed-entry` 构建参数；Create 冻结入口 wrapper（构建期生成或仓内 `packs/file-gateway/freeze_entry.py`）

- [ ] wrapper 入口：awam-file 是 `python -m awam_resource_gateway http` 模块式入口，PyInstaller 需薄脚本 `from awam_resource_gateway.__main__ import main; sys.exit(main(["http"]))`
- [ ] `build_pack_feed.py build --pack-id awam.file-gateway --src G:/Projects/19AI/skills/awam-file --entry <wrapper> --base-feed-url ...`（首版 `runtime:self` onefile；shared 档待 P2-3 切换）
- [ ] `packs/file-gateway/pack.json`：`permissions:["network","spawnSidecar"]`、sidecar `command:"awam-file-gateway.exe", args:["--host","127.0.0.1","--port","18787"]`、`workingDirRel:"sidecar"`、`runtime:"self"`；UI 贡献**从简**（Providers 面已存在，不新加侧栏；可加 `search` 静态入口一条）
- [ ] feed 条目 `baseUrl:"http://127.0.0.1:18787"`；`emit-feed-config`
- [ ] e2e：安装→启用（钩子建 record, mode=Managed）→ Health 页 `awam_gateway` 显示「AFM 托管」+ 启停→ensure 拉产物 spawn→`/health` ok→stop 只停自启
- [ ] NOTICE/THIRD-PARTY 归档 + `check-deps-surface`/`check-feature-pack-shell` 等门禁回归
- [ ] 复用性检验：本任务**不应新增任何 file 专属 Rust 分支**（若有，视为架构违规回看 ADR 0020/0022）

## Task P2-2: awam-knowledge 内置 pack

- [ ] `packs/knowledge/`：Node 零依赖产物**内置进安装包**（非 feed 首拉，ADR 0022 分发档位决策）；pack.json sidecar `command:"node"`（或内置 node 运行时路径）、`runtime:"shared(node)"`/内置档
- [ ] `projection.rs` 映射补 `"awam.knowledge" => Some("knowledge_service")`（条目 id 以 deps_probe 实况为准，需核对）
- [ ] knowledge 的 URL/token credential 与 `knowledge_service` settings 键的权威关系收口（对齐 e603acf 的 record 权威模式）
- [ ] e2e：启用→拉起 `node code/http-server.js`→搜索面板知识源可用；config-mode 填远程地址仍可达

## Task P2-3: 共享运行时落地

- [ ] AFM 自装共享 env 的精确定位（替代现平台默认 `python3`/`node` 兜底）：embeddable Python + 首启用 wheels，或 relocate 方案（venv 绝对路径不可 zip 直发——0022 隐藏成本②）
- [ ] `resolve_spawn_command` 的 shared 档指向自装 env
- [ ] file/tool-manager 从 `runtime:self` 切 `shared(python)`：产物退化为薄 wheel/源码，体积对比留档
- [ ] mcp 2.3.0 统一后的行为回归（tool-manager 817/818 基线已在跨仓侧完成，AFM 侧复跑）

## Task P2-4: 发布 CI（feed 真实发布）

- [ ] AFM release job：per-OS agent 跑 `build_pack_feed.py build`（PyInstaller 不可跨平台）+ `pack-dir` 产 zip + manifest 汇总上传
- [ ] 三依赖仓以 **git submodule 锁 tag/SHA** 引入 provenance；各仓补 lockfile + 构建文档
- [ ] CVE 修复触发全量重建的流程记录进 0022

## Task P2-5: 工程收尾

- [ ] `host.rs` 启用路径同步 `ureq` 拉 feed → `spawn_blocking` + 本地缓存
- [ ] ADR 0020 Q15 门措辞与 0022 文字对齐（一句话修订）
- [ ] 移除残余 `#[allow(dead_code)]`（已接线符号清单核对）

---

## 验证矩阵（每任务必跑）

`cargo check --tests` 零警告 → `cargo test --lib` 全绿 → `vue-tsc --noEmit` → `node scripts/check-i18n.mjs` → 相关 `check-*.mjs` 门禁 → P2-1/P2-2 附真机 e2e 手测记录。
