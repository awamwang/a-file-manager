# Glossary Map

## Contexts

- [AI Add-on（Skills / MCP）](./GLOSSARY.md)：面向 Agent 的公共库与按项目分发；权威源是 AFM 中央仓库。
- [Feature Pack / 工具管理运行时](./docs/glossary-feature-pack.md)：可后装产品附加包，以及 AFM 对本机工具网关的薄客户端面。
- [Core 搜索面板（Search Palette）](./docs/glossary-search-palette.md)：Ctrl+Shift+P 的统一检索入口，按搜索源分组并组合范围。

## Relationships

- **AI Add-on ↔ 工具管理运行时**：互不吞对方权威源。AI 只管配置分发到 Agent Target；工具管理运行时只做本机列举与调用。
- **Feature Pack → 两上下文**：AI Pack 与工具管理 Pack 都是 Feature Pack；安装/启停/权限由 ExtensionHost（Pack 壳）承担，不叫商店。
- **Core → Feature Pack**：Core 提供贡献点（侧栏/路由、命令、权限声明、sidecar 启停）；Pack 未安装时普通版不打包对应能力。
- **工具管理运行时 → 工具管理网关**：运行时是薄客户端；网关可附着已有进程或由 Pack 拉起 sidecar。
- **Open tool ↔ 工具管理运行时**：长期并存；IDE 预设不迁入网关目录。
- **ExtensionHost ↔ 工具管理运行时**：前者管 Pack 安装启停；后者是使用面（独立侧栏）。
- **Core 搜索面板 → 工具管理运行时**：面板只列工具/能力并跳转预选；能力执行与破坏性确认留在工具运行时页面。
- **Core 搜索面板 → Feature Pack**：Pack 可声明静态条目，统一进「扩展」搜索源；面板不调用 Pack 运行时查询。
