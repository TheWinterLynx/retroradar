# RetroRadar engineering contract

## Product
Windows-native Rust mission-control display, with orbital pages, themes,
orbital engine, sources and recording. Retain GPL-3.0-or-later, upstream credits,
asset licences and provenance. This is an independent repository, never a GitHub fork.
Missing values remain NO DATA; preserve LIVE/SIM/REPLAY and source-quality labels.
No network or propagation work on the rendering thread. No invented telemetry.

## Language and dependencies
Owner requirement: application code and third-party implementations must be Rust.
SQLite (libsqlite3-sys through rusqlite) is the explicitly approved native-code
exception. Do not introduce ring, aws-lc, OpenSSL or another C/C++ implementation.
Windows OS APIs, drivers and the platform linker are host services, not application
implementations. SQLite still needs a C compiler at build time.
Use the explicit RustCrypto TLS provider for HTTP and WebSocket connections;
never disable certificate/hostname checks. The provider is upstream experimental,
not production-ready: preserve this documented limitation. Run the Windows
dependency-policy gate after manifest/lock changes. Its compiler-consumer and
package checks are regression guards, not a universal source-language proof.

## Workflow
- main is stable. Work on one focused `agent/<milestone>` branch from current main.
- Push code directly to this repository; do not open PRs unless requested.
- Owner runs Windows gates locally. Report exact branch and commit and give one-line
  PowerShell commands, with prerequisites first and fail-fast checks.
- Do not merge a feature branch before owner local validation.
- Owner reporting `<milestone> FULL GATE GREEN` authorizes merging that exact tested
  head into main and continuing the next focused slice without another confirmation.
- GitHub Actions is permitted ONLY for rustfmt. Never build, test, release, deploy
  or benchmark in Actions. Do not copy upstream CI/release workflows.
- No log files. Show progress and diagnostics on the terminal.
- Source, focused regression coverage and companion docs travel together.
- Stop at the first failed gate. Never claim checks ran when they did not.
- Windows GPU/window acceptance must be reported separately from Linux checks.
- No subagents unless the owner explicitly requests delegation.

## Gate order
Clean checkout -> toolchain/Windows MSVC prerequisites -> fmt -> offline regressions
with RUSTFLAGS=-Dwarnings -> release binary -> doctor -> local GPU window smoke.
Network source checks are separate: an external outage is not a compiler regression.
Do not reset, stash or discard owner edits automatically.

## Architecture
Crate names, executable and application directories use retroradar.
Refer to NOTICE.md and
 docs/ROADMAP.md before adding functionality. Update changed-file documentation.
