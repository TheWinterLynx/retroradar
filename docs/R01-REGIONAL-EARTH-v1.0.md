# R01 regional Earth imagery v1.0

The desktop now loads geographic imagery for the nearby area as zoom increases.
It retains the embedded NASA day/night maps as the global fallback. This is a
Rust implementation using the existing JPEG/PNG, HTTPS and GPU dependencies.
No new crate or native implementation is introduced.

## Behaviour

- Globe camera clearance can reach 10 km above the reference sphere, instead of
  stopping at roughly 1600 km. Projection near distance follows clearance.
- VIEW wheel and +/- change optical field of view from 75° to approximately 0.879° (1×–100×).
  Vehicle state, actual altitude and simulation speed are unaffected by zoom.
- Drag/arrows slow with surface clearance; VIEW free-look slows with its FOV. Drag also follows viewport height so
  the same fraction of the panel gives the same motion at 1080p and 4K.
  Space pauses the clock for inspecting a location without the spacecraft moving.
- Nearby 3D Earth panels request an 8×8 WGS84 tile patch at an appropriate level,
  capped at level 13 (~9.55 m per texel at the equator). Different latitudes,
  viewport sizes and view directions select different detail levels.
- Tiles appear progressively, centre first. The embedded map remains visible in
  missing/transparent tiles and outside the patch. Credits and loaded tile count
  appear over the view. Optical zoom has its own indicator in VIEW.
- Initial downloads require connectivity and may take seconds. New view requests
  supersede old work; one current result is retained, without an unbounded queue.
- `--offline` and startup `--replay` only read cached tiles. `--no-earth-detail` disables the
  desktop tile controller. Headless shot/bench retain global imagery for repeatable
  capture/measurement; the GPU regression injects a fixed, local regional fixture.

## Data and limits

Provider: EOX Sentinel-2 cloudless **2016**, WGS84 / EPSG:4326.
It is a historical daytime composite, not live imagery. Published source detail
is about 10 m; advertised levels above 13 do not add native source detail, so they
are not requested. Night lights still come from the global Black Marble image.
No terrain elevation model is added: the renderer retains its reference sphere,
not survey-grade ellipsoidal geolocation or terrain relief. Native raster spacing
must not be confused with positional accuracy or unlimited zoom resolution.
Coverage gaps, water/no-data tiles and provider outages retain the global map.

The service capabilities advertise 256×256 pixels, matrix width 2^(z+1), height
2^z, north-west origin, and row before column in the resource URL. Requests use
HTTPS even though the capabilities' resource template advertises HTTP:

`https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless/default/WGS84/{z}/{row}/{col}.jpg`

The service can return transparent PNG no-data tiles at a `.jpg` URL; the decoder
recognises JPEG and PNG by content and checks 256×256 dimensions before decoding.

Licence: **CC BY 4.0** for the pinned 2016 layer. Required attribution is displayed
in the view, with the licence URL. The newer non-commercial layer is not selected.
Rendering/resampling on the reference sphere is an adaptation. Product source,
layout and offline fixture provenance: `assets/earth/regional-source-v1.0.json`.

References:

- https://tiles.maps.eox.at/wmts/1.0.0/WMTSCapabilities.xml
- https://cloudless.eox.at/documentation/usage
- https://cloudless.eox.at/license-non-commercial (2016 CC BY 4.0 exception)
- https://new.eox.at/2017/03/sentinel-2-cloudless-original-tiles-available/ (native resolution)
- https://creativecommons.org/licenses/by/4.0/

## Resources and failure handling

Network, filesystem and image decoding run on a dedicated worker. The render
thread only selects a geographic patch, polls a completed image and uploads it.
At most one HTTP request runs at a time, at most four per second; responses are
limited to 512 KiB, with an eight-second request timeout and 60-second provider
cooldown after a failure. Cached tiles bypass the network/cooldown. Invalid image
content is removed from cache and falls back to the global image.

The versioned cache is under `cache/earth-eox-s2-2016-v1`. Oldest files are trimmed
periodically to a 256 MiB budget (up to 16 small writes can accumulate between
trims). The regional GPU texture is 2048×2048 RGBA, an additional 16 MiB beyond
the selected global texture tier. CPU patch/results are bounded; obsolete work
is cancelled between tile requests. A request already in flight can finish within
its timeout; shutdown does not wait on it on the UI thread.

A moving spacecraft can outrun uncached high-resolution downloads at maximum
VIEW zoom; pause for inspection or use previously cached areas. Partial loading
is labelled rather than presented as complete high-resolution coverage.

## Acceptance

The R01 gate covers coordinate ranges, dateline wrapping, pole row limits,
offline/cache handling, a real cached tile decoded on the worker, closer camera
projection, independent VIEW zoom, and GPU patch placement/transparent fallback.
The GPU test uses one bundled CC BY 4.0 Madrid tile, with provenance; no public
network is used by the normal gate. Test images are under `target/earth-detail-v1.0`.

The ignored `live_eox_native_tile_is_available_over_verified_tls` probe is separate
from the gate. Its optional `RETRORADAR_TEST_CA` PEM trust root applies only to that
explicit diagnostic test, for a runner behind an intercepting proxy; production
clients retain Mozilla roots and hostname/certificate checks.

Windows visual acceptance: run the R01 gate, then start the normal desktop profile
with `--earth-quality high --no-record`. In Modern, zoom the globe and VIEW/NADIR,
verify the DETAIL counter and source credit, pause with Space, wait for tiles,
then pan/zoom and inspect the newly loaded terrain. Restart `--offline` on the same
profile and confirm cached areas remain available. A fresh offline area must show
the global fallback. Compare `--no-earth-detail` at the same camera position.

Agent validation (Linux, Rust 1.95.0): 364 normal tests passed, 10 ignored; three
explicit GPU regressions passed (existing goldens, close coasts and regional
placement/no-data fallback). Release build and no-write doctor passed. A public
native tile returned HTTP 200 and decoded as 256×256 JPEG. The Rust download
probe passed through the runner's HTTP proxy with its explicitly trusted test CA;
the runner's default proxy/trust setup could not complete that probe. Production
TLS trust was not relaxed. Formatting passed; Cargo.lock is unchanged. Windows
GPU and online-source acceptance remain owner checks.

## Night-light exposure at close zoom

Artificial-light gain now follows the night texture's screen footprint, in every
Earth view and theme. Overview gain is unchanged when a night-map texel is at
most one screen pixel wide. As the texel expands from one to eight screen pixels,
gain smoothly falls to 2.5% of the overview value. This is display exposure, not
a change to the historical light data, physical Sun illumination or the terrain.
It acts before bloom, so magnified lights cannot flood the regional image.

A deterministic GPU regression renders the Madrid fixture at maximum night zoom
with each built-in theme's bloom. It checks the urban area's peak brightness and
that regional texture contrast remains visible. Before the fix it fails with
255/255 saturation. The graphical gate runs it; screenshots are written under
`target/night-zoom-v1.0`. Windows visual acceptance remains an owner check.

Night-exposure validation (Linux / Rust 1.95.0): 364 normal tests passed, 11
ignored; all four explicit GPU regressions passed. The original shader failed
the new night test at 255/255. After the fix, measured urban-area peaks with bloom
were 82/255 (Phosphor, Retro), 55/255 (Modern), 68/255 (Retrowave), with regional
contrast retained in every theme. Formatting/release passed; Cargo.lock unchanged.
