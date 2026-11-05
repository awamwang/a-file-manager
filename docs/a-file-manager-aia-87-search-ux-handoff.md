<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-87 / 88 / 89 搜索 UX 交接

对照 TZ 决策与 `a-file-manager-search-status.md`。

## AIA-87 Discovery 用户向

- 去掉运维味 `status.detail` 与英文引擎枚举 chip
- 白话状态：`discovery.everythingReady*` / `fallbackOff|Standby|Active`
- 结果 meta 使用 `discovery.engines.*` 本地化名

## AIA-89 引擎卡片 + 分组配置

- 两张可切换卡片：Everything / 备用搜索
- 选中卡片调用 `discovery_set_fallback_enabled`（Everything→关 fallback 偏好；备用→开）
- 配置区随卡片切换：`data-engine-config="everything|fallback"`
- 黑名单权威面收拢到 Everything 卡片；Settings 隐藏 `core.discoveryBlacklist` 编辑（键仍持久化）

## AIA-88 Navigator 内嵌搜索条

- 仅 Navigator chrome：`#navigator-filter-bar`（`navigatorFilterOpen`）
- 筛选当前文件夹名/路径（非全盘 Discovery）
- Ctrl+Shift+F 在 Navigator 上打开筛选条；其他路由仍进 Discovery

## 检查

```text
npm run test:discovery-search
npm run test:navigator-filter
npm run test:discovery-unify
npm run test:global-search
npm run test:i18n
```

## 相关提交范围

本 handoff 覆盖 AIA-87/88/89；AIA-86 关闭确认见独立 handoff。
