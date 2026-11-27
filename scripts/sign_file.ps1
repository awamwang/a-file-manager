#Requires -Version 5.1
<#
.SYNOPSIS
  可选的 Authenticode 代码签名：未配置证书时静默跳过，不阻塞发版。

.DESCRIPTION
  证书来源（二选一，均从参数或 .env / 环境变量读取）：
    1) PFX 文件：AFM_SIGN_CERT=<路径> [+ AFM_SIGN_CERT_PASSWORD=<密码>]
    2) 证书存储指纹：AFM_SIGN_THUMBPRINT=<SHA1 指纹>（UKey / 云签名客户端导入证书后适用）

  其它环境变量：
    AFM_SIGN_TIMESTAMP_URL  时间戳服务器，默认 http://timestamp.digicert.com
    AFM_SIGNTOOL            显式指定 signtool.exe 路径；缺省时自动查找 Windows SDK 最新 x64 版本

  签名算法固定 SHA256 + RFC3161 时间戳（/tr + /td SHA256），证书到期后旧签名仍有效。
  已签名且签名有效的文件默认不重复签（避免每次发版叠加）；-Force 可强制重签。

.PARAMETER FilePath
  待签名文件，可多个（installer 与内部 exe 一并签）。

.PARAMETER SkipSign
  显式跳过签名，即使已配置证书。

.PARAMETER Force
  即使文件已有有效签名也强制重签（换证书时用）。

.EXAMPLE
  .\scripts\sign_file.ps1 -FilePath .\src-tauri\target\release\a-file-manager.exe
  # 未配置证书 → 输出"跳过签名"并返回 0

.EXAMPLE
  $env:AFM_SIGN_CERT = "C:\certs\afm.pfx"
  $env:AFM_SIGN_CERT_PASSWORD = "..."
  .\scripts\sign_file.ps1 -FilePath .\src-tauri\target\release\a-file-manager.exe
#>
param(
    [Parameter(Mandatory = $true)]
    [string[]]$FilePath,

    [string]$CertificatePath = "",
    [string]$CertificatePassword = "",
    [string]$Thumbprint = "",
    [string]$TimestampUrl = "",

    [switch]$SkipSign,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# ---- 配置解析：参数 > 环境变量 > 默认值 ----
if (-not $CertificatePath) { $CertificatePath = $env:AFM_SIGN_CERT }
if (-not $CertificatePassword) { $CertificatePassword = $env:AFM_SIGN_CERT_PASSWORD }
if (-not $Thumbprint) { $Thumbprint = $env:AFM_SIGN_THUMBPRINT }
if (-not $TimestampUrl) { $TimestampUrl = $env:AFM_SIGN_TIMESTAMP_URL }
if (-not $TimestampUrl) { $TimestampUrl = "http://timestamp.digicert.com" }

if ($SkipSign) {
    Write-Host "==> 已按要求跳过代码签名（-SkipSign）"
    return
}

if (-not $CertificatePath -and -not $Thumbprint) {
    Write-Host "==> 未配置代码签名证书（AFM_SIGN_CERT / AFM_SIGN_THUMBPRINT），跳过签名" -ForegroundColor DarkYellow
    Write-Host "    如需启用：在 .env.local 填写 AFM_SIGN_CERT=<pfx 路径> 和 AFM_SIGN_CERT_PASSWORD=<密码>；" -ForegroundColor DarkYellow
    Write-Host "    UKey / 云签名则填 AFM_SIGN_THUMBPRINT=<证书 SHA1 指纹>。" -ForegroundColor DarkYellow
    return
}

function Resolve-SignTool {
    param([string]$Explicit)

    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit)) { throw "AFM_SIGNTOOL 指向的 signtool 不存在: $Explicit" }
        return $Explicit
    }
    $envTool = $env:AFM_SIGNTOOL
    if ($envTool) {
        if (-not (Test-Path -LiteralPath $envTool)) { throw "AFM_SIGNTOOL 指向的 signtool 不存在: $envTool" }
        return $envTool
    }

    $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $pf32 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
    $roots = @(
        (Join-Path $pf32 "Windows Kits\10\bin"),
        (Join-Path $env:ProgramFiles "Windows Kits\10\bin")
    )
    $found = @()
    foreach ($root in $roots) {
        if (-not $root -or -not (Test-Path -LiteralPath $root)) { continue }
        $candidates = Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
            Sort-Object Name -Descending
        foreach ($dir in $candidates) {
            $tool = Join-Path $dir.FullName "x64\signtool.exe"
            if (Test-Path -LiteralPath $tool) { $found += $tool; break }
        }
    }
    if ($found.Count -eq 0) {
        throw "未找到 signtool.exe。请安装 Windows SDK（Signing Tools），或在 .env.local 设置 AFM_SIGNTOOL=<绝对路径>。"
    }
    return $found[0]
}

function Invoke-NativeCapture {
    param([Parameter(Mandatory = $true)][string]$Exe, [string[]]$ArgList)
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $Exe @ArgList 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prevEap
    }
    return [pscustomobject]@{ ExitCode = $code; Output = $output }
}

$signTool = Resolve-SignTool -Explicit ""
Write-Host "==> signtool: $signTool"

$certSource = ""
if ($Thumbprint) {
    $certSource = "证书存储指纹 $Thumbprint"
    if (-not ($Thumbprint -match '^[0-9a-fA-F]{40}$')) {
        # 支持带空格的指纹（从证书管理器复制时常见）
        $normalized = ($Thumbprint -replace '\s', '')
        if ($normalized -match '^[0-9a-fA-F]{40}$') { $Thumbprint = $normalized }
        else { throw "AFM_SIGN_THUMBPRINT 不是有效的 40 位 SHA1 指纹: $Thumbprint" }
    }
} else {
    if (-not (Test-Path -LiteralPath $CertificatePath)) {
        throw "AFM_SIGN_CERT 指向的 PFX 不存在: $CertificatePath"
    }
    $certSource = "PFX $CertificatePath"
}
Write-Host "==> 证书来源: $certSource"
Write-Host "==> 时间戳服务器: $TimestampUrl"

foreach ($file in $FilePath) {
    if (-not (Test-Path -LiteralPath $file)) { throw "待签名文件不存在: $file" }

    $sig = Get-AuthenticodeSignature -LiteralPath $file -ErrorAction SilentlyContinue
    if ($sig -and $sig.Status -eq "Valid" -and -not $Force) {
        Write-Host "==> 已有有效签名，跳过: $file （签名者: $($sig.SignerCertificate.Subject)）"
        continue
    }

    $signArgs = @("sign", "/fd", "SHA256", "/tr", $TimestampUrl, "/td", "SHA256")
    if ($Thumbprint) {
        $signArgs += @("/sha1", $Thumbprint)
    } else {
        $signArgs += @("/f", $CertificatePath)
        if ($CertificatePassword) { $signArgs += @("/p", $CertificatePassword) }
    }
    $signArgs += $file

    Write-Host "==> 签名中: $file"
    $result = Invoke-NativeCapture -Exe $signTool -ArgList $signArgs
    $result.Output | ForEach-Object { Write-Host $_ }
    if ($result.ExitCode -ne 0) {
        throw "签名失败（exit=$($result.ExitCode)）: $file。常见原因：PFX 密码错误 / 证书无代码签名 EKU / UKey 未插或 PIN 未输入。"
    }

    $after = Get-AuthenticodeSignature -LiteralPath $file -ErrorAction SilentlyContinue
    $status = if ($after) { $after.Status.ToString() } else { "Unknown" }
    $subject = if ($after -and $after.SignerCertificate) { $after.SignerCertificate.Subject } else { "(unknown)" }
    Write-Host "==> 签名完成: $file [状态: $status] [发布者: $subject]"
    if ($after -and $after.Status -ne "Valid") {
        Write-Host "    提示: 状态非 Valid 通常表示根证书未在本机信任链中（自签测试证书常见），不代表签名损坏。" -ForegroundColor DarkYellow
    }
}

Write-Host "==> 代码签名步骤结束"
