# Build pinned model-hub-gateway for Windows release bundle and local dev.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-gateway-rust.ps1
# Do not commit binaries under tools/gateway-rust/.

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Manifest = Join-Path $Root "gateway-rust\Cargo.toml"
$Built = Join-Path $Root "gateway-rust\target\release\model-hub-gateway.exe"
$OutDir = Join-Path $Root "tools\gateway-rust"
$Target = Join-Path $OutDir "model-hub-gateway.exe"

if (-not (Test-Path $Manifest)) {
    throw "gateway-rust Cargo.toml not found: $Manifest"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "Building gateway-rust (release) ..."
Push-Location $Root
try {
    cargo build --manifest-path $Manifest --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release failed; exit=$LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

if (-not (Test-Path $Built)) {
    throw "build artifact missing: $Built"
}

Copy-Item -Force $Built $Target

if (-not (Test-Path $Target)) {
    throw "copy failed: $Target"
}

$sha = [System.Security.Cryptography.SHA256]::Create()
try {
    $stream = [System.IO.File]::OpenRead($Target)
    try {
        $hashBytes = $sha.ComputeHash($stream)
    }
    finally {
        $stream.Dispose()
    }
}
finally {
    $sha.Dispose()
}
$hex = ($hashBytes | ForEach-Object { $_.ToString("x2") }) -join ""

Write-Host "OK: $Target"
Write-Host "SHA-256: $hex"
Write-Host "Dev override: `$env:MODEL_HUB_GATEWAY_BIN = '$Target'"
