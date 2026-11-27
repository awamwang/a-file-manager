# 资源管理器排序/分组：全字段下拉 + 可折叠分组

Windows 资源管理器视图栏有「排序依据 / 分组依据」下拉，覆盖全部可排序字段，分组还支持折叠与方向。此前 AFM 只有「按类型分组」布尔开关（`core.navigatorGroupByType`）和详细信息列头排序，与 explorer-baseline 不一致。

决定补齐为完整能力：

1. **Chrome 两个互斥下拉**（排序依据 / 分组依据）：字段为名称 / 修改日期 / 类型 / 大小（与详细信息可排序列一致），方向为递增 / 递减（arrowUp / arrowDown 图标）。**选字段保持当前方向**，方向是独立单选。详细信息列头 `toggleSort` 保留：同列翻转方向、换列重置升序。四项全部持久化（settings schema v17：`core.navigatorSortColumn` / `core.navigatorSortDirection` / `core.navigatorGroupBy` / `core.navigatorGroupDirection`）。
2. **分组泛化**：5 字段（无 + 4）。组头标签对齐 Win11——名称走 A–Z 拼音首字母桶（「#」桶永远垫底，不随方向翻转），修改日期走日期桶（更早/今天/…/未知，未知垫底），类型/大小走语义桶。~~文件夹永远单独成组置顶~~ **2026-11-27 修正（Win11 build 26200 自宿主 IExplorerBrowser 实测）**：文件夹与文件一样按自身属性入桶——名称/日期/类型正常参与（类型分组的文件夹桶标签即「文件文件夹」，按类型标签字母序排、随方向翻转；日期分组文件夹按 mtime 混入日期桶）；大小分组时文件夹无大小，落入「未指定」桶（`UNSPECIFIED_GROUP_KEY`，rank 最高且随方向翻转：升序垫底、降序置顶，与 Win11 的 Empty→…→未指定 / 未指定→…→Empty 一致；大小桶清单：空(0B)/超小(<16KB)/小(<1MB)/中(<128MB)/大(<1GB)/巨大(≥1GB)/未指定，其中「超小」段为 2026-11-28 对齐 Win11 Tiny(0-16KB) 补齐）。组头是可点击折叠的 button（aria-expanded + chevron），折叠态 `collapsedGroupKeys` 仅会话级记忆，不持久化。
3. **空白处右键子菜单**：排序/分组项复用 `ContextMenuItemDef.checked`（menuitemcheckbox），与 chrome 下拉同一状态源，选中即写同一批 settings 键。
4. **旧 `core.navigatorGroupByType` 退役方式**：后端定义与 v16→v17 迁移保留（老用户设置平滑搬家），FE Settings UI 隐藏该键（i18n 条目保留以满足 i18n-checks），交互只走新键。
5. **工具条顺序：分组紧跟排序**。两者是同一族操作（同一批 settings 键、同一套下拉形态），中间被「视图切换」隔开会让人以为分组属于视图而不是列表。顺序钉为 排序 → 分组 → 视图 → 穿透。
6. **分组 + 图标/平铺视图按网格排**（2026-11-27 修正）。此前分组时整个列表被强制成纵向单列（`list--virt-flat`），于是大图标视图下**每个图标一行一个** —— 既不像 Explorer 的「组头 + 组内图标网格」，也让分组失去意义（分组本该把同类项聚在一起看）。现在：组头横跨整行，组内条目按列流动，每个组从新行开始。

   代价是虚拟化口径变了：分组 + 网格视图按**行**（组头独占行、组内按列切行）而非按条目窗口化，且可见范围必须**对齐组边界**（组头与它所属的组同批渲染，否则网格里组内条目会顶到上一行）。列数由 JS 显式写进 CSS（`--group-cols`），不用 `auto-fill` —— 虚拟化要按「组内条目占几行」算块高，列数必须与浏览器实际排出的完全一致。方向键也改按**视觉行列**移动：组的条目数通常不是列数的整数倍，按「序号 ÷ 列数」算跨组就会错位。

   副窗格不分组（分组只作用于主窗格），因此副窗格此前会被全局分组开关牵连失去网格 —— 一并修正为始终平铺网格。

## Consequences

标签键经 `sortColumnLabelKey` / `groupFieldLabelKey` 复用 `chrome.sort*`；locale 新增 `chrome.sortBy` / `groupBy` / `groupNone` / `sortAscending` / `sortDescending` 与 `navigator.group*` 12 键（六语言同步），`chrome.viewGroupByType` 退役。中文首字母桶用 GB2312 边界表 + `Intl.Collator("zh")` 拼音序二分（`navigatorGrouping.ts` `pinyinInitial`）——依赖 full-ICU（Windows WebView2 与 Node 均满足），边界表字符/首字母表损坏会被真执行断言当场抓住。组头高度固定为 28px（`LIST_GROUP_HEADER_HEIGHT_PX`），让 CSS 与虚拟化的行高口径一致，否则滚动位置会漂。门禁：`scripts/check-navigator-sort-group.mjs`（静态针 + 工具条顺序断言 + 六语言 i18n + spawn 真执行 `navigator-sort-group-checks.ts`：分桶/垫底/折叠/默认方向/水合/`toggleSort` 语义），接入 `npm run test:navigator-sort-group`；`check-view-layout.mjs` 与 `list-virtual-window-checks.ts` 针同步更新（后者新增分组网格的行映射/行级窗口/视觉导航断言）；`check-list-virtual.mjs` 钉住「分组 + 图标必须走网格画布」与「副窗格不得被全局分组牵连」；settings v17 迁移有 4 个 cargo 测试；webdemo mock 同步新键。
