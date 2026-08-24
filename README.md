# SCRCPY Studio

**A clean Windows interface for scrcpy.**

SCRCPY Studio is an independent Tauri + React frontend for the official [Genymobile scrcpy](https://github.com/Genymobile/scrcpy) project. It turns common scrcpy workflows into compact, mode-specific controls while retaining automatic compatibility fallbacks and detailed logs when something fails.

## Download

Download the latest release from [GitHub Releases](https://github.com/techydruid/SCRCPY-Studio/releases/latest).

- **Setup EXE** — recommended for most users; installs for the current Windows user.
- **MSI** — useful for managed or manual Windows deployments.
- **Portable EXE** — runs without installation.

The binaries are currently unsigned, so Microsoft Defender SmartScreen may show an unknown-publisher warning. The release includes `SHA256SUMS-Windows-x64.txt` for download verification.

## Requirements

- Windows 10 or 11, 64-bit
- An Android device with Developer options and USB debugging enabled
- Android 5.0 or newer for screen mirroring
- Android 12 or newer for Camera Mode

SCRCPY Studio can download the latest official Windows scrcpy package and verify its published SHA-256 checksum before installing it to the app's local runtime directory. The downloaded package includes ADB, so users do not need to install ADB or scrcpy manually.

## Modes

### Mirror Phone

Everyday phone mirroring and control with resolution, frame rate, codec, bitrate, audio, orientation, crop, screen-off, awake, fullscreen, recording, screenshot, and media-folder controls. If an aggressive profile exits immediately, SCRCPY Studio retries safer codec, size, and frame-rate combinations.

### Camera Mode

Streams a supported phone camera directly through scrcpy. Camera/lens selection, zoom, torch, camera FPS, audio source, aspect ratio, recording, and high-speed capture are presented only where they apply.

### Desktop Mode

Creates or captures a secondary Android display without claiming that every virtual display is a desktop environment:

- **Virtual Display** is a generic secondary display created by scrcpy `--new-display`.
- **Android Desktop Windowing** is used only when Android exposes the required freeform/windowing behavior.
- **Samsung DeX** is not started by a scrcpy-created display. It is usable only when Samsung firmware already exposes an active DeX display that scrcpy can capture.

Desktop Mode can enable the supported Android freeform/resizable settings in one user-confirmed action, restart the phone, reconnect automatically, and later restore the settings it backed up. Exact commands, results, display details, activities, windowing state, and relevant Android settings are written to Desktop logs instead of cluttering the normal interface.

## Wireless setup

Connect a USB-authorized phone to switch it to wireless ADB automatically. Successful connections are remembered for quick reconnects. A collapsed **Manual setup** section exposes `adb pair` and `adb connect` for new Android 11+ phones or connection recovery.

User-supplied addresses and pairing codes are passed directly as process arguments; they are not interpolated into a shell command.

## Safety and privacy

SCRCPY Studio does not root the phone or install a persistent Android app. Android desktop settings are changed only after the user clicks the enable/restore action. Before changing them, the app saves the current values so they can be restored.

The app runs ADB and scrcpy locally. It accesses the internet only when installing the official scrcpy runtime.

## Development

Requirements:

- Node.js 22+
- Rust stable
- Tauri v2 platform prerequisites

```bash
npm install
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

CI runs frontend checks, Rust tests, Windows GUI-subsystem validation, and production Windows installer builds.

## Third-party software

SCRCPY Studio is not affiliated with Genymobile or the scrcpy authors. scrcpy is a separate project licensed under the Apache License 2.0. SCRCPY Studio downloads scrcpy only from Genymobile's official GitHub release and verifies the release checksum.

## License

SCRCPY Studio is released under the MIT License. See [LICENSE](LICENSE).
