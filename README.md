# Escape Simulator Autosplitter

A LiveSplit autosplitter using **Auto Splitting Runtime (ASR)** and a `.wasm` module.
It reads the game's Mono fields directly and replaces the legacy ASL setup.
No `asl-help` or Uhara dependency is required.

## Features

- Automatic start at the opening fade: any playable room in IL mode, or selected Full Game starting rooms.
- Splits on level completion or the last level of a pack.
- Load removal, with Game Time paused in menus.
- Automatic reset on a direct room restart, followed by a new start at the fade.
- Optional token splits: every pickup or the eighth pickup in a room, including previously collected tokens.

## Installation

Requires Windows x64 and LiveSplit running in 64-bit mode with the
[Auto Splitting Runtime component](https://github.com/LiveSplit/LiveSplit.AutoSplittingRuntime).

1. Place `EscapeSimulator.wasm` in your LiveSplit `Components` folder.
2. Disable the old Escape Simulator autosplitter in **Edit Splits** and remove any legacy ASL component from your layout.
3. In **Edit Layout**, add **Control → Auto Splitting Runtime** and select the `.wasm` file in its settings.
4. Enable your preferred start and split options. Select **Game Time** as your timing method.
5. Save your layout, then load a room from the menu to test.

Attaching in an already loaded room does not start the timer retroactively.
For automatic starts, enable Tutorial, First Chamber, or **Start on any room**
under IL / individual packs. The latter supports any playable room, including
extras and new rooms, without a hardcoded starting-room list.
Reset on return to the menu is a separate, optional setting.

Settings are grouped into **General**, **Level splits**, **Full Game**,
**IL / individual packs**, and **Tokens**. Hover over an option for details.
The General switches apply to every run type. To split only at pack endings,
disable the every-room option; to split only on tokens, disable both level-split options.
Extras are excluded from pack-ending splits because they have no fixed final room.
Use **Split after every completed room** for extras; IL auto-start still applies.

### Optional: token splits

Normal level timing works without a game plugin. Token splits require the included
`EscapeSimulator.Telemetry.dll`:

1. Install [BepInEx 5.4.23.5, Windows x64](https://github.com/BepInEx/BepInEx/releases/tag/v5.4.23.5)
   in the game folder, next to `Escape Simulator.exe`.
2. Place the telemetry DLL in `BepInEx/plugins/EscapeSimulator.Telemetry/`.
3. Restart the game and enable the desired token option in the ASR settings.

The plugin observes pickups without changing save data. Pickups made before the
splitter starts observing the room are not counted retroactively.

## Troubleshooting

- After replacing the WASM, reload the component or restart LiveSplit.
- After replacing the telemetry DLL, restart the game.
- For token issues, check `BepInEx/LogOutput.log` for `Token protocol 1 ready`.
- `ES status`, `ES tokens`, and `ES timing` are exposed as LiveSplit custom variables.
- If `ES timing` reports invalid data, reset the timer before starting a new attempt.
  Missing token telemetry does not disable level-completion splits.

Tested on the Windows Mono build using Unity **6000.3.6f1**. Game updates may still
require changes. Co-op and all level packs have not been fully tested.

## Credits

Based on the original Escape Simulator autosplitter by **Undalevein and Jonc4**.
Built with [LiveSplit ASR](https://github.com/LiveSplit/asr),
[BepInEx](https://github.com/BepInEx/BepInEx), and Harmony.

## Building from source

Requires Rust with the `wasm32-unknown-unknown` target and Windows MSVC build tools.
The optional plugin also requires the .NET SDK. The full build downloads the
checksum-verified BepInEx archive and restores pinned Unity 6000.3.6 reference
assemblies from BepInEx's public NuGet feed. No game installation is needed to build.

```powershell
rustup target add wasm32-unknown-unknown
# WASM only:
.\scripts\Build.ps1 -WasmOnly
# Full build, including tests and optional plugin:
.\scripts\Build.ps1
```

Both build modes copy the WASM to `release/EscapeSimulator.wasm` and write checksums
in `release/`. This directory is ignored by Git; binaries will be distributed as
GitHub release assets. Game assemblies,
build dependencies, and generated files are not tracked in Git.
ASR is vendored with a small documented patch; see [UPSTREAM.md](vendor/asr/UPSTREAM.md).

Run logic tests with `cargo test --release --locked --lib`. To inspect a running
game without controlling LiveSplit, first build the development-only reader with
`cargo build --release --locked --bin probe`, then use
`.\scripts\Diagnose.ps1 -Seconds 30`. The reader is not included in release assets.
The optional [runtime smoke test](tests/runtime-smoke/README.md) executes the WASM
using an installed LiveSplit runtime with simulated timer callbacks.

See [PUBLISHING.md](PUBLISHING.md) for the official catalog submission process.

## Updating a local LiveSplit installation

With LiveSplit open, install the WASM from `release/`:

```powershell
.\scripts\Install-LiveSplit.ps1
# Build and test the current source before installing:
.\scripts\Install-LiveSplit.ps1 -Build
# Or specify the installation folder when LiveSplit is closed:
.\scripts\Install-LiveSplit.ps1 -LiveSplitDir 'C:\Tools\LiveSplit'
```

The script detects the running LiveSplit installation, backs up the previous
WASM, and verifies the installed file's SHA256. It only installs the WASM;
the optional token plugin is managed separately. Reload the WASM in the Auto
Splitting Runtime settings, or restart LiveSplit before your next run.

## GitHub releases

Pushing a tag runs the release workflow: tests, full build, ZIP creation, and
publication to GitHub Releases. For example, tag `v0.1.2` produces
`EscapeSimulator-Autosplitter-v0.1.2-win-x64.zip` containing the four files from
`release/`. The ZIP checksum and standalone WASM are also uploaded.

```powershell
git tag v0.1.2
git push origin v0.1.2
```

Update Cargo.toml and Cargo.lock before tagging a new version. Ordinary branch
pushes do not publish releases. The workflow uses GitHub's built-in token; no
personal access token or game files need to be added as secrets.
