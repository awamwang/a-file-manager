# Glossary Map

## Contexts

- [核心概念（Core Concepts）](./docs/glossary-core-concepts.md)：工作组 / 项目 / AI 功能 / 收藏的权威定义；界面描述、上手提示与帮助文档的共同真相源。
- [AI Add-on（Skills / MCP）](./GLOSSARY.md)：面向 Agent 的公共库与按项目分发；权威源是 AFM 中央仓库。
- [Feature Pack / 工具管理运行时](./docs/glossary-feature-pack.md)：可后装产品附加包，以及 AFM 对本机工具网关的薄客户端面。
- [Core 搜索面板（Search Palette）](./docs/glossary-search-palette.md)：Ctrl+Shift+P 的统一检索入口，按搜索源分组并组合范围。
- [语音输入套件（Voice Input Suite）](./docs/glossary-voice-input.md)：全应用「说话转文字」能力：统一语音服务 + 可配置引擎；当前唯一接入点是搜索面板。
- [帮助与导引（Help & Guidance）](./docs/glossary-help-guidance.md)：应用内帮助体系：帮助文档 / 上手提示 / 引导教学三层；共用一份声明式导引定义，供新特性沿用。

## Relationships

- **AI Add-on ↔ 工具管理运行时**：互不吞对方权威源。AI 只管配置分发到 Agent Target；工具管理运行时只做本机列举与调用。
- **Feature Pack → 两上下文**：AI Pack 与工具管理 Pack 都是 Feature Pack；安装/启停/权限由 ExtensionHost（Pack 壳）承担，不叫商店。
- **Core → Feature Pack**：Core 提供贡献点（侧栏/路由、命令、权限声明、sidecar 启停）；Pack 未安装时精简版不打包对应能力。
- **工具管理运行时 → 工具管理网关**：运行时是薄客户端；网关可附着已有进程或由 Pack 拉起 sidecar。
- **Open tool ↔ 工具管理运行时**：长期并存；IDE 预设不迁入网关目录。
- **ExtensionHost ↔ 工具管理运行时**：前者管 Pack 安装启停；后者是使用面（独立侧栏）。
- **Core 搜索面板 → 工具管理运行时**：面板只列工具/能力并跳转预选；能力执行与破坏性确认留在工具运行时页面。
- **Core 搜索面板 → Feature Pack**：Pack 可声明静态条目，统一进「扩展」搜索源；面板不调用 Pack 运行时查询。
- **Core 搜索面板 → 语音输入套件**：面板是套件当前唯一接入点（语音搜索配置、麦克风按钮、快捷键），经语音服务调用；识别文本回填面板输入框，检索仍走面板既有链路，不旁路。
- **核心概念 → 帮助与导引 / 界面文案 / i18n**：工作组、项目、AI 功能、收藏的定义以核心概念术语表为真相；上手提示、界面描述与帮助文档「它是什么」统一措辞，六语对齐。
- **核心概念「项目」→ AI Add-on「Project Workspace」**：前者是使用者视角的概念升级（升级自工作组、创建工作目录并建立 AI 开发基础结构）；后者是 AI Add-on 上下文中承载 MCP/Skills 部署落点的工作区目录，两者不互相吞并。
