<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 界面截图（Screenshots）

本目录存放仓库根 [README.md](../../README.md)「界面预览」引用的**真实产品界面截图**（非生成图 / 占位图）：

| 文件 | 界面 |
|---|---|
| `explorer.png` | 资源管理器（侧栏树 + 多标签 + 双栏 + 预览） |
| `work-groups.png` | 工作组 |
| `projects.png` | 项目 |
| `ai.png` | AI（Agent 目标 / 技能 / MCP） |
| `extensions.png` | 扩展（Feature Pack） |

## 更新截图

界面改版后需重新截取。本地起应用后截取对应视图，按上表文件名覆盖保存即可（README 引用不变）：

```bash
npm install
npm run tauri:dev
```

用系统截图（`Win + Shift + S` / Snipping Tool）截取应用窗口，保存为对应 PNG。

## 规范

- 用真实数据界面即可，避免出现个人隐私路径 / 文件名；必要时选择演示目录或裁剪。
- 保持主题一致、各图窗口尺寸接近，并排显示更协调。
- 不要用大面积马赛克或模糊遮挡，宁可换一张干净的替代界面。
