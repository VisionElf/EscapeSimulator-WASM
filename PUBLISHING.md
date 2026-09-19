# Publishing through LiveSplit's catalog

Checked on September 19, 2026. This project has not been submitted to the catalog yet.

LiveSplit accepts WebAssembly autosplitters. Its documented submission process is
a pull request changing `LiveSplit.AutoSplitters.xml` in
[LiveSplit/LiveSplit.AutoSplitters](https://github.com/LiveSplit/LiveSplit.AutoSplitters#adding-an-auto-splitter).
Maintainers review and merge the request; inclusion is not automatic.

## Prepare a public release

1. Host the source and the English README in a public repository.
2. Tag the release using the version in Cargo.toml, also displayed by the runtime.
   The optional telemetry plugin has its own version.
3. Publish `EscapeSimulator.wasm`, the optional `EscapeSimulator.Telemetry.dll`,
   and checksums as release assets. Keep decompiled game sources, local logs,
   game DLLs, and personal setup notes out of the public release.
4. Document the tested game build, supported modes, remaining limitations,
   and source credits. Preserve the vendored ASR license files.

## Propose updating the existing entry

The [current catalog](https://github.com/LiveSplit/LiveSplit.AutoSplitters/blob/master/LiveSplit.AutoSplitters.xml)
already contains **Escape Simulator**, pointing to Undalevein's ASL plus asl-help
and Uhara. Edit that entry rather than adding another entry for the same game.
Leave **Escape Simulator 2** unchanged.

Suggested replacement, after substituting real repository and release URLs:

```xml
<AutoSplitter>
    <Games>
        <Game>Escape Simulator</Game>
    </Games>
    <URLs>
        <URL>https://github.com/OWNER/REPO/releases/download/v0.1.2/EscapeSimulator.wasm</URL>
    </URLs>
    <Type>Script</Type>
    <ScriptType>AutoSplittingRuntime</ScriptType>
    <Description>Auto start, splits, resets, and load removal. Optional token splits require the companion plugin; see setup instructions.</Description>
    <Website>https://github.com/OWNER/REPO#readme</Website>
</AutoSplitter>
```

This follows existing WASM entries in the catalog. A versioned asset makes the
reviewed binary explicit; later releases require updating the URL. The catalog
also contains stable release URLs that projects update themselves.

I recommend coordinating with the current maintainer before proposing a replacement,
especially to agree on support for older game builds. This is a recommendation,
not an approval requirement stated in the submission instructions.

## Explain the migration in the PR

Suggested title: **Escape Simulator: migrate autosplitter to ASR**.

Describe the broken SceneManager dependency, the replacement Mono reader, and
the observed start/load/restart/token tests. Link the source, release, and setup guide.
Explain that BepInEx and the telemetry DLL are installed separately in the game
folder and are needed only for token splits. Do not imply that the WASM installs them.
Ask maintainers to review that optional dependency explicitly; acceptance of this
particular design has not been confirmed.

After the catalog change is merged and downloaded by LiveSplit, runners can
activate the listed splitter from **Edit Splits**. They should remove any manually
added ASR component for the same game to avoid running two copies.

For ASR-specific questions, LiveSplit's documentation points to the
`#auto-splitting-v2` channel in the
[Speedrun Tool Development Discord](https://discord.gg/N6wv8pW).
