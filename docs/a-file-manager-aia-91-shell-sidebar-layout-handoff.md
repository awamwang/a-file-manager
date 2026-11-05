<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-91 侧栏顶满标题栏 / Navigator chrome 内收

## 问题

导航操作条与标签横跨整窗，左侧菜单被挤到下方，未顶到自定义标题栏。

## 布局

```text
titlebar（全宽）
body
  sidenav（全高，贴标题栏下）
  workspace（右侧列）
    navigator-chrome / filter / tabstrip（仅 isNavigatorRoute）
    main.content → router view
statusbar（全宽）
```

## 改动

- `AppShell.vue`：chrome 移入 `.workspace`；新增 `.workspace` 纵向 flex
- 检查：`npm run test:shell-sidebar-layout` · `test:navigator-chrome-layout` · `test:navigator-filter`
