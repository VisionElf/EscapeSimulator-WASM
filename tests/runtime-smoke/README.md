# LiveSplit runtime smoke test

This harness loads the WASM with an installed LiveSplit ASR library, reads the
running game, and prints its output. Timer callbacks are simulated: it does not
control your LiveSplit session. Run it with the game open to check attachment.

Requires the .NET 10 SDK. The .NET Framework target runs in x64 and requires 4.8.1.

```powershell
dotnet build tests/runtime-smoke -c Release -p:LiveSplitDir='PATH_TO_LIVESPLIT'
.\tests\runtime-smoke\bin\Release\net481\RuntimeSmoke.exe `
    .\release\EscapeSimulator.wasm 5 'PATH_TO_LIVESPLIT'
```

The process must have permission to read the game. A successful smoke run confirms
module loading and execution; it does not replace tests of actual gameplay events.
