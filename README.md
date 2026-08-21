# SCRCPY Studio

**A smart, outcome-first GUI for scrcpy.**

SCRCPY Studio is an independent desktop frontend for the official [Genymobile scrcpy](https://github.com/Genymobile/scrcpy) project. Instead of exposing every command-line flag at once, SCRCPY Studio asks what you want to do and creates a sensible profile automatically.

> Status: **v0.1 development milestone**. The core architecture and first usable workflows are implemented. Runtime bundling, release signing, broader device testing, and additional self-healing diagnostics are planned before the first public release.

## Why SCRCPY Studio is different

Most scrcpy GUIs are configuration panels. SCRCPY Studio is designed around **outcomes and recovery**:

- **Smart Auto-Tune** chooses sensible resolution, FPS, codec, audio, and session behavior based on the selected mode and connection type.
- **Self-Healing Launch** automatically retries safer combinations when a high-quality profile exits immediately: H.265 → H.264 → 1280 max size → 30 FPS.
- **Connection Doctor** turns common ADB states such as `unauthorized` and `offline` into plain-language fixes.
- **Creator Mode** prioritizes tutorial recording: high-quality capture, visible touches, audio where supported, and optional recording.
- **Useful settings only** keeps the main interface approachable while still exposing a compact Advanced panel.

## Current modes

### Mirror Phone
Compatibility-first everyday mirroring and control.

### Creator Mode
1080p-class capture where practical, 60 FPS over USB, visible touches, audio, H.265 when detected, and automatic fallback if needed.

### Camera Mode
Uses scrcpy camera mirroring with conservative 1080p/30 defaults. Requires Android 12+.

### Desktop Mode
Launches scrcpy's virtual-display path with compatibility-first settings. Actual virtual-display behavior depends on the Android version and device implementation.

## Wireless setup

SCRCPY Studio provides visual wrappers around:

- `adb pair HOST:PORT CODE`
- `adb connect HOST:PORT`

No shell is used for user-supplied addresses or codes; arguments are passed directly to the executable.

## Runtime discovery

SCRCPY Studio currently looks for `adb` and `scrcpy` in:

1. the application directory,
2. an adjacent `runtime/` directory,
3. an adjacent `scrcpy/` directory,
4. the operating system `PATH`.

A later milestone will download and verify official runtime releases automatically.

## Development

Requirements:

- Node.js 22+
- Rust toolchain
- Tauri v2 platform prerequisites
- `adb` and `scrcpy` to exercise device workflows

```bash
npm install
npm run check
npm run tauri dev
```

Build:

```bash
npm run build
npm run tauri build
```

Rust tests:

```bash
cd src-tauri
cargo test
```

## Safety and scope

SCRCPY Studio does not root the phone and does not install a persistent Android app. It executes `adb` and `scrcpy` as separate processes with argument arrays rather than passing user values through a shell.

## Roadmap

- [x] Tauri + React application shell
- [x] ADB/scrcpy runtime detection
- [x] USB and wireless device discovery
- [x] Device inspection (Android version, resolution, density)
- [x] H.265 encoder capability probe
- [x] Smart Auto-Tune profiles
- [x] Self-Healing launch fallbacks
- [x] Creator Mode
- [x] Wireless pair/connect UI
- [x] Connection Doctor v1
- [ ] Verified automatic scrcpy/ADB runtime installer
- [ ] Rich launch-failure capture and targeted fixes
- [ ] Camera lens discovery and selection
- [ ] Desktop-mode capability probe
- [ ] Saved per-device preferences
- [ ] Windows installer + portable release artifacts
- [ ] macOS/Linux release validation

## Third-party software

SCRCPY Studio is not affiliated with Genymobile or the scrcpy authors. scrcpy is a separate project licensed under the Apache License 2.0. This repository contains SCRCPY Studio source code; v0.1 does not vendor scrcpy binaries.

## License

SCRCPY Studio is released under the MIT License. See [LICENSE](LICENSE).
