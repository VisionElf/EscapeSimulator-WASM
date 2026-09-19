param([Parameter(Mandatory = $true)][string]$Tag)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$release = Join-Path $root 'release'
$expected = @('EscapeSimulator.wasm', 'EscapeSimulator.Telemetry.dll', 'README.md', 'SHA256.json')
$actual = @(Get-ChildItem -LiteralPath $release -File | ForEach-Object Name)
if (@(Compare-Object $expected $actual).Count -ne 0 -or @(Get-ChildItem -LiteralPath $release -Directory).Count -ne 0) {
    throw 'Unexpected release contents. Run a full build; package only WASM, telemetry, README and checksums.'
}
foreach ($entry in (Get-Content -LiteralPath (Join-Path $release 'SHA256.json') -Raw | ConvertFrom-Json)) {
    if ($entry.File -notin $expected -or $entry.File -eq 'SHA256.json') { throw 'Invalid checksum entry' }
    if ((Get-FileHash -LiteralPath (Join-Path $release $entry.File)).Hash -ne $entry.Hash) { throw "Checksum mismatch: $($entry.File)" }
}
$safeTag = $Tag -replace '[^a-zA-Z0-9._-]', '-'
if ([string]::IsNullOrWhiteSpace($safeTag)) { throw 'Empty release tag' }
$artifacts = Join-Path $root 'artifacts'
New-Item -ItemType Directory -Force $artifacts | Out-Null
$archive = Join-Path $artifacts "EscapeSimulator-Autosplitter-$safeTag-win-x64.zip"
Compress-Archive -LiteralPath @($expected | ForEach-Object { Join-Path $release $_ }) -DestinationPath $archive -Force
$checksum = (Get-FileHash -LiteralPath $archive).Hash.ToLowerInvariant() + '  ' + (Split-Path $archive -Leaf)
Set-Content -LiteralPath "$archive.sha256" -Value $checksum -Encoding ascii
Write-Output $archive
