# RetroRadar usage v1.0

Launch `retroradar.exe` for the interactive console. `--help` lists all commands.
Use `--data-dir <folder>` for portable config/cache/recording and `--no-record`
to disable recording. `--offline` uses cached data; unavailable data stays NO DATA.

| Keys | Function |
|---|---|
| F1–F11 | MASTER, TRAJ, GNC, EPS, TCS/ECLSS, COMM, PASS, PROX, VIEW, SYS, OPS |
| Tab / Shift-Tab | Next / previous subpage |
| T / V | Theme / vehicle |
| Space / [ / ] / 0 | Pause / reduce warp / increase warp / live time |
| Mouse drag / wheel | Rotate / zoom the orbit map; rotation slows as you zoom in |
| ? / Q | Help / quit |
| : | Console commands; Tab completes |

`--kiosk --theme retrowave` cycles vehicle scenes at desktop resolution.
`doctor` validates local config and prints paths without writing or using network/GPU.
`check` probes external sources; service availability is independent of offline gates.
`export --out <folder>` exports recorded tables as CSV. `--replay start` starts
recording playback. `shot --out-dir <folder> --pages all` renders screenshots.
`bench` measures rendering performance; use its help for options.

The default size is 1920x1080. Override with `--size 1280x720`, or use `--fullscreen`.
Configure observer sites and source URLs in the config file printed by doctor.
Full legal and dependency notices are available through `retroradar about`.

## Close Earth views

Mouse drag and arrow-key rotation share zoom sensitivity. Earth focus scales by
height above the surface; vehicle focus scales by distance to the vehicle. The
default overview speed is retained; the closest Earth view is about ten times
slower. VIEW free-look has a fixed field of view and retains its look sensitivity.
Kiosk's programmed angular drift and the simulation clock are unaffected.

All 3D Earth views, including VIEW/NADIR, share the selected `--earth-quality`
textures. Use `--earth-quality high` for 16K day / 8K night, subject to GPU limits.
Close views still magnify finite-resolution imagery: night 8K has about 4.9 km per
texel at the equator. The coastline annotation fades when a mask texel spans
1–4 screen pixels, avoiding broad stair-stepped outlines over the actual image.
This does not create additional ground detail.
