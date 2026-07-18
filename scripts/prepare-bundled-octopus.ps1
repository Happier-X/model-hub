# 下载并校验钉扎版本的 octopus Windows x64，供发布内嵌与本地开发使用。
# 用法: powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-octopus.ps1
# 不提交 tools/octopus/ 下的大二进制。

$ErrorActionPreference = "Stop"

$Version = "v0.9.28"
$Asset = "octopus-windows-x86_64.zip"
$Url = "https://github.com/bestruirui/octopus/releases/download/$Version/$Asset"
# 钉扎哈希（首次本机下载后固化；与任务已知值一致）
$ExpectedZipSha256 = "17b071b66218f15b574efe08c73b4ec56d6adfd9c08aab3b216728b29ac0f92f"
$ExpectedExeSha256 = "38c4238c5c8be0d3e718eb6192c9d06b2e1dcb4222179f625627c67b1e98c0d8"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$OutDir = Join-Path $Root "tools\octopus"
$ZipPath = Join-Path $OutDir $Asset
$Target = Join-Path $OutDir "octopus.exe"

function Get-FileSha256Lower {
    param([Parameter(Mandatory = $true)][string]$Path)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $stream = [System.IO.File]::OpenRead($Path)
        try {
            $hashBytes = $sha.ComputeHash($stream)
        } finally {
            $stream.Dispose()
        }
    } finally {
        $sha.Dispose()
    }
    $hex = ($hashBytes | ForEach-Object { $_.ToString("x2") }) -join ""
    return $hex
}

function Assert-Sha256 {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Expected,
        [Parameter(Mandatory = $true)][string]$Label
    )
    $actual = Get-FileSha256Lower -Path $Path
    if ($actual -ne $Expected.ToLowerInvariant()) {
        throw "$Label SHA-256 mismatch. expected=$Expected actual=$actual path=$Path"
    }
    Write-Host "OK SHA-256 ($Label): $actual"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$needDownload = $true
if (Test-Path $ZipPath) {
    try {
        Assert-Sha256 -Path $ZipPath -Expected $ExpectedZipSha256 -Label "zip(existing)"
        $needDownload = $false
        Write-Host "Reuse existing zip: $ZipPath"
    } catch {
        Write-Host "Existing zip failed verification, re-downloading..."
        Remove-Item -Force $ZipPath
    }
}

if ($needDownload) {
    Write-Host "Downloading $Url ..."
    Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing
}

Assert-Sha256 -Path $ZipPath -Expected $ExpectedZipSha256 -Label "zip"

Write-Host "Extracting to $OutDir ..."
Expand-Archive -Path $ZipPath -DestinationPath $OutDir -Force

$Exe = Get-ChildItem -Path $OutDir -Recurse -Filter "octopus*.exe" |
    Where-Object { $_.Name -ieq "octopus.exe" -or $_.Name -like "octopus*.exe" } |
    Select-Object -First 1
if (-not $Exe) {
    throw "octopus*.exe not found in archive"
}

if ($Exe.FullName -ne $Target) {
    Copy-Item -Force $Exe.FullName $Target
}

if (-not (Test-Path $Target)) {
    throw "failed to produce $Target"
}

Assert-Sha256 -Path $Target -Expected $ExpectedExeSha256 -Label "exe"

Write-Host "OK: $Target"
Write-Host "Version pin: $Version"
Write-Host "Source archive: https://github.com/bestruirui/octopus/archive/refs/tags/$Version.tar.gz"
Write-Host "Compliance: third-party/octopus/"
Write-Host "Dev override: `$env:MODEL_HUB_GATEWAY_BIN = '$Target'"
