#!/usr/bin/env bash
# Regenerate THIRD-PARTY-NOTICES.md: every library's licence text (cargo-about) plus the embedded assets.
# Needs `cargo install cargo-about --locked --features cli`. Run after changing dependencies.
set -euo pipefail
cd "$(dirname "$0")/.."
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
cargo about generate --locked --fail about.hbs > "$tmp"
{
    cat "$tmp"
    cat <<'ASSETS'
## Embedded assets

| Asset | Source | Licence |
|---|---|---|
| Earth day texture: Blue Marble Next Generation with topography and bathymetry, 2004 (R. Stöckli) | NASA Earth Observatory, https://visibleearth.nasa.gov | NASA media usage guidelines: not subject to copyright in the United States for this use; no endorsement is implied |
| Earth night lights: Black Marble 2012 (Suomi NPP VIIRS) | NASA Earth Observatory / NOAA NGDC, https://visibleearth.nasa.gov | NASA media usage guidelines, as above |
| Coastlines: Natural Earth 110m and 50m | https://www.naturalearthdata.com | Public domain |
| Font: Unscii-16 | http://viznut.fi/unscii/ | Public domain, per its author |
| Font: JetBrains Mono | https://github.com/JetBrains/JetBrainsMono | SIL Open Font License 1.1 (below) |

Regional imagery downloaded on demand: EOxCloudless https://cloudless.eox.at by EOX IT Services GmbH
(Contains modified Copernicus Sentinel data 2016), CC BY 4.0:
https://creativecommons.org/licenses/by/4.0/. Mapping/filtering is an adaptation.
The same licence applies to the offline Madrid test tile; exact source URLs and
checksums are in assets/earth/regional-source-v1.0.json.

The Earth day imagery includes 4096×2048, 8192×4096 and 16384×8192 versions;
night imagery includes 2048×1024, 4096×2048 and 8192×4096 versions. Higher
resolutions were downsampled from NASA originals, without generated detail.
Source URLs, dimensions and checksums are in `assets/earth/hd-textures-v1.0.json`.
The same information, with how each asset was checked, is in `assets/LICENSES.md`.

### SIL Open Font License 1.1 (JetBrains Mono)

```text
ASSETS
    cat assets/fonts/OFL-JetBrainsMono.txt
    printf '```\n'
} > THIRD-PARTY-NOTICES.md
echo "wrote THIRD-PARTY-NOTICES.md ($(wc -c < THIRD-PARTY-NOTICES.md) bytes)"
