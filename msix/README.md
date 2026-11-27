# Microsoft Store / MSIX 打包

本目录给 A File Manager 提供标准化的 MSIX 脚手架：把 Tauri 构建的 `a-file-manager.exe`
打成可提交 Microsoft Store 的 MSIX。微软在商店侧会**重新签名**，因此这条路径**不需要
购买代码签名证书**；证书只用于官网直发的 NSIS 安装包（见下「代码签名」）。

## 文件清单

| 文件 | 用途 |
| --- | --- |
| `.env.local` | 本机私有配置（已 gitignore）。身份三项已预填，签名项留空 |
| `msix/AppxManifest.xml.template` | full-trust 桌面应用 manifest，含 `windows.startupTask` 扩展 |
| `scripts/build_msix.ps1` | 渲染 manifest → 组装 staging → `makeappx pack` → `unpack` 往返校验 → 可选签名 |
| `scripts/sign_file.ps1` | 可选 Authenticode 签名（PFX / 证书指纹，无证书自动跳过） |
| `scripts/load_env.ps1` | 加载 `.env` / `.env.local` 到进程环境变量 |

## 配置

`.env.local` 已预填商店身份：

```
AFM_MSIX_IDENTITY_NAME=AwamMWang.AFileManager
AFM_MSIX_PUBLISHER=CN=205330CE-5E66-4AF5-B35C-25EC4109E6AB
AFM_MSIX_PUBLISHER_DISPLAY_NAME=Awam M Wang
```

**三项任一为空时**，`build_msix.ps1` 打印提示并 `exit 0`，保持默认 NSIS 打包方式，
不会拿错误身份去商店碰壁。

## 打包

前置：Windows SDK（MSIX 打包工具，`makeappx.exe`）；已构建 Tauri exe。

```powershell
# 1) 构建主线版（ai flavor）exe
node scripts/build-flavor.mjs ai --bundle

# 2) 打 MSIX（.env.local 已填身份）
pwsh -NoProfile -ExecutionPolicy Bypass -File "scripts\build_msix.ps1"

# 只渲染 staging 看 manifest，不打包
pwsh -NoProfile -ExecutionPolicy Bypass -File "scripts\build_msix.ps1" -DryRun

# 本地安装测试：额外自签（需要证书）
pwsh -NoProfile -ExecutionPolicy Bypass -File "scripts\build_msix.ps1" -SelfSign
```

产物：`dist/a-file-manager-<版本>-x64.msix`。版本从 `src-tauri/tauri.conf.json`
读取，三段自动补 `.0`。

### flavor 与 exe

`build-flavor.mjs ai` 与 `core` 会产出**同名** `a-file-manager.exe`（内容不同）。
打 MSIX 前请确认 `src-tauri/target/release/a-file-manager.exe` 是你想上架的那个 flavor；
需要指定其他路径时用 `-ExePath <绝对路径>`。商店版建议用主线版（ai）。

## 开机自启（windows.startupTask）

manifest 已声明 `desktop:windows.startupTask`（TaskId `AFMAutoStart`），默认
`Enabled=true`。打包脚本在包根写配置标记 `msix-startup.json`：

```json
{"storeBuild":true,"startupTask":true,"taskId":"AFMAutoStart"}
```

- `storeBuild`：**始终为 true**（只要是 MSIX 包）。这是打包时的商店版配置开关。
- `startupTask`：随 `-StartupTaskEnabled` 变化；关闭时 manifest `Enabled=false` 且此处为
  `false`，但 `storeBuild` 仍为 true，应用内勾选项照样隐藏。

**运行时行为（配置驱动，不探测包身份）**：Rust `platform_capabilities()` 读取 exe 同目录的
`msix-startup.json`，把 `storeBuild` 暴露为 `PlatformCapabilities.storeBuild`。设置策略据此
隐藏 `core.launchAtStartup` / `core.launchAtStartupHidden` 两行（HKCU Run 在 MSIX 里会被
虚拟化，应用内开关无效），设置页顶部改为显示提示：到 **Windows 设置 → 应用 → 启动 →
A File Manager** 开启或关闭。绿色版 NSIS 包没有该标记，勾选项保持原样。

关闭方法：商店版自启由 Windows「启动应用」页管理，用户到
**设置 → 应用 → 启动 → A File Manager** 关闭开关即可。i18n key
`settings.storeBuildAutostartNotice`（6 个 locale）提供该提示文案。

baseline 覆盖：`platform/mod.rs::store_build_marker_only_truthy_flag`（CARGO）、
`settings/platform_policy.rs::store_build_hides_in_app_autostart_rows`（CARGO）、
`scripts/check-autostart.mjs`（STATIC，含 FE↔BE 字段镜像断言），已接入 `npm test` 链。

## 代码签名（官网直发 NSIS 用）

NSIS 安装包和内部 exe 可以签名，以消除「未知发布者」并逐步积累 SmartScreen 信誉。

```powershell
# 无证书 → 自动跳过，不阻塞任何构建
pwsh -NoProfile -ExecutionPolicy Bypass -File "scripts\sign_file.ps1" -FilePath ".\src-tauri\target\release\a-file-manager.exe"
pwsh -NoProfile -ExecutionPolicy Bypass -File "scripts\sign_file.ps1" -FilePath ".\dist\release\A.File.Manager_0.4.0_x64-setup.exe"
```

证书来源二选一，填进 `.env.local`（勿提交）：

- `AFM_SIGN_CERT=<pfx 路径>` + `AFM_SIGN_CERT_PASSWORD=<密码>`
- `AFM_SIGN_THUMBPRINT=<证书 SHA1 指纹>`（UKey / 云签名客户端导入后）

签名固定 SHA256 + RFC3161 时间戳（`/tr` + `/td SHA256`），证书到期后旧签名仍有效。
已有有效签名不会重复签（`-Force` 可强制）。

**CI 集成（已接入）**：`.github/workflows/release.yml` 的 Windows job（`build-app`）在
`Collect canonical asset` 之后、`Upload to GitHub Release` 之前有 `Sign Windows installer
(optional)` 步骤，复用同一个 `scripts/sign_file.ps1`。无证书时自动跳过，不阻塞发版。
`check-multi-pack-release.mjs` 有断言锁住这个顺序。

**你只需配 GitHub Secrets**（仓库 Settings → Secrets and variables → Actions → New repository
secret），二选一：

| Secret | 用途 |
| --- | --- |
| `AFM_SIGN_CERT_B64` | PFX 文件的 base64（生成：`base64 -w0 afm.pfx`） |
| `AFM_SIGN_CERT_PASSWORD` | PFX 密码（与上一项配套）；走环境变量，不进命令行参数 |
| `AFM_SIGN_THUMBPRINT` | 证书存储 SHA1 指纹（UKey / 云签名客户端，替代 PFX 方案） |

> 签名必须在 GitHub Release 上传**之前**完成，否则下载到的文件与任何已发布校验和不一致。

## 交接清单（Partner Center 操作，我无法代劳）

1. 注册开发者账号（企业账号现已免费），预留应用名 `A File Manager`。
2. 核对 `Identity Name` / `Publisher` / `PublisherDisplayName` 与 Partner Center 产品 identity
   页**完全一致**——Publisher DN 里的 GUID 是账号唯一的，打错会被拒。
3. 准备隐私政策 URL、full-trust 权限说明（文件管理器需要文件系统访问）。
4. 上传 `dist/a-file-manager-<版本>-x64.msix` 并送认证。
5. **实机验证（商店版）**：WebView2 运行时（NSIS 用 `embedBootstrapper` 静默安装，
   MSIX 里这一步不存在，需确认目标机器已有 WebView2）、文件读写/拖拽、设置持久化路径、
   开机自启开关、managed dependency 下载、shell 集成。

## 未纳入本次范围

- 不修改 `.github/workflows/release.yml` / `prepare-release.mjs` 等 release plumbing。
- 不修改 Rust 自启实现与前端设置页（工作区有未提交改动）。
- 不使用 `Wide310x150Logo.png` / `Square310x310Logo` 作为 DefaultTile：makeappx 要求
  `Square310x310Logo` 与 `Wide310x150Logo` 同时出现，Tauri 未生成后者，故 manifest 用空
  `<uap:DefaultTile />`（已通过 `makeappx pack` 校验）。四个图标文件仍会复制进包，方便后续补磁贴。
