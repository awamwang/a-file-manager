# A File Manager

Windows-first file manager — **Tauri 2** + **Vue 3** + TypeScript.

Greenfield product checkout. Bootstrap archive (read-only): sibling folder `a-file-manager-sigma`.

## Download

Installers are published on [GitHub Releases](https://github.com/awamwang/a-file-manager/releases).

| Version | Notes |
|---|---|
| [v0.1.0](https://github.com/awamwang/a-file-manager/releases/tag/v0.1.0) | First tagged Windows NSIS build |

Grab the `.exe` asset from the release page, then install and run.

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

## Release

Version sources (keep in sync): `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`.

Pushing a `v*` tag (for example `v0.1.0`) runs [`.github/workflows/release.yml`](.github/workflows/release.yml):

1. Builds the Windows NSIS installer on `windows-latest`
2. Creates / updates the matching GitHub Release
3. Uploads the installer as a release asset

```bash
# after bumping version fields and committing
git tag v0.1.0
git push origin v0.1.0
```

You can also trigger **Actions → Release → Run workflow** manually (`workflow_dispatch`); that path uses `v__VERSION__` from `tauri.conf.json`.

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
