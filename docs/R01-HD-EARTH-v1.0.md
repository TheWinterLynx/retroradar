# R01 HD Earth v1.0

Real higher-resolution static Earth textures, embedded in the binary. No new
runtime dependencies or downloads. Acceptance on the owner's Windows GPU is pending.

## Quality and memory

| Startup option | Day | Night | Coast mask | Approximate texture memory, including mipmaps |
|---|---|---|---|---|
| `--earth-quality low` | 4096×2048 | 2048×1024 | 4096×2048 | 64 MiB |
| `--earth-quality medium` | 8192×4096 | 4096×2048 | 8192×4096 | 256 MiB |
| `--earth-quality high` | 16384×8192 | 8192×4096 | 8192×4096 | 896 MiB |
| `--earth-quality auto` (default) | Up to 8192×4096 | Up to 4096×2048 | Up to 8192×4096 | Up to 256 MiB |

Memory excludes framebuffers, staging allocations and the rest of the app.
Auto is conservative: supported dimensions do not indicate free VRAM. If high
runs out of memory, restart with medium or low. Quality changes require restart.
GPU dimension limits reduce all texture tiers consistently; CPU adapters are
capped at 2048×1024 day / 1024×512 night / 2048×1024 coast. Effective dimensions
appear at startup in the terminal and in SYS settings. No image is upscaled.

The wgpu device requests supported 2D dimensions up to 16384. Textures retain
mipmaps for distant views; uploads use strips of at most 16 MiB to avoid a
single 512 MiB staging write for the 16K RGBA base level. Natural Earth coastline
geometry is unchanged; higher mask resolution only improves its rasterisation.

## Sources and preparation

Day: NASA Blue Marble Next Generation, December 2004, topography and bathymetry,
[21600×10800 original](https://science.nasa.gov/earth/earth-observatory/blue-marble-next-generation/base-topography-bathymetry/).
Night: NASA/NOAA Black Marble 2012, Suomi NPP VIIRS,
[13500×6750 original](https://science.nasa.gov/earth/earth-observatory/earth-at-night/maps/).
These are historical global composites, not live imagery or street-map tiles.

Lanczos downsampling with Pillow 12.3.0 produces the bundled sizes; JPEG quality
is 78 for day 16K and 88 for the other new files, chroma subsampling 2, optimized.
No image generation or enlargement. The manifest
`assets/earth/hd-textures-v1.0.json` records exact original URLs, source dimensions,
source SHA-256 and output dimensions, SHA-256, size and encoding quality.
Credits and usage terms remain in `assets/LICENSES.md` and the embedded notices.

## Local Windows acceptance

Close running RetroRadar processes before updating/building. Run
`scripts/gate-r01-v1.0.ps1` from a clean checkout. It explicitly decodes both
largest JPEGs, checks dimensions and quality fallback, runs all regressions,
builds release, runs doctor, checks close-view coast fading on the GPU and starts
a 120-frame offline window with high quality.
External services are not required. With no cached orbit, NO DATA is expected.
`-SkipGpu` leaves graphical acceptance pending.

Then run `target\release\retroradar.exe --earth-quality high --no-record`.
Check the effective DAY/NIGHT/COAST dimensions in terminal/SYS, zoom the globe
in Modern and inspect day/night coast alignment, terminator, all themes, clipping,
resize and camera motion. Compare with `--earth-quality low` at the same camera.
A 16K texture increases detail when zoomed; it does not change window resolution
or guarantee a frame-rate increase. Report the commit with R01 FULL GATE GREEN
only after the local GPU and visual review succeeds. No merge before that report.

## Close-view follow-up

VIEW/NADIR and orbit-map panels share the same globe pass and textures. There is
no separate low-resolution VIEW texture. The cartographic coastline mask fades
under magnification: fully visible at one or more mask texels per screen pixel,
invisible when a mask texel covers four or more screen pixels. Day/night imagery
is unchanged. Night 8K has roughly 4.9 km per texel at the equator; low-orbit
views inevitably magnify that finite detail.

Manual drag and arrows use zoom-sensitive angular steps for both focus modes.
Earth sensitivity follows surface clearance, vehicle sensitivity follows distance
to the vehicle, capped at the normal overview speed. Fixed-FOV VIEW free-look,
programmed kiosk drift and physical orbital motion retain their existing rates.

The graphical gate renders a fixed test orbit over the Aegean, changes the coast
colour and compares pixels: the overlay must remain visible in overview and fade
in NADIR. Test screenshots are kept under `target/earth-close-v1.0`; they use a
test-only orbit, never a live feed.
