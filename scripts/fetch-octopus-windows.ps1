# 兼容入口：转发到 prepare-bundled-octopus.ps1（含 SHA-256 校验）。
# 用法: powershell -ExecutionPolicy Bypass -File scripts/fetch-octopus-windows.ps1

$ErrorActionPreference = "Stop"
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
& (Join-Path $Here "prepare-bundled-octopus.ps1")
if ($LASTEXITCODE -ne 0 -and $null -ne $LASTEXITCODE) {
    exit $LASTEXITCODE
}
