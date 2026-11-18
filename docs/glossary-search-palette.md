# Core 搜索面板（Search Palette）

由 Ctrl+Shift+P 打开的覆盖层，是命令、文件、项目、工作组、工具/能力、浏览器书签与 Pack 扩展条目的统一检索入口。它只负责“找到并触发下一跳”，不承担项目激活、布局修复等状态变更。多上下文索引见 [GLOSSARY-MAP.md](../GLOSSARY-MAP.md)。

## Language

**搜索面板（Search Palette）**：
Ctrl+Shift+P 打开的全局覆盖层；输入一次即在所有已选搜索源中检索，结果按源分类展示。
_Avoid_: 命令面板、命令窗口、全局命令窗口、Command Panel

**搜索源（Search Source）**：
面板内一个可检索的数据类别，以及它的“检索 + 激活”适配器；结果按源分组，跨源取并集。
_Avoid_: 引擎、后端、Provider

**浏览器书签源（Bookmarks Search Source）**：
源 id `bookmarks`，显示名「浏览器书签」；按需拉取 `awam://browser` 扁平快照并本地过滤（标题 / URL / 文件夹路径），与 Everything 文件命中分组并列、不混排；Enter 经网关 `open_url` 打开。不等于应用内 `favorites.json`。
_Avoid_: 收藏夹（易与 AFM 钉选混淆）、Provider 搜索、Discovery

**范围组合（Scope Set）**：
用户勾选参与检索的搜索源集合；结果为各源并集，跨会话持久化，空集按「全部」处理。
_Avoid_: 筛选器、模式、Tab、过滤

**全部**：
范围组合等于所有可用搜索源时的显示名；不是独立模式，也不是第四种筛选。
_Avoid_: 默认模式、混合模式

**类型标签（Type Label）**：
每个结果行显式展示的所属搜索源文字标签（命令 / 文件 / 项目 / 工作组 / 工具 / 能力 / 扩展）；文件结果细分为文件与文件夹。
_Avoid_: 角标、分组前缀

**结果主动作（Primary Action）**：
按 Enter 触发的动作，由搜索源定义：命令执行、文件打开、项目聚焦、工作组静默切换、工具/能力跳转（可经配置改为调用能力）。
_Avoid_: 双击、点击

**工具动作配置（Tool Action Config）**：
`command-palette.json` 中按 tool id 覆盖结果主动作的配置；无界面，缺省为跳转并预选。
_Avoid_: 快捷键配置、工具偏好

**静态 Pack 条目（Static Pack Entry）**：
Feature Pack 清单声明、由宿主本地过滤的扩展可搜索项，统一归入「扩展」类型；不发起运行时查询。
_Avoid_: 动态搜索源、Pack 查询

**Discovery（发现）**：
由 Everything 索引驱动的全盘文件搜索页面；面板文件源只展示其 Top N，完整结果在 Discovery 查看。
_Avoid_: 全局搜索（易与搜索面板混淆）、文件搜索面板
