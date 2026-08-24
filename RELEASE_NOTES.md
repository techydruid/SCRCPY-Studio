# SCRCPY Studio v0.1.1

The current stable Windows release of SCRCPY Studio.

## Highlights

- Compact, no-scroll Windows interface
- Mirror Phone with recording, screenshots, audio, orientation, crop, fullscreen, and smart launch fallbacks
- Camera Mode with lens, zoom, torch, audio-source, aspect-ratio, and high-speed controls
- Desktop Mode that distinguishes Virtual Display, Android Desktop Windowing, and accessible Samsung DeX displays
- One-click, user-confirmed Android desktop-windowing setup, reboot/reconnect, backup, and restore
- USB-to-wireless switching, saved phones, and collapsed manual pair/connect recovery
- Verified download of Genymobile's latest official Windows scrcpy runtime
- Detailed local Desktop logs without exposing technical diagnostics in the normal interface

## Downloads

- `SCRCPY-Studio-0.1.1-Windows-x64-Setup.exe` — recommended installer
- `SCRCPY-Studio-0.1.1-Windows-x64.msi` — MSI installer
- `SCRCPY-Studio-0.1.1-Windows-x64-Portable.exe` — portable app
- `SHA256SUMS-Windows-x64.txt` — SHA-256 checksums for the Windows downloads

## Requirements

- Windows 10 or 11, 64-bit
- Android 5.0+ for phone mirroring
- Android 12+ for Camera Mode
- Developer options and USB debugging enabled on the Android device

## Important notes

- Windows binaries are unsigned and may display an unknown-publisher or SmartScreen warning.
- A scrcpy virtual display does not automatically start Samsung DeX. Desktop behavior depends on the Android firmware and windowing environment actually exposed by the device.
- SCRCPY Studio does not root the device or install a persistent Android app.
