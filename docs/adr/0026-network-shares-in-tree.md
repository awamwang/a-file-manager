# Network shares in the navigation tree

Add network probing and surface it in two places: a top-level **Network** root
in the This PC sidebar (`FolderTree.vue`) and the Guide network section
(`NavigatorGuide.vue`). This **amends the AIA-236 invariant** ("sidebar is This
PC + drive roots only; every other shell place stays on the Guide"), which
previously listed "no Network entry in the tree" as an *accepted* deviation from
Explorer in `docs/explorer-baseline.md`.

**Status**: accepted

## Why not `net view` (measured on host)

The first cut discovered computers with the legacy SMB1 browser (`net view`),
which is **wrong on Win10/11**:

- bare `net view` (workgroup enumeration) **hangs indefinitely** (SMB1 /
  Computer-Browser is disabled by default) — this is what made the Network node
  spin forever;
- `net view \\server` (shares of a *known* server) **works** over SMB2;
- Explorer's 网络 pane lists computers/devices via **WS-Discovery / Function
  Discovery**, which `net view` cannot see.

So the tree reads the **same source Explorer does** — the Shell Network folder.

## Decision

1. **A Network root, sibling to This PC — not nested under it.** Explorer puts
   Network at the top level, so the reversal restores Explorer parity. The tree
   now has two roots: `data-tree-root="this-pc"` (drives, unchanged) and
   `data-tree-root="network"`. OneDrive / Libraries / Recycle / folder entry
   sections stay banned from the tree — the amendment is *only* for Network.

2. **Discovery is its own lazy command, never on the awaited first paint.**
   `navigator_shell_places` keeps its FAST contract: `network` = connected
   `net use` shares only (`mapped_network_places`). LAN discovery is a separate
   `navigator_network_shares` command (`places::network_shares`) = connected
   shares ∪ discovered computers, de-duplicated by lower-cased path. The Guide
   and the tree both call `loadNetworkShares()` (single-flight like
   `loadDrives`) **after** first paint (`scheduleAfterPaint`) or on first
   expand, so discovery never blocks drive cards or the `loadNavigatorPlaces()`
   await chain. `FolderTree` additionally **pre-warms** the enumeration on
   `scheduleIdle` at startup, so the cold ~30s WS-Discovery is already done (or
   in flight) by the time the user opens 网络; shared `networkSharesLoading` /
   `networkSharesLoaded` refs keep the node's loading / empty state correct
   whether the load was triggered by the pre-warm or by the user.

3. **Two-level tree (Network → computer → share) without a new component.**
   The Network root lists computers as bare `\\server` nodes; the existing
   `FolderTreeBranch` expands any node by calling `navigator_list`. We make
   `navigator_list` special-case a **bare server root**: `list::bare_server_unc`
   detects `\\server` / `\\server\` (one component, no share) and returns the
   server's shares from `places::list_server_shares` (`net view \\server`,
   bounded by `win_process::output_capped`, 8s). `\\server\share` falls through
   to a normal `read_dir`. Result: computers, shares, and subfolders all expand
   uniformly, and the address bar / double-click behave identically.

4. **Latency + failure are honest, not faked.** Shell enumeration of
   `::{Network}` is slow on a cold cache (measured ~30s for the first probe on
   the authoring host) but it *terminates* — unlike `net view`. There is no hard
   timeout on the COM enumeration (it cannot be cleanly interrupted mid-call);
   it runs on the blocking pool, so the UI stays interactive and the node shows
   a loading hint until it returns. We do **not** add a `PlatformCapabilities`
   bit (that would force edits to the un-testable-from-Windows macOS adapter);
   absence of shares is the only signal and the empty-state copy still tells the
   user to type `\\server\share`.

5. **macOS / Linux opt out cleanly.** `Platform::network_computers` has a
   default empty impl; only the Windows adapter overrides it (Shell folder).
   macOS discovery stays mounted network volumes from `mount -p`
   (`parse_mount_network`); `list_server_shares` is a no-op there.

## Implementation

- Windows adapter: `platform/windows.rs::network_computers` enumerates
  `FOLDERID_NetworkFolder` via `SHGetKnownFolderIDList` → `SHBindToObject` →
  `IShellFolder::EnumObjects` → `SHCreateShellItem` →
  `IShellItem::GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING)`, STA via
  `ensure_com_sta`, freeing every PIDL / `PWSTR` with `CoTaskMemFree`. Reached
  through `Platform::network_computers` (default empty) + `platform::network_computers()`.
- `win_process::output_capped` — bounded console-tool run (kills the tree on
  timeout) for `net view \\server`.
- `navigator/places.rs` — `network_shares`, `mapped_network_places`,
  `discovered_network_places` (cfg split), `list_server_shares`, and three pure
  parsers (`parse_net_use` / `parse_net_view_shares` / `parse_mount_network`).
- `navigator/list.rs` — `bare_server_unc` + the server-root branch in
  `list_directory`.
- Frontend: `state/guide/index.ts` (`networkShares`, `loadNetworkShares`),
  `FolderTree.vue` (Network root, lazy-load on expand + auto-expand on a UNC),
  `NavigatorGuide.vue` (`networkRows` = connected ∪ discovered).

## Verification

- **Host probe** (proves the discovery source): `cargo test --lib
  platform::windows::tests::probe_network_computers -- --ignored --nocapture`
  returned `["DESKTOP-QJA9OJG"]` on the authoring machine — the same computer
  Explorer's 网络 pane shows, where bare `net view` had hung.
- **BE-CARGO** (ADR 0025): pure parsers + `bare_server_unc` have `#[test]`
  fixtures (UNC label, Disk-only share filter, network-fstype filter, server-root
  match). The live-network probe is `#[ignore]`d (network-dependent, not in the
  mandatory chain).
- **STATIC-GATE**: `check-shell-places.mjs` / `check-folder-tree.mjs` assert the
  tree owns `data-tree-root="network"` + `loadNetworkShares` while excluding
  OneDrive / Libraries; `check-navigator-nonblocking.mjs` now lists
  `navigator_network_shares` among the `spawn_blocking` commands.
- **Not verified here**: end-to-end GUI rendering, and the share listing against
  a live `\\server` (covered by the `net view \\DESKTOP-QJA9OJG` host run that
  returned `Code` / `Stock`, not by an automated test). macOS paths were not
  executed at all.

## Considered alternatives

- **Fold discovered shares into `navigator_list_drives` (`volumes()`)** —
  rejected: violates the `list_drives` test invariants for UNC rows and hides
  the Network/This PC distinction.
- **A dedicated `navigator_list_server_shares` command + a custom Network branch
  component** — rejected in favour of special-casing `navigator_list`: it keeps
  the tree, address bar, and double-click consistent with one code path and adds
  no component.
- **A hard timeout on the Shell enumeration** — rejected: it cannot be
  interrupted mid-COM-call without abandoning the only working discovery path;
  the probe is lazy + non-blocking instead.
