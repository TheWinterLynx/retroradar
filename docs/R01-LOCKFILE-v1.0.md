# R01 Cargo.lock canonical order v1.0

Renaming the workspace crates left their lockfile package blocks and dependency
arrays in the previous positions. Cargo accepted the same resolution but rewrote
the canonical ordering during local commands. The next clean-checkout gate then
correctly refused the resulting unstaged lockfile change.

The committed lockfile now uses the new package-name ordering. Versions, sources,
checksums and dependency sets remain unchanged for all 529 package/version entries.
The Cargo.lock-only Git attribute enforces LF without changing other tracked files.
The Windows gate checks that Cargo left the lockfile unchanged after the tests.

Recovery for a checkout with this generated edit must first compare the working
lockfile's normalized Git blob with the updated remote lockfile. Restore only that
file if they match and it has no staged edits, then fast-forward and run the gate.
A mismatch requires inspection; do not reset, stash or discard other owner edits.

Validation: parsed and compared all package resolutions before/after normalization;
all 529 are unchanged. Cargo 1.95.0 metadata (`--locked --offline --no-deps`)
accepted the workspace and left the lockfile blob unchanged. The canonical blob
4daf0b40e8347abeac186f43d222909cd318a043 also matches the Windows-generated
lockfile shown in the owner's failure report. Formatting check passed.
