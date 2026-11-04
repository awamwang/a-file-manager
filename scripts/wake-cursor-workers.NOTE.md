# Wake Cursor workers

Canonical wake scripts live at the Projects root (not only in this repo):

- `G:\Projects\wake-cursor-workers.ps1`
- `G:\Projects\wake-cursor-workers.cmd` (ExecutionPolicy Bypass launcher)

Related: `G:\Projects\start-awam-workers.bat` (Software Kit / THS set).

## Usage

```bat
G:\Projects\wake-cursor-workers.cmd
```

Status only:

```bat
G:\Projects\wake-cursor-workers.cmd -StatusOnly
```

Also wake the awam-software / kit / ths-linker set:

```bat
G:\Projects\wake-cursor-workers.cmd -IncludeAwamSet
```

## Exact CLI on this machine

Prefer `agent` on PATH. Fallback resolves `cursor-agent.cmd` under Cursor’s agent-cli versions dir.

Start command: `agent worker start` (cwd = checkout).
