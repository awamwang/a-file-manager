# Provider 查询参数与隐藏 Everything（search）设计

日期：2026-11-12  
状态：已批准（待实现）  
仓库：`a-file-manager` + `awam-file`（`G:\Projects\19AI\skills\awam-file`）

## 背景

AFM 通过 awam-file 网关发现虚拟 FS 后端。其中：

- backend id `search`（显示名 Everything search）与 AFM 已接入的 Discovery Everything 搜索能力重复。
- 部分路径在缺少 query 时无法列出，例如：
  - `awam://search/everything` → 需要 `?q=`
  - `awam://prowlarr/search` → 需要 `?q=`
  - `awam://hf/remote` → 需要 `?q=`

当前 Navigator / Providers 仅导航到 `awam://<backend>`，没有传参 UI；进入缺参叶子路径会直接报错。

## 目标

1. **隐藏并禁用** Everything 网关后端（`search`）：Providers、资源管理器根、Guide、预加载均不出现；手动输入 `awam://search…` 也拒绝，并提示改用 Discovery「搜索」。
2. **为需要 query 的 provider 路径**提供地址栏旁的查询输入区；按契约元数据渲染一个或多个字段；回车后导航到带 query 的 URI（写入历史）。
3. **扩展 awam-file 契约**，使所需参数可自描述；改完 awam-file 先单独提交，再实现 AFM 消费端。

## 非目标

- 不改动 Discovery / Everything IPC 搜索本身。
- 不在本设计中为所有后端增加通用「表单配置」页。
- 不实现破坏性写操作或下载投递（Prowlarr 仍只浏览搜索结果）。

## 决策摘要

| 项 | 选择 |
|---|---|
| 查询 UI 位置 | 地址栏旁常驻字段区，仅在当前路径需要 query 时显示 |
| 元数据附着 | 父目录条目 + 当前 `DirListing`（进入前可知、进入后自描述） |
| Everything | 完全屏蔽（含手动 URI） |
| 提交查询 | 回车 → 导航到带 query 的 URI（可后退） |
| 多参数 | 按 `requiredQuery` 数组渲染多个字段 |
| 空查询 | 去掉对应 query，停留在待查询态，不报错打扰 |
| 改动范围 | awam-file + AFM；awam-file 先 commit |

## 架构

```
awam-file                          AFM
─────────                          ───
adapters 声明 requiredQuery   →    gateway.rs 解析可选字段
DirListing / ResourceEntry    →    BackendInfo / DirListing / ProviderEntry
                                   ↓
                              过滤 backend id == "search"
                                   ↓
                              Navigator chrome：按 requiredQuery 渲染字段
                                   ↓
                              Enter → navigateTo(uri?key=value…)
```

## 契约（awam-file）

### QueryParam

```json
{
  "key": "q",
  "label": "Query",
  "required": true
}
```

- `key`：必填，写入 URI query 的键名。
- `label`：可选，UI 占位/标签；缺省可用 `key`。
- `required`：可选，默认 `true`。本期 AFM 对 `required: true` 的字段在提交时校验非空（清空走「待查询」路径时不提交空 required）。

### ResourceEntry

新增可选字段：

```json
"requiredQuery": [ { "key": "q", "label": "Query", "required": true } ]
```

用于父目录列出子项时声明「进入该子项需要这些参数」。可同时保留现有 `extra.hint` 作人类可读提示，但 **AFM 以 `requiredQuery` 为准**。

### DirListing

新增可选字段：

```json
"requiredQuery": [ { "key": "q", "label": "Query", "required": true } ]
```

当**当前 URI** 处于需要 query 的路径时始终带上（无论是否已有 `q`、是否有结果）。

### 缺参行为

对需要 query 且当前缺少必填参数的路径：

- `entries`: `[]`
- `requiredQuery`: 非空
- `error`: `null`（或仅软提示字符串；**不得**再依赖 “Missing query…” 作为唯一信号）

这样 AFM 可展示待查询空态，而不是错误横幅。

### 适配器改造范围（最低集）

| 后端 | 父条目声明 | 当前 list 声明 |
|---|---|---|
| `search` / `everything` | `awam://search/` 下 everything 条目 | `awam://search/everything` |
| `prowlarr` / `search` | `awam://prowlarr/` 下 search 条目 | `awam://prowlarr/search` |
| `hf` / `remote` | `awam://hf/` 下 remote 条目 | `awam://hf/remote` |

`docs/uri-spec.md` 补充 `requiredQuery` 说明。

### 兼容

旧客户端忽略未知字段仍可工作。旧网关无该字段时，AFM 不显示查询条（不回退到白名单硬编码，除非后续另开任务）。

## AFM 行为

### 隐藏 `search`

在以下路径统一过滤 `backend.id === "search"`（大小写不敏感）：

- `provider_backends` / Providers 页卡片
- `awam://root` 列表条目
- NavigatorGuide providers 区
- depth-1 预加载

`navigateTo` / 地址栏提交：若目标 URI backend 为 `search`，拒绝导航，提示使用 Discovery 搜索页（i18n）。

说明：网关侧仍可保留 Everything 适配器供其它消费者；AFM 不展示、不启用。

### 查询 UI

- 数据源优先级：当前 `DirListing.requiredQuery` → 若从父条目带入的缓存元数据（进入瞬间 list 尚未返回时可用）。
- 有非空 `requiredQuery` 时，在地址栏旁渲染对应输入框（多字段按数组顺序）。
- 字段初始值：从当前 URI query 解析。
- **Enter**（或显式「搜索」）：合并/更新 query → `navigateTo` 完整 URI → 写入导航历史。
- **清空必填主查询并提交/确认清空**：去掉对应 query 键，导航到无参 URI，停留待查询态；`error` 不展示为失败。
- 非 query 路径：不渲染该区域。

### Providers 页

- 后端卡片打开时仍 `openInExplorer(awam://id)`；若该后端根本身不需要 query，行为不变。
- 进入子路径后由 Navigator chrome 承接查询条。
- URI 手工输入框可继续用于高级调试；`search` 仍被导航层拒绝。

## 错误处理

| 情况 | 行为 |
|---|---|
| 手动 `awam://search…` | 拒绝 + 提示改用搜索页 |
| 缺参待查询 | 空列表 + 查询条；无错误横幅 |
| 有参但引擎失败（如 Everything 未运行） | 保留现有真实 `error` 展示 |
| 旧网关无 `requiredQuery` | 无查询条；用户仍可用地址栏手写 `?q=`（除 `search` 外） |

## 测试要点

**awam-file**

- everything / prowlarr / hf 父条目与缺参 list 均含 `requiredQuery`
- 缺参时 `error` 为空（或约定软提示），`entries` 为空
- 有 `q` 时行为与现网一致

**AFM**

- backends / root / guide / preload 不含 `search`
- 导航到 `awam://search…` 被拒绝
- 进入 `prowlarr/search`、`hf/remote` 显示查询字段；Enter 后 URI 含 query 且可后退
- 多字段（若契约返回多个）均渲染并写入 URI
- 清空查询回到待查询态且无错误横幅

## 提交策略

1. **awam-file**：契约字段 + 三适配器 + uri-spec → 单独 commit  
2. **a-file-manager**：过滤 `search` + 解析契约 + chrome 查询条 + i18n → 另 commit（实现阶段按用户指示提交）

## 开放风险

- AFM 与网关版本可能短暂不一致：无字段时 query UI 不可用（可接受）。
- `prowlarr?q=` 挂在根 URI 的现有怪异行为不作为产品路径；以 `/search?q=` + 契约为准。
