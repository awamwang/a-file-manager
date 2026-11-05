<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# A File Manager UI 巡检（AIA-92）

巡检范围：`AppShell` + 主路由视图（Navigator / Discovery / Work Groups / Projects / Providers / Extensions / Settings）。基于当前 `master` 源码与近期 AIA-78～93 落地状态，非像素级截图对比。设计语汇：Fluent tokens + 自研图标，**非** Sigma UI kit。

## 1. Shell 骨架

| 区域 | 现状 | 判定 |
|---|---|---|
| 标题栏 | 品牌 + Command Panel 入口 + 自绘窗控（min/max/close）；关闭走确认/托盘策略 | OK（AIA-80/86） |
| 侧栏 | `.body` 内左侧顶满标题栏下方；Navigator / WG / Projects / Extensions / Discovery / Providers / Settings | OK（AIA-91） |
| 工作区 | `.workspace` 右列；仅 Navigator 路由挂载 `navigator-chrome`（地址栏/WG/筛选/标签） | OK（AIA-79/81/91） |
| 状态栏 | Navigator 底栏 + toast 避让 `statusbar` 高度 | OK（AIA-78/93） |
| Toast | `#afm-toast-host` fixed 右下，不占文档流 | OK（AIA-93） |

## 2. 分页面要点

### Navigator
- **优点**：双栏、筛选条、Places→「近期访问」、收藏侧栏、上下文菜单文件操作、OpenState 静默保存（AIA-90）。
- **问题 / 建议**：
  1. 主列表仍用 in-flow `fluent-banner` 表达 loading/error/empty/filterEmpty——合理，勿迁 toast。
  2. 双栏开启时次栏路径与主栏视觉权重接近，窄窗易挤；可考虑次栏折叠或最小宽度守卫（非阻塞）。
  3. 筛选条与地址栏同列时信息密度高；长路径截断依赖 `title`，键盘用户够用、鼠标悬停提示可再统一。

### Discovery
- **优点**：引擎卡片分态、搜索 UX 与 Navigator 内嵌筛选口径对齐（AIA-87/88/89）；失败走 toast。
- **问题 / 建议**：
  1. 空结果仍为 in-flow info banner——正确。
  2. Everything 特殊语法（`!`/`?`/`()`）对用户不透明；可在空结果或帮助链补充一句「语法提示」（产品文案，非本次必做）。
  3. Settings 已隐藏 `core.discoveryBlacklist` 编辑；若黑名单仍可通过其它入口改，需保证失败 toast 仍可达。

### Work Groups / Projects
- **优点**：soft sync / promote / 绑定反馈已迁 toast；Projects 板/列表双视图。
- **问题 / 建议**：
  1. Projects 持久错误（绑定失败、WG 突变）仍用错误横幅——保留。
  2. Extensions 安装/卸载仍用 in-flow soft banner；与 toast 体系不一致，可列为跟进（非 93 必迁范围）。

### Providers / Settings / Extensions
- **优点**：Settings 分区清晰（含 health / shortcuts / favorites 子路由）。
- **问题 / 建议**：
  1. Providers 与 Discovery 能力边界对用户仍偏「工程师向」；命名/副文案可再口语化。
  2. Extensions 页 notice 未进 toast（见上）。
  3. Settings 错误横幅与加载态一致沿用 banner，可接受。

## 3. 跨页一致性

| 主题 | 观察 | 优先级 |
|---|---|---|
| 瞬时成功/失败反馈 | Navigator/Discovery/WG/Projects/Chrome soft → toast；Extensions 仍 banner | P2 |
| 页面态（loading/empty/error） | 统一 `fluent-banner*` | OK |
| 侧栏导航顺序 | Navigator 优先，Settings 垫底 | OK |
| i18n | 「近期访问」已替换 Places；硬编码英文字符串风险低 | OK |
| 图标 | FluentGlyph；dualPane 虚线样式已修 | OK（AIA-84） |
| 暗色主题 | token 驱动；toast 用 control/status token，需暗色目视一次 | P3 |

## 4. 可访问性与键盘

- 标题栏窗控、Command Panel（Ctrl+Shift+P）、Navigator 历史/地址栏有 `aria-label`。
- Toast：`aria-live="polite"`；关闭按钮 aria-label 目前硬编码 `"Dismiss"`，应走 i18n（P2）。
- 关闭确认对话框有 `role="dialog"` / `aria-modal`。

## 5. 建议跟进（不阻塞 91/93）

1. ~~Extensions 瞬时 notice → toast~~ — 已做（AIA-97）。
2. ~~Toast dismiss aria-label i18n~~ — 已做（`common.dismiss`）。
3. ~~Discovery 语法帮助一句~~ — 已做（`discovery.syntaxHint`）。
4. 双栏窄窗布局守卫（可选，未强制）。
5. 暗色主题下 toast 对比度目视（可选，未强制）。

## 6. 结论

Shell 布局（AIA-91）与浮动 toast（AIA-93）已到位；AIA-97 已清掉 §5 中的 P2 项。窄窗/暗色仍为可选跟进。
