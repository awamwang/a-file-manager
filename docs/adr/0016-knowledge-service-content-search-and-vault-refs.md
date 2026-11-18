# 知识内容检索与账号引用（awam-knowledge HTTP 服务）

搜索面板只有 Everything 文件名命中：知识库（Logseq 脱敏副本）里的内容命中、以及笔记中「见保险库 [vault:...]」引用的凭据取用，都没有入口。awam-knowledge 仓已有 BM25（标题×4、增量缓存）与零明文脱敏体系（`[REDACTED:…]` / `[vault:…]` token、解锁窗口、审计），但只以 stdio MCP 与脚本形态存在，AFM 无法直接消费，且密钥明文绝不许经过 AI/MCP 管线。

决定引入 awam-knowledge 常驻 HTTP 服务作为第 7 个搜索源 `knowledge`（Alt+7，结果分组排在 files 之前）：BM25 内容命中映射为文件行（Enter 打开 .md、Ctrl+Enter 资源管理器定位）；命中块中提取的 `[vault:…]` / `[REDACTED:…]` 引用映射为密钥行（vault 紫色 + 锁图标，默认掩码）。服务地址与 token 走 Power 设置 `power.awamKnowledgeServiceUrl` / `power.awamKnowledgeServiceToken`（环境变量 `AFM_KNOWLEDGE_SERVICE_URL` / `AFM_KNOWLEDGE_SERVICE_TOKEN` 可覆盖），与 awam-file 网关同一配置模式；token 错误/服务离线时源显示状态行，不阻塞其它源。密钥取用链路为：AFM Rust（`vault_reveal` / `vault_copy` 命令）→ HTTP 服务（token 鉴权）→ 脱敏体系人机通道（`agent_secret.py resolve`，要求解锁窗口开启 + `human_channel` agent 注册 + 审计无值）；复制由 Rust 直接写系统剪贴板，明文不进 Vue 状态、不进 AI/MCP 上下文、不落任何日志；短显 5 秒自动回掩码。语料与路径由 awam-knowledge 服务端配置决定（索引根 = 脱敏工作副本，沿用其「禁止默认真源」约定），AFM 不感知任何具体路径。

## Consequences

Agent 通道红线不变：`agent_secret.py` 的 run 仍只注入子进程 env、无读值出口；人机通道（`resolve`）是唯一受控明文读出口，仅对本机 UI 场景开放且全程审计。搜索响应只含脱敏语料内容，天然零明文。面板密钥行不提供「查看全部」页；解锁/锁定经确认对话框转发服务端 unlock/lock。后续若其它客户端（如 T3、闪电说）要接同一服务，复用 token + 解锁窗口即可，无需改 AFM。
