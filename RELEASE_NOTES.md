# SCRCPY Studio v0.1.0

The first public Windows release of SCRCPY Studio.

## Highlights

- Compact, no-scroll Windows interface with mode-specific settings
- Mirror Phone with recording, screenshots, audio, orientation, crop, fullscreen, and smart launch fallbacks
- Camera Mode with lens, zoom, torch, audio-source, aspect-ratio, and high-speed controls
- Desktop Mode that distinguishes Virtual Display, Android Desktop Windowing, and accessible Samsung DeX displays
- One-click, user-confirmed Android desktop-windowing setup, reboot/reconnect, backup, and restore
- USB-to-wireless switching, saved phones, and collapsed manual pair/connect recovery
- Automatic download and SHA-256 verification of the latest official Windows scrcpy runtime
- Hidden Windows console processes and a taskbar-safe startup window
- Detailed local Desktop logs without exposing technical diagnostics in the normal interface

## Windows downloads

- `SCRCPY-Studio-0.1.0-Windows-x64-Setup.exe` — recommended installer
- `SCRCPY-Studio-0.1.0-Windows-x64.msi` — MSI installer
- `SCRCPY-Studio-0.1.0-Windows-x64-Portable.exe` — portable app
- `SHA256SUMS.txt` — checksums for all Windows binaries

## Requirements

- Windows 10 or 11, 64-bit
- Android 5.0+ for phone mirroring
- Android 12+ for Camera Mode
- Developer options and USB debugging enabled on the Android device

## Important notes

- These first-release binaries are unsigned. Windows may display an unknown-publisher or SmartScreen warning.
- A scrcpy virtual display does not automatically start Samsung DeX. Desktop behavior depends on the Android firmware and the windowing environment actually exposed by the device.
- SCRCPY Studio does not root the device or install a persistent Android app.
