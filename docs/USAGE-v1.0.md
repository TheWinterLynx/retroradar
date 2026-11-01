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
| Mouse drag / wheel | Rotate / zoom the orbit map |
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
