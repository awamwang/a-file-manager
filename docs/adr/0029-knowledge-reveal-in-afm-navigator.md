# 知识页「在 AFM 中显示」：留在应用内，末尾新标签定位

ADR-0017 的知识弹窗右上角「在资源管理器中显示」走的是 `revealItemInDir`——拉起的是**系统**文件管理器（Windows 资源管理器 / macOS 访达），点一下就把人带离 AFM。而笔记所在目录本来就是本机路径（知识服务返回的索引根 + 相对路径拼出的绝对路径），AFM 自己完全能打开。

决定把这个按钮改成 **AFM 内定位**：

1. **留在应用内**：点按钮 → 关闭知识页 → 切资源管理器路由 → 在**当前打开组（标签条）的最后**新建一个标签打开笔记所在目录 → 在列表里选中该 .md 并滚动到它。系统文件管理器不再由这个按钮拉起（要开系统文件管理器：落进 AFM 后对文件右键「在资源管理器中显示」，那是另一条既有通路）。
2. **落位口径是「末尾新标签」**：即使同一目录已经在别处打开，也不复用那个标签、也不跳回标签条中间——新标签永远追加到当前打开组的最后并被激活。这与侧栏 / 发现 / 收藏等入口的 open-or-focus 口径**刻意不同**，由 `revealItemsInNavigator({ appendTab: true })` 显式表达；`revealInNavigator` 的默认分支仍是 open-or-focus，不影响既有调用方。
3. **文案随行为改**：按钮文案改为「在 AFM 中显示 / Show in AFM」——它指向的是 AFM 自己的资源管理器，不能再写系统文件管理器专名（Windows「资源管理器」/ macOS「访达」）造成歧义。`knowledgeView.openInExplorer` 这个 key 退役（6 语言同步），其 macOS 覆盖（Finder / 访达）一并删除；`check-platform-terms.mjs` 里「知识页必须写 Finder」的断言随之撤掉——该 key 已不属于系统文件管理器文案集合（该门禁的注释本就把「AFM 自己的 Navigator / Explorer」排除在覆盖名单外）。
4. **边界不变**：AFM 仍不感知知识索引根的具体路径，定位用的仍是服务端返回 `root` 拼出的绝对路径；只读定位，不写不删不改。

## Consequences

知识页是全局 overlay（z-index 1050），所以按钮必须**先** `closeKnowledgeDoc()` 再跳资源管理器路由，否则弹窗会盖住刚落位的目录。搜索面板知识源命中的 Ctrl+Enter 仍走 `revealItemInDir`（系统文件管理器）——本 ADR 只覆盖知识页按钮，两条通路口径暂时不一致，后续可单独统一。门禁：`scripts/check-knowledge-reveal-in-navigator.mjs`（新增，接入 `npm run test:knowledge-reveal`），钉住「不再 `revealItemInDir`」「appendTab 落位」「6 语言文案与旧 key 退役」。
