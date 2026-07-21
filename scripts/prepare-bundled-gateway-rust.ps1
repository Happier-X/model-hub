# 构建并复制钉扎路径的 model-hub-gateway Windows 二进制，供发布内嵌与本地开发使用。
# 用法: powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-gateway-rust.ps1
# 不提交 tools/gateway-rust/ 下的大二进制。

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Manifest = Join-Path $Root "gateway-rust\Cargo.toml"
$Built = Join-Path $Root "gateway-rust\target\release\model-hub-gateway.exe"
$OutDir = Join-Path $Root "tools\gateway-rust"
$Target = Join-Path $OutDir "model-hub-gateway.exe"

if (-not (Test-Path $Manifest)) {
    throw "未找到 gateway-rust Cargo.toml: $Manifest"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "Building gateway-rust (release) ..."
Push-Location $Root
try {
    cargo build --manifest-path $Manifest --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release 失败，exit=$LASTEXITCODE"
    }
} finally {
    Pop-Location
}

if (-not (Test-Path $Built)) {
    throw "构建产物不存在: $Built"
}

Copy-Item -Force $Built $Target

if (-not (Test-Path $Target)) {
    throw "复制失败: $Target"
}

$sha = [System.Security.Cryptography.SHA256]::Create()
try {
    $stream = [System.IO.File]::OpenRead($Target)
    try {
        $hashBytes = $sha.ComputeHash($stream)
    } finally {
        $stream.Dispose()
    }
} finally {
    $sha.Dispose()
}
$hex = ($hashBytes | ForEach-Object { $_.ToString("x2") }) -join ""

Write-Host "OK: $Target"
Write-Host "SHA-256: $hex"
Write-Host "Dev override: `$env:MODEL_HUB_GATEWAY_IMPL = 'rust'"
Write-Host "              `$env:MODEL_HUB_GATEWAY_RUST_BIN = '$Target'"
