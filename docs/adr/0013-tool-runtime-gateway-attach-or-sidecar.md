# 工具管理网关：先附着、否则 sidecar

工具管理运行时优先连接本机已有 awam-tool-manager 实例；若不可用，再由 Feature Pack 拉起本地 HTTP sidecar。破坏性 capability 由 AFM UI 确认后携带 `allowDestructive` 调用；策略仍以网关为准。Open tools（IDE 预设）与本运行时长期分立。

网关不可用时只展示连接/启动态（可附着或拉起），不以本地 JSON 缓存冒充权威目录。非只读调用默认先预览再执行；只读可直接跑。Navigator 选中路径可可选预填 capability 常见字段，不做每工具定制右键菜单。

**Status**: accepted
