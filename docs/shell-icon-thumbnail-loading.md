<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 壳图标与内容缩略图加载方案

面向 Navigator 图标/平铺等视图：条目数、行高、视口尺寸均可变。目标不是「尽快下完所有缩略图」，而是**当前可见区先稳住、可感知地变清晰，滚动时不被慢路径拖死**。

相关实现：`src/state/shellIcons.ts`、`shellIconThumbPolicy.ts`、`listVirtualWindow.ts`、`views/NavigatorView.vue`。约定见 [dev-performance-conventions.md](./dev-performance-conventions.md)；度量见 [perf-baseline.md](./perf-baseline.md)（S4）。

## 1. 体验目标

| 场景 | 期望 |
|------|------|
| 进入目录 | 第一屏目录与公共壳图标几乎立刻稳定 |
| 停在第一屏 | 图像/视频等内容缩略图按上→下补齐，补完后不再乱跳 |
| 快速甩动 | 只保证壳图标/占位；不排队高清内容缩略图 |
| 减速停稳 | 再对当前视口补内容缩略图 |
| 离开视口 | 取消排队与未完成 attempt，缓存可 soft 保留供回滚 |

## 2. 最优策略（结论）

行业共识（Dropbox Carousel、Glide 预取、web.dev 首屏 eager、Uppy/虚拟列表 cancel）收敛为六条：

1. **视口优先**：只对「可见 + 小 overscan」发请求；离开即 cancel。
2. **首屏急切**：真实高度量到前不发大窗内容缩略图；首屏不要当成「懒加载」拖到 layout 之后。
3. **分层优先级**：快共享（壳/类型图标）> 当前屏内容缩略图 > 滚动方向小预取 > 屏外可丢弃。
4. **速度门控**：甩动时挂起内容缩略图排队；停稳后再补（两档质量思想；本应用无小图档时用「占位 + 停稳再上缩略图」，**不做**壳图标→缩略图两阶段跳变）。
5. **稳定顺序**：优先窗整块上→下入队，同步不得 LIFO 打乱（避免扫描推进感）。
6. **有界缓存**：path/size LRU + soft 淘汰；回滚命中秒显。

## 3. 架构

```
NavigatorView
  scroll / resize / listing watch
    → 滚动速度采样（甩动则挂起内容缩略图）
    → computeThumbRequestWindow（几何窗）
    → cancel 离开窗的路径
    → requestIconsFor（按策略打 preferThumbnail）
        → shellIcons.promotePriorityWindow / takeNextBatch
            → shell_icon_get_many（Rust：串行 COM；目录/非 thumb → type 缓存）
```

| 层 | 职责 |
|----|------|
| 几何请求窗 | `computeThumbRequestWindow`：可见行 + `THUMB_REQUEST_OVERSCAN_ROWS`；`h≤0` 时仅 `THUMB_REQUEST_FALLBACK_ROWS` 兜底 |
| 类型策略 | `entryWantsContentThumbnail`：仅图像/视频等走内容缩略图 |
| 队列 | 单队列；`preferThumbnail=false` 优先抽干（`BATCH_ICON`）；thumb 小批（`BATCH_THUMB`）便于 cancel |
| 后端 | `prefer_thumbnail && is_file` 才走 `IShellItemImageFactory`；否则壳图标 + `type:<ext>` 共享缓存 |

## 4. 优先级细则

### 4.1 快通道（`preferThumbnail: false`）

- 目录（`kind === "dir"`）
- 非媒体扩展名（文档、压缩包、无扩展名等）——通常只有**按扩展名共享的公共壳图标**
- 列表/详情等非图标视图（小尺寸壳图标）

### 4.2 慢通道（`preferThumbnail: true`）

- 图标视图下、策略判定为图像/视频等、且**允许内容缩略图**时（见门控）

### 4.3 门控

| 条件 | 行为 |
|------|------|
| `viewportHeight ≤ 0` | 只请求快通道；不排队内容缩略图 |
| 滚动速度 ≥ 阈值（甩动） | 挂起：从队列摘掉未开始的 content-thumb 项；本轮只请求快通道 |
| 滚动停稳（settle 定时） | 恢复：对当前窗完整请求（含内容缩略图） |
| 路径离开请求窗 | `cancelShellIconRequests`（FIL-33） |
| 换目录 / 换图标尺寸 | `clearShellIconQueue` |

### 4.4 入队顺序

- `priority` 时用 `promotePriorityWindow`：**整窗按传入顺序置于队头**（上→下），禁止逐条 `unshift` 造成 LIFO。
- 图标视图请求列表内：快通道稳定排在内容缩略图之前。
- 主栏条目在前、副栏在后，合并一次 promote，避免后同步栏挤掉第一屏。
- 已在队列中的项不因重复同步反复挪位（避免「扫描推进」）。

## 5. 不确定量的处理

| 不确定 | 策略 |
|--------|------|
| 条目数 | 永不按全量 listing 请求；只跟几何窗走 |
| 行高/列数 | 用当前视图估算行高与列数算窗；ResizeObserver 更新后重同步 |
| 窗口大小 | `h≤0` 小兜底行数 + 仅快通道；量到高度后再上内容缩略图 |
| 滚动 | 速度门控 + 停稳补齐；离开 cancel |

## 6. 明确不做

- 壳图标显示后再换成内容缩略图的两阶段绘制（避免跳变）。
- 用 IntersectionObserver 在图标格全量 DOM 上反复 disconnect/observe（FIL-50：会误 cancel 慢批）。
- 批内并行 Shell COM（破坏 SageThumbs 等 handler）。
- 内容缩略图与壳图标混用同一大 batch 且无 cancel 粒度。

## 7. 验收标准

1. 进入含文件夹 + 视频的目录：文件夹图标先于视频缩略图稳定出现。
2. 多屏目录：第一屏内容缩略图不晚于第二屏；无自下而上「扫描推进」感。
3. 快速甩动：队列不以视频缩略图堆积；停稳后当前屏再补齐。
4. 滚出视口的项不再长期占用排队；滚回可命中缓存或重新排队。
5. 契约脚本：`scripts/check-shell-icons.mjs`、`check-shell-icon-queue.mjs` 通过。

## 8. 参考

- [Dropbox Carousel：滚动速度与两档缩略图](https://dropbox.tech/mobile/building-carousel-part-iii-drawing-images-on-screen)
- [web.dev：首屏不要 lazy](https://web.dev/articles/browser-level-image-lazy-loading)
- [Glide RecyclerView 预取](https://bumptech.github.io/glide/int/recyclerview.html)
- Uppy：视口内生成缩略图，unmount 时 cancel
