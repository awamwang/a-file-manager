# AFM 作为集成方：能力后端从源码 in-tree 构建，UI+后端打成一个 Feature Pack 自发布

把 awam-file / awam-tool-manager / awam-knowledge 三个**自有**依赖的产物来源，从「上游仓各自产 release 资产、AFM 白名单下载」（ADR 0019）改为「**AFM 作为集成方，从各仓源码 in-tree 构建产物、打包发布**」；并把每个能力的 **UI 贡献 + 后端 sidecar 统一打进同一个 Feature Pack**、单一版本、单一安装/启停。取代 ADR 0019 的「产物来源=GitHub Release 资产、SHA 取自附属资产、白名单元清单下载」条款；修订 ADR 0020 的「R3 隔离产物来源」与「versionCompat 门作用对象」。

**Status**: accepted (supersedes ADR 0019 「产物来源 / 下载三方 release 资产」条款；revises ADR 0020 R3 来源与 versionCompat 门范围)

## Decision

- **集成方模型**：三个依赖是自有项目。AFM 承担 integrator 角色，**构建产物归 AFM**，不再要求上游仓先具备 release/打包 job（消解 ADR 0019 列为「当前均不满足」的前置条件与跨仓耦合）。AFM 流水线需具备 **per-OS 构建 agent**（PyInstaller/venv 冻结不可跨平台，必须在目标 OS 上产 Python 产物；Node 产物按平台 bundle）。
- **源码引入 = git submodule 锁 tag/SHA**（provenance + 可复现）。上游仓只需提供**可复现构建**（lockfile + 构建步骤文档），不再各自产 release 资产。
- **发行单元：一个 Feature Pack = UI 贡献 + 后端 sidecar**。单一版本、单一安装/启停；`contributions`（nav/route/command/search）与 `sidecars`（后端）共存于同一 `pack.json`。强化 ADR 0012「pack 为唯一可后装单元」与 C-1。
- **进程模型不变**：Python/Node 服务仍跑在 AFM 进程**之外**（HTTP over attach-or-spawn）。「打成一个 pack」是**分发**统一，不是把后端塞进主进程。进程/健康态仍与 `InstalledPackRecord` **分离**（同一 pack 的两个 facet；维持 ADR 0020 / C-4，**不**把 spawned_pid/health 并入 pack 记录）。
- **分发档位**：`awam-knowledge`（Node 零依赖、体积小）**内置**进 AFM 安装包；`awam-file` / `awam-tool-manager`（PyInstaller 体积敏感）pack 内留 stub，**首启用时从 AFM 自有发布 feed 拉取**隔离产物。feed = **AFM 自己的 release，不是第三方 GitHub 仓库**；白名单商店语义相应退化为「AFM 自有发布通道」。
- **versionCompat 门收窄（与 0013/0020 关系反转，需理顺）**：
  - **managed 产物由 AFM 自构建 → 版本构建时即锁定 → 跳过内部门**。
  - 门**仅保留给 config-mode 附着的外部 / 用户自跑实例**（版本未知，需探测）。实例未暴露版本时，降级为**警告提示**而非硬阻断。
- **运行时模型：shared-first + per-artifact 自带优先（precedence）**。AFM 提供**共享运行时**（一套共享 Python 环境 + 一套共享 Node 运行时——Python 与 Node 不能合成一个，故至少两种），file/tool-manager 优先共用同一共享 Python 环境。每个产物 / `SidecarContribution` 在清单里声明其运行时来源：
  - `runtime: self` —— 该产物**自带**运行时（如 PyInstaller onefile），spawn 时用它自带的；
  - `runtime: shared(python|node)` —— 该产物**不自带**，依赖 AFM 提供的共享运行时，spawn 时用共享解释器执行其 wheel/源码。
  - **规则：自带则用自带，否则用共享。** 这是 Linux 发行版 / AppImage「优先系统库，否则捆绑库」precedence 的同型做法；它让「共享」成为默认收益而不排除个别仍需隔离的产物。
- **共享成立的前置 = 先对齐版本（见跨仓待办）**：把 file / tool-manager 的 Python 与 `mcp` 收敛到同一版本，使单一共享 venv 可同时承载二者（原 0019/0020 的 mcp `>=2.0` vs `<2.0` 互斥是当初只能 R3 的根因，本次以改自有源码消解它）。R3 隔离产物不再作默认，降为 `runtime: self` 这一可选档。
- **隐藏成本（须在动手验证清单里覆盖）**：① mcp 1.x→2.x 对 tool-manager 不止 import 报错，需回归 server/transport 行为；或反向把 file 压回 1.x。② 共享 venv **不可直接打包成 zip 分发**（venv 把绝对路径写死、不可重定位），需用 embeddable-python + 首启用 wheels，或 relocate 方案。③ knowledge 是 Node，共享的是 Node 运行时、与 Python 无关。

## 跨仓待办（自有项目可直接改，决策后记录）

- ✅ **已完成（2026-11-19）：跨三仓对齐 Python 与 mcp 版本**——统一目标定为 **Python 3.12 + `mcp>=2,<3`**（PyPI 实测最新 2.3.0）。`awam-file` 已 `mcp>=2.0`/`py>=3.11`，与 3.12 兼容且高层 API 在 2.3.0 import 通过，**无需改动**；`awam-knowledge` 走 Node `>=18` 零依赖，与 Python 对齐无关。`awam-tool-manager` 已从 mcp 1.x 升到 2.x 并完成回归（详见下「落地记录」）。完成后 file/tool-manager 产物可切 `runtime: shared(python)`、knowledge 切 `runtime: shared(node)`。
- ✅ **license 复核（2026-11-19）**：三仓 LICENSE 实测**均为 MIT**（awam-file / awam-tool-manager / awam-knowledge），与 AFM 的 `GPL-3.0-or-later` **兼容**——MIT 可被 GPL 接受并合并。捆绑只需：① 在产物/仓库内**保留各仓 MIT 版权声明**（NOTICE / THIRD-PARTY）；② 对捆绑产物**提供对应源码**（GPL 组合作品义务，由 AFM 自有仓源码满足）。**不阻塞**本次版本对齐；「统一为 GPL-3.0-or-later」降级为可选简化项，非必需。
- **可复现构建**：各仓补 lockfile + 构建文档 + 固定 tag，供 AFM CI in-tree 构建。
- **feature-pack-format**：`docs/feature-pack-format.md` 的「来源/分发」段改为 AFM 自构建产物指针（取代 0019 的 GitHub Release 资产指针 + 摘要取处）。

## 落地记录：awam-tool-manager 迁移 mcp 2.x（2026-11-19）

- **统一结果**：Python 3.12 + `mcp 2.3.0`。`awam-tool-manager` 已从 `mcp>=1.29,<2` 升到 `mcp>=2,<3`。
- **server 端**：`server.py` 改用 v2 lowlevel `Server(name, version=, instructions=, on_list_tools=…, on_call_tool=…, on_read_resource=…, on_get_prompt=…)` 构造注册（v1 的 `@app.list_tools()` 等装饰器已移除）；handler 统一签名 `async (ctx, params) -> 完整 Result`（`ListToolsResult` / `CallToolResult` / `ReadResourceResult` / `GetPromptResult`），Python 字段 snake_case。
- **对外 wire 不变（AFM 零改动）**：v2 SDK 序列化线上 JSON 仍 camelCase（实测 `isError` / `structuredContent` / `readOnlyHint` 不变）；且 `call_tool` 内把业务失败 `ToolError` 显式重包为 `CallToolResult(is_error=True)`，复刻 v1「异常自动转 isError」的客户端可见行为。⇒ AFM `tool_runtime/mcp_client.rs` 与 config-mode 协议**无需任何改动**。
- **v2 移除项的对策**：① SDK 不再自动按 `inputSchema` 校验入参 → `tool_specs.normalize_arguments` 补回 **enum** 维度（拼错的 `status` 等仍客户端可见 isError）；② 协议移除 `ping` → `backend_client` 心跳由 `send_ping()` 改为轻量 `list_tools()` 探活（对仍停在 mcp 1.x 的第三方子后端也兼容）；③ client 侧字段读取由 camelCase 改 snake_case（`is_error`/`input_schema`），修掉「v2 下 `hasattr(isError)` 恒 False 吞掉后端错误」的隐患。
- **回归**：mcp 2.3.0 下单元测试 **817/818 通过**，与 mcp 1.30 基线**等价**（唯一红 = 沙箱 `os.stat` WindowsApps\winget.exe Store stub 的 WinError 1920，环境噪声、与版本无关）。golden 基线经 `model_dump(by_alias=True)` 保持原样未漂移。e2e 已对 v2 import 干净，但需真实第三方后端运行时 + 子进程，本机沙箱未实跑。
- **AFM 侧落地（2026-11-19，`cargo test --lib` 433 passed；唯一红 = clipboard 在无交互会话的 `0x8007058A` 环境限制，与本改动无关）**：候选2 的统一接缝 `managed_dependency::orchestrator::attach_or_spawn` / `provision::provision_artifact` / `compat` / `registry`（`record.spawned_pid` 为进程态权威、1:1 绑 pack）此前已落地并单测，`tool_runtime::sidecar::start_sidecar_with_probe` 已委托 `attach_or_spawn`（probe→spawn 逻辑单一）。本次新增 `SidecarContribution.runtime: self | shared(python|node)` 档（serde 向后兼容，旧清单缺字段=`self`），`tool_runtime::sidecar::resolve_spawn_command` 按档解析可执行：`self` 用清单 `command` 自带解释器；`shared` 用共享解释器（暂取平台 `python3`/`node`）作 argv0、把 `command` 前置为入口——落实 ADR 0022 的「自带则用自带，否则用共享」precedence；managed 与 tool_runtime 两条 spawn 路径共用同一解析。
- **C-4 双 pid 收口（2026-11-19，`cargo test --lib` 434 passed、`vue-tsc` 通过）**：删除 `ToolRuntimeConfig.last_sidecar_pid`（前端 `client.ts` / `ToolRuntimeView.vue` 同步去字段）；`tool_runtime::sidecar` 的 `start_sidecar_with_probe` / `ensure_gateway_with` 改为把 spawned pid 读写进 `ManagedDependencyRecord`（`record.id = pack id`，启用时 `register_for_enabled_pack` 已建、缺失时 `tool_runtime_record` 就地兜底建），进程态唯一权威收敛到 `record.spawned_pid`，与 `managed_dependency_ensure`（健康页）同源 → 「工具管理运行时页拉起进程」与「健康页停启」不再各看一份 pid。`EnsureGatewayResult.pid` 的 IPC 形状不变，前端无感。
- **AFM 侧未触及（留后续）**：① **共享运行时精确定位**——`shared` 档目前取平台默认解释器（`python3` / `node`），AFM 自装共享 env 的路径由 `provision` 接线；② 共享 venv 可重定位打包；③ 各仓 lockfile/CI。

## Consequences

- ADR 0019 的白名单/元清单/SHA-from-GitHub 全套下载逻辑不再作为 managed-mode 的唯一前提；managed 改为 AFM 自产产物。`store.rs`（allow-list + release 下载 + 摘要校验）从「待接线的主路径」降为「config 兜底 + 自有 feed」可选实现。
- AFM 的 CI 复杂度**重构**：不再为每个依赖各冻结一个自带解释器的隔离包，而是维护**共享 Python 环境 + 共享 Node 运行时**两套（per-OS），各能力产物退化为 wheel/源码薄产物。代价 = 一次 mcp 跨大版本迁移与共享环境可重定位打包；收益 = 体积下降、构建目标减少。`runtime: self` 产物仍可走原隔离档作为逃生口。
- 安装包体积：共享运行时只装一份 Python + 一份 Node，明显小于「每个服务各带一个解释器」；knowledge 内置，file/tool-manager 薄产物按需拉自有 feed。
- 版本门语义在 0013/0020 文字上需对齐：0020 Q15「managed spawn 前过 versionCompat 门」对本自构建产物**不适用**，改为对 config-mode 外部实例适用。
- 更新节奏：能力后端 bugfix 随 **AFM 发布/更新 feed** 分发，而非独立商店轮询。
