# 统一托管依赖模型与控制面拆分

把 awam-file 网关、awam-tool-manager 网关、awam-knowledge 服务三个依赖收敛成一套 Core `ManagedDependency` 抽象：既可「依旧用配置连接」，也可「由 AFM 托管」。依赖 ADR 0019（白名单商店）；取代/修订 ADR 0014；沿用 ADR 0013 的 attach-or-spawn 语义。

**Status**: accepted — 被 ADR 0022 修订两处：R3 隔离产物来源改「AFM 自构建」（不再上游 release 资产）；Q15 versionCompat 门从「managed spawn 前」移至「config-mode 附着外部实例」（自构建产物版本构建时锁定）。其余抽象、控制面拆分、1:1、停启权边界不变。

## Decision

- **两种模式（非互斥，带 fallback 的偏好）**：每条 `ManagedDependency` 记录声明
  - `config-mode`：连接远程 / 已有实例（现有能力保留，默认入口）。
  - `managed-mode`：AFM 拉起 / 守护**本机的自带隔离产物**（R3，产物来自 ADR 0019 商店下载；非共享运行时，见下「运行时矩阵与约束」）。
- **优先级（沿用 0013）**：先探 `config-mode` 指向的实例，健康即附着；不健康或未配置才走 `managed-mode` 拉起。
- **停启权边界**：`managed-mode` 的停止 / 重启**只作用于 AFM 自己 spawn 且记录 pid 的进程**；绝不 kill / 重启用户自启或非 AFM 持有的远程实例。
- **配置归属切换（无迁移期）**：awam-knowledge 的 URL/token（现 `knowledge_service` / Core settings）与 awam-file 网关地址（现 `AWAM_GATEWAY_URL` / Settings→高级）**读写路径直接切到** `ManagedDependency` 记录；旧 Core settings 键**直接废弃删除**。**软件未上线、无存量用户数据，不做向后兼容迁移、不做旧值回退读、不设回滚棘轮**（Q16）。Settings→高级 退化为 `config-mode` 的「连已有实例」入口。
- **声明版兼容门的版本来源（Q15=a）**：managed spawn 前，用**元清单为每个 release 产物声明的 `version`** 落进 pack `versionCompat{min,max}` 区间校验（复用 `semver_compat::version_in_range`）；不匹配拒绝拉起。config-mode 只附着已有实例，**不经此门**。（字段由 `gatewayCompat` 更名 `versionCompat`，Q19。）
- **停启权权限语义（Q17c）**：`spawnSidecar` 权限**隐含**「AFM 可停止自己 spawn 的该产物进程」；**不新增** `manageSidecar`/`stopSidecar` 权限，避免权限爆炸。停仍严格受限自启 pid（见上）。
- **1:1 强制度（Q21）**：`ManagedDependencyRecord` ↔ `InstalledPackRecord` 的 1:1 **靠单写路径保证**——只能通过 pack 启用/禁用的 API 建/销托管记录，注册表不开放独立增删；启动时再扫一遍**兜底**（删孤儿：注册表有但 pack 已卸；报错不一致）。注册表与 pack 记录仍是**两套存储、进程态不并入 `InstalledPackRecord`**（保持解耦）。
- **泛化 sidecar 编排**：把现硬编码到 `awam.tool-runtime` 单 pack 的 `find_tool_runtime_pack` / `TOOL_RUNTIME_PACK_ID` 编排提为 pack 无关——任意 pack 的 `SidecarContribution` 都走同一 attach-or-spawn。`InstalledPackRecord` 仍**不**承载进程态；进程 / 健康态属于 `ManagedDependency` 记录（与包记录分离）。
- **控制面拆分**（用 `ManagedDependency` 注册表缝合两个面）：
  - **Health / deps_probe = 统一「依赖」面**：承载运行时启停、健康、mode 选择。给 `DependencyItem` 增 `mode(config|managed)` 与可动作（start / stop / health）字段；文案「未运行→请你自己启动」升级为「未运行→[由 AFM 启动]」。
  - **ExtensionHost `/extensions` = 包 / 商店 / 版本面**：一个依赖要被托管，先作为 pack 安装/启用（贡献 spawn recipe、UI、商店条目）。
- **Pilot（R3 直达，不走共享运行时分）**：先给 awam-file 做 pack 封装，但**前置**是 awam-file 仓先产出带 release 摘要的隔离产物；再打通「商店下载隔离产物 → managed spawn → Health 探活/启停 → config fallback」全链路。knowledge 因零依赖最轻，可并行；tool-manager 因 mcp 版本敏感，必须用隔离产物（严禁与 file 共享环境）。

## 运行时矩阵与约束

调查四仓后钉死的事实（决定"AFM 提供运行时"只能是 R3 隔离产物，不能是共享环境）：

| 依赖 | 运行时 | 版本约束 | 常驻服务? | 端口 | 鉴权 |
|---|---|---|---|---|---|
| awam-file | Python | ≥3.11，`mcp>=2.0` | ✅ | 18787 | 无 |
| awam-tool-manager | Python | `mcp>=1.29,<2.0`（2.x 起不来）| ✅ | 18778/mcp | Bearer token，`/health` 免鉴权 |
| awam-knowledge | **Node** | ≥18，**零依赖** | ✅ | HTTP（配置 URL/token）| token |
| knowledge-desensitization | Python | 3.10+ | ❌ 批处理 | — | — |

- **环境不可共享**：file 与 tool-manager 的 mcp 大版本互斥 → 绝不能同一 venv；运行时种类 = Python + Node 两种。结论：AFM 不"提供单一共享运行时"，而是**每依赖 spawn 各自隔离产物**（R3）。
- **knowledge-desensitization 排除出托管**：它是一次性脱敏 pipeline（真源→工作副本→`build_index` 喂 knowledge），无常驻进程可 attach/spawn，**不是 `ManagedDependency`**。9b 的「托管依赖:pack = 1:1」只适用于 file / tool-manager / knowledge 三个**服务**。
- **统一启动的边界**：接口层可统一（本机 HTTP + `/health` 探活 + `SidecarContribution{command,args,workingDirRel}` 表达三者）；差异点是 **credential 来源**（tool-manager token 文件/`AWAM_HTTP_TOKEN`、knowledge token、file 无）须由 ManagedDependency 记录带 `credential` 字段统一承载。

## Consequences

- 新增一个跨 file/tool-manager/knowledge 的 `ManagedDependency` 注册表，是与现有 `ProviderRegistry`、`ExtensionHost` 并列的新一等概念；新增 i18n key **单独命名空间 `managedDep.*`**（5 locale，便于 `test:i18n` 定位，Q17b）。
- **门禁边界（Q17a）**：`check-feature-pack-shell.mjs` 静态门禁只守「壳接线」（既有职责，可加一条断言 deps_probe 从注册表读、`DependencyItem.mode` 存在）；托管**正确性**交给 `managed_dependency` 模块单测 + 一条 R3 e2e，不塞进静态门禁。
- deps_probe 由只读状态面转为可操作面，须保证「只停自启 pid」的边界在 UI 与后端双向落实。
- **双失败态映射（Q18）**：不新增 `DependencyItem.status` 枚举（保持 `ok|missing|warn|unbridged|indeterminate` 向后兼容），改用**正交字段** `unavailableReason: enum{configOnly, managedOnly, both}`。`status=missing` + `actionable=false` + `unavailableReason=both` 表达「config 不可达且无 managed 产物」，避免误导成「AFM 能帮你启动」。
- 配置切换无迁移成本（软件未上线）：旧 `AWAM_GATEWAY_URL` / `knowledge_service` settings 键直接从代码删除。
- 商店（0019）是托管的前提：未接入白名单下载产物前，`managed-mode` 仅对已具备本地产物的 pack 可用。
