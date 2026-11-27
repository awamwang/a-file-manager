#Requires -Version 5.1
<#
.SYNOPSIS
  从仓库根目录 .env / .env.local 加载键值到当前进程环境变量。

.DESCRIPTION
  - 已存在于进程中的非空变量不被覆盖（命令行/CI 优先）
  - 支持 KEY=VALUE、export KEY=VALUE、可选单双引号
  - 以 # 开头的行视为注释
#>
param(
    [string]$Root = ""
)

$ErrorActionPreference = "Stop"

if (-not $Root) {
    $Root = Split-Path -Parent $PSScriptRoot
}

function Import-DotEnvFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return 0 }
    $count = 0
    foreach ($raw in Get-Content -LiteralPath $Path -Encoding utf8) {
        $line = $raw.Trim()
        if (-not $line -or $line.StartsWith("#")) { continue }
        if ($line -match '^\s*export\s+(.+)$') {
            $line = $Matches[1].Trim()
        }
        $eq = $line.IndexOf("=")
        if ($eq -lt 1) { continue }
        $key = $line.Substring(0, $eq).Trim()
        $val = $line.Substring($eq + 1).Trim()
        if ($key -notmatch '^[A-Za-z_][A-Za-z0-9_]*$') { continue }
        if (
            ($val.Length -ge 2) -and (
                ($val.StartsWith('"') -and $val.EndsWith('"')) -or
                ($val.StartsWith("'") -and $val.EndsWith("'"))
            )
        ) {
            $val = $val.Substring(1, $val.Length - 2)
        }
        $existing = [Environment]::GetEnvironmentVariable($key, "Process")
        if (-not [string]::IsNullOrWhiteSpace($existing)) { continue }
        Set-Item -Path "Env:$key" -Value $val
        $count++
    }
    return $count
}

$loaded = 0
foreach ($name in @(".env", ".env.local")) {
    $path = Join-Path $Root $name
    $n = Import-DotEnvFile -Path $path
    if ($n -gt 0) {
        Write-Host "==> 已从 $name 加载 $n 个环境变量"
        $loaded += $n
    }
}

if ($loaded -eq 0) {
    $hasEnv = (Test-Path -LiteralPath (Join-Path $Root ".env")) -or
        (Test-Path -LiteralPath (Join-Path $Root ".env.local"))
    if ($hasEnv) {
        Write-Host "==> .env 已存在，但无可写入项（可能均已被进程环境覆盖）"
    }
}
