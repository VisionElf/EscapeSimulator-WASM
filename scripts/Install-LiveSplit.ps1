param(
    [string]$LiveSplitDir,
    [switch]$Build
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
if ($Build) {
    & (Join-Path $PSScriptRoot 'Build.ps1') -WasmOnly
}
$source = Join-Path $root 'release/EscapeSimulator.wasm'
if (!(Test-Path -LiteralPath $source -PathType Leaf)) {
    throw 'WASM not found. Run this script with -Build first.'
}
if (!$LiveSplitDir) {
    $paths = @(Get-Process -Name LiveSplit -ErrorAction SilentlyContinue |
        ForEach-Object { if ($_.Path) { Split-Path $_.Path -Parent } } |
        Sort-Object -Unique)
    if ($paths.Count -ne 1) {
        throw 'Open one LiveSplit installation, or provide -LiveSplitDir pointing to its folder.'
    }
    $LiveSplitDir = $paths[0]
}
$LiveSplitDir = (Resolve-Path -LiteralPath $LiveSplitDir).Path
if (!(Test-Path -LiteralPath (Join-Path $LiveSplitDir 'LiveSplit.exe') -PathType Leaf)) {
    throw 'LiveSplitDir must contain LiveSplit.exe.'
}
$components = Join-Path $LiveSplitDir 'Components'
if (!(Test-Path -LiteralPath $components -PathType Container)) {
    throw 'LiveSplit Components folder not found.'
}
$destination = Join-Path $components 'EscapeSimulator.wasm'
$sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
if (Test-Path -LiteralPath $destination) {
    if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -eq $sourceHash) {
        Write-Output "Already up to date: $destination"
        return
    }
    $backup = "$destination.backup-$(Get-Date -Format 'yyyyMMdd-HHmmss-fff')"
    Copy-Item -LiteralPath $destination -Destination $backup
    Write-Output "Backup: $backup"
}
Copy-Item -LiteralPath $source -Destination $destination -Force
if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -ne $sourceHash) {
    throw 'Installed WASM checksum does not match the build.'
}
Write-Output "Installed: $destination"
Write-Output 'Reload the WASM in the Auto Splitting Runtime settings, or restart LiveSplit before your next run.'
