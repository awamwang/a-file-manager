# Provider 查询参数 UI 与隐藏 Everything 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 扩展 awam-file 的 `requiredQuery` 契约；AFM 完全屏蔽 `search`（Everything）后端；在地址栏旁为需要 query 的 provider 路径提供多字段查询条。

**Architecture:** awam-file 在 `ResourceEntry` / `DirListing` 上声明 `requiredQuery`；缺参时返回空列表且 `error=null`。AFM 在发现与导航层过滤 `search`，修复 `toProviderUri` 对 query 的解析，并在 AppShell 地址栏旁按契约渲染字段；Enter 导航到带 query 的 URI。

**Tech Stack:** Python 3.11+（awam-file / pytest）、Rust（Tauri providers）、Vue 3 + TypeScript（AFM）、Node 脚本契约检查。

**Spec:** `docs/superpowers/specs/2026-11-12-provider-query-and-hide-search-design.md`

## Global Constraints

- 注释使用中文；提交信息使用中文；不删原有注释。
- awam-file 改完后**必须先单独 commit**，再改 AFM。
- AFM 完全屏蔽 backend id `search`（含手动 URI）；提示改用 Discovery 搜索。
- 查询 UI：地址栏旁；Enter → `navigateTo` 带 query URI（进历史）；清空 → 去掉 query、待查询态、无错误横幅。
- 多参数：按 `requiredQuery` 数组渲染；本期适配器至少提供 `q`。
- 旧网关无字段时不显示查询条（不做白名单硬编码回退）。

---

## File map

### awam-file（`G:\Projects\19AI\skills\awam-file`）

| 文件 | 职责 |
|---|---|
| `src/awam_resource_gateway/models.py` | `QueryParam`、`ResourceEntry.required_query`、`DirListing.required_query` 与序列化 |
| `src/awam_resource_gateway/adapters/everything_adapter.py` | everything 父条目 + 缺参 list 声明 |
| `src/awam_resource_gateway/adapters/prowlarr.py` | search 父条目 + 缺参 list 声明 |
| `src/awam_resource_gateway/adapters/huggingface.py` | remote 父条目 + 缺参 list 声明 |
| `docs/uri-spec.md` | 文档化 `requiredQuery` |
| `tests/test_required_query.py` | 契约单测（新建） |

### a-file-manager

| 文件 | 职责 |
|---|---|
| `src-tauri/src/providers/types.rs` | `QueryParam`、DirListing/Entry 字段 |
| `src-tauri/src/providers/gateway.rs` | 解析网关 JSON；过滤 `search` |
| `src-tauri/src/providers/mod.rs` | `discover_backends` / root list / preload 过滤 `search`；拒绝 `awam://search` list |
| `src/state/provider/path.ts` | 正确解析/保留/合并/清除 URI query；检测 blocked search |
| `src/state/navHistory.ts` | 同步 query 感知的 URI/面包屑 |
| `src/state/provider/listing.ts` | wire 类型透传 `requiredQuery` |
| `src/state/provider/browse.ts` | 缺参待查询不抛错；暴露 requiredQuery |
| `src/state/providerCache.ts` | 会话类型含 requiredQuery |
| `src/state/navigatorBrowse.ts` | 拦截 `search` 导航 |
| `src/components/shell/AppShell.vue` | 地址栏旁查询字段区 |
| `src/views/ProvidersView.vue` / `NavigatorGuide.vue` | 依赖过滤后的 backends（一般无需特判） |
| `src/i18n/locales/{zh-CN,en}.json`（及其它已有 locale） | 屏蔽提示与查询占位文案 |
| `scripts/provider-path-checks.ts` / `check-provider-path.mjs` | 扩展 path/query 与屏蔽断言 |

---

### Task 1: awam-file — 模型与序列化

**Files:**
- Modify: `G:\Projects\19AI\skills\awam-file\src\awam_resource_gateway\models.py`
- Create: `G:\Projects\19AI\skills\awam-file\tests\test_required_query.py`

**Interfaces:**
- Produces: `QueryParam(key: str, label: str | None = None, required: bool = True)`
- Produces: `ResourceEntry.required_query: list[QueryParam] | None`
- Produces: `DirListing.required_query: list[QueryParam] | None`
- Produces: JSON 键名 camelCase `requiredQuery`（与 AFM 一致）

- [ ] **Step 1: 写失败测试**

在 `tests/test_required_query.py`：

```python
from awam_resource_gateway.models import DirListing, QueryParam, ResourceEntry


def test_dir_listing_serializes_required_query():
    listing = DirListing(
        uri="awam://hf/remote",
        entries=[],
        backend="hf",
        error=None,
        required_query=[QueryParam(key="q", label="Query", required=True)],
    )
    data = listing.to_dict()
    assert data["requiredQuery"] == [{"key": "q", "label": "Query", "required": True}]
    assert data["error"] is None


def test_resource_entry_serializes_required_query():
    entry = ResourceEntry(
        name="remote",
        uri="awam://hf/remote",
        is_dir=True,
        is_file=False,
        required_query=[QueryParam(key="q")],
    )
    data = entry.to_dict()
    assert data["requiredQuery"][0]["key"] == "q"
    assert data["requiredQuery"][0]["required"] is True
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd G:\Projects\19AI\skills\awam-file && python -m pytest tests/test_required_query.py -v`  
Expected: FAIL（`QueryParam` / 未知参数）

- [ ] **Step 3: 实现模型**

在 `models.py` 增加：

```python
@dataclass
class QueryParam:
    key: str
    label: str | None = None
    required: bool = True

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {"key": self.key, "required": self.required}
        if self.label is not None:
            data["label"] = self.label
        return data
```

`ResourceEntry` / `DirListing` 增加 `required_query: list[QueryParam] | None = None`。  
`to_dict` 中输出 camelCase：有值时写 `"requiredQuery": [p.to_dict() for p in ...]`；`None` 时省略该键（或显式 `null`，二选一后全仓统一；推荐有值才输出）。

`ResourceEntry.to_dict` 勿再裸用 `asdict` 而不转换嵌套（改为手写或后处理）。

- [ ] **Step 4: 跑测试确认通过**

Run: `python -m pytest tests/test_required_query.py -v`  
Expected: PASS

- [ ] **Step 5: Commit（awam-file）**

```bash
cd G:\Projects\19AI\skills\awam-file
git add src/awam_resource_gateway/models.py tests/test_required_query.py
git commit -m "feat: 为 DirListing/ResourceEntry 增加 requiredQuery 契约"
```

---

### Task 2: awam-file — 三适配器缺参友好 + 父条目声明

**Files:**
- Modify: `adapters/everything_adapter.py`
- Modify: `adapters/prowlarr.py`
- Modify: `adapters/huggingface.py`
- Modify: `docs/uri-spec.md`
- Modify: `tests/test_required_query.py`

**Interfaces:**
- Consumes: `QueryParam`、`required_query`
- Produces: 缺参 list → `entries=[]`, `error=None`, `requiredQuery=[{key:q,...}]`
- Produces: 父目录子项带同结构 `requiredQuery`

常量可复用：

```python
Q_PARAM = [QueryParam(key="q", label="Query", required=True)]
```

- [ ] **Step 1: 扩展失败测试**

```python
from awam_resource_gateway.gateway import ResourceGateway


def test_everything_missing_query_is_soft():
    listing = ResourceGateway().list("awam://search/everything")
    assert listing.error is None
    assert listing.entries == []
    assert listing.required_query and listing.required_query[0].key == "q"


def test_everything_parent_declares_required_query():
    listing = ResourceGateway().list("awam://search/")
    everything = next(e for e in listing.entries if e.name == "everything")
    assert everything.required_query and everything.required_query[0].key == "q"


def test_prowlarr_search_missing_query_is_soft():
    listing = ResourceGateway().list("awam://prowlarr/search")
    assert listing.error is None
    assert listing.required_query and listing.required_query[0].key == "q"


def test_hf_remote_missing_query_is_soft():
    listing = ResourceGateway().list("awam://hf/remote")
    assert listing.error is None
    assert listing.required_query and listing.required_query[0].key == "q"


def test_hf_parent_remote_declares_required_query():
    listing = ResourceGateway().list("awam://hf/")
    remote = next(e for e in listing.entries if e.name == "remote")
    assert remote.required_query and remote.required_query[0].key == "q"
```

- [ ] **Step 2: 跑测试确认失败**

Run: `python -m pytest tests/test_required_query.py -v`  
Expected: FAIL（仍有 Missing query / 无 required_query）

- [ ] **Step 3: 改适配器**

**everything_adapter.py**
- `awam://search/` 下 everything 条目：`required_query=Q_PARAM`（可保留 `extra.hint`）
- 缺 `uri.q`：`DirListing(..., entries=[], error=None, required_query=Q_PARAM)`
- 有 `q` 时：`required_query=Q_PARAM` 仍带上（便于 UI 回填）

**prowlarr.py**
- 根下列出 search 占位时：条目带 `required_query`
- `path == "search"` 且无 query：`error=None`, `entries=[]`, `required_query=Q_PARAM`（不要再返回带 hint 的占位子项冒充结果）
- 有 query 搜索成功/失败时：成功可带 `required_query`；真实后端失败仍设 `error`

**huggingface.py**
- `_virtual(..., name="remote")` 或 list 根：remote 条目设 `required_query`
- `section == "remote"` 且无 query：`error=None`, `required_query=Q_PARAM`
- 有 query 时 list 结果带 `required_query`

**uri-spec.md** 增加一小节说明 `requiredQuery` 与缺参软空态。

- [ ] **Step 4: 跑测试**

Run: `python -m pytest tests/test_required_query.py tests/test_gateway_smoke.py -v`  
Expected: 全部 PASS

- [ ] **Step 5: Commit awam-file（本阶段收口）**

```bash
cd G:\Projects\19AI\skills\awam-file
git add src/awam_resource_gateway/adapters/everything_adapter.py \
  src/awam_resource_gateway/adapters/prowlarr.py \
  src/awam_resource_gateway/adapters/huggingface.py \
  docs/uri-spec.md tests/test_required_query.py
git commit -m "feat: search/prowlarr/hf 声明 requiredQuery 且缺参不再硬报错"
```

若 Task 1 尚未单独提交，可将 Task 1+2 合并为一次 commit，消息用上面这条或拆成两条——**以「awam-file 先于 AFM 提交」为准**。

**重启本机 awam-file 网关进程**，使 AFM 后续联调吃到新契约。

---

### Task 3: AFM Rust — 类型、网关解析、过滤 `search`

**Files:**
- Modify: `src-tauri/src/providers/types.rs`
- Modify: `src-tauri/src/providers/gateway.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/registry.rs`（root list 若走 builtin 也过滤）

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueryParam {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "default_required_true")]
    pub required: bool,
}

fn default_required_true() -> bool { true }
```

- `ProviderEntry.required_query: Option<Vec<QueryParam>>`
- `DirListing.required_query: Option<Vec<QueryParam>>`
- `fn is_hidden_provider_backend(id: &str) -> bool` → `id.eq_ignore_ascii_case("search")`

- [ ] **Step 1: 写 Rust 单测（gateway 过滤 + list 拒绝 search）**

在 `gateway.rs` tests：

```rust
#[test]
fn filters_search_from_backends_payload() {
    let value = serde_json::json!({
        "backends": [
            {"id":"search","name":"Everything search","description":"x","healthy":true,"capabilities":["list"]},
            {"id":"rclone","name":"rclone","description":"x","healthy":true,"capabilities":["list"]}
        ]
    });
    let backends = parse_backends_json(&value).unwrap();
    let mapped: Vec<_> = backends.into_iter()
        .filter(|b| !is_local_backend(&b.id) && !is_hidden_provider_backend(&b.id))
        .map(map_backend)
        .collect();
    assert!(mapped.iter().all(|b| b.id != "search"));
    assert!(mapped.iter().any(|b| b.id == "rclone"));
}
```

在 `mod.rs` / `uri` 相关测试：`list_dispatch("awam://search/everything?q=a")` 返回带错误的 DirListing（或 Err），文案含引导使用 Discovery（英文/稳定 code 即可，前端再 i18n）。

- [ ] **Step 2: 跑测试确认失败或编译失败**

Run: `cd src-tauri && cargo test providers:: -- --nocapture`  
Expected: 编译失败或断言失败

- [ ] **Step 3: 实现**

1. `types.rs` 增加 `QueryParam` 与字段。  
2. `gateway.rs`：`GatewayEntry` / `GatewayDirListing` 增加 `required_query`；`map_entry` / `list` 映射；`fetch_backends` 过滤 `search`（与 `local` 并列）。  
3. `discover_backends` / builtin fallback / `list_root` / preload：统一跳过 `search`。  
4. `list_dispatch`：若 `parse_awam_uri` 得 `backend == "search"`，直接返回：

```rust
DirListing {
    uri: uri_str.to_string(),
    backend: "search".into(),
    entries: vec![],
    error: Some("PROVIDER_SEARCH_BLOCKED".into()),
    required_query: None,
}
```

（前端将 code 映射到 i18n，勿依赖英文长句。）

- [ ] **Step 4: 跑测试**

Run: `cargo test providers:: -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit（AFM）**

```bash
git add src-tauri/src/providers/types.rs src-tauri/src/providers/gateway.rs \
  src-tauri/src/providers/mod.rs src-tauri/src/providers/registry.rs
git commit -m "feat(providers): 解析 requiredQuery 并屏蔽 search 后端"
```

---

### Task 4: AFM — `path.ts` / `navHistory` 正确处理 query

**Files:**
- Modify: `src/state/provider/path.ts`
- Modify: `src/state/navHistory.ts`（保持与 path 同步的内联逻辑，或改为 import path 辅助函数——优先抽公共纯函数避免双份漂移）
- Modify: `scripts/provider-path-checks.ts`

**Interfaces:**
- Produces:

```ts
export type ProviderQueryParam = {
  key: string;
  label?: string | null;
  required?: boolean;
};

export function providerBackendId(path: string): string | null;
export function isBlockedSearchProvider(path: string): boolean; // backend === "search"
export function getProviderQuery(path: string): Record<string, string>;
export function withProviderQuery(
  path: string,
  patch: Record<string, string | null | undefined>,
): string;
/** null 值或空字符串表示删除该键 */
```

`toProviderUri` 必须：
- 拆出 `?query` 再规范化 backend/path
- 重建时保留 query（键排序稳定可选）
- `awam://prowlarr?q=ubuntu` → backend=`prowlarr`, path 空, query=`{q:ubuntu}` → `awam://prowlarr?q=ubuntu`（或 `awam://prowlarr/?q=ubuntu`，与网关一致即可；推荐无 path 时 `awam://prowlarr?q=...`）

`providerParentPath("awam://prowlarr/search?q=a")` → `awam://prowlarr`（父级去掉 query）

- [ ] **Step 1: 扩展 `provider-path-checks.ts` 失败断言**

```ts
assert.equal(toProviderUri("awam://prowlarr/search?q=ubuntu"), "awam://prowlarr/search?q=ubuntu");
assert.equal(providerBackendId("awam://Search/everything?q=a"), "search");
assert.equal(isBlockedSearchProvider("awam://search/everything?q=a"), true);
assert.equal(isBlockedSearchProvider("awam://prowlarr/search?q=a"), false);
assert.equal(providerParentPath("awam://hf/remote?q=bert"), "awam://hf");
assert.equal(
  withProviderQuery("awam://hf/remote", { q: "bert" }),
  "awam://hf/remote?q=bert",
);
assert.equal(
  withProviderQuery("awam://hf/remote?q=bert", { q: null }),
  "awam://hf/remote",
);
```

- [ ] **Step 2: 跑检查确认失败**

Run: `node scripts/check-provider-path.mjs`  
Expected: FAIL

- [ ] **Step 3: 实现 path 辅助并同步 navHistory**

- [ ] **Step 4: 跑检查**

Run: `node scripts/check-provider-path.mjs`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/state/provider/path.ts src/state/navHistory.ts scripts/provider-path-checks.ts
git commit -m "fix(providers): awam URI 规范化保留并支持合并 query"
```

---

### Task 5: AFM — listing / browse / navigate 消费 `requiredQuery` 与屏蔽

**Files:**
- Modify: `src/state/provider/listing.ts`
- Modify: `src/state/provider/browse.ts`
- Modify: `src/state/providerCache.ts`
- Modify: `src/state/navigatorBrowse.ts`
- Modify: `src/i18n/locales/zh-CN.json`, `en.json`（及其它维护中的 locale 同步键）

**Interfaces:**
- `ProviderDirListingWire.requiredQuery?: ProviderQueryParam[] | null`
- `ProviderNavigatorListing.requiredQuery?: ProviderQueryParam[] | null`
- `ProviderEntryWire.requiredQuery?: ProviderQueryParam[] | null`
- `loadProviderListing`：仅当 `error` 非空且**不是**待查询软态时抛错。规则：`error == null` → 成功；若将来仍收到旧网关 Missing query，可在有 `requiredQuery` 时吞掉 error 当软态（可选兼容，非必须）。
- `navigateTo`：若 `isBlockedSearchProvider(trimmed)` → 设 `listError` 为 i18n（`providers.searchBlocked`），不调用 list。

文案示例：
- zh-CN: `"searchBlocked": "Everything 已接入「搜索」，请使用搜索页，而不是提供程序中的 search 后端。"`
- en: `"searchBlocked": "Everything is available via Search. The search provider backend is disabled here."`

- [ ] **Step 1: 在 `provider-path-checks.ts` 增加 listing 映射断言**

```ts
const soft = providerListingFromDir({
  uri: "awam://hf/remote",
  backend: "hf",
  entries: [],
  requiredQuery: [{ key: "q", label: "Query", required: true }],
});
assert.deepEqual(soft.requiredQuery?.[0]?.key, "q");
assert.equal(soft.entries.length, 0);
```

- [ ] **Step 2: 实现类型透传 + browse 软态 + navigate 拦截**

在 `navigatorBrowse.ts` provider 分支开头：

```ts
if (isBlockedSearchProvider(trimmed)) {
  listLoading.value = false;
  listError.value = /* 由调用方注入 t() 或使用固定 key 由 View 翻译 */
  listing.value = null;
  return;
}
```

注意：`navigatorBrowse.ts` 目前无 vue-i18n；可选：
- 抛出/设置稳定 code `PROVIDER_SEARCH_BLOCKED`，由 NavigatorView / AppShell 映射；或
- 设置英文 code，View 层 `watch` 映射。

推荐：`listError.value = "PROVIDER_SEARCH_BLOCKED"`，展示层已有/新增 mapError。

- [ ] **Step 3: 跑 `node scripts/check-provider-path.mjs` 与相关 provider 脚本**

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/state/provider/listing.ts src/state/provider/browse.ts \
  src/state/providerCache.ts src/state/navigatorBrowse.ts \
  src/i18n/locales/zh-CN.json src/i18n/locales/en.json
git commit -m "feat(providers): 消费 requiredQuery 并拦截 search 导航"
```

---

### Task 6: AFM — AppShell 地址栏旁查询条

**Files:**
- Modify: `src/components/shell/AppShell.vue`
- Modify: `src/i18n/locales/zh-CN.json`, `en.json`（`chrome.providerQuery*`）

**Interfaces:**
- Consumes: `currentPath`、`listing`（需从 `navigatorBrowse` 暴露 `requiredQuery`；若 listing 是 Navigator 形状，已在 Task 5 挂上）
- 显示条件：`isProviderPath(currentPath) && (listing?.requiredQuery?.length ?? 0) > 0`
- 字段：`v-for="param in requiredQuery"`，`v-model` 本地 draft map
- Enter / 提交按钮：`navigateTo(withProviderQuery(currentPath, drafts))`
- 清空必填字段并提交：对应键传 `null`，导航到无参 URI

UI 位置：地址栏 `address-go` 与现有 `navigator-filter-inline` 之间（或紧挨 Go），class 遵循 fluent-control；**不要**做成卡片堆叠。

```vue
<form
  v-if="providerQueryParams.length"
  class="provider-query-bar"
  @submit.prevent="submitProviderQuery"
>
  <input
    v-for="param in providerQueryParams"
    :key="param.key"
    v-model="providerQueryDrafts[param.key]"
    class="fluent-control provider-query-input"
    type="search"
    :placeholder="param.label || param.key"
    :aria-label="param.label || param.key"
  />
  <button type="submit" class="fluent-control">{{ t('chrome.providerQueryApply') }}</button>
</form>
```

路径变化时：用 `getProviderQuery(currentPath)` 重置 draft。

- [ ] **Step 1: 实现 UI 与提交逻辑**

- [ ] **Step 2: 手工验收清单**

1. Providers / Guide：**无** Everything/search 卡片。  
2. 地址栏输入 `awam://search/everything?q=test` → 拒绝并提示。  
3. 打开 `awam://prowlarr/search` → 出现查询框、空列表、无错误横幅；输入 ubuntu → Enter → 有结果；后退回到待查询。  
4. `awam://hf/remote` 同样。  
5. 清空查询再提交 → 待查询、无报错。

- [ ] **Step 3: Commit**

```bash
git add src/components/shell/AppShell.vue src/i18n/locales/zh-CN.json src/i18n/locales/en.json
git commit -m "feat(ui): 地址栏旁为 Provider 查询路径提供参数输入"
```

---

### Task 7: 契约脚本与收尾

**Files:**
- Modify: `scripts/check-provider-path.mjs`（如需断言 searchBlocked 文案键存在）
- Modify: `scripts/check-provider-cache.mjs`（若涉及 backends 假定含 search，改为不含）

- [ ] **Step 1: 跑**

```bash
node scripts/check-provider-path.mjs
node scripts/check-provider-cache.mjs
node scripts/check-provider-nonblocking.mjs
cd src-tauri && cargo test providers::
```

- [ ] **Step 2: 修到全绿**

- [ ] **Step 3: 最终 commit（若有脚本-only 改动）**

```bash
git commit -m "test(providers): 覆盖 query URI 与 search 屏蔽契约"
```

---

## Spec coverage checklist

| Spec 要求 | Task |
|---|---|
| awam-file `QueryParam` / entry+listing `requiredQuery` | 1–2 |
| 缺参 soft empty（error=null） | 2 |
| everything/prowlarr/hf 适配 | 2 |
| uri-spec 文档 | 2 |
| awam-file 先 commit | 1–2 |
| AFM 过滤 search（backends/root/guide/preload） | 3 |
| 手动 URI 拒绝 search | 3、5 |
| 解析 requiredQuery | 3、5 |
| 地址栏旁多字段查询、Enter 导航 | 6 |
| 清空待查询无报错 | 5–6 |
| path query 规范化 | 4 |

## 执行说明

1. 先完成 Task 1–2 并提交 awam-file，重启网关。  
2. 再执行 Task 3–7（AFM）。  
3. 不要在未提交 awam-file 的情况下开始依赖新字段的联调。
