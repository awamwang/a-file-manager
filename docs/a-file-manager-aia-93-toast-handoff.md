<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-93 右下角浮动 Toast

## 机制

- `src/state/toast.ts`：队列 `showToast` / `toastSuccess` / `toastError` / `toastInfo`
- `ToastHost.vue`：`position: fixed` 右下角，不占文档流
- `AppShell` 挂载 `#afm-toast-host`

## 已迁入

- Navigator / Discovery 操作反馈（含收藏 add/remove via `@notice`）
- WG soft sync / promote / chrome 错误（`workGroupSoftNotice` / `workGroupChromeError` → toast）
- Projects 绑定/成员/上下文/打开链接与 soft sync
- Discovery 搜索/黑名单失败

## 仍保留 in-flow

- 列表 loading / empty / filterEmpty / Settings·Extensions 加载态与错误横幅（页面状态，非瞬时反馈）

## 检查

`npm run test:toast`
