# Agent instructions

Operational context for AI coding agents working in this repo. It complements
[README.md](./README.md) and [CONTRIBUTING.md](./CONTRIBUTING.md) rather than repeating
them: setup, dev/build commands, the gate suite overview, commit conventions and the
branch model already live in those files. This file holds what an agent cannot recover by
reading a manifest — the unwritten conventions, the gotchas that burn time, and the
boundaries. Read it before changing code.

## Read this first (load the one that matches the task)

| When you are about to… | Read |
| --- | --- |
| Set up, build, or pick a test command | [CONTRIBUTING.md](./CONTRIBUTING.md) §Development setup, §Testing |
| Use or coin a domain term (Pack, Provider, Navigator, Work Group, flavor, seam, gate) | [GLOSSARY.md](./GLOSSARY.md) and [GLOSSARY-MAP.md](./GLOSSARY-MAP.md) — match the context row before naming |
| Change something a prior decision locked in (search, packs, MCP distribution, knowledge service, voice) | `docs/adr/` — find the relevant numbered ADR; do not silently contradict it |
| Locate which file owns a feature | [docs/a-file-manager-feature-map.md](./docs/a-file-manager-feature-map.md) |
| Add or change a user-visible feature / Tauri command | §Testing new / changed features below + [ADR 0025](./docs/adr/0025-external-function-regression-baseline.md) — every external behavior must carry a baseline test |
| Touch list/render performance or shell icon loading | [docs/dev-performance-conventions.md](./docs/dev-performance-conventions.md), [docs/shell-icon-thumbnail-loading.md](./docs/shell-icon-thumbnail-loading.md) |
| Change AI skills / MCP distribution | [docs/ai-skills-mcp-distribution-design.md](./docs/ai-skills-mcp-distribution-design.md) |

## Verification order

`package.json` scripts are the single source of truth for commands; the names below are
real script keys or the documented cargo entry. Run the smallest check that covers the
change, not everything, while iterating:

- Rust only (`src-tauri/`): `cargo check --tests --manifest-path src-tauri/Cargo.toml` for a
  fast pass, then `npm run test:rust` (or `cargo test --lib`) for behaviour.
- Frontend types / templates: `npm run build` (runs `vue-tsc --noEmit` then `vite build`).
- A specific feature's static gate: `node scripts/check-<feature>.mjs`.
- The whole gate suite: `npm test` — a chain that **stops at the first failure**, so isolate
  your own change by running its gate(s) directly (see gotcha below).

A change is verified when the checks that cover it pass — backend tests when you touched
`src-tauri/`, `npm run build` for any frontend/template change, and the matching
`check-*.mjs` for the feature you edited.

## Testing new / changed features

The point of this repo's gate suite is **regression baseline**: a user-visible feature must not
silently break because of an unrelated edit. So **every new or changed user-visible behavior or
Tauri command ships with a matching baseline test, in the same change.** The full rationale is
[ADR 0025](./docs/adr/0025-external-function-regression-baseline.md); this is the operational
version. Classify the behavior into one of three layers (a behavior can need more than one):

1. **Backend behavior** (file/IO/state that produces the user-visible result) → add a cargo
   `#[test]` under `src-tauri/src/**` that really executes it in a temp dir or pure compute. It
   runs through `npm run test:rust`. Example: `rename_work_group_in_list` / `delete_work_group_in_list`.
2. **Frontend logic** that is pure, **or made pure by a behavior-preserving extraction** → add
   `scripts/<name>-checks.ts` and `spawn` it from a `scripts/check-<name>.mjs`. If the orchestration
   lives in a `.vue`, extract the pure part to a standalone module first. Example: `folderPathsForWorkGroup`
   → `src/components/navigator/workGroupPaths.ts`.
3. **Surface / wiring** (command registration, class names, i18n keys) → a static grep gate in
   `scripts/check-<name>.mjs`. For any Tauri command the gate **must** assert FE↔BE **name parity**:
   `invoke("X")` and `#[tauri::command] pub fn X` share the exact name (rename = silent break).

Inclusion is by **chain, not by call-site**: if a command/facet sits on a path that ends in a
user-visible result, it needs coverage even when nothing calls it from the frontend (e.g. the
managed-dependency feed drives download/install → in scope). An `#[ignore]`d test does **not**
count as coverage.

Hard rules (see ADR 0025):
- **No e2e.** Do not add Playwright / app-driving / browser tests. This is a native windowed app;
  behavior is covered by cargo (real execution) + FE pure units (headless logic).
- **No mandatory network/flaky test in `npm test`.** Cover the deterministic part offline — inject
  the fetch/IO seam, use temp dirs (see `managed_dependency/provision.rs`). Keep a real-network test
  `#[ignore]`d; the `&&` chain halts on the first failure.
- **Don't fake a FE behavior test** for reactivity/`invoke`-fused code that can't run headless — keep
  pos+neg grep assertions and let the backend cargo contract hold the behavior.

After writing the test, **wire it in** (an orphan gate never runs): add the `test:<name>` key in
`package.json` **and** insert `npm run test:<name>` into the aggregated `test` chain (H8 once found
5 gates that were defined but absent from the chain). Then run your gate green in isolation, and
add/refresh its row in the coverage ledger
[docs/external-behavior-baseline-coverage.md](./docs/external-behavior-baseline-coverage.md).
Removing, weakening, or `#[ignore]`-ing an existing gate is an **Ask-first** boundary (§Boundaries).

## Where to edit

- **New or renamed UI string** → add the key to **every** locale file under
  `src/i18n/locales/` (the set grows over time; `npm run test:i18n` enforces key
  parity across whatever locales exist).
- **New setting / changed settings shape** → `src-tauri/src/settings/` (`defaults.rs`,
  `types.rs`) plus a migration in `migrate.rs` (see gotcha).
- **Shared visual primitives** (chips, banners, accent edges, page chrome) →
  `src/styles/fluent-tokens.css`; the `fluent-*` class names are asserted by gates.
- **Open-with / IDE tools** → backend `src-tauri/src/open_with/` (`ide_list_apps`) and the
  shared frontend client under `src/state/`.
- **Preview / QuickLook** → `src-tauri/src/preview_bridge/`; treat `quicklook_available` as
  the one definition of "is it running" that both the bridge and the health probe share.
- **AI agent targets** → `src-tauri/src/ai/agents.rs`.

Reach for the feature map table when a row above does not name the file.

## Gotchas (read before refactoring)

- **Gates grep source symbols.** Many `scripts/check-*.mjs` assert an *exact string* read
  out of a `.vue`, `.rs`, or locale file. Example: `check-open-project-folder.mjs` asserts
  `async function openWorkspaceFolder` and its `@click` handler; `check-fluent-p1-color.mjs`
  and `check-fluent-p2-primitives.mjs` assert class names such as `fluent-accent-edge--down`,
  `fluent-chip--down`, and the required primitives (`fluent-page`, `fluent-banner`). When you
  rename a function, swap a CSS class, or move a string, **update the matching gate needle in
  the same change** — search `scripts/` for the symbol you touched. The gate must fail because
  behaviour regressed, not because a label moved.
- **A settings shape change is a migration.** Edit `SETTINGS_SCHEMA_VERSION`
  (`src-tauri/src/settings/types.rs`) and add the upgrade step in `migrate.rs` whenever the
  settings document gains, renames, or drops a key. `migrate.rs` rejects a stored version
  newer than supported, so a missing migration means user settings silently fail to load.
- **Isolate your gate failure from history.** Because `npm test` halts on the first failing
  gate, run the gates your change actually touches so a pre-existing failure is not mistaken
  for yours, then run the full suite once near the end.
- **Invoke Node tools from the repo.** On this machine `npx <tool>` can fail by shelling out
  to `wsl.exe` (no WSL distro). Run gates and the type-checker through the repo's
  `node_modules` (e.g. `node scripts/check-*.mjs`, or the `npm run …` script) rather than
  `npx`.

## Boundaries

**Always**
- Run the checks that cover the change and see them pass before calling the work done.
- Update every dependent locale and every dependent gate needle together with the change.
- Stage only the files you modified for the current task.

**Ask first**
- Bumping `SETTINGS_SCHEMA_VERSION` or editing `migrate.rs` beyond an additive change.
- Removing, weakening, or skipping a `check-*.mjs` gate, or loosening an i18n/colour token.
- Anything touching version/release plumbing (`sync-version`, `prepare-release`,
  `build:ai` / `build:core` flavors, packaging).
- Deleting or reverting files, or undoing a `git merge`/`rebase` with `--abort`.

**Never** (paired with the positive action)
- Push to any remote unless the user explicitly asks — by default commit locally and report
  what is staged.
- Stage someone else's in-progress edits: when the working tree mixes your change with
  unrelated work, split by hunk and commit only your own; never `git add -A` over their WIP.

## Commit style

Follow CONTRIBUTING.md: Conventional Commits with a short scope (`fix(navigator): …`), on top
of `master`, no AI attribution in the message. Write the subject in the user's language when
they ask for it; keep code comments and repo-facing docs in English.

## Agent skills

### Issue tracker

GitHub Issues on `awamwang/a-file-manager` via `gh`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five roles (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Root `GLOSSARY-MAP.md` points at per-area glossaries; ADRs in `docs/adr/`. See `docs/agents/domain.md`.
