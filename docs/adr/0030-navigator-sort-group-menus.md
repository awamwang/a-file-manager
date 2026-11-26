# 资源管理器排序/分组：全字段下拉 + 可折叠分组

Windows 资源管理器视图栏有「排序依据 / 分组依据」下拉，覆盖全部可排序字段，分组还支持折叠与方向。此前 AFM 只有「按类型分组」布尔开关（`core.navigatorGroupByType`）和详细信息列头排序，与 explorer-baseline 不一致。

决定补齐为完整能力：

1. **Chrome 两个互斥下拉**（排序依据 / 分组依据）：字段为名称 / 修改日期 / 类型 / 大小（与详细信息可排序列一致），方向为递增 / 递减（arrowUp / arrowDown 图标）。**选字段保持当前方向**，方向是独立单选。详细信息列头 `toggleSort` 保留：同列翻转方向、换列重置升序。四项全部持久化（settings schema v17：`core.navigatorSortColumn` / `core.navigatorSortDirection` / `core.navigatorGroupBy` / `core.navigatorGroupDirection`）。
2. **分组泛化**：5 字段（无 + 4）。组头标签对齐 Win11——名称走 A–Z 拼音首字母桶（「#」桶永远垫底，不随方向翻转），修改日期走日期桶（更早/今天/…/未知，未知垫底），类型/大小走语义桶；**文件夹永远单独成组置顶**。组头是可点击折叠的 button（aria-expanded + chevron），折叠态 `collapsedGroupKeys` 仅会话级记忆，不持久化。
3. **空白处右键子菜单**：排序/分组项复用 `ContextMenuItemDef.checked`（menuitemcheckbox），与 chrome 下拉同一状态源，选中即写同一批 settings 键。
4. **旧 `core.navigatorGroupByType` 退役方式**：后端定义与 v16→v17 迁移保留（老用户设置平滑搬家），FE Settings UI 隐藏该键（i18n 条目保留以满足 i18n-checks），交互只走新键。

## Consequences

标签键经 `sortColumnLabelKey` / `groupFieldLabelKey` 复用 `chrome.sort*`；locale 新增 `chrome.sortBy` / `groupBy` / `groupNone` / `sortAscending` / `sortDescending` 与 `navigator.group*` 12 键（六语言同步），`chrome.viewGroupByType` 退役。中文首字母桶用 GB2312 边界表 + `Intl.Collator("zh")` 拼音序二分（`navigatorGrouping.ts` `pinyinInitial`）——依赖 full-ICU（Windows WebView2 与 Node 均满足），边界表字符/首字母表损坏会被真执行断言当场抓住。门禁：`scripts/check-navigator-sort-group.mjs`（静态针 + 六语言 i18n + spawn 真执行 `navigator-sort-group-checks.ts`：分桶/垫底/折叠/默认方向/水合/`toggleSort` 语义），接入 `npm run test:navigator-sort-group`；`check-view-layout.mjs` 与 `list-virtual-window-checks.ts` 针同步更新；settings v17 迁移有 4 个 cargo 测试；webdemo mock 同步新键。
