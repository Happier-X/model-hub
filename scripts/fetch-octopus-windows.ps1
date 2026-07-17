# 下载钉扎版本的 octopus Windows x64 二进制到 tools/octopus/
# 用法: powershell -ExecutionPolicy Bypass -File scripts/fetch-octopus-windows.ps1

$ErrorActionPreference = "Stop"

$Version = "v0.9.28"
$Asset = "octopus-windows-x86_64.zip"
$Url = "https://github.com/bestruirui/octopus/releases/download/$Version/$Asset"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$OutDir = Join-Path $Root "tools\octopus"
$ZipPath = Join-Path $OutDir $Asset

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "Downloading $Url ..."
Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing

Write-Host "Extracting to $OutDir ..."
Expand-Archive -Path $ZipPath -DestinationPath $OutDir -Force

$Exe = Get-ChildItem -Path $OutDir -Recurse -Filter "octopus*.exe" | Select-Object -First 1
if (-not $Exe) {
    throw "未在压缩包中找到 octopus*.exe"
}

$Target = Join-Path $OutDir "octopus.exe"
if ($Exe.FullName -ne $Target) {
    Copy-Item -Force $Exe.FullName $Target
}

Write-Host "OK: $Target"
Write-Host "Version pin: $Version"
Write-Host "Set env: `$env:MODEL_HUB_GATEWAY_BIN = '$Target'"
