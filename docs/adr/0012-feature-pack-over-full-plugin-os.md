# Feature Pack 优先于通用插件 OS

要把 AI 与工具管理等附加能力做成「普通版不打包、可后装」，同时控制复杂度：采用 Feature Pack（Core 贡献点 + 可选 sidecar），而不是先建进程内任意热加载的插件 OS；AI 暂维持编译期风味包，工具管理运行时作为第一个 Pack 验证合同。

**Status**: accepted

**Considered Options**:
- 仅编译期多风味安装包（体验不像「装插件」）
- Feature Pack（侧栏/路由/命令/权限/sidecar）
- 真·ExtensionHost 插件 OS（热加载 UI + Rust）

**Consequences**:
- 工具管理重活留在 awam-tool-manager；AFM 只做薄客户端 Pack。
- ExtensionHost 需从 stub 升到 Pack 壳，但仍不承诺通用动态库加载。
- 现有 AI `ai-addon` 双包可继续，待 Pack 合同稳定后再迁。
