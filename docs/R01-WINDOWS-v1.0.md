# R01 Windows adaptation v1.0

Changes relative to upstream bf7072fdf1080fec6890a9889531e78297e491eb:

| File | Change |
|---|---|
| Cargo.toml / Cargo.lock | Public package retroradar, same pinned dependency graph |
| src/main.rs | Portable root initialization before workers; read-only doctor; desktop kiosk; terminal logging |
| mctl-core/src/config.rs | Process-wide immutable portable root; application directories isolated from mctl |
| mctl-gui/src/app.rs | X11/WSL code compiled only on Linux; RetroRadar window title |
| mctl-sources/Cargo.toml, src/lib.rs, src/observer.rs | D-Bus dependency/module isolated to Linux; explicit unsupported-location note elsewhere |
| mctl-gui/src/about.rs, src/screens.rs | Preserve attribution; replace Linux-only user path advice |
| tests/cli.rs, tests/portable.rs | Executable identity; isolated read-only doctor and invalid configuration regressions |
| scripts/gate-r01-v1.0.ps1 | Fail-fast Windows acceptance with visible progress and unique temporary data root |
| .github/workflows/rustfmt.yml | Formatting only; upstream CI/release workflows excluded |

The portable root is set once before configuration or background workers start;
there is no unsafe process environment mutation. Without an explicit root, the
`directories` crate uses native platform locations. The original internal crate
names and upstream source versions remain unchanged to keep the delta reviewable.

Kiosk reuses the existing scene renderer. It does not yet provide a new desktop
layout or editable settings. GeoClue is skipped on Windows; manual sites and the
existing optional IP adapter retain the upstream resolution order.

Tests for doctor use child processes and unique paths. They assert resolved paths,
no filesystem writes, and rejection of invalid TOML. Windows runtime/GPU and real
source connectivity require owner acceptance independently of Linux tests.

## Agent validation (2026-11-01)

Linux x86_64, Rust 1.95.0: `cargo fmt --all -- --check` passed;
`RUSTFLAGS=-Dwarnings cargo test --locked --workspace --all-targets` passed
(332 passed, 0 failed, 6 ignored); release binary build passed with the same warning
policy. Portable doctor passed without creating its data root. Dependency lock
versions and checksums match upstream; only the root package name changed.

Windows MSVC build, PowerShell gate execution, Windows GPU smoke, all-page visual
acceptance and real-source connectivity have not been run here. They remain owner
gates; Linux success is not Windows acceptance. Inherited ignored tests remain
ignored because they require external fixtures, GeoClue or rendering acceptance.
