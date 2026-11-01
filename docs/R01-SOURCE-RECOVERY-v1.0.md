# R01 external source recovery v1.0

## Observed failures

The Windows binary compiled and ran. A Horizons request received HTTP 503, followed
by sky cutout connection timeouts for multiple Hubble programs. These are source
availability/connection failures rather than compiler failures. The existing
trajectory supervisor already retries with exponential backoff and retains cached
data. A successful query from another location/time is not proof of Windows
connectivity or uninterrupted external-service availability.

## Changes

- Built-in CDS cutouts try the documented second HTTPS endpoint with the exact
  same survey, coordinates, projection, field of view and image size. Custom
  cutout URLs remain exclusive; their coordinates are not forwarded to CDS.
- A transport/read failure, HTTP 429 or HTTP 5xx pauses requests to that sky
  provider origin for 60 seconds across different targets. Other providers and
  validated cache hits remain available; eligible mirrors are tried immediately.
- Empty bodies and invalid JPEG signatures are rejected before image caching.
  Invalid images left in an older cache are ignored. JPEG signature checks do
  not replace the renderer's full image decoding.
- Offline sky requests return cached data or an unavailable state without HTTP.
- Horizons network operations share a process-wide request slot, released before
  backoff/parsing, so vehicle workers do not issue simultaneous JPL requests.
  Other trajectory providers retain independent workers.
- The default User-Agent now reports the current 0.1.1 version. Explicit user
  configuration overrides remain respected.

This adds no dependency and does not change orbital calculations, TLS certificate
checks, the requested ephemeris range or the honesty labels for unavailable data.
If both CDS endpoints are unreachable, the application still reports unavailable
images. A persistent JPL 503 remains subject to the existing source backoff.

## Verification

Local deterministic tests exercise HTTP 503 mirror recovery, reuse of validated
cache data, invalid-image rejection, endpoint cooldown and subsequent recovery,
offline cache hits/misses without connections, serialized Horizons callbacks and
independence of other providers. Normal gates use loopback fixtures, not public
services. R01 runs focused source tests before the workspace suite.

Manual Windows check: use the normal online profile, inspect JWST source health
and Hubble field pictures. Then use --offline with the same profile and confirm
cached pictures work without connection attempts. Public outages may still keep
uncached pictures or ephemerides unavailable.

## Primary references

- https://alasky.cds.unistra.fr/hips-image-services/hips2fits — two independent CDS endpoints.
- https://ssd-api.jpl.nasa.gov/ — one request at a time, backoff and application User-Agent policy.
- https://ssd-api.jpl.nasa.gov/doc/horizons.html — official request format and 503 interpretation.

Agent validation (Linux, Rust 1.95.0): formatting and workspace regression tests
with RUSTFLAGS=-Dwarnings passed (345 passed, 0 failed, 6 ignored). Cargo.lock is
unchanged. The exact JWST URL from the report returned HTTP 200 with a complete
ephemeris during a read-only probe. Both documented CDS cutout routes returned
HTTP 503 with upstream-connect timeout text in this environment. These probes
characterize that route/time, not all users' connectivity. Windows runtime and
real-source recovery remain owner acceptance checks.

## Windows loopback fixture close

The R01 Windows gate reported Winsock 10054 while reading the local fixture's
`ready` response in
`endpoint_failures_pause_other_targets_and_can_retry_after_the_pause`.
This test uses 127.0.0.1 and does not query CDS or JPL.

The first teardown-only fix passed Linux tests but still failed on Windows:
accepted sockets inherited nonblocking mode from the listener, producing Winsock
10035 during header reads and close handling. Timeouts do not restore blocking
mode. The server now explicitly calls `set_nonblocking(false)` immediately after
accepting each connection, before setting timeouts or reading/writing. It owns a
single socket, borrows it while reading request headers, writes the complete
response, shuts down its send direction and waits for the peer close before
dropping it. Both read/write
operations are bounded by timeouts. A peer may cancel a non-success response;
client-side response and cooldown assertions remain strict.

Regression coverage checks fragmented request headers, small/large/empty replies,
HTTP 200 and 503, early peer half-close and a peer waiting for the response EOF.
Every response must arrive intact followed by clean EOF. Existing cooldown tests
still require the second target to be paused and the third to receive `ready`.

All changes are under `cfg(test)`; production source fetching, retry policies,
URLs, cache and TLS behaviour are unchanged. The existing `Sky service recovery`
gate step automatically runs the new case. Windows acceptance remains pending
the owner's rerun.

Loopback-fix validation on Linux / Rust 1.95.0: all 12 sky tests passed in 20
consecutive runs (240 checks); the workspace passed with `-Dwarnings` (354
passed, 0 failed, 8 ignored). Formatting passed and Cargo.lock is unchanged.
This does not claim reproduction or acceptance on Windows.

A socket-mode regression explicitly puts an accepted connection in nonblocking
mode on every platform, invokes the same fixture configuration and verifies that
an empty read waits for the configured timeout rather than failing immediately.
An intentional negative control (keeping nonblocking mode) fails this regression
on Linux; restoring blocking mode passes. This covers the platform difference
that the previous Linux-only runs missed.

Reference: [Microsoft Winsock accept properties](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-accept).

Socket-mode-fix validation on Linux / Rust 1.95.0: 20 consecutive runs of all
13 sky tests passed (260 checks), including the forced-nonblocking regression.
The workspace passed with `-Dwarnings`: 355 passed, 0 failed, 8 ignored.
Formatting passed and Cargo.lock is unchanged. Windows acceptance is pending.
