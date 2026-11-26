# winget 分发渠道：Awam.AFileManager @ microsoft/winget-pkgs

AFM 此前只有官网直链下载。为了让 Windows 用户能用系统自带包管理器安装，决定接入 winget 社区源（microsoft/winget-pkgs），并分三层落地：

1. **渠道层**：`scripts/package-winget.mjs` 生成多文件清单（version / installer / locale.en-US / locale.zh-CN，ManifestVersion 1.28.0），CLI `npm run package:winget -- --version x.y.z` 下载主线版 NSIS 资产、算 SHA-256、落盘 `dist/winget/manifests/<首字母>/<Publisher>/<Name>/<版本>`、跑 `winget validate`、打印提交流程。PackageIdentifier 定为 `Awam.AFileManager`（Publisher=Awam 与 `src-tauri/tauri.conf.json` 的 `bundle.publisher` 对齐）；安装器只指向**主线版素名资产**（复用 `release-assets.mjs` 的 `nsisAssetName`，ADR 0024：无后缀=主线 ai flavor）。
2. **纪律**：只提交**正式 Release**（`assertPlainVersion` 拒绝预发布后缀）；发布当天跑脚本→人工确认→fork winget-pkgs 开分支提 PR，不引入 bot/secret；不回填历史版本。过审后才在官网亮出安装命令。
3. **展示层**：站点走 `installMethods` 命令卡（`winget install --id Awam.AFileManager -e`），不动 `downloadSources`（下载源专指文件直链）；命令卡在 awam-software 侧单独分支、winget-pkgs 过审后再合并部署。

## Consequences

`tauri.conf.json` 增加 `bundle.publisher: "Awam"`（NSIS 元数据补全，winget Publisher 的事实源）。多文件清单两个易错点已踩平并被门禁钉死：目录首级为 PackageIdentifier **首字母小写**；**默认语言（DefaultLocale）清单的 `ManifestType` 必须是 `defaultLocale`**（附加语言才是 `locale`），否则 winget validate 报 "multi file manifest is incomplete"。`winget validate` 成功也可能带非零退出码（succeeded with warnings = -1978335192），脚本以解析输出文本为准。门禁：`scripts/check-winget-channel.mjs`（静态针 + 真执行断言），接入 `npm run test:winget-channel` 与聚合 `npm test` 链；发布 runbook：v0.4.0 发布当天执行 `npm run package:winget -- --version 0.4.0`，按输出指引提交 PR，过审后合并 awam-software 的 `winget-install-methods` 分支并部署。
