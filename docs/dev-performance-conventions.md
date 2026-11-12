<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 开发约定：性能与非阻塞 UI

面向新增/改动特性、导航列举、图标缩略图、网络与虚拟路径，以及任何可能触达磁盘、Shell、COM、网络或外部进程的路径。与 Project 约定一致；度量细节见 [perf-baseline.md](./perf-baseline.md)、[PERF.md](../PERF.md)。壳图标/内容缩略图最优加载策略见 [shell-icon-thumbnail-loading.md](./shell-icon-thumbnail-loading.md)。

1. **特性先做性能影响分析。** 开工或评审前写清：可能的慢路径、是否触 UI/主线程、是否可取消、失败与导航离开时如何收敛。可短，不能不做。
2. **耗时或耗性能操作进入性能基线。** 重操作须有可重复度量（现有 `test:*` / 脚本基线 / 审计文档其一），合入后不得无说明地回退基线。
3. **阻塞 UI 不允许。** 同步 FS/COM/网络不得冻住 WebView 或主线程；慢工作异步或 `spawn_blocking`，导航离开须可取消或作废。轻量加载提示 OK；模态锁死界面不行。卡顿即缺陷。

**边界：** 纯文案/样式改动不强制长文分析；一旦触及 I/O、IPC、列举、图标/缩略图、启动或特殊路径，本约定即生效。
