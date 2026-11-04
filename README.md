# A File Manager

Windows-first file manager — **Tauri 2** + **Vue 3** + TypeScript.

Greenfield product checkout. Bootstrap archive (read-only): sibling folder `a-file-manager-sigma`.

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

## Identity

| | |
|---|---|
| Display | A File Manager |
| npm | `a-file-manager` |
| Tauri identifier | `com.a-file-manager.app` |
| Rust crate / lib | `a-file-manager` / `a_file_manager_lib` |

S0: empty Rust core + `health_get` only. No Sigma UI kit.
