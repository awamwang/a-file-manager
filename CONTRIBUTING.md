# Contributing to A File Manager

Thank you for your interest in improving **A File Manager**! This guide covers how to set
up a development environment and what we look for in a pull request.

## Code of conduct

By participating, you are expected to be respectful and constructive. Be kind to others.

## Project at a glance

- **Stack:** Tauri 2 (Rust backend in `src-tauri/`) + Vue 3 + TypeScript frontend.
- **Platform:** Windows-first (also builds for macOS). WebView2 Runtime is required at
  runtime on Windows.
- **License:** GPL-3.0-or-later.

## Development setup

Prerequisites:

- **Node.js ≥ 20** (see `.nvmrc` / `package.json` → `engines`) and npm 10
- **Rust** stable toolchain (see `rust-toolchain.toml`)
- On Windows: **MSVC** build tools and the Windows SDK

```bash
npm install          # install JS dependencies
npm run tauri:dev    # run the desktop app in dev mode
```

Web-only checks (no Tauri shell):

```bash
npm run build            # vue-tsc type-check + vite build
npm run test:rust        # cargo test for the Rust backend
```

## Testing & quality gates

This repo uses a large suite of static gate scripts plus unit tests. Before opening a PR,
make sure the relevant checks pass locally:

```bash
cargo test --lib                              # Rust backend tests
npm run build                                 # TypeScript / vue-tsc type-check
node scripts/check-i18n.mjs                   # i18n key parity across locales
npm test                                      # aggregated static gates (see package.json)
```

Notes:

- **i18n parity:** translations live in 5 locales (`zh-CN`, `en`, `de`, `fr`, `ja`) and the
  key sets must stay identical. If you add or rename a string, update **all** locales.
- `npm test` runs many `scripts/check-*.mjs` gates and stops at the first failure, so run
  individual gates while iterating to isolate new failures from pre-existing ones.

## Branches & commits

- Base new work on `master`.
- Use **Conventional Commits** (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`, …) with a
  short scope, e.g. `fix(navigator): keep selection after delete`.
- Keep PRs focused; describe *what* and *why*, not just *how*.

## Pull requests

- Fill in the PR template; link the issue you are addressing.
- Include before/after screenshots or a short recording for user-visible UI changes.
- For a new or changed user-visible feature, register it in `src/help/registry.ts`
  (help article, getting-started hint and, where useful, guided tour) so the
  Help & Guidance hub can discover it.
- Put all user-facing copy in i18n under `help.*` / `tour.*` (or the existing
  namespace) and add a real translation to **all five locales**: `en`, `zh-CN`,
  `ja`, `de`, `fr`. Never leave a locale falling back to English.
- Prefer user language over developer terms (no internal IDs, ADR numbers or
  code names in descriptions, hints, tooltips or confirmations).
- Ensure CI is green. A maintainer will review and squash-merge when ready.

## Reporting bugs & requesting features

Please use the issue templates: [new bug report](.github/ISSUE_TEMPLATE/bug_report.yml) or
[feature request](.github/ISSUE_TEMPLATE/feature_request.yml). Include the version,
platform, build flavor (Core / AI), and reproduction steps.

## Releasing (maintainers)

Releases are cut from a single `v*` Git tag; see the **Release** section of the
[README](./README.md) for the multi-pack (Core / AI / Feature Pack) workflow.
