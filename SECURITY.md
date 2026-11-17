# Security Policy

Thanks for helping keep **A File Manager** and its users safe.

## Supported versions

We provide security fixes for the **latest released version** of A File Manager.
Older tags remain downloadable, but they are not actively patched — please update to
the newest [GitHub Release](https://github.com/awamwang/a-file-manager/releases/latest)
(`*-setup.exe`) when reporting or waiting on a fix.

| Version | Supported |
| ------- | --------- |
| Latest release | ✅ |
| Older releases | ❌ |

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security problems.

Report suspected vulnerabilities privately via one of:

- **GitHub Security Advisory** — use *“Report a vulnerability”* on the
  [security tab](https://github.com/awamwang/a-file-manager/security/advisories/new)
  of this repository (preferred).
- A private channel with the maintainer (`@awamwang`), describing the issue and how to
  reproduce it.

A good report includes: affected version, the platform (e.g. Windows 10/11), the build
flavor (**Core** or **AI**), and minimal steps or a proof-of-concept.

## Scope notes

A File Manager is a local desktop application (Tauri 2 + Vue 3). It integrates with the
OS shell and optional external tools (e.g. rclone, an `awam-file` gateway, QuickLook /
PowerToys Peek, IDEs). Issues that require you to **already** control the machine, or
that depend on a third-party tool, are generally out of scope unless our app mishandles
them.

## Timeline

We aim to acknowledge private reports within a few business days and will coordinate
disclosure timing with you before publishing a fix.
