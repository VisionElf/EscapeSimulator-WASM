$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$deps = Join-Path $root '.deps'
$archive = Join-Path $deps 'BepInEx_win_x64_5.4.23.5.zip'
$destination = Join-Path $deps 'BepInEx-5.4.23.5'
New-Item -ItemType Directory -Force $deps | Out-Null
if (!(Test-Path -LiteralPath $archive)) {
    Invoke-WebRequest 'https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.5/BepInEx_win_x64_5.4.23.5.zip' -OutFile $archive
}
if ((Get-FileHash -LiteralPath $archive).Hash -ne '82F9878551030F54657792C0740D9D51A09500EEAE1FBA21106B0C441E6732C4') {
    throw 'BepInEx archive checksum mismatch'
}
Expand-Archive -LiteralPath $archive -DestinationPath $destination -Force
