# macOS Port and Build Notes

This note records the macOS support added to SCRCPY Studio, how to build it, what was verified, and the source files changed.

## Platform behavior

SCRCPY Studio remains a Tauri 2 + React application. The macOS app uses Homebrew-installed `scrcpy` and ADB; it does not bundle third-party executables or install packages into the Homebrew prefix. At runtime, the backend checks the standard Homebrew and MacPorts binary locations (`/opt/homebrew/bin`, `/usr/local/bin`, and `/opt/local/bin`). Subprocesses also receive a `PATH` containing common Homebrew `bin` and `sbin` locations, followed by the inherited path.

If either tool is missing, the UI displays the appropriate Homebrew command and offers **Check again** rather than presenting the Windows runtime installer. Users can install `scrcpy` with `brew install scrcpy`; if ADB is not available, install Android Platform Tools with `brew install android-platform-tools`.

The app bundle and frontend were built for Apple Silicon (`aarch64-apple-darwin`). The Intel Homebrew search path is implemented, but an Intel build was not produced during this verification.

## Prerequisites

- macOS with Apple's Command Line Tools and macOS SDK available
- Node.js 22 or newer
- Rust stable and Cargo
- Tauri v2 platform prerequisites
- Homebrew plus `scrcpy` and ADB for device operation

## Build

From the repository root:

```bash
npm install
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --bundles app
```

The app bundle is written to `src-tauri/target/release/bundle/macos/SCRCPY Studio.app`. On a normal developer Mac, build the app and disk image together with:

```bash
npm run tauri build -- --bundles app,dmg
```

The build documented here ran on an Apple Silicon MacBook Air (`MacBookAir10,1`) with macOS 27.0.1, Node.js 24.12.0, npm 11.15.0, and Rust 1.99.1 (`aarch64-apple-darwin`). Apple's Command Line Tools were active; full Xcode was not installed. The app bundle target completed successfully with these tools.

The `.app` built in this environment was then ad-hoc signed for local validation and zipped as `SCRCPY Studio for macOS.zip`. It is not Developer ID signed or notarized; release distribution should use the project's approved signing and notarization process. A DMG build was attempted, but this environment's local execution policy denied `hdiutil` access to the disk-image device. This is an environment limitation, not a reported app build error.

## Icon decoding and startup fix

The original PNG opened through macOS imaging tools but was rejected by Tauri's PNG decoder (`Unknown filter method`). The resulting default runtime icon had inconsistent dimensions, causing Tauri to abort during startup with an invalid-icon panic. The image was re-encoded as a standard PNG, the platform icons were regenerated with Tauri's icon generator, and the PNG was listed first in the bundle icon configuration. The regenerated PNG also passes Tauri's own `tauri icon` decode path.

To regenerate platform icons from a clean source PNG:

```bash
sips -s format png src-tauri/icons/icon.png --out /tmp/scrcpy-studio-icon.png
npm run tauri -- icon /tmp/scrcpy-studio-icon.png --output src-tauri/icons
```

## Validation performed

- `npm run check` — passed.
- `npm run build` — passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` — 42 passed, 0 failed.
- `npm run tauri build -- --bundles app` — passed and produced the arm64 `.app` bundle.
- The repaired app launched and remained running for more than 10 seconds; no new crash report was produced during that check.
- `codesign --verify --deep --strict` — passed for the locally ad-hoc-signed app and again after extracting the ZIP.
- No physical Android device was attached for end-to-end mirroring, camera, or wireless testing.

## File-by-file changes

- `README.md` — describes macOS support, Homebrew dependencies, runtime behavior, and local build steps.
- `docs/macos-build.md` — records the build environment, commands, validation, known constraints, and this file inventory.
- `src/App.tsx` — detects macOS, explains missing `scrcpy`/ADB dependencies, and changes the runtime action to **Check again**.
- `src/styles.css` — adds system light appearance and adjusts desktop label sizing.
- `src-tauri/src/runtime.rs` — searches standard Apple Silicon, Intel Homebrew, and MacPorts paths; reports macOS-specific install guidance.
- `src-tauri/src/commands.rs` — gives subprocesses a Homebrew-aware `PATH`.
- `src-tauri/tauri.conf.json` — uses a larger resizable window, registers the macOS icon, and updates the app description.
- `src-tauri/icons/icon.png`, `icon.ico`, and `icon.icns` — replaces the malformed artwork with valid Tauri-generated platform assets.
- `src-tauri/Cargo.lock` — locks the Rust dependency resolution used for the app build.

Tauri generates `src-tauri/gen/schemas/` during local builds; these reproducible generated files are intentionally omitted from the pull request.

The `.app` and its ZIP are build outputs, not source files intended for this pull request.
