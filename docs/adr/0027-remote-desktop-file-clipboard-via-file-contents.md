# Remote-desktop file clipboard goes through FileDescriptor/FileContents, not a bare path

AFM's file clipboard was built only to interoperate with File Explorer on the **same** machine:
copy writes `CF_HDROP` + `Preferred DropEffect`, paste reads `CF_HDROP` and then copies from those
paths on the local filesystem (`src/state/fileClipboard.ts` → `file_ops_copy`). That contract
silently breaks across a remote-desktop broker such as RustDesk. A broker speaks MS-RDPECLIP
(Cliprdr): the clipboard carries `CFSTR_FILEDESCRIPTOR` + lazily-rendered `CFSTR_FILECONTENTS`
(pulled per file), and the `CF_HDROP` entry it leaves on the local clipboard points at the **far**
machine, so a path-based paste resolves to nothing. This ADR fixes the contract for both
directions so a remote tool sees real file bytes rather than a meaningless path string.

**Status**: accepted (read + write landed on `feat/remote-clipboard`; two-machine round-trip
verified by the design owner against a real RustDesk session — both AFM→remote paste and
remote→AFM paste carry real file contents. Code compiles; descriptor parsing, the fallback
decision, and both OLE clipboard round-trips (write and read) are covered by tests, the two
round-trips being gated).

## Decision

1. **The file clipboard contract is "contents, not paths" whenever the two sides are not the
   same filesystem.** Any consumer that cannot resolve the source path locally (remote desktop,
   some VNC/RDP clients) MUST be able to get the bytes through
   `CFSTR_FILEDESCRIPTOR` (name/size) + `CFSTR_FILECONTENTS` (data, per index). `CF_HDROP` alone
   is not a valid copy payload across a machine boundary.

2. **Write side (AFM as copy source).** `set_shell_clipboard` publishes the Shell file data
   object (`SHCreateShellItemArrayFromIDLists` → `BHID_DataObject`, then `OleSetClipboard`)
   instead of hand-writing a `CF_HDROP` blob. The Shell object already serves `CF_HDROP` *and*
   `CFSTR_FILEDESCRIPTOR`/`CFSTR_FILECONTENTS` with lazy rendering, which is the same object
   Explorer relies on (`patches/drag` FIL-38 — "don't hand-roll `IDataObject` again"). Preferred
   DropEffect is applied onto the object to preserve cut vs copy. If the Shell object cannot be
   built for some item, it falls back to the legacy hand-written `CF_HDROP` so same-machine paste
   never regresses
   (`src-tauri/src/file_ops/clipboard.rs` → `set_shell_clipboard`, `publish_shell_data_object`).

3. **Read side (AFM as paste target).** `get_shell_clipboard` keeps the `CF_HDROP` fast path when
   the copied paths exist locally (unchanged same-machine behavior). When the path list is empty,
   or contains an entry that does **not** resolve on this host, it re-reads through the clipboard
   data object: enumerate `CFSTR_FILEDESCRIPTOR`, then per file call
   `IDataObject::GetData(CFSTR_FILECONTENTS)` with `FORMATETC.lindex = i` (the index selects the
   file — not `DVTARGETDEVICE`), materialize the `TYMED_HGLOBAL`/`TYMED_ISTREAM` bytes into a
   fresh temp directory, and hand those local paths back as a forced `copy`. The existing
   `file_ops_copy` pipeline then pastes them unchanged
   (`src-tauri/src/file_ops/clipboard.rs` → `get_shell_clipboard`,
   `materialize_filecontents_via_ole`, `read_filegroup_descriptors`).

4. **`FILEDESCRIPTORW`/`FILEGROUPDESCRIPTORW` are `#[repr(packed)]`.** Fields are read as
   unaligned bytes via `std::mem::offset_of!`, never by borrowing a packed field (a packed-field
   reference is UB, and the compiler rejects it with E0793). The array offset and per-field
   offsets are derived from the real type layout, not hardcoded.

5. **Plain-text copy is unaffected.** `CF_UNICODETEXT` (`src-tauri/src/clipboard.rs`) already
   flows through RustDesk in both directions; this ADR concerns only the file payload.

6. **`OleSetClipboard` / `OleGetClipboard` require `OleInitialize`, not just `CoInitializeEx`.**
   The clipboard commands run on Tauri worker threads whose apartment is only COM-initialized;
   without OLE init the call fails and the write side silently degrades to a bare `CF_HDROP`
   (no contents). Both backends therefore call a shared `ensure_ole()`
   (`CoInitializeEx` + `OleInitialize`, idempotent, never uninit) before touching the OLE
   clipboard (`src-tauri/src/file_ops/clipboard.rs` → `ensure_ole`).

## Considered Options

- **Keep hand-writing `CF_HDROP` and additionally emit `CFSTR_FILECONTENTS` by hand.** Rejected:
  requires owning a marshaled `IDataObject` for cross-process lazy reads (the very thing FIL-38
  removed from `patches/drag`); the Shell object gives the same formats for free and is what
  Explorer itself uses.
- **Paste by resolving the far-machine path verbatim.** Rejected: the path does not exist locally;
  this is precisely the silent failure being fixed.
- **Route files through a broker-specific side channel (RustDesk "file transfer") instead of the
  clipboard.** Rejected: out of AFM's control and not portable across brokers; the goal is to
  make AFM's *clipboard* behave, so it interoperates with any Cliprdr client.

## Consequences

- Adds the `Win32_System_Ole` dependency; pulls OLE clipboard interop (`OleGetClipboard` /
  `OleSetClipboard`) into the `file_ops` clipboard backend. The capability bit
  (`shell_clipboard`) and the `ShellClipboard` DTO are unchanged, so the frontend and i18n do not
  move.
- Also adds a direct `windows-core = "0.62"` (cfg(windows)) dependency so the test stub's
  `#[implement(IDataObject)]` resolves: the macro expands to bare `windows_core::` paths and needs
  the crate in the extern prelude (matching the version `windows 0.62` already pulls). Note the
  0.60.2 idiom differs from the older `implement!` macro: implement on `RemoteContentsStub_Impl`
  (not the user struct), methods are **safe `fn`** with inner `unsafe {}` for pointer derefs, and
  `OleSetClipboard` rejects an `EnumFormatEtc` that returns `E_NOTIMPL` — so the stub builds a real
  enumerator via `SHCreateStdEnumFmtEtc`.
- **Test coverage.** Same-machine copy/paste and the descriptor-parsing / fallback-decision logic
  are covered by always-on `#[cfg(windows)]` unit tests (`cargo test --lib file_ops::clipboard`).
  Two **gated** (`#[ignore]`) round-trip tests drive the real OLE clipboard, run explicitly with
  `cargo test --lib file_ops::clipboard -- --ignored`:
  - `gated_write_side_publishes_readable_file_contents` — copies a real temp file through
    `set_shell_clipboard`, then reads `CFSTR_FILECONTENTS` back off the OLE clipboard to assert the
    real bytes are exposed (the write-side contract a broker depends on).
  - `gated_read_side_materializes_remote_file_contents` — installs a synthetic `IDataObject` that
    serves only `CFSTR_FILEDESCRIPTOR` + `CFSTR_FILECONTENTS` (no resolvable `CF_HDROP`), then
    calls `get_shell_clipboard` and asserts the bytes are materialized to a temp dir as a forced
    `copy`. This is the read-side end-to-end proof (foreign-object path), independent of a second
    machine.
  A process-global `CLIPBOARD_TEST_LOCK` mutex serializes both (safe under cargo's default
  multi-threaded run), and a `ClipboardSnapshot` guard flushes our ownership, empties the
  clipboard, and restores the prior text on drop, so the tests never leave synthetic payloads on
  the user's clipboard.
- Remote **directories** are handled as flat file entries: the directory marker is skipped and
  only file records are materialized. Recursive directory clipboard payloads (which Cliprdr
  itself supports unevenly) are out of scope for v1.
- Aligns with ADR 0023 (providers behind `ClipboardProvider`): the new behavior stays inside the
  Windows backend; non-Windows hosts register no provider and keep degrading without panicking.
- Touches the regression-baseline surface from ADR 0025: the copy/paste behavior execution is
  BE-CARGO (temp-dir materialization), while the `transferEntries` orchestration branch remains
  a grep gate, consistent with that ADR's "invoke/reactivity → grep" rule.
