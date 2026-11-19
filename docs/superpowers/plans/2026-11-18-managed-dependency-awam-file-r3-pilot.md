# 统一托管依赖 · awam-file R3 Pilot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 awam-file 网关作为**第一个 R3 托管依赖**，端到端打通「商店下载隔离产物 → managed spawn → Health 探活/启停 → config fallback」，并把今天硬编码到 `awam.tool-runtime` 单 pack 的编排**泛化为 pack 无关**的 `ManagedDependency` 运行时，为 knowledge / tool-manager 复用铺路。

**Architecture:** AFM 新增 Core `ManagedDependency` 注册表（连接/进程/健康/credential 权威），`deps_probe`/Provider 降级为它的投影。一个依赖先作为 Feature Pack 安装启用（1:1），pack 贡献 `SidecarContribution`（command 指向商店下载的**自带隔离产物**，非共享运行时）。spawn 走 attach-or-spawn：先探 config-mode 实例，健康即附着，否则拉起自启产物（只 kill 自启 pid）。控制面拆分：Health/deps_probe 管运行时启停，ExtensionHost 管商店/版本/安装。

**Tech Stack:** Tauri 2 · Rust（`extension_host`、新 `managed_dependency` 模块、`ureq`/reqwest 下载）· Vue 3 · Feature Pack 壳（前置）· awam-file 隔离产物（PyInstaller onefile / zipapp）

**Depends on:**
- `docs/superpowers/plans/2026-11-12-feature-pack-shell.md`（贡献/IPC/启用）
- ADR 0019（白名单商店 / R3 隔离产物 / release 摘要）、ADR 0020（注册表 / 控制面拆分 / 运行时矩阵）、ADR 0013（attach-or-spawn）

**Spec:** `docs/glossary-feature-pack.md`（托管依赖词汇）· `docs/feature-pack-format.md`

## 实施进度（2026-11-18）

Rust 后端底座已实现并通过全量 `cargo test --lib`（420 passed / 0 failed，现有 tool_runtime 行为无回归）：

- ✅ Task 1 `managed_dependency/registry.rs` — 数据模型 + 原子读写 + `register_for_enabled_pack`（1:1 单写入口）+ `unregister_for_pack` + `prune_orphans`
- ✅ Task 2 `managed_dependency/orchestrator.rs` — `attach_or_spawn` + `stop_self_spawned`（只 kill 自启 pid，非自启拒绝）+ `find_pack_by_id`/`pick_sidecar_for_pack`；`tool_runtime/sidecar.rs::start_sidecar_with_probe` 已改走通用编排
- ✅ Task 3 `managed_dependency/store.rs` — 元清单解析 + `is_host_allowed`（拒绝伪装子域）+ `verify_sha256` + `download_and_verify`（不在 allow-list 不发请求）+ `materialize`
- ✅ Task 4 `managed_dependency/credential.rs`（注入式解析）+ `managed_dependency/compat.rs`（`ensure_declared_version_compat`，用 `versionCompat` 语义）
- ✅ Task 3+4 集成胶 `managed_dependency/provision.rs` — `provision_artifact`：门→allow-list→下载→校验→落盘（门/主机不过时**绝不发起 fetch**）

⏳ 未完成：Task 0 跨仓 release 产物（awam-file 打包 job + 元清单远端）；Task 6 e2e 与复制到 knowledge/tool-manager。

✅ Task 5 全部完成（2026-11-19，提交 e3b8ac8 / ba2ed43 / c57d727）：
- 后端接线：host 启用/卸载/启动加载三处 1:1 钩子（窄条件 + 尽力而为）+ 启动 `prune_orphans`；IPC `managed_dependency_list/ensure/stop`（内核注入式单测；stop 只 kill 自启 pid）。
- D 一次性投影（按用户决策）：`DependencyItem` 增 `mode/actionable/unavailableReason`；`projection.rs` 注册表为权威覆写，无记录依赖维持纯探测（config-mode 支撑现状等 pack 化）；注册表读失败仅告警。
- B+E 前端：HealthView 来源标签 + 启停按钮（动作后重拉健康）；`both` 隐身启动入口；`managedDep.*` 独立命名空间 ×5 locale。
- 验证：`cargo test --lib` 430 passed / 0 failed；`check-i18n` 1415 keys ×5 locale parity OK；`vue-tsc --noEmit` 0 错；`check-deps-surface` OK。
- 待 Task 5 接线后移除的 `#[allow(dead_code/unused_imports)]`：store 的真网络函数仍预留（前端商店入口未做），其余已消费。

---

## Global Constraints

- 术语：托管依赖 / config-mode / managed-mode / 隔离产物(R3) / 白名单商店；见 glossary
- **R3 直达**：不引入共享运行时；file 与 tool-manager 的 mcp 大版本互斥（`>=2.0` vs `<2.0`），各依赖必须自带隔离产物
- **knowledge-desensitization 不是托管依赖**（无常驻进程），不进注册表
- **停启权**：managed stop/restart 只作用于 AFM 自启且记录 pid 的进程；绝不碰非自启/远程实例
- **allow-list 只约束 managed-mode 产物来源**；config-mode 远程地址不受限
- 版本兼容门用**声明版**：取**元清单为每个 release 产物声明的 `version`** 落进 pack `versionCompat` 区间（字段原 `gatewayCompat`，Q19 更名）；config-mode 不 gate
- token 不进 Core settings / AI 中央配置；连接参数**直接读写注册表**——**软件未上线，旧 `AWAM_GATEWAY_URL`/`knowledge_service` settings 键直接删，无迁移/无回退读/无回滚棘轮**（Q16）
- 停启权限语义：`spawnSidecar` **隐含可停自启**，不新增权限（Q17c）
- **元清单地址可配置**（GitHub 默认，可切国内镜像）；**元清单不可达→商店不可用→对应依赖 managed 安装入口直接不显示**（不报错）；产物不可达仅该依赖失败可回退本机 zip（Q20=c）
- **1:1 强制度**：托管记录只能通过 pack 启用/禁用 API 建/销（单写路径），启动扫一遍兜底删孤儿（Q21）
- **双失败态映射（Q18）**：`DependencyItem.status` 枚举不扩，加正交字段 `unavailableReason ∈ {configOnly, managedOnly, both}`；`missing + actionable=false + both` 表达双不可用
- 提交信息中文；注释中文

---

## File map

| Path | Responsibility |
|------|----------------|
| `src-tauri/src/managed_dependency/mod.rs` | 注册表入口、命令门闩 |
| `src-tauri/src/managed_dependency/registry.rs` | `ManagedDependency` 记录读写、权威态 |
| `src-tauri/src/managed_dependency/orchestrator.rs` | pack 无关 attach-or-spawn（泛化自 `tool_runtime/sidecar.rs`）|
| `src-tauri/src/managed_dependency/credential.rs` | credential 来源解析（token 文件 / env / 无）|
| `src-tauri/src/managed_dependency/store.rs` | 元清单（地址可配置，GitHub→可切国内镜像）allow-list + release 资产下载 + 摘要校验 + 连不上降级 |
| `src-tauri/src/extension_host/types.rs` | `MarketplaceCatalog.fetches_remote` 受约束真值 |
| `src-tauri/src/deps_probe.rs` | `DependencyItem` 加 `mode`/`actionable`，读注册表投影 |
| `packs/file-gateway/pack.json` | awam-file 正式 Pack 清单 |
| `src/state/managedDependency/*` | FE 启停/健康视图 |
| `src-tauri/src/lib.rs` | 注册 managed_dependency commands |

**前置（跨仓，不在 AFM 内）：** awam-file 仓需先加 release 打包 job 产出带 SHA-256 的隔离产物；元清单 allow-list 远端需建立。两者未就绪时，AFM 侧 managed-mode 用「本机预放产物」占位跑通链路，不阻塞 P1–P4 的接口实现。

---

### Task 0: 跨仓前置（awam-file + 元清单）

- [ ] awam-file 加 PyInstaller/zipapp build → GitHub Release 资产 + 摘要（入口等价 `python -m awam_resource_gateway http`，默认 127.0.0.1:18787）
- [ ] 建「元清单」allow-list，指针指向该 release；列出被批准 pack、产物 release 指针、**产物 `version` 字段**（供 Q15 声明版门）；**地址在设置内可配**（GitHub 默认 → 预留国内镜像位）
- [ ] pack 清单写 `versionCompat{min,max}`（原 `gatewayCompat`，随字段更名同步改 Rust `GatewayCompat`→`VersionCompat` 与 `packs/tool-runtime/pack.json`），元清单 `version` 须落在区间内

### Task 1: `ManagedDependency` 注册表（权威态）

**Files:** Create `managed_dependency/{mod,registry}.rs`

**Interfaces:**
- `ManagedDependencyRecord { id, pack_id, mode: Config|Managed, base_url, credential: CredentialSource, spawned_pid: Option<u32>, health: HealthState, declared_version: String }`
- `CredentialSource { None, TokenFile(PathBuf), EnvVar(String) }`
- 落 `{app_data}/managed-dependencies/{id}.json`；与 `InstalledPackRecord` 1:1（`pack_id`），**只能由 pack 启用/禁用 API 建/销**（单写路径，Q21）

- [ ] 单测：记录读写 + 1:1 校验（无对应启用 pack 时拒绝 managed）+ 启动扫孤儿
- [ ] 红 → 实现 → 绿

### Task 2: 泛化 orchestrator（从单 pack 抽离）

**Files:** Create `managed_dependency/orchestrator.rs`；refactor `tool_runtime/sidecar.rs` 复用

- [ ] 把 `find_tool_runtime_pack`/`TOOL_RUNTIME_PACK_ID` 编排改为按 `pack_id` 取任意 pack 的 `SidecarContribution`
- [ ] attach-or-spawn：先 `probe(config.base_url)`，健康→附着；否则 spawn 自启产物，记录 pid
- [ ] stop：**只 kill `spawned_pid`**；非自启返回拒绝（单测断言）
- [ ] `tool_runtime` 改为调用通用 orchestrator（保留行为回归）

### Task 3: 商店下载 + release 摘要校验

**Files:** Create `managed_dependency/store.rs`；modify `extension_host/types.rs`

- [ ] `MarketplaceCatalog.fetches_remote` 由恒 false → allow-list 约束真值
- [ ] 下载 release 资产 → 校验附属 SHA-256 → `install_from_directory` 原子写（临时再 rename）
- [ ] **元清单不可达**：不显示对应依赖的 managed 安装/启停入口（隐身，不报错）；config-mode 不受影响；可用本机缓存的上次元清单
- [ ] **产物不可达**：仅该依赖失败，提示重试或回退本机 zip/目录；不牵连其他 pack；config-mode 地址不经此路

### Task 4: credential + 声明版兼容门

**Files:** Create `managed_dependency/credential.rs`

- [ ] 解析 `CredentialSource`（tool-manager token 文件/`AWAM_HTTP_TOKEN`、knowledge token、file 无）
- [ ] `version_in_range(元清单产物version, pack.versionCompat.min, pack.versionCompat.max)`；不匹配拒绝 managed spawn（沿用 tool-runtime 既有 `semver_compat`）
- [ ] config-mode 跳过此门；stop 权限走 `spawnSidecar` 隐含语义，不引入新权限

### Task 5: 控制面（Health 转可操作 + 配置切换）

**Files:** modify `deps_probe.rs` / `settings` / `knowledge_service`；Create `src/state/managedDependency/*`

- [ ] `DependencyItem` 增 `mode`+`actionable`+`unavailableReason∈{configOnly,managedOnly,both}`；状态源改读注册表投影（Provider/settings 降级）；status 枚举不扩（Q18）
- [ ] Health 页：未运行→[由 AFM 启动]、运行中→[停止]（仅自启 pid 可停）；`missing + actionable=false + both` 双不可用时隐身启动入口，不误导
- [ ] 直接切读写路径：`AWAM_GATEWAY_URL`/`knowledge_service` settings 键从代码删除（软件未上线，无兼容层、无回退读、无回滚）
- [ ] i18n：新增 key 落 **`managedDep.*` 命名空间** × 5 locale
- [ ] `check-feature-pack-shell.mjs` 静态门禁只加一条断言（deps_probe 从注册表读、`DependencyItem.mode` 存在）；托管正确性走 `managed_dependency` 单测 + 一条 R3 e2e，不进静态门禁

### Task 6: 端到端验证 + 复制路线

- [ ] 跑通 P1→P5：商店下载→managed spawn→Health 启停→config fallback→stop 只停自启
- [ ] 记录复制到 knowledge（零依赖，Node，最先）与 tool-manager（mcp 敏感，必隔离产物）的差异点
