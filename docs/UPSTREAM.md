# Upstream provenance

RetroRadar starts from https://github.com/LiamDroog/mctl at commit
`bf7072fdf1080fec6890a9889531e78297e491eb` (retrieved 2026-11-01).
It is an independent Git repository, with no fork relationship or inherited Git history.
The Rust sources and bundled assets are adapted from mctl, not independently rewritten.

mctl is by Liam Droog, with the assistance and contributors credited in its original
About screen and README. The original README is preserved as UPSTREAM-README.md.
Original source licence: GPL-3.0-or-later; see ../LICENSE. Asset and dependency
notices remain in ../assets/LICENSES.md and ../THIRD-PARTY-NOTICES.md.
RetroRadar adaptations are by TheWinterLynx and retain GPL-3.0-or-later.

R01 changes: Windows platform isolation, executable/application identity, portable
paths, offline doctor, desktop kiosk, terminal diagnostics and local Windows gates.
Internal crate names and upstream historical documents intentionally retain mctl.
Upstream Linux installation and release instructions are historical references;
RetroRadar's README and AGENTS.md define this project's current workflow.
