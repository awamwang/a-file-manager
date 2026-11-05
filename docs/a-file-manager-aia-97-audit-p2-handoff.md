<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-97 UI 巡检 P2

按 `docs/a-file-manager-ui-audit.md` §5：

1. Extensions 瞬时 notice → toast
2. Toast 关闭按钮 `aria-label` → `common.dismiss`
3. Discovery 空结果下展示 `discovery.syntaxHint`（Everything `! ? ( )`）

可选窄窗/暗色未做。检查：`npm run test:toast`、`npm run test:i18n`。
