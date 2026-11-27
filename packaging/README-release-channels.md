# Release channel manifest

`release-channels.json` is the **single source of truth** for how A File Manager
is distributed. The software portal at `software.awam.wang` (repo
`awam-software`) reads this file via `scripts/sync-release-channels.mjs` and
regenerates the product JSON's `installMethods` and per-version
`downloadSources` blocks.

## Layout

- **`identity`** — product name, GitHub repo, license.
- **`platformNotes`** — one-liner shown near the header of the site page.
- **`channels.<name>`** — each distribution channel with a `status`:
  - `live` — install method rendered normally.
  - `pending` — install method rendered but flagged as "即将上线" with the
    `pendingNote`. Command is still shown for copy-paste on the target day.
  - `disabled` — install method / download source omitted entirely.
- **`releases[]`** — per-version asset list. The site script maps each entry
  straight into `versions[]` with `downloadSources[]`.

## Channel-specific rules

- **winget** (`Awam.AFileManager`) — `status: pending` until the PR at
  microsoft/winget-pkgs merges AND the package appears in `winget search`.
  Flip to `live` afterwards; the site will stop showing the "即将上线" tag.
  Local manifests are produced by `scripts/package-winget.mjs` per ADR 0032.
- **Homebrew** (`awamwang/homebrew-tap`) — the release workflow keeps the cask
  in sync automatically; `status: live` reflects that. Ad-hoc builds are the
  reason we self-host the tap: official `homebrew/cask` rejects unsigned
  builds from Homebrew 5 (2026-09) onward.
- **OSS** — bucket `awam-software`, prefix `releases/a-file-manager`. Kept
  `disabled` until binaries are uploaded via `pnpm release:upload` in the
  awam-software repo. Flip to `live` and add `ossObjectKey` on each download
  source to publish the self-hosted link.

## Update flow

1. Publish a new version (GitHub Release + tap sync).
2. Edit `packaging/release-channels.json` — add the release entry, bump
   `identity`, flip channel statuses.
3. In the awam-software repo, run
   `pnpm release:sync-channels -- --slug a-file-manager`
   to regenerate `content/products/a-file-manager.json`.
4. `pnpm test && pnpm deploy:site`.

The sync script is idempotent and preserves any field it does not own
(summary, tags, cover, gallery, relatedLinks, …).
