param([string]$GameDir, [switch]$WasmOnly)
$ErrorActionPreference = 'Stop'
if (!$WasmOnly -and !$GameDir) { throw 'Provide -GameDir for the full build, or use -WasmOnly.' }
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo test --release --locked --lib
    if ($LASTEXITCODE) { throw 'Rust tests failed' }
    cargo build --release --locked --bin probe
    if ($LASTEXITCODE) { throw 'Probe build failed' }
    cargo rustc --release --locked --target wasm32-unknown-unknown --lib --crate-type cdylib
    if ($LASTEXITCODE) { throw 'WASM build failed' }
    New-Item -ItemType Directory -Force release | Out-Null
    Copy-Item target/wasm32-unknown-unknown/release/escape_simulator_autosplitter.wasm release/EscapeSimulator.wasm
    if (!$WasmOnly) {
        dotnet restore telemetry --locked-mode
        if ($LASTEXITCODE) { throw 'Telemetry restore failed' }
        dotnet build telemetry -c Release --no-restore "-p:GameDir=$GameDir"
        if ($LASTEXITCODE) { throw 'Telemetry build failed' }
        Copy-Item telemetry/bin/Release/net472/EscapeSimulator.Telemetry.dll release/
    }
    Copy-Item target/release/probe.exe release/
    Copy-Item README.md release/
    Get-ChildItem release -File | Where-Object Name -ne 'SHA256.json' | Get-FileHash | Select-Object @{n='File';e={Split-Path $_.Path -Leaf}},Hash |
        ConvertTo-Json | Set-Content release/SHA256.json -Encoding UTF8
} finally { Pop-Location }
