# 知识展示页：搜索按文件去重、Enter 开知识页、密钥 chip 行内交互

ADR-0016 落地后发现三件事：搜索面板的密钥行把「文件命中」挤到组尾，且同一文件多块命中各占一行（同一笔记反复出现）；Enter 直接用相对路径 `openLocalEntry` 打开 .md，因命中 path 相对索引根而必然失败（资源管理器定位同理）；解锁确认框（ConfirmDialogHost z-index 85）被命令面板（z-index 1000）压住，用户根本看不到确认按钮，表现为「密钥取用失败」而服务端审计只记录 `resolve denied`——面板遮挡是根因。

决定把知识内容的消费从「面板行内」整体迁到独立知识页：

1. **服务端去重与路径收口**（awam-knowledge `http-server.js`）：`POST /search` 新增 `dedup:"file"`（先取 topk×5 chunk 再按文件分组取最高分，凑满 topk 个不同文件），响应附 `root`（索引根绝对路径）；新增 `POST /doc`（body `{path}` 相对路径）返回文档全文——服务端强制边界（解析后必须仍在索引根内，拒 `..` 穿越 / 绝对路径 / 跨盘；2MiB 上限；utf8 读取），错误码 `invalid_path` / `doc_not_found` / `doc_too_large`。AFM 一律经服务端读文档，不在客户端直读文件系统，路径防越界收口在一处。
2. **搜索面板只留文件行**：knowledge 源去掉密钥行与响应级 refs 行；文件行 Enter → `router.push("/knowledge?file=…")` 打开知识页，Ctrl+Enter → 用响应 `root` 拼绝对路径 `revealItemInDir`（修复旧的相对路径失效）。`SearchItemType` 移除 `"secret"`、`SearchResultItem` 移除 `secretEntry`；密钥取用链路（`takeVaultSecret` / `lockVaultWindowAction`）抽到 `src/state/vaultAccess.ts` 供命令注册表与 chip 复用。
3. **知识页**（`/knowledge`，`KnowledgeView.vue`）：Rust 新命令 `knowledge_service_read_doc`（客户端预校验拒 `..` / 盘符 / 绝对路径，服务端二次校验）读全文；渲染管线为——文内 `[REDACTED:label:hash8]` / `[vault:label:hash8]` token 先替换为私用区占位符（\uE000…\uE001）→ `marked` 解析 → `DOMPurify` 消毒 → 占位符替换为 chip 槽位（`data-afm-chip`）→ 手动 `createVNode` + `render` 挂载 `KnowledgeSecretChip`（chip 从父页接 `t` 函数，避免挂载点 appContext 依赖）。chip 提供复制（Rust 直写剪贴板）与显示（`vaultChannel` 5s 短显自动回掩码）。
4. **解锁框遮挡修复**：`ConfirmDialogHost` z-index 85 → 1100（高于命令面板 1000），解锁确认框从此可见。

## Consequences

明文边界不变：复制明文由 Rust 直写剪贴板；短显明文只存在 `vaultChannel` 非响应式 Map（5s 自动回掩码）；明文不进 Vue 响应式状态、不进 AI/MCP 上下文、不落日志；服务日志只记「方法+路径+状态码」。`[vault:…]` 形式 token 在前端归一为 `[REDACTED:label:hash8]` 条目名（与服务端 `extractRefs` 同一归一规则），chip 取用键一致。搜索响应不再渲染密钥行，密钥交互全部发生在知识页内（上下文更完整、误触更少）。面板组上限 8 全部留给文件行；服务端 dedup 保证每行一个文件。旧 ADR-0016 的「面板密钥行」形态由本 ADR 取代，密钥取用链路（unlock 确认 → 重试一次 → 审计）与 token/服务配置模式不变。
