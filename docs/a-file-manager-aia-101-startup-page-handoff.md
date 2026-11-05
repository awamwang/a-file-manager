<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-101 启动页

Settings `core.startupPage`：`navigator` | `projects` | `discovery`（默认 navigator）。

冷启动时 `App.vue` 在 `/`→`/navigator` 之后按设置 `router.replace`。

## 检查

`npm run test:startup-page` · `npm run test:i18n`
