<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-100 重启恢复导航器标签

## 问题

Open State 已写入部分工作组，但关闭/托盘/隐藏时防抖未刷盘，或启动后空条被默认路径覆盖，导致重启丢标签。

## 修复

- `flushOpenStatePersist()`：取消防抖并立即 `work_groups_save_open_state`
- 关闭确认 / 托盘 / 退出 / `visibilitychange` / unmount 时刷盘
- bootstrap：有保存标签则 `navigateTo(..., syncOpenState:false)`；仅空条时 `loadDefaultPath` 并刷盘

## 检查

`npm run test:openstate-restore` · `npm run test:open-state`
