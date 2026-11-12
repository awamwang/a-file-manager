---
name: afm-multi-pack-release
description: >-
  Runs A File Manager multi-pack GitHub releases (one vX.Y.Z tag with Core NSIS,
  AI NSIS, and Feature Pack zip assets). Use when bumping version, tagging,
  preparing a release, building release artifacts, uploading with gh, dry-running
  packs, or when the user mentions prepare-release, build-release-artifacts,
  feature-pack zip, FIL-63, or multi-pack release.
---

# AFM 多包发版（Core / AI / Feature Pack）

同一 **semver**、同一 Git tag / GitHub Release；挂多个资产，**不是**三个不同版本号。

## 何时使用

- 用户要发下一版、打 `v*` tag、改 `prepare-release` / `release.yml`
- 需要本地 dry-run、只打 Feature Pack、或排查 Release 缺包

## 资产命名

| 种类 | 文件名 |
|------|--------|
| Core | `A.File.Manager_${ver}_x64-setup.exe` |
| AI | `A.File.Manager_${ver}_x64-ai-setup.exe` |
| Feature Pack | `A.File.Manager_${ver}_feature-pack-${dir}.zip` |

`packs/_template` 不打包；`packs/*/pack.json` 才会进 zip。zip 内根目录含 `pack.json`（可本机安装）。

## 推荐流程（下一版）

1. 干净 `master`，确认不重发已有 tag（如勿再推 `v0.2.0` 除非刻意重做）。
2. Dry-run 计划：

```bash
npm run prepare-release -- 0.3.0 --dry-run
npm run build:release-artifacts -- --dry-run --version 0.3.0
```

3. 同步版本 + annotated tag（先不推也可）：

```bash
npm run prepare-release -- 0.3.0 --commit
git push origin HEAD && git push origin v0.3.0
```

4. 推送 tag 后 Actions（`.github/workflows/release.yml`）会：
   - 建 **draft** Release + 自动 notes
   - matrix 构建 Core / AI NSIS，改名为上表规范名并 `gh release upload`
   - 打包 Feature Pack zip 并上传
   - 全部成功后 undraft（`workflow_dispatch` 可将 `publish=false` 留 draft）

5. 本地只验证 Feature Pack / 命名（不跑完整 NSIS）：

```bash
npm run package:feature-packs -- 0.3.0
npm run test:multi-pack-release
```

## 输入

| 输入 | 说明 |
|------|------|
| version | `0.3.0` / `v0.3.0`；含 `-` 视为 prerelease |
| packs | `all`（默认）或 `core,ai,feature-pack` 子集 |
| `--dry-run` | `prepare-release` / `build-release-artifacts` / `package:feature-packs` |
| `--sync-version` | 仅 `build-release-artifacts`：构建前写入版本三文件 |
| workflow_dispatch | `version`、`packs`、`publish` |

## 脚本地图

| 脚本 | 作用 |
|------|------|
| `scripts/sync-version.mjs` | 同步 package / tauri / Cargo 版本 |
| `scripts/prepare-release.mjs` | bump → 可选 commit → annotated tag |
| `scripts/build-flavor.mjs` | `ai\|core`；`--bundle` 打 NSIS |
| `scripts/package-feature-packs.mjs` | Feature Pack → `dist/release/*.zip` |
| `scripts/build-release-artifacts.mjs` | 本地编排多包到 `dist/release/` |
| `scripts/release-assets.mjs` | 命名与 pack 发现 |
| `scripts/check-multi-pack-release.mjs` | 门禁（无真实打包） |

## Secrets / 签名

- CI：`GITHUB_TOKEN`（`contents: write`）即可创建 Release 与上传。
- 本地上传：`gh auth login`；或环境变量 `GH_TOKEN` / `GITHUB_TOKEN`。
- **当前无代码签名证书**；NSIS 为未签名包。若日后加签名，在 `build-app` 注入证书 secret，**不要**把证书写入仓库。

## 失败模式

| 现象 | 处理 |
|------|------|
| 本地已有同名 tag | 换版本或删本地 tag；勿强推覆盖已发布 tag |
| 工作区脏且未 `--commit` | 只改版本文件；建议干净后 `--commit` |
| 某一 flavor / feature-pack job 失败 | Release **保持 draft**；修完后可 `workflow_dispatch` 重跑或本地 `gh release upload … --clobber` |
| 缺少 `packs/*/pack.json` | Feature Pack job 失败；先补清单 |
| 本机无 `tar`/`gh` | Windows 10+ 自带 bsdtar；安装 [GitHub CLI](https://cli.github.com/) |
| 误想重发 0.2.0 | 默认不要；用 dry-run 验证流程，正式发下一版 |

## Agent 约束

- 默认 **dry-run** 先跑；用户未明确要求时不要 `git push` tag、不要正式 publish。
- 不要把 Core / AI / Feature Pack 拆成三个 semver tag。
- 提交信息用中文；保留原有注释。
- 详细说明见仓库 README「Release」与 Project 文档 `afm-multi-pack-release.md`（若有）。
