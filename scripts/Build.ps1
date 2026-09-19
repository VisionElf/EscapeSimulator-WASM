param([switch]$WasmOnly)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo test --release --locked --lib
    if ($LASTEXITCODE) { throw 'Rust tests failed' }
    cargo rustc --release --locked --target wasm32-unknown-unknown --lib --crate-type cdylib
    if ($LASTEXITCODE) { throw 'WASM build failed' }
    New-Item -ItemType Directory -Force release | Out-Null
    Copy-Item target/wasm32-unknown-unknown/release/escape_simulator_autosplitter.wasm release/EscapeSimulator.wasm
    if (!$WasmOnly) {
        & (Join-Path $PSScriptRoot 'Initialize-BuildDependencies.ps1')
        dotnet restore telemetry --locked-mode
        if ($LASTEXITCODE) { throw 'Telemetry restore failed' }
        dotnet build telemetry -c Release --no-restore
        if ($LASTEXITCODE) { throw 'Telemetry build failed' }
        Copy-Item telemetry/bin/Release/net472/EscapeSimulator.Telemetry.dll release/
    }
    Copy-Item README.md release/
    Get-ChildItem release -File | Where-Object Name -ne 'SHA256.json' | Get-FileHash | Select-Object @{n='File';e={Split-Path $_.Path -Leaf}},Hash |
        ConvertTo-Json | Set-Content release/SHA256.json -Encoding UTF8
} finally { Pop-Location }
