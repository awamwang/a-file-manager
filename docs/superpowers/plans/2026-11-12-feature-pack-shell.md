# Feature Pack 壳 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 ExtensionHost 从内存 stub 升为可本机安装的 Feature Pack 壳：持久化、贡献点（侧栏/路由/命令/sidecar）、权限校验；不做远程商店、不加载任意动态库。

**Architecture:** Core 持有 `FeaturePackHost`（演进自 `ExtensionHost`）：从 zip/目录安装到 `%LOCALAPPDATA%/a-file-manager/feature-packs/{id}/`，读取 `pack.json`，校验 `apiVersion` 与权限，启用后把贡献写入可查询的 registry。前端 `contributionRegistry` 订阅已启用 Pack 的侧栏/路由/命令；路由组件通过 `entry` 映射到**已知内置模块 id**（如 `builtin:tool-runtime`）或 Pack 目录下预构建资源（`convertFileSrc`），v1 不执行第三方任意 JS 热加载。AI Add-on 仍用编译期风味，本计划不迁 AI。

**Tech Stack:** Tauri 2 · Rust · Vue 3 · TypeScript · 现有 `extension_host` · `ureq` 不在本计划使用

**Spec / glossary:** `docs/glossary-feature-pack.md` · ADR 0012 · 0014 · `GLOSSARY-MAP.md`

## Global Constraints

- 中文注释；保留原有注释；提交信息中文
- 不做远程 marketplace / 商店 UI
- 不承诺进程内加载任意 Rust 动态库
- GPL-3.0-or-later SPDX 头保持
- 重 I/O 走 `spawn_blocking` / 已有约定，不堵 UI
- 术语：Feature Pack、ExtensionHost（Pack 壳）、勿称商店

---

## File map

| Path | Responsibility |
|------|----------------|
| `src-tauri/src/extension_host/types.rs` | 扩展 `pack.json` / 贡献 / 权限类型 |
| `src-tauri/src/extension_host/pack_fs.rs` | 安装根路径、解压/拷贝、读写清单、持久化状态 |
| `src-tauri/src/extension_host/host.rs` | 安装/启用/卸载/列表 + 贡献聚合 |
| `src-tauri/src/extension_host/commands.rs` | Tauri IPC |
| `src-tauri/src/extension_host/mod.rs` | 模块导出 |
| `src/state/featurePack/contributions.ts` | FE 贡献缓存与 hydrate |
| `src/shell/sideNav.ts` | 合并 Pack 侧栏贡献 |
| `src/router/index.ts` | 合并 Pack 路由（仅 builtin 映射） |
| `src/commands/registry.ts` | 合并 Pack 命令 |
| `src/views/ExtensionsView.vue` | 本机安装 Pack UI（选目录/zip） |
| `packs/_template/pack.json` | 清单样例（文档用） |
| `scripts/check-feature-pack-shell.mjs` | 静态门禁（可选，Task 内决定） |

---

### Task 1: Pack 清单类型与权限

**Files:**
- Modify: `src-tauri/src/extension_host/types.rs`
- Modify: `src-tauri/src/extension_host/permissions.rs`
- Test: `src-tauri/src/extension_host/permissions.rs`（既有 `mod tests`）+ `types` 相关单测可放 `host.rs` tests

**Interfaces:**
- Produces:
  - `ExtensionPermission::SpawnSidecar`（parse: `"spawnSidecar"`）
  - `FeaturePackManifest`（serde camelCase）字段至少：`id`, `name`, `version`, `apiVersion`, `description`, `permissions: Vec<String>`, `contributions: FeaturePackContributions`, `gatewayCompat: Option<GatewayCompat>`
  - `FeaturePackContributions { sideNav, routes, commands, sidecars }`
  - `SideNavContribution { id, labelKey, to, order }`
  - `RouteContribution { path, name, entry }` — `entry` 形如 `builtin:tool-runtime`
  - `CommandContribution { id, labelKey, route? }`
  - `SidecarContribution { id, command, args, workingDirRel? }`
  - `GatewayCompat { minVersion, maxVersion }`（semver 字符串，本 Task 只存；比较在工具 Pack 计划）
  - 保留 `ExtensionManifestVNext` 兼容：`install_from_manifest` 仍可用；新路径走 `FeaturePackManifest`

- [ ] **Step 1: 写失败测试 — SpawnSidecar 解析**

在 `permissions.rs` tests 增加：

```rust
#[test]
fn parse_spawn_sidecar_permission() {
    assert_eq!(
        ExtensionPermission::parse("spawnSidecar"),
        Some(ExtensionPermission::SpawnSidecar)
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parse_spawn_sidecar_permission -q`  
Expected: 编译失败或 assert 失败（尚无 `SpawnSidecar`）

- [ ] **Step 3: 实现类型与 parse**

在 `ExtensionPermission` 增加 `SpawnSidecar`；`parse` 接受 `"spawnSidecar"` / `"spawn_sidecar"`。  
新增上述 `FeaturePack*` 结构体；`apiVersion` 常量可复用 `EXTENSION_HOST_API_VERSION`（`"vnext.1"`）或 bump 为 `"feature-pack.1"`——**本计划选定 bump 为 `feature-pack.1`**，旧 demo manifest 仍走 vnext 路径。

```rust
pub const FEATURE_PACK_API_VERSION: &str = "feature-pack.1";
```

- [ ] **Step 4: 跑测试通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parse_spawn_sidecar_permission -q`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/extension_host/types.rs src-tauri/src/extension_host/permissions.rs
git commit -m "feat(extension-host): 增加 Feature Pack 清单类型与 spawnSidecar 权限"
```

---

### Task 2: Pack 文件系统与持久化

**Files:**
- Create: `src-tauri/src/extension_host/pack_fs.rs`
- Modify: `src-tauri/src/extension_host/mod.rs`
- Test: `pack_fs.rs` 内 `#[cfg(test)]`

**Interfaces:**
- Produces:
  - `fn packs_root(app_data_dir: &Path) -> PathBuf` → `{app_data_dir}/feature-packs`
  - `fn pack_dir(root, id) -> PathBuf`
  - `fn read_pack_json(dir) -> Result<FeaturePackManifest, String>`
  - `fn write_enabled_state(root, id, enabled: bool) -> Result<(), String>` — 状态文件 `{root}/state.json`：`{ "enabled": { "<id>": true } }`
  - `fn load_state(root) -> Result<PackStateFile, String>`
  - `fn install_from_directory(app_data_dir, source_dir) -> Result<FeaturePackManifest, String>`：校验 `pack.json` 的 `apiVersion == feature-pack.1`，拷贝到 `pack_dir`（先写临时目录再 rename）
  - `fn uninstall_pack(app_data_dir, id) -> Result<(), String>`
- Consumes: Task 1 类型；`app_data_dir` 由调用方传入（与现有 settings/ai 路径解析方式对齐，查 `dirs`/`app.path()` 既有用法）

- [ ] **Step 1: 写失败测试 — 从目录安装并读回**

```rust
#[test]
fn install_from_directory_copies_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src-pack");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("pack.json"),
        r#"{"id":"demo.pack","name":"Demo","version":"0.1.0","apiVersion":"feature-pack.1","permissions":[],"contributions":{}}"#,
    )
    .unwrap();
    let app = tmp.path().join("appdata");
    let man = install_from_directory(&app, &src).unwrap();
    assert_eq!(man.id, "demo.pack");
    assert!(pack_dir(&packs_root(&app), "demo.pack").join("pack.json").is_file());
}
```

若项目无 `tempfile`，在 `Cargo.toml` dev-dependencies 加 `tempfile`，或用 `std::env::temp_dir` + 随机子目录并 `drop` 时清理。

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml install_from_directory_copies_manifest -q`  
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现 `pack_fs.rs` 并 `mod pack_fs`**

拒绝 `apiVersion` 不匹配；`id` 必须匹配 `^[A-Za-z0-9._-]+$`；拷贝时跳过符号链接逃逸（只拷普通文件/目录）。

- [ ] **Step 4: 测试通过**

Run: 同上 Expected: PASS；另加 `rejects_bad_api_version` 测试

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/extension_host/pack_fs.rs src-tauri/src/extension_host/mod.rs src-tauri/Cargo.toml
git commit -m "feat(extension-host): Feature Pack 本机目录安装与状态持久化"
```

---

### Task 3: Host 聚合贡献 + IPC

**Files:**
- Modify: `src-tauri/src/extension_host/host.rs`
- Modify: `src-tauri/src/extension_host/commands.rs`
- Modify: `src-tauri/src/lib.rs`（注册新 command）

**Interfaces:**
- Produces Tauri commands（camelCase 前端调用名与现有 `extension_host_*` 风格一致）:
  - `extension_host_install_pack(sourcePath: String) -> InstallResult` — 目录或 zip（zip：v1 若复杂可先 **仅目录**，zip 用 PowerShell/`zip` crate 第二小步；计划要求 v1 支持 zip——用 `zip` crate 解压到临时目录再 `install_from_directory`）
  - `extension_host_list_packs() -> Vec<InstalledPackRecord>`
  - `extension_host_set_pack_enabled(id, enabled) -> Result<(), String>`
  - `extension_host_uninstall_pack(id) -> Result<(), String>`
  - `extension_host_list_contributions() -> AggregatedContributions` — 仅 **Enabled** packs
- `InstalledPackRecord { id, name, version, state, permissions, installPath, lastError? }`
- Host 启动时 `load_from_disk(app_data_dir)`（在 `lib.rs` setup 注入 path）

- [ ] **Step 1: 单测 — 启用后 list_contributions 含 sideNav**

构造临时 appdata，安装带 `contributions.sideNav` 的 pack，`set_enabled(true)`，断言 `list_contributions().side_nav` 含该 id。

- [ ] **Step 2: 跑红**

- [ ] **Step 3: 实现 host + commands；`lib.rs` `generate_handler!` 注册**

保留旧 `install_from_manifest` 行为（内存 demo）以免打断 Extensions 现有 demo；新 Pack API 并行。

- [ ] **Step 4: `cargo test` 相关用例 PASS；`cargo check`**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(extension-host): Pack 启用态贡献聚合与 IPC"
```

---

### Task 4: 前端贡献注册表 + 壳合并

**Files:**
- Create: `src/state/featurePack/contributions.ts`
- Create: `src/state/featurePack/builtinEntries.ts` — `Record<string, () => Promise<Component>>`，v1 可先空对象或占位
- Modify: `src/shell/sideNav.ts`
- Modify: `src/router/index.ts`
- Modify: `src/commands/registry.ts`
- Modify: `src/App.vue` 或 `main.ts` — 启动 hydrate

**Interfaces:**
- Produces:
  - `async function hydrateFeaturePackContributions(): Promise<void>`
  - `function getSideNavExtras(): SideNavItem[]`
  - `function getRouteExtras(): RouteRecordRaw[]` — 仅当 `entry` 在 `builtinEntries` 有映射时注册
  - `function getCommandExtras(): ...` 对齐现有 command registry 形状

- [ ] **Step 1: 写 `scripts/check-feature-pack-shell.mjs`（或 viteless 节点断言）**  
断言 `sideNav.ts` 调用 `getSideNavExtras`；`contributions.ts` 存在 `hydrateFeaturePackContributions`。

- [ ] **Step 2: 跑红**

- [ ] **Step 3: 实现 hydrate：`invoke('extension_host_list_contributions')`，合并侧栏（按 `order`），路由 `router.addRoute`，命令 push 进 registry**

未启用 Pack 时与今天行为一致（无额外项）。

- [ ] **Step 4: 门禁脚本 PASS；`npm run build` 类型检查不炸（或 `vue-tsc`）**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: 前端合并 Feature Pack 侧栏/路由/命令贡献"
```

---

### Task 5: Extensions UI — 本机安装

**Files:**
- Modify: `src/views/ExtensionsView.vue`
- Modify: i18n `src/locales/zh-CN.json` / `en.json`（或项目既有 i18n 路径）

**Interfaces:**
- Consumes: Task 3 IPC
- UI：列表已装 Pack；「从文件夹安装」；启用开关；卸载；文案明确非商店、仅本机

- [ ] **Step 1: 用 `@tauri-apps/plugin-dialog`（若已依赖）或既有选目录 API 选文件夹；调用 `extension_host_install_pack`**

若无 dialog 插件，查现有打开目录用法（Navigator / Settings）复用。

- [ ] **Step 2: 手动验收清单写入 PR 描述：**  
  - Core 启动无额外侧栏  
  - 安装样例 pack（仅 sideNav 指向已有 `/settings` 作冒烟）后刷新出现项  
  - 禁用/卸载后消失  

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(extensions): 本机安装与管理 Feature Pack"
```

---

### Task 6: 样例 pack 与文档锚点

**Files:**
- Create: `packs/_template/pack.json`
- Modify: `docs/glossary-feature-pack.md`（若清单字段需一句说明，保持无实现细节堆砌）
- Optional: `docs/feature-pack-format.md` — 仅字段表（可从类型生成或手写精简版）

样例：

```json
{
  "id": "example.pack",
  "name": "Example Pack",
  "version": "0.1.0",
  "apiVersion": "feature-pack.1",
  "description": "Template only",
  "permissions": ["network", "spawnSidecar"],
  "gatewayCompat": { "minVersion": "0.0.0", "maxVersion": "99.0.0" },
  "contributions": {
    "sideNav": [{ "id": "example", "labelKey": "nav.example", "to": "/example", "order": 50 }],
    "routes": [{ "path": "/example", "name": "example", "entry": "builtin:example" }],
    "commands": [],
    "sidecars": []
  }
}
```

- [ ] **Step 1: 落盘模板 + 短格式说明**
- [ ] **Step 2: Commit**

```bash
git commit -m "docs: Feature Pack 清单模板与格式说明"
```

---

## Self-review

| Spec 点 | Task |
|---------|------|
| 本机 zip/目录安装 | 2–3、5 |
| ExtensionHost 为 Pack 壳 | 3、5 |
| 贡献：侧栏/路由/命令/sidecar 声明 | 1、3、4（sidecar 启停执行留给工具 Pack 计划） |
| 无远程商店 | 全局约束 + UI 文案 |
| 不热加载任意 Rust | 全局；`entry` 仅 builtin |
| 与 AI 双包分立 | 不改 flavor |

**Placeholder scan:** 无 TBD。zip 在 Task 3 用 `zip` crate 落地。  
**sidecar 真正 spawn：** 本计划只存贡献元数据；启动/附着在 `2026-11-12-tool-runtime-pack.md`。
