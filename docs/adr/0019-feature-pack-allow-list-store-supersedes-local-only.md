# Feature Pack 商店采用白名单（取代「仅本机安装」）

取代 ADR 0014 的「v1 仅本机安装、不做远程目录/商店」一条。其余 0014 条款仍然有效，并由 ADR 0020 沿用：token 不进 Core settings / AI 中央配置、连接偏好存 Pack 私有配置、Capability 参数表单 v1 仅覆盖 string/number/boolean/enum、清单声明的版本兼容范围不匹配则拒绝盲调（该字段由 `gatewayCompat` **更名为 `versionCompat`**，因托管对象不止「网关」——knowledge 是服务、file/tool-manager 角色各异；Q19）。

**Status**: superseded-on-provenance by ADR 0022 (was: accepted, supersedes 0014 「不做商店」条款) — 「产物来源=GitHub Release 资产、元清单下载、SHA 取自附属资产、要求上游 release job」被 ADR 0022 改为「AFM 从源码 in-tree 自构建、自有 feed 分发」；allow-list 退化为 AFM 自有发布通道，本机 zip/目录兜底保留。其余关于 versionCompat 更名、token 不进 Core settings 等条款由 0020 沿用、0022 收窄门范围。

## Decision

- 引入**非自由的商店**：仅一组受批准来源（特定 GitHub 仓库 / 直链），不开放任意第三方投稿。
- 来源形态为**元清单**：AFM 拉取一份「元清单」，列出被批准的 pack、其产物 release 指针与**产物 `version`**（供版本门，见 ADR 0020）；再按需下载产物到 pack 目录（沿用 `install_from_directory` 的临时目录再 rename 原子写）。
- **元清单地址可配置（Q20=c）**：地址在 设置 内可改，默认指向 **GitHub**（本组织 release/静态资源），**后续可切国内镜像**；本机可缓存上一次成功的元清单。
- **连不上降级**：① **元清单不可达** → 商店不可用 → **直接不显示对应依赖的安装/managed 入口**（不是报错，是隐身；config-mode 不受影响，仍可用）。② **某产物 release 资产不可达** → 仅该依赖 managed 安装失败、提示重试或回退本机 zip/目录；其余 pack 不受牵连。
- 产物形态为**每依赖自带隔离产物（R3）**：不是共享系统运行时，而是自带解释器/依赖的隔离包（PyInstaller onefile / 含 `.venv` 的 zip / wheel+launcher）。pack 的 `SidecarContribution.command` 直接 spawn 该产物，使「商店下载」成为「托管启动」的前提（见 ADR 0020）。
  - 之所以必须是隔离产物而非共享环境：awam-file 要 `mcp>=2.0`，awam-tool-manager 锁 `mcp>=1.29,<2.0`，二者大版本互斥，**无法共用一个 venv**；且 knowledge 是 Node、另两个是 Python，运行时是两种。隔离产物彻底消解冲突。
- **前置条件（当前均不满足）**：三来源仓现在**只有 `publish-check.yml`，无任何 release/打包 job**，GitHub Release 不产出可下载资产。R3 落地前，每个来源仓必须先加「产出带校验值的 release 资产」步骤，否则 managed-mode 无产物可下。
- 产物校验值（SHA-256）**取自 GitHub Release 附属资产**（release 内自动生成/登记的摘要），元清单只做指针、不手填哈希；资产缺失则拒绝安装、回退本机 zip/目录。
- 仍**不承诺任意代码热加载**：只安装清单校验通过、来源在 allow-list 内的 pack。
- **allow-list 只约束 managed-mode 产物来源**；`config-mode` 用户可填任意远程地址，不受 allow-list 限制（Q10 确认为有意的不对称）。
- 实现面：把 `MarketplaceCatalog.fetches_remote` 从恒 `false` 改为受 allow-list 约束的真值；`extensions` 由空置升为白名单条目。

## Consequences

- `docs/feature-pack-format.md` 需补「来源/分发」段：清单/元清单声明产物 release 指针、产物 `version` 与校验值（指向 GitHub Release 资产）；`gatewayCompat` 字段更名 `versionCompat`。
- 元清单与 allow-list 的维护者 = 本项目（自有仓库），非社区自由投稿；远端只读，地址可配置（GitHub 默认 → 可切国内镜像），本机缓存上一次成功结果以耐元清单不可达。
- 网络与完整性成为安装路径的一部分：下载产物按其 GitHub Release 附属摘要校验，失败回退到「本机选择 zip/目录」的旧路径（0014 能力保留为兜底，而非唯一）。
- **跨仓耦合**：R3 把 AFM 的托管能力**前置依赖**到 file/tool-manager/knowledge 三仓各自新增的打包 release job；任一方未就绪，该依赖的 managed-mode 即不可用（config-mode 不受影响）。
