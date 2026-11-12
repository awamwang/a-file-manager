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

## Run

```bash
npm install
npm run tauri:dev
```

Web-only check:

```bash
npm run build
npm run test:rust
```

## Dev conventions

性能与非阻塞 UI：[docs/dev-performance-conventions.md](./docs/dev-performance-conventions.md)（基线见 [docs/perf-baseline.md](./docs/perf-baseline.md)、[PERF.md](./PERF.md)）。

## Release

发版由 **Git tag** 指定版本并触发打包。推送 `v*` 标签后，[`.github/workflows/release.yml`](.github/workflows/release.yml) 会：

1. 按 tag 同步版本到 `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml`
2. 用 GitHub `generate-notes`（失败则回退 `git log`）写发版说明
3. 在 `windows-latest` 打 Windows NSIS 安装包
4. 创建 / 更新对应 GitHub Release，并上传 `.exe` 产物

版本号含 `-`（如 `v0.2.0-rc.1`）会标为 prerelease。

```bash
# 推荐：同步版本、提交、打 annotated tag（不推送）
npm run prepare-release -- 0.2.0 --commit

# 推送提交与 tag → 触发 Actions
git push origin HEAD && git push origin v0.2.0

# 一步推送
npm run prepare-release -- 0.2.0 --commit --push
```

也可在 **Actions → Release → Run workflow** 手动跑；可填版本号，留空则用当前 `package.json` 的 version。

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
