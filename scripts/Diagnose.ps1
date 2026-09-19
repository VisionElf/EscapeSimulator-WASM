param([int]$Seconds = 30)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$game = Get-Process -Name 'Escape Simulator' -ErrorAction Stop | Select-Object -First 1
$probe = Join-Path $root 'target\release\probe.exe'
$logs = Join-Path $root 'logs'
New-Item -ItemType Directory -Force $logs | Out-Null
$logPath = Join-Path $logs ('probe-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.log')
& $probe $game.Id $Seconds 2>&1 | ForEach-Object { '[' + (Get-Date -Format 'HH:mm:ss.fff') + '] ' + $_ } | Tee-Object -FilePath $logPath
Write-Output "Diagnostic saved: $logPath"
