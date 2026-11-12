# A File Manager

Windows-first file manager — **Tauri 2** + **Vue 3** + TypeScript.

Greenfield product checkout. Bootstrap archive (read-only): sibling folder `a-file-manager-sigma`.

## Download

Windows NSIS installers (`.exe`) are attached to each GitHub Release created from a `v*` tag.

| Get | Link |
|---|---|
| **Latest** (recommended) | [releases/latest](https://github.com/awamwang/a-file-manager/releases/latest) |
| **All versions** | [Releases](https://github.com/awamwang/a-file-manager/releases) |

On a release page, download the `*-setup.exe` asset, then install and run **A File Manager**. Older tags stay available on the Releases list after newer ones ship.

NSIS 安装包内嵌 **WebView2 bootstrapper**：目标机没有 WebView2 Runtime 时会在安装过程中静默安装。

## Requirements

| 阶段 | 依赖 | 声明位置 |
|------|------|----------|
| 构建 | Node.js ≥ 20、npm | `package.json` → `engines` / `.nvmrc` |
| 构建 | Rust stable | `rust-toolchain.toml` |
| 构建 | Windows + MSVC（CI 用 `windows-latest`） | GitHub Actions |
| 运行（必装） | Windows 10/11 + WebView2 Runtime | 安装包自动处理；设置 → 健康 可体检 |
| 运行（可选） | rclone、awam-file 网关、QuickLook / PowerToys Peek、IDE… | 设置 → 健康：缺什么、怎么补 |

本地知识库目录请设环境变量 `AFM_KNOWLEDGE_ROOT` 或 `AWAM_KNOWLEDGE_ROOT`（不再默认绑定本机盘符）。MCP 需自行导入，不会写入开发机绝对路径。

## Run

```bash
npm install
npm run tauri:dev
```

Web-only check:

```bash
npm run build
npm run test:deps-surface
npm run test:rust
```

## Dev conventions

性能与非阻塞 UI：[docs/dev-performance-conventions.md](./docs/dev-performance-conventions.md)（基线见 [docs/perf-baseline.md](./docs/perf-baseline.md)、[PERF.md](./PERF.md)）。

## Release

发版由 **Git tag** 指定**一个** semver，并在**同一个** GitHub Release 下挂多个包（Core / AI / Feature Pack），不是三个不同版本号。

推送 `v*` 后，[`.github/workflows/release.yml`](.github/workflows/release.yml) 会：

1. 按 tag 同步版本到 `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml`
2. 用 GitHub `generate-notes`（失败则回退 `git log`）写发版说明，并创建 **draft** Release
3. Matrix 构建 **Core** 与 **AI** Windows NSIS；再打包 `packs/*/pack.json` 为 Feature Pack zip
4. 上传规范命名资产后 undraft（可在 workflow_dispatch 里关闭 publish）

| Asset | 含义 |
|---|---|
| `A.File.Manager_${ver}_x64-setup.exe` | Core（`VITE_AFM_AI_ADDON=0` + Cargo `--no-default-features`） |
| `A.File.Manager_${ver}_x64-ai-setup.exe` | AI（默认 features / ai-addon） |
| `A.File.Manager_${ver}_feature-pack-*.zip` | Feature Pack（本机 zip 安装） |

版本号含 `-`（如 `v0.3.0-rc.1`）会标为 prerelease。Agent 流程见 [`.cursor/skills/afm-multi-pack-release/SKILL.md`](.cursor/skills/afm-multi-pack-release/SKILL.md)。

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

## License

[GPL-3.0-or-later](./LICENSE). Personal / private use is fine. Redistribution or commercial use must follow the GPL (including providing corresponding source).

## Identity

| | |
|---|---|
| Display | A File Manager |
| npm | `a-file-manager` |
| Tauri identifier | `com.a-file-manager.app` |
| Rust crate / lib | `a-file-manager` / `a_file_manager_lib` |

S0: empty Rust core + `health_get` only. No Sigma UI kit.
