<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-86 关闭确认交接

## 行为

关闭主窗口（标题栏 × / `CloseRequested`）时：

| `core.closeBehavior` | 行为 |
| --- | --- |
| `ask`（默认） | Fluent 对话框：隐藏到托盘 / 退出 / 取消；可选「记住此选择」 |
| `tray` | `window.hide()` → 现有系统托盘（与自启动隐藏同一托盘） |
| `quit` | `app_quit` → `app.exit(0)`（与托盘菜单 Quit 同路径） |

Settings → Core「关闭窗口时」可改回「每次询问」。

## 文件

- `src/state/closeBehavior.ts`
- `AppShell.vue`：`onCloseRequested` + `#close-confirm-dialog`
- `settings/defaults.rs`：`core.closeBehavior` enum
- `app_quit` command in `lib.rs`

## 检查

`npm run test:close-confirm` · `test:i18n` · `test:autostart`
