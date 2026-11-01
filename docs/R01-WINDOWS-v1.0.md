# R01 Windows adaptation v1.0

RetroRadar provides a Windows-native Rust console with orbital pages, telemetry,
pass predictions, recording/replay, desktop kiosk and portable storage.

| Area | Implementation |
|---|---|
| Identity | retroradar executable, retroradar-* crates, title, helper names and app directories |
| Storage | Immutable portable root initialized before workers; native profile paths otherwise |
| Window | winit native Windows window; X11/WSL isolated to Linux |
| Location | Configured sites on Windows; GeoClue/D-Bus isolated to Linux |
| Diagnostics | Terminal output; read-only doctor; no application log files |
| TLS | Explicit RustCrypto providers for HTTP/WSS; certificate/name verification enabled |
| Native exception | SQLite only in the active Windows compiler-dependency tree |
| Acceptance | Local PowerShell gate; GitHub Actions exclusively rustfmt |

## Windows path regression

The first Windows gate exposed a test defect: it compared a printed native path
against a string containing a literal forward slash in `config/config.toml`.
The application reported the correct Windows config directory. Tests now construct
path components separately and parse reported paths as Path values; config, cache,
data and no-write assertions remain enabled. The invalid-TOML fixture uses native
component joins as well.

## Validation

The workspace regression suite passes on Linux x86_64 with Rust 1.95.0 and
RUSTFLAGS=-Dwarnings: 336 passed, 0 failed, 6 normally ignored. Formatting and release
build are checked for the current revision. The active Windows dependency tree
contains no ring/aws-lc/OpenSSL; SQLite is the only cc consumer.
Windows owner acceptance remains pending after the reported regression.
Linux checks do not establish Windows GPU or PowerShell acceptance.

See RUST-DEPENDENCIES-v1.0.md for the alpha TLS provider's production limitation,
ROADMAP.md for planned functionality, and ../NOTICE.md for legal notices.

The six rendered regression references were regenerated with RetroRadar branding,
visually inspected for the About/Credits/kiosk identity, and compared again without
reference regeneration. This headless Linux rendering check is separate from the
owner's Windows window/GPU gate. The local gate now runs portable-path regressions
before the full workspace to surface this failure directly.
