# Contributing

SCRCPY Studio favors simple workflows over exposing every scrcpy flag.

Before proposing a feature, ask whether it solves a common user outcome, improves automatic decision-making, or provides a clearer recovery path. Rare low-level options generally belong in Advanced settings rather than the primary interface.

## Development checks

```bash
npm install
npm run check
npm run build
cd src-tauri && cargo test
```

Please do not bundle third-party binaries without documenting their source, version, license, and integrity-verification method.
