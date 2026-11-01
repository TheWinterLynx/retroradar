# Rust dependency policy v1.0

Owner decision: application code and dependency implementations in Rust, with
SQLite explicitly permitted. RetroRadar contains no application C/C++ sources.
SQLite remains bundled through rusqlite/libsqlite3-sys. Its native build and the
Windows MSVC linker/SDK explain the Build Tools prerequisite; removing ring does
not remove SQLite's compiler requirement. Windows APIs and GPU drivers are host
services; no application can require their internal implementation to be Rust.

## TLS change

ring is the cryptographic implementation originally used by rustls for HTTPS/WSS.
It includes C and assembly. RetroRadar replaces it with the pinned
`rustls-rustcrypto = 0.0.2-alpha`, disables ring/default providers in rustls and
ureq, and passes an explicit RustCrypto provider to every HTTP agent and the
Lightstreamer WebSocket client. No process-global TLS provider is needed.

**Known limitation:** the provider is experimental and its upstream README says
not to use it in production. Passing application tests does not establish a
cryptographic security audit, complete server compatibility or production maturity.
This project is a personal display consuming public data. Do not repurpose this
TLS configuration for a production service or private credentials without a new
review. Sources using unsupported algorithms must report an error, never bypass
verification or silently fall back to ring or unencrypted transport.

Primary references:
- https://github.com/RustCrypto/rustls-rustcrypto (provider and upstream warning)
- https://docs.rs/ureq/3.5.0/ureq/tls/struct.TlsConfigBuilder.html (explicit provider)
- https://docs.rs/rustls/0.23.46/rustls/crypto/struct.CryptoProvider.html

## Regression guards

`scripts/check-rust-deps-v1.0.ps1` inspects Cargo's active Windows dependency tree, including test/build dependencies.
It rejects ring, aws-lc, OpenSSL/native-tls and known native compression backends,
and rejects cc/cmake consumers except libsqlite3-sys -> cc. R01 runs it before tests.
Cargo.lock and cargo metadata can retain unselected optional providers, including
ring. Their presence in that resolution inventory does not mean they are built;
the gate checks activated features using cargo tree.

This checks selected packages/compiler dependencies, not every possible native
source/build mechanism; future dependency additions still need a source review.
Linux's inherited Wayland path has a separate cc dependency and is outside the
Windows runtime policy. No Linux-only workaround is proof of a Windows build.

TLS tests negotiate TLS 1.2 and 1.3 entirely in memory, reject an untrusted root,
reject a wrong hostname, and check HTTP configuration retains verification and an
explicit provider. Tests use a dedicated localhost certificate and test-only key
under `crates/retroradar-sources/fixtures/tls`, never the public root store. The test
certificate was generated for these tests with a long validity; it is not a secret
or an application credential. OpenSSL was only an offline fixture-generation tool;
it is not an application dependency or required to run tests.
