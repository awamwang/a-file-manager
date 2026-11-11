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

Version sources (keep in sync): `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`.

Pushing a `v*` tag (for example `v0.1.0`) runs [`.github/workflows/release.yml`](.github/workflows/release.yml):

1. Generates release notes (GitHub auto notes, with `git log` fallback since the previous tag)
2. Builds the Windows NSIS installer on `windows-latest`
3. Creates / updates the matching GitHub Release
4. Uploads the installer as a release asset on that tag

```bash
# after bumping version fields and committing to the default branch
git tag v0.1.0
git push origin v0.1.0
```

You can also trigger **Actions → Release → Run workflow** manually (`workflow_dispatch`); that path uses `v` + `package.json` version as the release tag name.

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
