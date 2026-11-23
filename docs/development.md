<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# A File Manager — 开发指南

本文件收纳面向**开发者 / 维护者**的内容（从 README 拆出）。普通用户请见仓库根 [README.md](../README.md)；贡献流程见 [CONTRIBUTING.md](../CONTRIBUTING.md)；架构与功能全貌见 [a-file-manager-feature-map.md](./a-file-manager-feature-map.md)。

## 技术栈

- **后端**：Tauri 2（Rust，位于 `src-tauri/`），按域拆分：`navigator` · `file_ops` · `work_groups` · `projects` · `discovery` · `providers` · `favorites` · `settings` · `extension_host` 等。
- **前端**：Vue 3 + TypeScript，Fluent 设计语言（非 Sigma UI kit）。
- **平台**：Windows-first，同时构建 macOS。

## 本地运行

前置：

- **Node.js ≥ 20**（见 `.nvmrc` / `package.json` → `engines`）与 npm 10
- **Rust** stable toolchain（见 `rust-toolchain.toml`）
- Windows 需 **MSVC** build tools 与 Windows SDK（CI 用 `windows-latest`）

```bash
npm install
npm run tauri:dev      # 以开发模式运行桌面应用
```

纯 Web 校验（不启动 Tauri 外壳）：

```bash
npm run build          # vue-tsc 类型检查 + vite build
npm run test:deps-surface
npm run test:rust      # cargo test（Rust 后端）
```

## 环境变量与可选后端

- 本地知识库目录：设置环境变量 `AFM_KNOWLEDGE_ROOT` 或 `AWAM_KNOWLEDGE_ROOT`（不默认绑定本机盘符）。
- MCP 需自行导入，**不会**写入开发机的绝对路径。

### 依赖项目

| 项目 | 作用 |
|------|------|
| [awam-tool-manager](https://github.com/awamwang/awam-tool-manager) | 本机工具名单的统一入口（MCP）：查找文件 / 应用 / 游戏 / 快捷动作 / 包，或调用其它 MCP 后端的子工具 |
| [awam-file](https://github.com/awamwang/awam-file) | 文件网关（运行可选依赖；「设置 → 健康」可体检） |

## 质量门禁

改用户可见行为时，按 [CONTRIBUTING.md](../CONTRIBUTING.md) 要求在同一变更里附带基线测试并接入 test 链；行为约定与覆盖率分别登记于：

- 文件操作原子行为基线：[file-ops-scenario-baseline.md](./file-ops-scenario-baseline.md)
- 产品语义拍板：[explorer-baseline.md](./explorer-baseline.md)
- 覆盖率账本：[external-behavior-baseline-coverage.md](./external-behavior-baseline-coverage.md)

`npm test` 串联大量 `scripts/check-*.mjs` 静态门禁并在首个失败处停止；i18n key 集必须在全部 locale（en / zh-CN / zh-TW / ja / de / fr）保持一致（`npm run test:i18n` 会校验）。

## 性能与 UI 约定

性能与非阻塞 UI：[dev-performance-conventions.md](./dev-performance-conventions.md)（基线见 [perf-baseline.md](./perf-baseline.md)、[PERF.md](../PERF.md)）。

## 构建缓存维护

Cargo 不会自动回收 `src-tauri/target/`；AI/Core 两种 flavor 与 `build` vs `check --tests` profile 使其增长很快。只清理重复 fingerprint 与旧增量缓存（保留在用产物）：

```bash
npm run clean:target        # dry run：列出将被清理的内容
npm run clean:target:apply  # 将候选移入回收站；清空回收站释放磁盘
```

底层脚本支持 `--keep-incremental=N`（默认 `3`）调节每 crate 保留的最近增量缓存数量。清理后 `cargo build` 与 `cargo check --tests` 应仍命中缓存；若重新编译，先自回收站恢复再排查回归。

## 发版（维护者）

发版由 **Git tag** 指定**一个** semver，并在**同一个** GitHub Release 下挂多个包（主线版 Mainline / 精简版 Lite / Feature Pack），不是三个不同版本号。同一版本号下，**主线版（默认含 AI Add-on）是首推下载**，精简版为可选次级（见 [ADR 0024](./adr/0024-mainline-lite-positioning.md)）。

推送 `v*` 后，[`.github/workflows/release.yml`](../.github/workflows/release.yml) 会：

1. 按 tag 同步版本到 `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml`
2. 用 GitHub `generate-notes`（失败则回退 `git log`）写发版说明，并创建 **draft** Release
3. Matrix 构建 **主线版（ai）** 与 **精简版（core）** Windows NSIS；再打包 `packs/*/pack.json` 为 Feature Pack zip
4. 上传规范命名资产后 undraft（可在 workflow_dispatch 里关闭 publish）

| Asset | 含义 |
|---|---|
| `A.File.Manager_${ver}_x64-setup.exe` | **主线版 Mainline**（默认 features / `ai-addon`，= 普通正式版，**首推**） |
| `A.File.Manager_${ver}_x64-lite-setup.exe` | **精简版 Lite**（`VITE_AFM_AI_ADDON=0` + Cargo `--no-default-features`，可选次级） |
| `A.File.Manager_${ver}_${arch}[-lite].dmg` / `.app.zip` | **macOS**（Apple Silicon / Intel；主线版无后缀 / 精简版带 `-lite`） |
| `A.File.Manager_${ver}_feature-pack-*.zip` | Feature Pack（与版本无关，本机 zip 安装） |

版本号含 `-`（如 `v0.3.0-rc.1`）会标为 prerelease。Agent 流程见 [`.cursor/skills/afm-multi-pack-release/SKILL.md`](../.cursor/skills/afm-multi-pack-release/SKILL.md)。

```bash
# 先看计划（不改仓库）
npm run prepare-release -- 0.3.0 --dry-run
npm run build:release-artifacts -- --dry-run --version 0.3.0

# 同步版本、提交、打 annotated tag（不推送）
npm run prepare-release -- 0.3.0 --commit

# 推送提交与 tag → 触发多包 Release
git push origin HEAD && git push origin v0.3.0

# 一步推送
npm run prepare-release -- 0.3.0 --commit --push

# 仅本地打 Feature Pack zip
npm run package:feature-packs -- 0.3.0
```

也可在 **Actions → Release → Run workflow** 手动跑；可填 `version` / `packs`（`all` 或 `core,ai,feature-pack`）/ `publish`。

## 标识（Identity）

| | |
|---|---|
| Display | A File Manager |
| npm | `a-file-manager` |
| Tauri identifier | `com.a-file-manager.app` |
| Rust crate / lib | `a-file-manager` / `a_file_manager_lib` |
