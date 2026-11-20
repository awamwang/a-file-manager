# AFM 托管产物 Feed 运维 Runbook（本机 dogfood → 线上发布）

> 配套决策见 `docs/adr/0022-afm-integrator-in-tree-backend-build.md`（AFM 作为集成方、自有 feed）。本文只讲**运维动作**：怎么构建、怎么发布、怎么切线上、怎么 bump、升级/检查何时触发、出问题怎么查。

## 0. 一句话拓扑

feed 是 **AFM 自有发布通道**（不是第三方 GitHub release）。一份 `manifest.json` 同时描述两个面：

- `entries[]` —— 每个托管后端的**产物**（如 `awam-tool-manager.exe`）。managed 拉起时按需下载。
- `packs[]` —— 可安装的 **Feature Pack 安装包**（zip：`pack.json` + UI 资源）。扩展页一键安装。

AFM 客户端只认**一个 `manifestUrl`**。本机→线上切换 = 只改客户端 `managed-dependency-feed.json` 的 URL 与 allow-host，**代码与产物格式不变**。

## 1. 文件与字段速查

**客户端配置** `{app_config}/managed-dependency-feed.json`（Windows 下 `{app_data}` 与 `{app_config}` 同为 `C:\Users\<u>\AppData\Roaming\com.a-file-manager.app\`）：

```json
{
  "manifestUrl": "http://127.0.0.1:8078/manifest.json",
  "allowHosts": ["127.0.0.1"]
}
```

**feed 目录布局**（被某个 http 服务喂出，本地 dogfood 用 `python -m http.server 8078`）：

```
afm-feed/
├─ manifest.json
├─ artifacts/awam-tool-manager.exe      ← entries.assetUrl 指向这里
└─ packs/awam.tool-runtime.zip          ← packs.installUrl 指向这里
```

**manifest schema**（与 `store.rs::StoreManifest` 对齐，`schema` 固定为 `1`）：

```json
{
  "schema": 1,
  "entries": [
    {
      "packId": "awam.tool-runtime",
      "artifactVersion": "0.1.0",
      "assetUrl": "http://127.0.0.1:8078/artifacts/awam-tool-manager.exe",
      "sha256": "<64位十六进制>",
      "fileName": "awam-tool-manager.exe",
      "baseUrl": "http://127.0.0.1:18778"
    }
  ],
  "packs": [
    {
      "packId": "awam.tool-runtime",
      "name": "工具管理运行时",
      "version": "0.1.0",
      "installUrl": "http://127.0.0.1:8078/packs/awam.tool-runtime.zip",
      "sha256": "<64位十六进制>"
    }
  ]
}
```

字段含义与坑：

| 字段 | 作用 | 注意 |
|---|---|---|
| `entries[].sha256` | **版本权威**：AFM 用它比对盘上产物、判断是否"有可用升级" | 变了 = 有新产物；务必与真正上传的文件一致 |
| `entries[].artifactVersion` | 展示/记录用版本 | 建议随 sha 一起 bump |
| `entries[].baseUrl` | managed 拉起后 AFM 探活/附着的地址 | **必须等于实例实际监听端口**（tool-manager 现为 `18778`），否则连不上 |
| `entries[].fileName` | 落盘名（缺省从 assetUrl 末段推断） | onefile 名 = spawn 的 exe 名 |
| `packs[].installUrl/sha256` | 一键安装的 zip 来源与摘要 | zip 根须含 `pack.json` |

**allow-list 不变量（安全关键，勿破）**：`manifestUrl`、每个 `assetUrl`、每个 `installUrl` 的主机都必须在 `allowHosts` 内（精确主机或点分子域）。**不在 → AFM 绝不发起该下载**，直接报错。切线上时这三类 URL 的域都要加进 `allowHosts`。

## 2. 构建期动作（发布机 / 本地 dogfood）

统一用 `scripts/build_pack_feed.py`，三个子命令：

```bash
# (1) 冻结某依赖仓源码 → 单文件 onefile 产物 + 写 manifest.entries（合并、保留 packs 段）
python scripts/build_pack_feed.py build \
  --pack-id awam.tool-runtime --name awam-tool-manager \
  --src "G:/Projects/19AI/skills/awam-tool-manager" \
  --entry src/server.py --version 0.1.0 \
  --out "<feed 目录>" --base-feed-url http://127.0.0.1:8078 \
  --base-probe-url http://127.0.0.1:18778

# (2) 把成形 pack 目录打成安装包 zip + 写 manifest.packs（合并、保留 entries）
python scripts/build_pack_feed.py pack-dir \
  --pack-id awam.tool-runtime --name "工具管理运行时" --version 0.1.0 \
  --src "packs/tool-runtime" --out "<feed 目录>" --base-feed-url http://127.0.0.1:8078

# (3) 生成客户端 feed 配置（指向某 feed 源）
python scripts/build_pack_feed.py emit-feed-config \
  --out "<feed 目录>" --app-config-dir "<AFM app_config_dir>" \
  --base-feed-url http://127.0.0.1:8078 --allow-host 127.0.0.1
```

要点：

- **`build` 会带 `--add-data "<src>/data;data"`** —— 这是托管实例 `list_tools` 不致空的关键（server.py 在 frozen 下从 `_MEIPASS/data` 读声明）。换其它 Python 后端同理需带各自的 data。
- **每个 OS 单独 build**：PyInstaller 冻结不可跨平台。Win 产物只能在 Win 上产；Linux/macOS 各跑对应 agent。
- `--base-probe-url` 写进 entry 的 `baseUrl`，要与该后端实际监听端口一致（tool-manager = 18778；改端口要三处同步：后端 `DEFAULT_PORT`、feed `baseUrl`、AFM 默认）。

## 3. 接线上 feed（dogfood → 生产）

1. 把 `artifacts/*`、`packs/*.zip`、`manifest.json` 上传到 AFM 自有可访问位置（AFM release 资产 / 对象存储 / 自建 http）。URL 用 `https`。
2. 改客户端 `managed-dependency-feed.json`：
   - `manifestUrl` → `https://<AFM 线上域>/.../manifest.json`
   - `allowHosts` → 加上线上域（manifest 与 asset/install 同域即可，多域则都列）
3. AFM 无需改代码：feed 拉取/下载/SHA 校验/落盘/spawn/升级全走既有接缝。
4. **CI 已接（P2-5）**：`release.yml` 的 `build-pack-feed` job 在打 tag 时 `checkout` 带 `submodules: recursive` → 按各仓 lockfile 从 `upstream/awam-*` 源码产 onefile + manifest（flat URL）→ 上传 GitHub Release（主源）；配了 `OSS_*` secrets 再镜像对象存储（回落源），未配则跳过。**当前仅产 Windows 产物**；macOS/Linux 需加 runner + `manifest.<os>.json` 与按 OS 选 manifestUrl（后续）。整链以一次真实 release 运行为准。

## 3.1 源码来源（submodule）

AFM 通过 `upstream/` 下的 git submodule 锁定三个后端源码（`awam-file`@`056e020`、`awam-tool-manager`@`5537eb5`、`awam-knowledge`@`381b83c`，均 `github.com/awamwang/*`）。

- 克隆 AFM 后初始化：`git submodule update --init --recursive`。
- in-tree 构建直接用锁定源码，CI 与本地一致：`build_pack_feed.py build --src upstream/awam-tool-manager --entry src/server.py …`。
- 升级某后端：在该仓提交并 push → AFM `git submodule update --remote upstream/<name>` → 提交新 gitlink（provenance 记录在 AFM 历史）。
- 本地 dogfood 也可 `--src` 指你自己的工作副本路径，submodule 是"来源固定"的权威，不强制。

## 3.5 下载回落链（GitHub 主源 → 对象存储镜像）

发布是**一条链、两个源**：优先 GitHub Release（AFM 自有 release 资产），主源不可达 / 传输失败 / 摘要不符时**自动回落对象存储镜像**。具体 OSS bucket / endpoint 配置后续接，但接缝已就位：

- 客户端 `managed-dependency-feed.json` 填 `fallbackManifestUrl`（OSS 上的同一 manifest.json），并把 OSS 主机加进 `allowHosts`。
- manifest 的 `entries[].assetFallbackUrl` / `packs[].installFallbackUrl` 指 OSS 镜像同内容文件。
- **默认惰性**：不填上述回落字段 = 只走主源，行为不变。
- **allow-list 不变量对回落同样成立**：回落源主机不在 `allowHosts` 内 → 绝不 fetch 它。两路都失败时返回首个错误（供前端提示 / 启动静默跳过）。
- 构建期用 `build` / `pack-dir --fallback-base-url <OSS基址>`、`emit-feed-config --fallback-base-feed-url <OSS基址>` 自动写回落字段。

## 4. 版本 bump 标准流程（发一次新产物）

1. 依赖仓改源码 → 固定 tag / commit SHA（provenance）。
2. 目标 OS 上跑 `build` → 新 exe + 新 `sha256` 覆盖写 `manifest.entries[packId]`。
3. 若 UI / `pack.json` 也变 → 跑 `pack-dir` → 新 zip `sha256` 写 `manifest.packs[packId]`。
4. 上传 + 更新线上 `manifest.json`（或本地 feed 即时生效，`http.server` 直接读盘）。
5. **客户端无需重装/重启也可拿到提示**：用户点扩展页「检查更新」即检出 `available_version`；下次启动也会自动检查。

> sha 即真相：只要 `entries[].sha256` 与盘上产物不一致，AFM 就判定"有可用升级"。`artifactVersion` 只用于展示「当前 → 可用 vX」。

## 5. 运行期触发时机（答疑用）

| 时机 | 动作 | 是否下载 | 是否拉起/替换 |
|---|---|---|---|
| **AFM 启动** | 后台 `check_updates`（纯读刷标记）→ `reconcile`（attach_or_spawn 拉起/附着已启用托管 pack） | 缺产物才下 | 会（拉起/附着） |
| **启用 pack** | host 按 feed entry 切 Managed + 写 `base_url`/`declared_version`；前端随即 `ensure` → provision + spawn | 缺产物才下 | 会 |
| **点「检查更新」** | `managed_dependency_check_all_updates`：清单拉一次 + 哈希比对盘上产物，刷 `available_version` | **否** | **否**（纯读） |
| **点「升级」** | `managed_dependency_upgrade`：**停自启 pid → force_provision 重下校验替换 → 重新拉起 → 清 `available_version`** | 是 | 是（替换 + 重启） |

**非静默铁律**：除「升级」按钮外，任何路径都**不会**替换已落盘的产物。`ensure`/启动恢复检测到 sha 不符时只置 `available_version`、继续用旧产物服务。**不存在后台偷偷换二进制**。

**停旧是为了替换**：Windows 上运行中的 exe 无法被覆盖，所以升级先 `stop_self_spawned`（且只 kill AFM 自己启的 pid，Q8）再 `force_provision`。

## 6. 现状边界与遗留（运维须知的"还没自动化"的部分）

- feed 发布目前**手动跑脚本 + 手动上传**；CI release job 未接。
- 当前只有 `awam.tool-runtime` 一个成形 pack（`packs/tool-runtime`）+ 产物；`awam-file` / `awam-knowledge` 尚无 pack 目录/产物 feed 条目（knowledge 计划"内置"、file 计划"按需拉"，均待建）。
- `runtime: shared`（AFM 自带共享运行时）语义已决策但**产物仍全走 `runtime: self`（onefile 自带解释器）**；共享 env 安装/薄产物分发未落地。
- 遗留任务全清单见本次交付说明与 ADR0022「跨仓待办 / 落地记录」。

## 7. 排障速查

- **网关连不上 / 报 timeout 某地址**：① entry `baseUrl` 端口是否 == 实例实际端口（默认 tool-manager=18778 / file=18787 / knowledge=18790；**若该端口被非本服务占用，AFM 会自动顺延到邻近可绑端口并写回 record**，见下「端口被占」）；② 该 host 是否在 `allowHosts`；③ managed pack 的地址是**只读**的（来自 record.base_url），不在工具页改；config-mode 实例地址才在工具页填。
- **端口被占**：托管 spawn 前若目标端口探活不通且不可绑（被别的服务占），AFM 顺延到 `+1..+32` 内第一个可绑端口、写回 `record.base_url`、经 pack 的 `port_env`（`AWAM_HTTP_PORT`/`AWAM_GATEWAY_PORT`/`KM_SERVICE_PORT`）注入子进程，并把实际地址同步进 file/knowledge 客户端。所以 entry `baseUrl` 的端口只是**首选默认**，不必为偶发冲突去改后端或 feed；想复位回默认端口：扩展页对该 pack **禁用再启用**（启用从 feed entry 重读 base_url）。pack 未声明 `port_env` 则不回退（沿用其内置默认端口）。

- **工具列表空（托管）**：产物没带 `--add-data data` → 重 build。托管与源码 config-mode 应一致返回声明工具数。
- **点「升级」失败**：多为旧进程未停导致 exe 文件锁 → 看 `record.spawnedPid`，AFM 升级会先停它；若 pid 不属于 AFM（你手动起的）则停不掉，需先手动停那个进程。feed 主机不在 allow-list → 报 `not allow-listed`。
- **`available_version` 反复出现**：盘上产物 sha 与 entry 不符。要新版就点「升级」接受；不要就回退 manifest entry。AFM 不会自动消解这个标记。
- **feed 不可达**：启动检查静默跳过、不报错阻断；手动「检查更新」会如实报错。managed pack 仍用**已落盘的旧产物**继续服务（这正是非静默的容错）。

## 8. 安全不变量清单（改动时逐条守住）

1. allow-list 检查**先于任何网络请求**（manifest 与 asset/install 都查）。
2. 下载后**必须 SHA-256 校验**才原子落盘（`.tmp` + rename）。
3. **非静默替换**：仅手动「升级」可覆盖运行产物。
4. 停止权只作用 AFM 自启 pid（`record.spawned_pid`）；附着/外部实例绝不 kill。
5. 进程态唯一权威 = `ManagedDependencyRecord.spawned_pid`（无 `last_sidecar_pid` 残留）。
