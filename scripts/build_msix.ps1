#Requires -Version 5.1
<#
.SYNOPSIS
  把 Tauri 产物 a-file-manager.exe 打成 MSIX（Microsoft Store 提交用）。

.DESCRIPTION
  流程：复制图标 → 渲染 AppxManifest.xml → 组装 build\msix\staging →
        makeappx pack → makeappx unpack 往返校验 → 可选 Authenticode 签名。

  Identity / Publisher 必须与 Partner Center 预留应用时的一致，否则上传被拒。
  通过参数或环境变量提供（.env.local 已预填 A File Manager 的身份）：
    AFM_MSIX_IDENTITY_NAME          Identity Name（AwamMWang.AFileManager）
    AFM_MSIX_PUBLISHER             Publisher DN（CN=...）
    AFM_MSIX_PUBLISHER_DISPLAY_NAME 发布者显示名（Awam M Wang）

  三项任一为空时脚本打印提示并 exit 0（跳过 MSIX、保持默认 NSIS 打包），
  不会猜一个错误的 Publisher 让你上传后被拒。

.PARAMETER Version
  MSIX 版本（X.Y.Z.Q 四段）。默认从 src-tauri\tauri.conf.json 读取并补 .0。

.PARAMETER ExePath
  已构建的 Tauri exe，默认 src-tauri\target\release\a-file-manager.exe。
  注意 Tauri 的 ai / core 两个 flavor 会产出同名 exe，请先用对应 flavor 构建。

.PARAMETER OutputPath
  输出 .msix 路径，默认 dist\a-file-manager-<Version>-x64.msix。

.PARAMETER StartupTaskEnabled
  是否让 windows.startupTask 默认启用（默认 true，与设置 core.launchAtStartup 默认值一致）。
  关闭时 manifest Enabled=false 且不写 msix-startup.json。

.PARAMETER SelfSign
  用本机证书对 MSIX 再签一次（本地安装测试用；Store 提交可省略，微软会重签）。
  证书来源同 sign_file.ps1（AFM_SIGN_CERT / AFM_SIGN_THUMBPRINT）。

.PARAMETER SkipValidate
  跳过 pack → unpack 往返校验（排障用，不建议）。

.EXAMPLE
  .\scripts\build_msix.ps1
  # .env.local 已填三项身份 → 用默认 exe 打 MSIX

.EXAMPLE
  .\scripts\build_msix.ps1 -StartupTaskEnabled $false -ExePath .\src-tauri\target\release\a-file-manager.exe
#>
param(
    [string]$IdentityName = "",
    [string]$Publisher = "",
    [string]$PublisherDisplayName = "",

    [string]$Version = "",
    [string]$ExePath = "",
    [string]$OutputPath = "",

    [string]$MakeAppx = "",
    [bool]$StartupTaskEnabled = $true,

    [switch]$SelfSign,
    [switch]$SkipValidate,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
& (Join-Path $PSScriptRoot "load_env.ps1") -Root $Root

if (-not $IdentityName) { $IdentityName = $env:AFM_MSIX_IDENTITY_NAME }
if (-not $Publisher) { $Publisher = $env:AFM_MSIX_PUBLISHER }
if (-not $PublisherDisplayName) { $PublisherDisplayName = $env:AFM_MSIX_PUBLISHER_DISPLAY_NAME }

$missing = @()
if (-not $IdentityName) { $missing += "AFM_MSIX_IDENTITY_NAME（或 -IdentityName）" }
if (-not $Publisher) { $missing += "AFM_MSIX_PUBLISHER（或 -Publisher）" }
if (-not $PublisherDisplayName) { $missing += "AFM_MSIX_PUBLISHER_DISPLAY_NAME（或 -PublisherDisplayName）" }
if ($missing.Count -gt 0) {
    Write-Host "==> 未配置 MSIX 身份（$($missing -join ', ')），跳过 MSIX，保持默认 NSIS 打包方式。" -ForegroundColor Yellow
    Write-Host "    如需提交商店，请在 .env.local 填入三项身份值；详见 msix/README.md。"
    exit 0
}

function Get-TauriVersion {
    $confPath = Join-Path $Root "src-tauri\tauri.conf.json"
    if (-not (Test-Path -LiteralPath $confPath)) { throw "找不到 $confPath" }
    $conf = Get-Content -LiteralPath $confPath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($conf.version) { return [string]$conf.version }
    throw "无法从 tauri.conf.json 读取 version"
}

if (-not $Version) { $Version = Get-TauriVersion }
if ($Version -notmatch '^\d+\.\d+\.\d+(?:\.\d+)?$') {
    throw "MSIX 版本须为 X.Y.Z 或 X.Y.Z.Q，当前: $Version"
}
$msixVersion = if ($Version -match '^\d+\.\d+\.\d+$') { "$Version.0" } else { $Version }

if (-not $ExePath) { $ExePath = Join-Path $Root "src-tauri\target\release\a-file-manager.exe" }
if (-not (Test-Path -LiteralPath $ExePath)) {
    throw "找不到 exe: $ExePath；请先运行 node scripts/build-flavor.mjs ai --bundle"
}
if (-not $OutputPath) { $OutputPath = Join-Path $Root "dist\a-file-manager-$msixVersion-x64.msix" }

function Resolve-MakeAppx {
    param([string]$Explicit)
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit)) { throw "指定的 makeappx 不存在: $Explicit" }
        return $Explicit
    }
    $envTool = $env:AFM_MAKEAPPX
    if ($envTool) {
        if (-not (Test-Path -LiteralPath $envTool)) { throw "AFM_MAKEAPPX 指向的 makeappx 不存在: $envTool" }
        return $envTool
    }
    $cmd = Get-Command makeappx.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $pf32 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
    $roots = @((Join-Path $pf32 "Windows Kits\10\bin"), (Join-Path $env:ProgramFiles "Windows Kits\10\bin"))
    foreach ($root in $roots) {
        if (-not $root -or -not (Test-Path -LiteralPath $root)) { continue }
        $dirs = Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } | Sort-Object Name -Descending
        foreach ($dir in $dirs) {
            $tool = Join-Path $dir.FullName "x64\makeappx.exe"
            if (Test-Path -LiteralPath $tool) { return $tool }
        }
    }
    throw "未找到 makeappx.exe。请安装 Windows SDK（MSIX 打包工具）或设置 AFM_MAKEAPPX。"
}

function Invoke-NativeChecked {
    param([Parameter(Mandatory = $true)][string]$Exe, [string[]]$ArgList)
    Write-Host "==> & $Exe $($ArgList -join ' ')"
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $Exe @ArgList 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prevEap
    }
    $output | ForEach-Object { Write-Host $_ }
    if ($code -ne 0) { throw "命令失败（exit=$code）: $Exe" }
}

$makeAppxCli = Resolve-MakeAppx -Explicit $MakeAppx
Write-Host "==> makeappx: $makeAppxCli"
Write-Host "==> Identity: $IdentityName"
Write-Host "==> Publisher: $Publisher"
Write-Host "==> Version: $msixVersion"
Write-Host "==> Exe: $ExePath"

$stageRoot = Join-Path $Root "build\msix"
# 唯一目录：不删除旧 staging（宿主/杀软的文件删除拦截会让打包失败）
$stageDir = Join-Path $stageRoot ("staging-" + [guid]::NewGuid().ToString("N"))
$assetsDir = Join-Path $stageDir "Assets"
New-Item -ItemType Directory -Path $assetsDir -Force | Out-Null

# 1) 图标：Tauri 已生成 MSIX 所需尺寸，直接复制
$iconDir = Join-Path $Root "src-tauri\icons"
foreach ($name in @("StoreLogo.png", "Square44x44Logo.png", "Square150x150Logo.png", "Square310x310Logo.png")) {
    $src = Join-Path $iconDir $name
    if (-not (Test-Path -LiteralPath $src)) { throw "缺少 MSIX 图标: $src" }
    Copy-Item -LiteralPath $src -Destination (Join-Path $assetsDir $name) -Force
}

# 2) 渲染 manifest（UTF-8 无 BOM）
$templatePath = Join-Path $Root "msix\AppxManifest.xml.template"
if (-not (Test-Path -LiteralPath $templatePath)) { throw "缺少 manifest 模板: $templatePath" }
$manifest = Get-Content -LiteralPath $templatePath -Raw -Encoding utf8
$manifest = $manifest.Replace("__IDENTITY_NAME__", $IdentityName)
$manifest = $manifest.Replace("__PUBLISHER__", $Publisher)
$manifest = $manifest.Replace("__PUBLISHER_DISPLAY_NAME__", $PublisherDisplayName)
$manifest = $manifest.Replace("__VERSION__", $msixVersion)
$manifest = $manifest.Replace("__STARTUP_TASK_ENABLED__", $StartupTaskEnabled.ToString().ToLowerInvariant())
if ($manifest -match '__[A-Z_]+__') {
    throw "manifest 仍有未替换占位符: $([regex]::Matches($manifest, '__[A-Z_]+__') | ForEach-Object { $_.Value } | Select-Object -Unique)"
}
[System.IO.File]::WriteAllText((Join-Path $stageDir "AppxManifest.xml"), $manifest, [System.Text.UTF8Encoding]::new($false))

# 3) 商店版自启配置标记：包内文件，供未来 Rust 侧识别 MSIX 模式用
if ($StartupTaskEnabled) {
    $startupConfig = @{ startupTask = $true; taskId = "AFMAutoStart" } | ConvertTo-Json -Compress
    [System.IO.File]::WriteAllText(
        (Join-Path $stageDir "msix-startup.json"),
        $startupConfig,
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-Host "==> 已写入 msix-startup.json（商店版启用 startupTask）"
} else {
    Write-Host "==> 未写入 msix-startup.json（manifest 自启已关闭）"
}

# 4) 复制 exe（MSIX 内文件名须与 manifest Executable 一致）
Copy-Item -LiteralPath $ExePath -Destination (Join-Path $stageDir "a-file-manager.exe") -Force

# 5) 校验 staging 结构
$requiredFiles = @("AppxManifest.xml", "a-file-manager.exe", "Assets\StoreLogo.png", "Assets\Square150x150Logo.png", "Assets\Square44x44Logo.png")
if ($StartupTaskEnabled) { $requiredFiles += "msix-startup.json" }
foreach ($required in $requiredFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $stageDir $required))) {
        throw "staging 缺少必需文件: $required"
    }
}

Write-Host "==> staging 就绪: $stageDir"
if ($DryRun) {
    Write-Host "[DryRun] 跳过 makeappx pack；可用以下命令手动打包："
    Write-Host "  & `"$makeAppxCli`" pack /d `"$stageDir`" /p `"$OutputPath`""
    exit 0
}

# 6) 打包（/o = 覆盖，避免先删后写触发宿主 / 杀软拦截）
$outDir = Split-Path -Parent $OutputPath
if ($outDir -and -not (Test-Path -LiteralPath $outDir)) { New-Item -ItemType Directory -Path $outDir -Force | Out-Null }
Invoke-NativeChecked -Exe $makeAppxCli -ArgList @("pack", "/o", "/d", $stageDir, "/p", $OutputPath)

# 7) 往返校验（用唯一目录，避免删除旧目录被拦截）
if (-not $SkipValidate) {
    $verifyDir = Join-Path $stageRoot ("verify-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $verifyDir -Force | Out-Null
    Invoke-NativeChecked -Exe $makeAppxCli -ArgList @("unpack", "/p", $OutputPath, "/d", $verifyDir)
    $verifyRequired = @("AppxManifest.xml", "a-file-manager.exe", "Assets\StoreLogo.png")
    if ($StartupTaskEnabled) { $verifyRequired += "msix-startup.json" }
    foreach ($required in $verifyRequired) {
        if (-not (Test-Path -LiteralPath (Join-Path $verifyDir $required))) {
            throw "MSIX 往返校验失败，包内缺少: $required"
        }
    }
    Write-Host "==> MSIX 往返校验通过（pack → unpack 内容完整）"
}

# 8) 可选签名（本地安装测试）
if ($SelfSign) {
    & (Join-Path $PSScriptRoot "sign_file.ps1") -FilePath $OutputPath
}

$sizeMB = [math]::Round((Get-Item -LiteralPath $OutputPath).Length / 1MB, 2)
Write-Host "==> 完成: $OutputPath ($sizeMB MB)"
Write-Host "    提交到 Partner Center 前请确认 Identity / Publisher 与预留应用完全一致。"
