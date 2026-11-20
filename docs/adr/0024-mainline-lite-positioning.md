# 主线版 / 精简版定位与产物命名反转

把 ADR 0018 里「AI 包转为正式主包（名称即普通正式版）、核心包转为精简版」这句附带的话，
正式确立为产品定位，并贯穿到打包/命名/产物/文档。**只动面向用户的命名外壳，不动内核标识符。**

**Status**: accepted（落地见本 ADR 末尾「实现」段；命名层代码改动另见提交，docs/commit 同步）

## 决策

1. **两个编译期 flavor 的角色对调（语义层）**：
   - **主线版 Mainline** = `ai` flavor（默认 Cargo feature `ai-addon` + `VITE_AFM_AI_ADDON=1`）。
     它就是普通正式版，**占用素名** `A.File.Manager_${ver}_x64-setup.exe`（macOS：`…_${arch}.dmg` /
     `.app.zip` 无后缀）。Release / README / 下载页把它列为**首推默认**。
   - **精简版 Lite** = `core` flavor（`--no-default-features` + `VITE_AFM_AI_ADDON=0`）。
     带 `-lite` 标记后缀 `…_x64-lite-setup.exe`（macOS `…_${arch}-lite.dmg` / `-lite.app.zip`），
     列为次级「Lite（可选）」。
2. **内核标识符不变**（D5）：Cargo feature `ai-addon`、`VITE_AFM_AI_ADDON`、脚本参数 `ai`/`core`、
   workflow `packs=core,ai,feature-pack`、矩阵 `vite_ai` 全部保留。`flavor=="ai"` 即主线、
   `flavor=="core"` 即精简，只在资产命名函数里映射。**不**做 core→lite / ai→mainline 的全链路改名。
3. **精简版「精简」的边界**（D2）：精简版仅去 **AI Add-on**（Agent / MCP / Skills 面板、语音输入等
   `ai-addon` 门控能力）。**扩展中心（`/extensions` / ExtensionHost / Feature Pack）两版都有**，
   与现状一致，无需新增代码门控。两版的真实差异 = AI Add-on 这一条编译轴。
4. **knowledge 不是特例**：经核实 `awam-knowledge` 与 `awam-file` / `awam-tool-manager` 一样，
   是 `runtime: self` 的 **PyInstaller onefile Python sidecar**（`awam-knowledge.exe` @18790；
   仓内 `code/*.js` 仅为遗留/对标 harness）。所谓「knowledge 是 Node、体积小、内置进安装包」是
   **过时描述**。本 ADR 取代 ADR 0022「分发档位」与 `glossary-feature-pack`「共享运行时」里把
   knowledge 单列为 Node/内置的说法：**三者一致，均为 feed 分发、首启按需 provision 的 Feature Pack，
   不打进任何安装器，主线版与精简版都可安装。**
   - 注：`package-feature-packs.mjs` 已会把 `packs/*`（含 knowledge）打成 `feature-pack-*.zip`。原
     `release.yml` 的 `build-pack-feed` 只对 tool-runtime + file-gateway 产 managed 产物与 feed 条目、
     **漏了 knowledge**；该缺口已在后续 commit 补齐——`build-pack-feed` 现对三后端都跑 `build` +
     `pack-dir`，knowledge 用 `--paths src --collect awam_knowledge`（onefile 于 release runner 各 OS 产）。
     详见 `docs/feed-operations-runbook.md` §2/§6。

## 不做什么

- 不改 `tauri.conf.json` 的 `productName`（两版同一产品名 "A File Manager"，靠安装包资产名区分）。
- 不做向后兼容别名 / 旧名回落：项目未上线、无存量用户。历史上以素名发布的 `v0.3.0` 实为无 AI 构建，
  重定位后素名指向主线版，属可接受的语义变更。
- 不把扩展中心从精简版代码级剔除（D2 已否）。

## 实现

- 命名外壳：`scripts/release-assets.mjs`（`nsisAssetName` / `dmgAssetName` / `appZipAssetName` 对调）、
  `scripts/prepare-release.mjs`（资产计划标签）、`.github/workflows/release.yml`（头注释 / dispatch
  包说明 / Packages 表，主线版置顶）。
- 门禁**原子同提交**（`npm test` 是 `&&` 链、首失败即停）：`scripts/check-multi-pack-release.mjs`、
  `scripts/check-ci-macos.mjs` 同步钉死新命名。`scripts/check-packaging-flavors.mjs` 仅断内部标识符，不改。
- 文档：ADR 0018 加指向、`docs/adr/0012` / `docs/adr/0022` / `docs/glossary-feature-pack.md` /
  `GLOSSARY-MAP.md` 把「普通版」（旧指无 AI 核心构建）改词为「精简版」，并纠正 knowledge 归属；
  `README.md` 与 `.cursor/skills/afm-multi-pack-release/SKILL.md` 资产表更新。
- 应用内文案：`src-tauri/src/deps_probe.rs` AI Add-on 条目、`src/help/content/ai.md`。
  i18n 文案（`locales/*` 的「带 AI 包 / AI 版本」措辞）因该文件正被并发改动，单独错峰处理。
