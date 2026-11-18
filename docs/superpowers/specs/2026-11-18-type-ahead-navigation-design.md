# Navigator 列表「键入定位」（Type-Ahead）设计

日期：2026-11-18
状态：待评审（本期只出设计，不实现）
仓库：`a-file-manager`（前端，Tauri 2 + Vue 3）

## 背景

Windows 资源管理器在列表视图中「键入字符 → 选中首字符匹配项」是高频肌肉记忆操作
（文件夹选项里的 **"在视图中选择键入的项目"** 模式）。AFM 目前只有方向键选择
（`navigatorKeyboardNav.ts` + `onListKeyDown`），敲字母无任何反应；Ctrl+E / F3 是
**递归文件夹搜索**（过滤列表），语义不同，不能替代"在当前可见列表里定位"。

用户诉求：让主栏列表响应字母，像 Windows 一样快捷定位（输入 `A` 跳到第一个 `A`
打头的文件）。**要求先调查清楚实现机制，且必须考虑多语言。**

## 目标

1. 主栏列表获得焦点时，输入**拉丁字母 / 数字** → 选中当前可见顺序中第一个"显示名
   以该字符（串）开头"的条目，滚动到可视区并聚焦该行。
2. 行为对齐 Windows：大小写/重音不敏感；连续快敲组成前缀；重复单字母在候选中循环；
   约 1.2s 无输入重置；Esc / 切目录 / 改搜索 / 改排序 / 失焦时清空缓冲。
3. 匹配与**现有排序规则使用同一套 locale collator**，保证"跳转顺序 = 视觉顺序"。
4. 输入法（IME）组字期间**不误触发**跳转（多语言的底线要求）。
5. 纯逻辑抽成可 Node 直测的纯模块 + 静态守卫，纳入 `npm test`。

## 非目标（本期不做）

- **不做 CJK 提交串定位**：中文拼音 / 日文假名输入法运营状态下上屏的汉字，本期不用于
  定位（见「二期：IME sink」）。本期仅保证 IME 组字时不乱跳。
- **不覆盖副栏**：双栏布局下副栏（secondary）本期不接 type-ahead，与现有方向键仅主栏
  处理保持一致。
- 不新增 UI 文案 / 设置开关：纯键盘交互，缓冲不上屏（避免 5 语言 parity 负担）。
- 不改变 Ctrl+E / F3 文件夹搜索、Search Palette、全局快捷键的行为。

## 决策摘要

| 项 | 选择 |
|---|---|
| 本期字符集 | 拉丁字母 + 数字（Unicode 码点级 `Array.from`，非 grapheme） |
| 覆盖栏位 | 仅主栏（`listRoot` / `visibleEntries` / `orderedPaths`） |
| 匹配语义 | 显示名**前缀**匹配，base-sensitivity collator（大小写/重音不敏感） |
| collator | `new Intl.Collator(undefined, { sensitivity: "base" })`，与 `navigatorSort.ts` 一致 |
| 缓冲重置 | 1.2s 超时 + Esc / 目录 / 搜索 / 排序 / 失焦事件 |
| 循环 | 缓冲为单个字符且再次敲同一字符 → 在同名前缀候选间循环（含 wrap） |
| IME | 拦截 `isComposing` / `keyCode === 229` / `key === "Process"`，组字期不触发 |
| 实现形态 | 新纯模块 `src/state/navigatorTypeAhead.ts` + 视图接线 + 守卫脚本 |
| 下一步 | 本设计评审通过后进入实现；实现后本地 commit（不 push） |

## 行为调研（对齐标准）

### Windows 的真实规则

- 文件夹选项 **"当在列表视图中键入时" → "在视图中选择键入的项目"**（另一个选项
  "自动键入到搜索框"是不同行为，AFM 取前者）。
- 敲 `p` → 选中/滚动到第一个 `p` 打头项；紧接着敲 `r` → 组成 `pr`，跳到 `pr` 打头项；
  间隔过长（约 1s 量级）则视为新的一次输入。
- 连续敲同一字母（缓存只有一个字符）→ 在"首字母相同"的多个条目间逐个循环。
- 匹配是**语言敏感**的：德/法语里敲 `e` 能命中 `é`/`è`；排序与定位共用同一 collation。
- CJK：Windows 用 IME 组字，上屏后按"提交的字符"匹配（本期不实现，见二期）。

### 多语言约束（本期边界）

| 场景 | 本期行为 | 依据 |
|---|---|---|
| 英文大小写 | 命中（`A` ≡ `a`） | collator `sensitivity: "base"` |
| 德/法重音 | 命中（`e` ≡ `é` ≡ `è`，匹配 `Äpfel` 用 `a`） | 同上；与 `navigatorSort.ts:63` 一致 |
| 数字 | 命中（按显示名前缀） | 数字键无裸快捷键绑定 |
| 中文/日文文件名 | 敲拉丁键不会误定位到它们（首字非拉丁）；IME 组字期完全不触发 | IME 拦截 + 前缀语义 |
| IME 组字期 | **一律忽略**按键，列表不动 | `isComposing` / `229` / `Process` 拦截 |
| emoji / 组合字符开头的文件名 | 拉丁/数字前缀匹配天然不会误命中 | 取码点前缀比较 |

## 现状接入点（代码事实）

| 关注点 | 位置 | 说明 |
|---|---|---|
| 键盘入口 | `NavigatorView.vue:1509 onListKeyDown` | 已挡 alt/meta/ctrl 与 `isEditableTarget`；方向键在此处理 |
| 可见顺序 | `visibleEntries` (`:640`) / `orderedPaths` (`:680`) | 已排序（搜索激活时为搜索结果）；type-ahead 直接复用 |
| 排序规则 | `navigatorSort.ts:38-65 compareEntries` | `localeCompare(undefined, { sensitivity: "base" })`，文件夹优先 |
| 选中写入 | `selectedPaths` / `selectionAnchor`（`navigatorSelection.ts`） | `selectOnly(path)` 单选用它 |
| 行聚焦 | `focusEntryRow` (`:1489`) | `querySelector([data-entry-path])` + `scrollIntoView({block:"nearest"})` |
| 行 DOM | `data-entry-path` 出现在 `:2364`(主) / `:2536`(副) | 主栏行可被 `listRoot` 内查询命中 |
| 虚拟列表 | `listVirtualWindow.ts` | > 80 条时只挂载视口附近行；`rowStridePx` 目前是私有函数 |
| 目录/搜索监听 | `watch([navigatorFilterQuery, currentPath])` (`:665`) | 可在此清空缓冲 |
| 裸键快捷键 | `registry.ts`：`\``(命令面板)、`space`(预览)、F2/F3/F4/F5/F6、Delete、浏览器前进/后退 | 数字 1-9 仅 `Ctrl+` 绑定（`:291`），裸数字安全；空格必须排除 |
| 焦点循环 | `focusCycle.ts` / 列表 `data-focus-default` (`:2246`) | F6 进入内容区默认聚焦列表；二期 IME sink 会冲击此处 |

## 架构

```
键盘事件（listRoot @keydown）
   │
   ├─ 方向键 / 功能键 / 修饰键 / isEditableTarget ──► 现有逻辑（不变）
   │
   ├─ IME 组字（isComposing / 229 / Process）──────► return（本期不触发）
   │
   └─ 单个可打印字符（字母/数字，排除 space 与 `）
          │
          ▼
   纯模块 navigatorTypeAhead.ts
     feedChar()  → 缓冲状态（buffer / cycle / lastAt）
          │
          ▼
     findMatch(names, buffer, fromIndex, collator, cycle) → 命中索引 i（或 -1）
          │
          ▼
   NavigatorView 接线
     selectOnly(orderedPaths[i]) → 计算 scrollTop(i) → 设 listRoot.scrollTop
          → nextTick + rAF 重试 focusEntryRow(path)
```

纯模块零 Vue、零 IPC、零 DOM；collator 由视图注入，便于测试与复用排序配置。

## 纯模块契约：`src/state/navigatorTypeAhead.ts`

```ts
import type { NavigatorEntry } from "./navigatorBrowse.ts";

export const TYPE_AHEAD_TIMEOUT_MS = 1200;

export type TypeAheadState = {
  buffer: string;          // 归一化（小写）后的累积前缀，如 "app"
  cycle: boolean;          // 本次是否"循环步进"（缓冲单字符重复敲）
  lastAt: number;          // 上次输入时间戳（performance.now() 或 Date.now()）
  lastMatchedIndex: number;// 上次命中索引，循环时从它之后继续
};

export function initialTypeAheadState(): TypeAheadState;
export function resetTypeAhead(): TypeAheadState;

/** 输入一个码点字符，返回新缓冲状态。超时重置；单字符重复→cycle。 */
export function feedChar(
  state: TypeAheadState,
  ch: string,
  now: number,
): TypeAheadState;

/** 在有序显示名里找前缀命中索引；-1 表示无匹配。 */
export function findTypeAheadMatch(args: {
  names: readonly string[];      // 与 visibleEntries 同序
  buffer: string;
  fromIndex: number;             // 当前选中索引（cycle 时从其后开始）
  collator: Intl.Collator;
  cycle: boolean;
}): number;

/** 前缀判定：取 name 前 buffer 码点数个字符，用 collator 做 base 比较。 */
export function nameStartsWith(
  name: string,
  buffer: string,
  collator: Intl.Collator,
): boolean;
```

### 关键算法

- **归一化**：输入字符 `ch.toLocaleLowerCase()` 后累积；比较交给注入的 base collator，
  天然覆盖大小写与重音，无需手写去音符号表。
- **前缀匹配**：`nameStartsWith` 取 `Array.from(name).slice(0, Array.from(buffer).length)`
  再 `collator.compare(head, buffer) === 0`。码点级切分避免代理对被切坏（emoji 开头
  文件名不会误命中）；grapheme 级（`Intl.Segmenter`）留给二期 CJK。
- **超时与循环**：`now - lastAt > 1200` → 重置为 `buffer = ch, cycle = false`；
  否则 `buffer === ch`（单字符且相同）→ `buffer` 不变、`cycle = true`；
  其余 → `buffer += ch, cycle = false`。
- **查找**：非循环从 `0` 找第一个前缀命中（符合"跳到第一个 A 打头"的直觉）；
  循环从 `lastMatchedIndex + 1` 起环形扫描（含 wrap）。未命中返回 `-1`，调用方
  保持原选中不动。
- **排序一致性守卫**：纯模块测试里断言 `nameStartsWith("Äpfel","a",collator)` 与
  `navigatorSort` 使用同一 collator 配置（`sensitivity: "base"`）。

## 视图接线（`NavigatorView.vue`）

在 `onListKeyDown` 现有早退之后、方向键分支之前插入：

1. `if (event.isComposing || event.keyCode === 229 || event.key === "Process" || event.key === "Unidentified") return;`
   —— IME 组字期一律放过（含 WebView2 对无编辑上下文元素送 `229` 的情况）。
2. 取 `event.key`；仅当**单个码点**且是字母/数字（`/^\p{L}|\p{N}$/u` 的简化判定：
   `Array.from(key).length === 1 && /[\p{L}\p{N}]/u.test(key)`）才进入；排除空格与 `` ` ``
   （有裸键绑定），排除已被方向键处理的键。允许 Shift（Shift+字母等同字母）。
3. `const now = performance.now()`；`typeAhead.value = feedChar(typeAhead.value, key, now)`。
4. 取 `visibleEntries.value` 的显示名数组（`e.name`）与 `orderedPaths.value`；
   `i = findTypeAheadMatch({ names, buffer: typeAhead.value.buffer, fromIndex: primarySelectionIndex(...), collator: TYPE_AHEAD_COLLATOR, cycle: typeAhead.value.cycle })`。
   - `TYPE_AHEAD_COLLATOR` 为模块级 `new Intl.Collator(undefined, { sensitivity: "base" })`。
5. `i >= 0` → `selectOnly(orderedPaths[i])`；更新 `typeAhead.value.lastMatchedIndex = i`；
   `event.preventDefault(); event.stopPropagation()`；调用 `revealTypeAheadRow(i, path)`。
6. `i < 0` → 不消费事件（保持原行为），但缓冲已更新，下次敲键继续。

**清空缓冲的时机**：现有 `watch([navigatorFilterQuery, currentPath])`（`:665`）里重置；
另在排序切换（`toggleSort` 调用处）、列表 `blur`/活动 pane 切换、Esc 键分支重置。
Esc 需先确认无打开的弹窗/改名会话占用（列表键盘环境下通常安全）。

## 虚拟列表下的滚动与聚焦（主要难点）

直接 `focusEntryRow(path)` 在 > 80 条时可能失败：目标行不在挂载窗口内，`querySelector`
返回 `null`。处理顺序：

1. 由命中索引 `i` 计算目标 `scrollTop`：
   `row = Math.floor(i / columns)`（`columns` 用方向键同款 `paneColumns(listViewport.w, primaryGridGeometry)`，
   非图标视图为 1），`stride = rowStridePx(rowHeight, columns, mode)`，
   `target = row * stride`。
   - 把 `rowStridePx`（`listVirtualWindow.ts:66`，当前私有）导出，或新增纯函数
     `scrollTopForIndex({index, columns, mode, rowHeight})`，供 type-ahead 与未来
     "按索引滚动"共用，并纳入单测。
2. `listRoot.value.scrollTop = clamp(target, 0, maxScroll)` —— 触发 `@scroll` →
   `primaryScrollTop` 更新 → `primaryVirtual` 重算 → 目标行挂载。
3. `nextTick` 后**有界 rAF 重试** `focusEntryRow(path)`（最多 ~10 次，参考
   `revealInNavigator.ts:34 scrollSelectionIntoView`），命中即 `row.focus({preventScroll:true})`
   + `scrollIntoView({block:"nearest"})` 兜底微调。
4. 失败也不破坏选中态：`selectedPaths` 已写入，用户继续方向键仍从该项出发。

## 键冲突核对（已查证）

| 键 | 裸键绑定 | 本期处理 |
|---|---|---|
| `a`-`z` | 无（`Ctrl+` 组合不算） | 用作 type-ahead |
| `0`-`9` | 无（仅 `Ctrl+1..9` 切标签，见 `registry.ts:291`） | 用作 type-ahead |
| 空格 | `navigator.previewBridge`（`registry.ts:211`） | **排除**，保留 Space 预览 |
| `` ` `` | `commandPanel.toggleBacktick`（`:35`） | **排除** |
| F2/F3/F4/F5/F6 / Delete / 浏览器前进后退 | 有裸键 | 现有逻辑处理，不进入 type-ahead |
| 方向键 | `onListKeyDown` 已有分支 | 先于 type-ahead 处理 |

## 二期：IME sink（不在本期，仅留路径）

中文/日文要像 Windows 那样"组字上屏后按提交串定位"，需要可编辑上下文接收
`compositionend`。方案：在列表内放一个透明/零尺寸的 `input` 作为 IME sink，键盘焦点
落在它上面，`compositionend` 取 `event.data` 作为 buffer 走同一 `findTypeAheadMatch`
（需把 grapheme 级切分升级为 `Intl.Segmenter`，并处理假名/汉字归一化）。

**为什么放二期**：sink 会同时冲击 F6 焦点循环（`data-focus-default`）、行聚焦
（`focusEntryRow`）、方向键/Enter/Space/Delete 的全部 keydown 转发，是一次焦点模型
重构，风险与回归面远大于本期收益。本期先把纯匹配内核与守卫做扎实，二期只换"输入源"。

## 测试要点

**纯模块（`scripts/type-ahead-checks.ts` + `check-type-ahead.mjs`，并入 `npm test`）**

- `feedChar`：超时重置；单字符重复 → `cycle`；不同字符扩展；大小写归一化。
- `findMatch`：大小写/重音命中（`a` ↔ `Äpfel`、`e` ↔ `Été`）；数字；前缀（`app` → `apple`）；
  无匹配返回 `-1`；`cycle` 从 `fromIndex+1` 环形扫描并 wrap；空 buffer 返回 `-1`。
- 与排序 collator 一致性：同一组名字的 type-ahead 命中顺序等于 `navigatorSort` 顺序。
- `scrollTopForIndex` / 导出后的 `rowStridePx`：多列/单列、details/icons 的 stride 与索引换算。

**视图接线守卫（源码断言）**

- `onListKeyDown` 含 `feedChar` / `findTypeAheadMatch` / `isComposing` / `229` 拦截。
- 目录与搜索 `watch` 中调用 `resetTypeAhead`。
- 排除空格与 `` ` `` 的判定存在；仅操作主栏（`listRoot` / `visibleEntries`）。

**手工验收（实现阶段）**

- 大目录（> 80 条）输入字母能滚动到远处命中行并聚焦。
- 法语/德语文件名重音命中；重复字母循环；1.2s 后重新起串。
- 中文输入法组字期间列表不跳动；切目录/搜索后缓冲已清。

## i18n / parity

本期无新增用户可见字符串，不改 5 个 locale，不改 `src/i18n/locales/*.json`。
若二期加"当前前缀"状态栏提示，需 5 语言补齐并通过 `npm run test:i18n`。

## 提交策略

1. 本设计文档：单独 commit（`docs: add type-ahead navigation design`）。
2. 实现阶段按用户指示另起 commit：纯模块 + 守卫 + 视图接线；不 push。

## 开放风险

- Windows 对"重复字母 vs 真的想输 `aa` 前缀"的边界处理与本文规则可能略有差异；实现时
  以手工验证为准，必要时把循环判定收紧为"仅缓冲长度为 1"。
- `keyCode` 已废弃但 WebView2 下仍是 IME 组字的有效信号；同时保留 `isComposing` 与
  `key === "Process"` 三重判定兜底。
- 虚拟列表滚动依赖 `@scroll` 回写 `primaryScrollTop`；若该回写被节流/跳帧，rAF 重试次数
  需实测调整。
- 搜索激活时（`folderSearchActive`）`visibleEntries` 来自递归搜索结果，type-ahead 仍按
  其当前顺序定位；结果未返回（loading）时 `names` 为空，命中 `-1`，行为安全。
