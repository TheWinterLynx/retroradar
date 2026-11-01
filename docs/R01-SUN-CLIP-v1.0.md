# R01 globe overlay clipping v1.0

The globe's wgpu pass already applied its panel viewport/scissor. The subsequent
2D annotations used the console-wide egui painter, allowing the projected Sun disc
and SUN label to cross into other panels when rotating or zooming the camera.

Console preparation now creates a painter clipped to the globe rectangle and
uses it for orbit annotations, planned trajectories, vehicle labels and the
window-view celestial annotations. The normal console painter retains its own
clip so headings, tables and the footer remain visible. Projection and orbital
coordinates are unchanged; partial markers are clipped at the panel boundary.

A headless regression prepares a real console frame with a sourced orbit solution
and places the Sun disc across the globe's right edge. It inspects submitted mesh
commands: visible Sun geometry must use the panel clip, while the rest of the
console retains a wider clip. It requires no GPU or network. R01 runs this focused
regression before the full workspace suite.

Windows visual acceptance: rotate/zoom the MASTER and TRAJ globes until the Sun
crosses each edge. Its disc and label must disappear at the frame boundary, with
adjacent panels intact. Check VIEW celestial annotations and the footer as well.

Agent validation (Linux, Rust 1.95.0): formatting and workspace tests with warnings
denied pass: 337 passed, 0 failed, 6 ignored. The new regression was also run with
the panel clip removed and failed as expected; the fixed source was restored and
the full suite passed. Windows release/window and visual checks remain owner gates.
