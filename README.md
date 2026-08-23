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
Desktop Mode reports what the phone actually exposes instead of treating every secondary display as a desktop:

- **Virtual Display** creates a generic Android secondary display through scrcpy `--new-display`. It may use a normal phone-style launcher.
- **Android Freeform Windows** means the display accepts movable/resizable tasks, but the OEM does not expose a complete desktop shell. Some apps may initially open maximized and can be restored from their window title bar.
- **Android Desktop Windowing** is reported only when the created display's WindowManager state is actually freeform/desktop.
- **Samsung DeX** is reported only when DeX is already active on an HDMI or Miracast display and Android exposes a display ID that scrcpy can capture. A scrcpy-created virtual display does not itself trigger DeX on current One UI.

Every probe and launch writes a Desktop Diagnostics JSON log containing the exact scrcpy command, exit result and output, display ID/name, resolution/DPI, running activity, observed windowing mode, relevant Android settings, and OEM capability evidence. Developer settings are treated as inputs, never proof of a desktop shell.

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
- [x] Evidence-based Virtual Display / Android Desktop Windowing / Samsung DeX probe
- [ ] Saved per-device preferences
- [ ] Windows installer + portable release artifacts
- [ ] macOS/Linux release validation

## Third-party software

SCRCPY Studio is not affiliated with Genymobile or the scrcpy authors. scrcpy is a separate project licensed under the Apache License 2.0. This repository contains SCRCPY Studio source code; v0.1 does not vendor scrcpy binaries.

## License

SCRCPY Studio is released under the MIT License. See [LICENSE](LICENSE).

