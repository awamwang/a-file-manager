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
  - ✅ **lockfile 已加（2026-11-19，各仓本地提交，未 push）**：`awam-file` `uv.lock`（056e020）、`awam-tool-manager` `requirements.lock`（uv pip compile 全 pin，5537eb5）、`awam-knowledge` `package-lock.json`（零依赖，381b83c）。
  - ⏸ **git submodule（AFM 内固定来源 commit）待推送**：submodule 记录的 SHA 必须存在于远端才能 clone。三仓 remote 均为 `github.com/awamwang/awam-{file,tool-manager,knowledge}`；截至本会话，`awam-file` 含本次 lockfile 共 1 个、`awam-tool-manager` 4 个（mcp2.x/端口/data/lock）、`awam-knowledge` 6 个未推送提交。**需授权 `git push` 后**才能把 submodule 固定到当前源码；否则只能 pin 到旧的远端 commit（tool-manager/knowledge 会缺 mcp 迁移等，不正确）。**本地 in-tree 构建现已可用**（`build_pack_feed.py --src <本地仓路径>` + 锁文件），submodule 主要是 CI 端 checkout 来源。
- **feature-pack-format**：`docs/feature-pack-format.md` 的「来源/分发」段改为 AFM 自构建产物指针（取代 0019 的 GitHub Release 资产指针 + 摘要取处）。（注：清单字段 `gatewayCompat` 已在 P1-3 落地说明，「来源/分发」改由 `docs/feed-operations-runbook.md` 覆盖。）
- **awam-knowledge：Node→Python 转换（评估结论 2026-11-19，**暂不接入**，保持 Node）**：实测 knowledge 代码 ~5.2k LOC（生产 ~3.0k：bm25-index 354 / http-server 624 / mcp-server 565 / import-md-dir 607 / index-root 134 / store 113 / fs-util 103 / secret-provider 79 …；测试/评测 ~2.2k）。**只用 Node 内置**（path/fs/os/crypto/child_process/readline/http），**零外部依赖、无 sqlite**（索引 = store.js JSON + 内存 BM25）。分词口径确定且无 Intl：CJK bigram over `[\u4e00-\u9fff]` 正则、`toLowerCase`、BM25(k1=1.2,b=0.75) `Math.log`（IEEE-754 double）、`toFixed(4)`——均有**逐字等价的 Python**（`re.sub`+split、`str.lower`、`math.log`、f"{x:.4f}"，Python float 同为 double）。且现成有 `eval-recall.js`+`query-set-check.js`+`queries-t18.json` 作**检索等价性 oracle**。**代价=一次机械语言移植（中）+ 用既有评测套件复验 BM25/tokenize 等价（风险点：float/round 与 CJK/ASCII 大小写边角，由评测兜底）**；**收益=三后端统一到单一 Python 运行时**（复用 file/tool-manager 的 mcp≥2 + starlette/uvicorn + PyInstaller onefile + feed/provision 全链路），**直接消解 P2-6（shared Node 运行时）**。建议作为独立后续：① 先移植 bm25-index（含分词）+ eval-recall 跑通等价；② 移植 http-server/mcp-server 对齐 `design/afm-http-service-design.md` 契约；③ 之后 knowledge 与 tool-manager 同路（onefile + pack + feed + managed spawn）。

## 落地记录：awam-tool-manager 迁移 mcp 2.x（2026-11-19）

- **统一结果**：Python 3.12 + `mcp 2.3.0`。`awam-tool-manager` 已从 `mcp>=1.29,<2` 升到 `mcp>=2,<3`。
- **server 端**：`server.py` 改用 v2 lowlevel `Server(name, version=, instructions=, on_list_tools=…, on_call_tool=…, on_read_resource=…, on_get_prompt=…)` 构造注册（v1 的 `@app.list_tools()` 等装饰器已移除）；handler 统一签名 `async (ctx, params) -> 完整 Result`（`ListToolsResult` / `CallToolResult` / `ReadResourceResult` / `GetPromptResult`），Python 字段 snake_case。
- **对外 wire 不变（AFM 零改动）**：v2 SDK 序列化线上 JSON 仍 camelCase（实测 `isError` / `structuredContent` / `readOnlyHint` 不变）；且 `call_tool` 内把业务失败 `ToolError` 显式重包为 `CallToolResult(is_error=True)`，复刻 v1「异常自动转 isError」的客户端可见行为。⇒ AFM `tool_runtime/mcp_client.rs` 与 config-mode 协议**无需任何改动**。
- **v2 移除项的对策**：① SDK 不再自动按 `inputSchema` 校验入参 → `tool_specs.normalize_arguments` 补回 **enum** 维度（拼错的 `status` 等仍客户端可见 isError）；② 协议移除 `ping` → `backend_client` 心跳由 `send_ping()` 改为轻量 `list_tools()` 探活（对仍停在 mcp 1.x 的第三方子后端也兼容）；③ client 侧字段读取由 camelCase 改 snake_case（`is_error`/`input_schema`），修掉「v2 下 `hasattr(isError)` 恒 False 吞掉后端错误」的隐患。
- **回归**：mcp 2.3.0 下单元测试 **817/818 通过**，与 mcp 1.30 基线**等价**（唯一红 = 沙箱 `os.stat` WindowsApps\winget.exe Store stub 的 WinError 1920，环境噪声、与版本无关）。golden 基线经 `model_dump(by_alias=True)` 保持原样未漂移。e2e 已对 v2 import 干净，但需真实第三方后端运行时 + 子进程，本机沙箱未实跑。
- **AFM 侧落地（2026-11-19，`cargo test --lib` 433 passed；唯一红 = clipboard 在无交互会话的 `0x8007058A` 环境限制，与本改动无关）**：候选2 的统一接缝 `managed_dependency::orchestrator::attach_or_spawn` / `provision::provision_artifact` / `compat` / `registry`（`record.spawned_pid` 为进程态权威、1:1 绑 pack）此前已落地并单测，`tool_runtime::sidecar::start_sidecar_with_probe` 已委托 `attach_or_spawn`（probe→spawn 逻辑单一）。本次新增 `SidecarContribution.runtime: self | shared(python|node)` 档（serde 向后兼容，旧清单缺字段=`self`），`tool_runtime::sidecar::resolve_spawn_command` 按档解析可执行：`self` 用清单 `command` 自带解释器；`shared` 用共享解释器（暂取平台 `python3`/`node`）作 argv0、把 `command` 前置为入口——落实 ADR 0022 的「自带则用自带，否则用共享」precedence；managed 与 tool_runtime 两条 spawn 路径共用同一解析。
- **C-4 双 pid 收口（2026-11-19，`cargo test --lib` 434 passed、`vue-tsc` 通过）**：删除 `ToolRuntimeConfig.last_sidecar_pid`（前端 `client.ts` / `ToolRuntimeView.vue` 同步去字段）；`tool_runtime::sidecar` 的 `start_sidecar_with_probe` / `ensure_gateway_with` 改为把 spawned pid 读写进 `ManagedDependencyRecord`（`record.id = pack id`，启用时 `register_for_enabled_pack` 已建、缺失时 `tool_runtime_record` 就地兜底建），进程态唯一权威收敛到 `record.spawned_pid`，与 `managed_dependency_ensure`（健康页）同源 → 「工具管理运行时页拉起进程」与「健康页停启」不再各看一份 pid。`EnsureGatewayResult.pid` 的 IPC 形状不变，前端无感。
- **AFM 侧未触及（留后续）**：① **共享运行时精确定位**——`shared` 档目前取平台默认解释器（`python3` / `node`），AFM 自装共享 env 的路径由 `provision` 接线；② 共享 venv 可重定位打包；③ 各仓 lockfile/CI。

## 落地记录：managed 产物 feed 接缝接线（B-正式，2026-11-19）

把 ADR 0022「AFM 自构建产物 → 经自有 feed 发布 → managed 拉起时按需 provision」从内核（store/provision 已就绪但无人调用）变成**活的端到端能力**：

- **块1 feed 配置层**：新增 `managed_dependency::feed`——`FeedConfig { manifestUrl, allowHosts }` 存 `{app_config}/managed-dependency-feed.json`（**置于 records_root 之外**，避免 `list_records` 把它当托管记录解析）；`fetch_manifest`（未配置→`None`；URL 主机不在 allow-list→`Err` 且**绝不发起 fetch**；不可达→`Err` 由调用方降级）；`resolve_managed_binding`→`ManagedBinding { declared_version, base_url }`。`host.rs` 启用 pack 时按 feed 元清单把**有 entry 的 pack 切成 `Managed`** 并写入 `base_url`/`declared_version`，元清单不可达/无 entry → 静默回退 `Config`（不阻断启用）。`StoreEntry` 增可选 `base_url`（managed 拉起后 AFM 探活/附着该实例的本机地址，作为产物分发元数据随 feed 声明）。新增 `managed_dependency_set_feed` / `managed_dependency_get_feed` IPC（lib.rs 注册）。
- **块2 provision 接线**：`provision::ensure_artifact`（幂等）——`mode != Managed` 或 feed 无 entry → 产物随 pack 内置/`Ok(None)`；有 entry 且目标 `{pack_dir}/{workingDirRel|sidecar}/{fileName}` 已在 → 不下载；缺失 → `download_and_verify`（allow-list+SHA-256）+ `materialize` 落 sidecar 目录。managed 自构建产物**跳过声明版兼容门**（构建即锁版本，与本文一致）。`commands::prepare_contrib_for_spawn` 在 `managed_dependency_ensure` spawn 前调用它，命中则把 `SidecarContribution.command` 改写为**落盘产物绝对路径**（`runtime:self` 档作 argv0）；feed 不可达/无 entry → 原样返回（不误伤已有产物）。`store::fetch_manifest_text`/`http_get_bytes` 的 `#[allow(dead_code)]` 移除（已被生产路径真调用）。
- **块3 in-tree 构建脚本**：`scripts/build_pack_feed.py`——用 PyInstaller `--onefile` 把 awam-tool-manager 源码冻结为自带解释器的单文件 `awam-tool-manager.exe`（≈25MB），算 SHA-256，生成与 `store::StoreManifest` 对齐的 `manifest.json`（含 `baseUrl`），产物 + 清单落 feed 目录；`emit-feed-config` 子命令产 AFM 运行时 `managed-dependency-feed.json`。构建坑：`--collect-all mcp` 分析期会 import `mcp.cli` 需 `typer`/`rich`，已纳入构建环境。
- **块4 端到端验证**：① Rust gated 测试 `ensure_artifact_real_http_feed_download_roundtrip`（`cargo test --lib -- --ignored`）——起本地 http feed，用 **AFM 真实 ureq 函数** `fetch_manifest_text`+`http_get_bytes` 拉清单、下载、SHA 校验、落盘，并断言二次就位幂等不重下。② 真 build 产物完整链——feed→下载 24MB→SHA 校验→落 sidecar→spawn 真 exe→`/health` `{status:ok,version:0.1.0}`→MCP `initialize`→`tools/list` 19 工具→`tools/call list_tools` `isError=false`。`cargo check --tests` 零警告、`cargo test --lib` **443 passed / 0 failed**、`vue-tsc` 通过。`packs/tool-runtime/pack.json` 的 sidecar 切成 onefile 产物形态（`command: awam-tool-manager.exe`、`args:["--transport","http"]`、`runtime:self`、`workingDirRel:sidecar`）。
- **仍未做（界面级 + 部署，留后续）**：① **真 UI 端到端**需跑 tauri 桌面、在 `ExtensionsView` 选 pack 目录 + 配 feed（接缝已程序化验证，IPC 已注册）；② `runtime:shared` 的 AFM **自装共享 env** 精确定位（现取平台默认 `python3`/`node`）；③ feed 真实发布 = 部署侧 AFM release job（脚本已产 manifest+artifacts，上传待接 CI）；④ `host.rs` 启用时同步 ureq 拉 manifest，生产宜移 `spawn_blocking` + 本地缓存（当前本地/快速 feed 可接受）。

## 落地记录：可安装 Store 接缝（feed manifest packs 段，2026-11-19）

把 feed 从「仅产物来源」升级为「可安装目录」，扩展页可列出 feed 里的 pack 并**一键安装**（候选5 / ADR 0019 留的最后一块）。切换安装源头本机→线上 = 仅改 `feed.json` 的 `manifestUrl`，架构不动。

- **store 层**：`StoreManifest` 增 `packs: Vec<PackEntry>`（`PackEntry {packId,name,version,installUrl,sha256}`，`installUrl` 指 pack 安装包 zip；旧清单缺 `packs` 段=空，向后兼容）；下载校验抽出通用 `download_verify_url(url, sha, allow_hosts, fetch)`，产物 `download_and_verify` 与 pack 安装共用（先查 allow-list 再 fetch 再 SHA，离线可注入）。
- **host 层**：`list_installable_packs()`（读 feed packs + 本机已装/版本标记）；`install_pack_from_feed(pack_id)`（按 installUrl 下载 zip → SHA 校验 → 落临时 → 复用既有 `install_pack(zip)`，装后默认 Installed）。IPC `extension_host_list_installable` / `extension_host_install_pack_from_feed`（均 spawn_blocking，含网络与解压）。
- **前端**：`ExtensionsView` 新增「可从 feed 安装」区（列出 feed packs，已装项禁用按钮），点装→`install_pack_from_feed`→刷新+软重载；feed 不可达单记 `feedError` 不污染主错误。i18n 5 locale 齐（`storeTitle/Meta/Empty/Unavailable/Available/Installed/Install`）。
- **构建脚本**：`build_pack_feed.py pack-dir` 子命令——把成形 pack 目录打成安装包 zip + 计算 SHA + 写入/合并 feed 元清单 `packs` 段。
- **装 vs 启用分离（S-3 默认）**：列表点「安装」仅装成 Installed；用户再「启用」才触发 host 按 feed entries 切 Managed → `ensure` 按需 provision 产物 → spawn。整链可分段观察。
- **验证**：`cargo check --tests` 零警告；`cargo test --lib` 445 passed（唯一红 = clipboard 无交互环境限制）；`vue-tsc` 0；`check-i18n` parity 1422 keys×5；本地 feed 已列出 `awam.tool-runtime`（pack zip 915B，`GET` 200）。真 UI 端到端（列表显示→点装→启用→拉产物→用）由本轮界面手测覆盖。

## 落地记录：界面手测收尾与托管服务可用性（2026-11-19）

界面跑通 managed 全链后，据手测反馈收口以下行为：

- **默认端口 8766→18778**（旧端口与本机其它服务冲突）：tool-manager `http_transport.DEFAULT_PORT` 为权威，AFM 侧 `config.rs` 默认、前端 `ToolRuntimeView` 默认、`sidecar` 断言、feed/provision/store 测试 fixture、`pack.json`、ADR0020 表格全部同步，产物重建。
- **网关地址单一权威**：`tool_runtime` 的 status / ensure / mcp 调用统一走 `sidecar::effective_base_url_for`——`managed→feed 写入的 record.base_url`，`config→ToolRuntimeConfig.base_url`；消除「config 残留地址与 feed 漂移」（曾出现探 18777 而 feed 权威是 18778）。
- **managed 网关地址只读**：扩展页对托管 pack 禁用网关地址输入框 + 只读提示（前端 `getManagedBinding` 读 `record.mode`）。
- **启用即拉起**：扩展页启用 pack 后自动 `managed_dependency_ensure`；config-mode 后端 no-op（不自动拉起，由工具页手动）。
- **启动自动恢复（选项 A）**：`managed_dependency::commands::reconcile_managed_on_startup` 在 `setup` 后台 `spawn_blocking` 遍历 `Managed` 记录逐个 ensure（已运行→附着，未运行→provision+spawn），单条失败仅记录、不阻断启动 → 启用过的托管 pack 重启 AFM 后自动可用。
- **抖动消除**：贡献缓存 `cached` 与 `SIDE_NAV_ITEMS` 改 `shallowRef`；安装/启用/禁用/卸载统一走增量 `reloadContributions`（含路由回退），不再整窗 remount；扩展页 `refresh` 仅首次显「加载中」，后续静默更新。
- **实测确认**：managed spawn 的 exe 监听 18778，`/health` ok、MCP `tools/list` 19 工具、`tools/call` success；但 `list_tools`（被管理工具）`count=0`。
- **产物内嵌 data（续，2026-11-19）**：`count=0` 的根因是托管 onefile 产物**没带声明数据目录**——`server.py` 的 `DATA_DIR = __file__/../data` 在冻结下指向不存在的临时目录，`store.list_tools` 对缺失目录优雅返回空。修复：① `server.py` DATA_DIR 支持 `sys.frozen → _MEIPASS/data`；② 构建 `build_pack_feed.py` 加 `--add-data "<仓库>/data;data"`；③ 加 `data/tools/system-powershell.json`（kind=app，launch `powershell.exe`，本机必存可启动）作最小可见样例。重建后实测托管 exe `list_tools` 返回 **8 个工具**（含 system-powershell），与源码 config-mode 一致。
- **产物 SHA 校验 + 非静默手动升级（续，2026-11-19）**：取代「删旧产物触发重下」的待办。`ensure_artifact` 现返回 `ArtifactState{path, up_to_date}`——产物已在时算磁盘 `store::file_sha256` 与 feed entry `sha256` 比对：相符 `up_to_date=true`；不符 `false`=**有可用升级**。关键契约是**非静默**：`ensure`/启动恢复**绝不自动替换**已运行产物，仍返回旧产物继续服务，仅把 `record.available_version` 置为 feed 声明版本。新增 `record.installed_version` / `available_version`（serde 向后兼容，空=无升级）。用户手动触发的 `managed_dependency_upgrade(id)` 命令才完成升级：**停旧进程（释放 Windows exe 文件锁）→ `provision::force_provision` 强制重下校验原子覆盖 → 重新拉起**，成功后清 `available_version`。前端扩展页 pack 卡片 join `managed_dependency_list`，对 `mode==managed && availableVersion` 显「有可用更新 → vX」+「升级」按钮。业界对齐：VS Code/LibreOffice 更新扩展都是「通知 + 用户选择重启加载」，非热替换。
- **检查更新（纯读，2026-11-19）**：升级标记原先只作为「拉起服务」路径（`prepare_contrib_for_spawn`）的副产品，且每条记录各拉一次清单。补一条**独立纯读**检测：`provision::inspect_artifact_offline`（只算盘上 sha、绝不下载/不 spawn）+ `commands::check_updates`（清单**只拉一次**遍历 managed 记录刷新 `available_version`/`installed_version`）。两个入口：**手动「检查更新」按钮**（`managed_dependency_check_all_updates` IPC）与**启动后台自动检查**（`setup` spawn_blocking 先 check_updates 再 reconcile）。缺产物也标为可用升级（升级即覆盖式安装）。feed 不可达时手动检查如实报错、启动检查静默跳过。`ensure_artifact` 的「已在」分支改复用同一离线函数，避免双份哈希比对逻辑。
- 验证：`cargo test --lib` 456 passed、`vue-tsc` 0、`check-i18n` parity 1431×5；tool-manager 单测 819 passed。

## 落地记录：versionCompat 门单一实现化（候选4 / P1-3，2026-11-19）

取证发现 versionCompat **早已实现并接线**——`tool_runtime/semver_compat` + `check_gateway_compat` 在 `with_mcp_client` 每次网关 MCP 调用前按 pack `gatewayCompat` 比对实例上报版本，未知/非三元组降级跳过（符合本文 Q15/门语义），并有 live 测试。此前所谓「门悬空」指的是 `managed_dependency::compat`（`ensure_declared_version_compat`/`VersionRange`）这套**重复实现**——它仅被同样无生产消费者的 `provision::provision_artifact` 引用。

处置：删冗余 seam、令 `gatewayCompat` 成为唯一版本兼容机制。
- 删除 `managed_dependency::compat` 模块 + `provision::provision_artifact`（managed 产物构建即锁版本、provision 路径本就不应过门；config-mode 门在网关调用处）。
- `provision.rs` 去 `compat` import 与 4 个 `provision_artifact` 测试；删 `ARTIFACT_SUBDIR` 常量（ensure/force 一律落 sidecar 目录）；`managed_dependency::compat` re-export 与 mod 注释更新。
- `tool_runtime::mod` 的 `known_triple` 再导出原仅服务已删 compat，移除（仅留 `version_in_range`，line 214 用全限定名）。
- `feature-pack-format.md` 字段说明从「更名为 versionCompat」纠正为实际 `gatewayCompat` + 注明由 tool_runtime 执行、未知版本降级跳过。
- 验证：`cargo test --lib` 446 passed 无新警告。

## 落地记录：feed 下载 GitHub→对象存储回落链（P2-5，2026-11-19）

发布通道做成**单链条、多源回落**：优先 GitHub Release（AFM 自有 release 资产），传输失败 / 不可达 / 摘要不符则回落**对象存储镜像**。OSS 具体配置（bucket / endpoint / 同步）后续接，但接缝已就位且**默认惰性**——未填回落地址时行为与单源完全一致。

- **feed 配置**：`FeedConfig` 加 `fallbackManifestUrl`（serde default 空）；`fetch_manifest` 改主→回落，逐源都守 allow-list（不在则绝不 fetch 该源），主源任一错才试回落，两路皆败返回首个错误。`managed_dependency_set_feed` 加可选 `fallbackManifestUrl` 参数。
- **条目级回落**：`StoreEntry` 加 `assetFallbackUrl?`、`PackEntry` 加 `installFallbackUrl?`（均 serde default）。新原语 `store::download_verify_url_with_fallback(url, fallback, sha, allow, fetch)`：主源先（allow-list→fetch→SHA 校验），失败/不可达/摘要不符再对回落源同样 allow-list→fetch→同 SHA 校验。`download_and_verify`（产物）与 `install_pack_from_feed`（pack 安装包）走该链；`fetch` 边界 `FnOnce→FnMut`。
- **构建脚本**：`build` / `pack-dir` 加 `--fallback-base-url`、`emit-feed-config` 加 `--fallback-base-feed-url`，未传则不写回落字段。
- **待接（后续）**：CI release job 把 `build` 产物同时上传 GitHub Release + OSS 镜像，并写 `fallbackBase*`；OSS bucket/endpoint/密钥由部署侧提供。
- 验证：`cargo test --lib` 450 passed 无警告（+2 feed +2 store 回落测试；含「不合法源绝不 fetch」「主不可达→镜像」）。

## 落地记录：awam-file managed pack 产物构建跑通（P0-2，2026-11-19）

awam-file 与 tool-manager 同为自有 Python 后端，可复用同一条 in-tree 构建→onefile→feed→managed spawn 路。关键差异与修法：

- **包式入口**：tool-manager 是扁平 `src/server.py`；awam-file 是包 `awam_resource_gateway/__main__.py`（`from awam_resource_gateway.*` + adapters）。`build_pack_feed.py build` 因此加了 `--collect`（→`--collect-all awam_resource_gateway`）、`--paths`（→`--paths src`）、`--add-data` 透传 PyInstaller 参数。
- **静态资源内嵌**：`http_api.py` 模块期 `Path(__file__).parent/"filesystems.html".read_text()`；仅 `--collect-all` 不收该 .html → frozen 下 `FileNotFoundError`。修复：`--add-data "src/awam_resource_gateway/filesystems.html;awam_resource_gateway"`。
- **验证（隔离端口 18799，避开用户自跑 18787 实例）**：onefile 产物独立 serve——`/health` `{ok:true,service:awam-resource-gateway}`、`/` filesystems 页 http 200（html 已内嵌）。manifest 现含 `entries: [tool-runtime, file-gateway]`、`packs: [tool-runtime, file-gateway]`（`pack-dir` 产 928B 安装包）。
- **managed/config 共存（按建议）**：pack 为 managed 权威（enable→feed entry→spawn 18787）；现有 `providers/gateway` config-mode 作为连用户自跑实例的兜底，二者共用 18787、不改 providers、互不冲突。
- **仍待（P0-2 余项）**：file UI 表面沿用现有 awam:// 资源/Providers（pack 本身 sidecar-only，无新内置视图）；真机在装好 AFM 里「装→启用→managed spawn」端到端手测。

## 落地记录：移除遗留 vNext 扩展宿主（候选1 / C-2，2026-11-19）

兑现 C-2 拍板「删 vNext 宿主、仅保留 permission 枚举」——把"能力扩展唯一单元 = Feature Pack"落到代码，消除四套里的最后一套遗留：

- **后端** `extension_host`：删 `ExtensionManifestVNext` / `InstalledExtensionRecord` / `MarketplaceCatalog` / `InstallResult` / `UiCommandRequest` / `UiCommandResult` / `FsProviderContribution` 与 `EXTENSION_HOST_API_VERSION`；host 删 `extensions`/`fs_hooks` 字段及 `install_from_manifest`/`set_enabled`/`uninstall`/`dispatch_ui_command`/`list_fs_provider_hooks`/`register_fs_provider_hook`/`marketplace_catalog`（含 `__crash__` 桩）。`extension_host_status` 改为 **pack 派生**计数（apiVersion=`feature-pack.1`、installed/enabled/crashed 来自 packs）。删对应 IPC 与 lib.rs 注册、删 vNext 单测。
- **保留**：`ExtensionPermission` 枚举（`spawnSidecar`/`uiCommands`/`network`/`fsProvider` 仍在，pack.json 在用，不动以免破坏已装清单）、全部 Feature Pack 路径（install_pack / feed 安装 / list_packs / set_pack_enabled / uninstall_pack / list_contributions / load_from_disk）。
- **前端** `ExtensionsView`：删「示例扩展」区 / 「Marketplace」卡 / 「FS provider 钩子」区 / `DEMO_MANIFEST` / `installDemo`/`runHello`/`setEnabled`/`uninstall`；status 卡简化为 pack 计数。i18n 旧键（demoTitle/marketplace/fsHooks 等）保留但不再引用（不影响 parity）。
- **验证**：`cargo test --lib` 453 passed、`vue-tsc` 0、`check-i18n` parity 1431×5。至此"三套扩展机制"对外只剩 **Feature Pack** 一种能力扩展单元（AI Add-on / Open tools 仍按 C-1 归外部托管面，不抢"扩展"词）。

## Consequences

- ADR 0019 的白名单/元清单/SHA-from-GitHub 全套下载逻辑不再作为 managed-mode 的唯一前提；managed 改为 AFM 自产产物。`store.rs`（allow-list + 下载 + SHA-256 摘要校验）由「待接线」升为 managed **主路径**：经 `feed::fetch_manifest` 取 AFM 自有元清单、`provision::ensure_artifact` 按需下载校验落盘（见上「B-正式」落地记录）；config-mode 仍可连用户自跑实例不经此路。
- AFM 的 CI 复杂度**重构**：不再为每个依赖各冻结一个自带解释器的隔离包，而是维护**共享 Python 环境 + 共享 Node 运行时**两套（per-OS），各能力产物退化为 wheel/源码薄产物。代价 = 一次 mcp 跨大版本迁移与共享环境可重定位打包；收益 = 体积下降、构建目标减少。`runtime: self` 产物仍可走原隔离档作为逃生口。
- 安装包体积：共享运行时只装一份 Python + 一份 Node，明显小于「每个服务各带一个解释器」；knowledge 内置，file/tool-manager 薄产物按需拉自有 feed。
- 版本门语义在 0013/0020 文字上需对齐：0020 Q15「managed spawn 前过 versionCompat 门」对本自构建产物**不适用**，改为对 config-mode 外部实例适用。
- 更新节奏：能力后端 bugfix 随 **AFM 发布/更新 feed** 分发，而非独立商店轮询。
