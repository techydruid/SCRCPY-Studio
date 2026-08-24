# SCRCPY Studio v0.1.1

The first cross-platform release of SCRCPY Studio, adding native Linux x86_64 packages while preserving the Windows interface and behavior from v0.1.0.

## Highlights

- Same compact, no-scroll interface on Windows and Linux
- Mirror Phone with recording, screenshots, audio, orientation, crop, fullscreen, and smart launch fallbacks
- Camera Mode with lens, zoom, torch, audio-source, aspect-ratio, and high-speed controls
- Desktop Mode that distinguishes Virtual Display, Android Desktop Windowing, and accessible Samsung DeX displays
- One-click, user-confirmed Android desktop-windowing setup, reboot/reconnect, backup, and restore
- USB-to-wireless switching, saved phones, and collapsed manual pair/connect recovery
- Verified download of Genymobile's latest official scrcpy runtime on Windows and Linux x86_64
- Detailed local Desktop logs without exposing technical diagnostics in the normal interface

## Downloads

Windows x64:

- `SCRCPY-Studio-0.1.1-Windows-x64-Setup.exe` — recommended installer
- `SCRCPY-Studio-0.1.1-Windows-x64.msi` — MSI installer
- `SCRCPY-Studio-0.1.1-Windows-x64-Portable.exe` — portable app

Linux x86_64:

- `SCRCPY-Studio-0.1.1-Linux-x64.AppImage` — portable package for most distributions
- `SCRCPY-Studio-0.1.1-Linux-x64.deb` — Debian/Ubuntu package
- `SCRCPY-Studio-0.1.1-Linux-x64.rpm` — Fedora/RHEL-family package

Each platform includes its own SHA-256 checksum file.

## Requirements

- Windows 10/11 64-bit, or a modern x86_64 Linux distribution with WebKitGTK 4.1
- Android 5.0+ for phone mirroring
- Android 12+ for Camera Mode
- Developer options and USB debugging enabled on the Android device
- Linux only: ADB from the distribution package manager (`adb` on Debian/Ubuntu or `android-tools` on Fedora) and `curl` for automatic scrcpy installation

## Important notes

- Windows binaries are unsigned and may display an unknown-publisher or SmartScreen warning.
- A scrcpy virtual display does not automatically start Samsung DeX. Desktop behavior depends on the Android firmware and windowing environment actually exposed by the device.
- SCRCPY Studio does not root the device or install a persistent Android app.
