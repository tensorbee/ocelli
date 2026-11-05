# Release policy

**F-IDs that contributed:** F-117, F-217, F-220
**Last updated:** 2026-11-05

`/release` is the only command that publishes. This page does not publish
and it does not create a `v` tag. The workspace version stays 0.1.0.

A stable claim is false while D-22 is open. It is also false unless the
certified row count equals the matrix row count and that count is an
integer of at least 1. A certified count short of the row count is not
enough. The current matrix has no certified row, and D-22 is open, so a
caller who passes those facts gets false.

`0.x` carries no stability promise. The removal window is null. A patch
does not schedule a removal, so its window is null. A name deprecated in
a published minor stays for one further minor. A major may remove a
name, so the window is 0. Any other channel is null.

Tiers A, B and C are not applicable. The functions do not render.

`THIRD_PARTY_NOTICES` carries the upstream OpenJPH notice and the vendored
ritk-codecs MIT grant. `NOTICE` names Tensorbee Limited and is published with
both npm packages. `python3 scripts/pin_and_size_check.py
--require-redistribution` passes. A stable claim stays false while D-22 is
open. The notice being present does not by itself close that row.

`THIRD_PARTY_LICENSES` carries the full texts of the Rust crates and the Rust
standard library compiled into the wasm module and the native binaries. The
closure is the third-party crates reachable on a normal or build edge from
the wasm entry point for wasm32-unknown-unknown and from the native entry
point with no platform filter. Workspace members are excluded.
`python3 scripts/licence_check.py --write binaries` regenerates the file.
`gate packages` refuses a tarball that carries a `.wasm` file and lacks
`THIRD_PARTY_LICENSES` or `THIRD_PARTY_NOTICES`. The milestone that lifts
`ocelli-native` `publish = false` ships a native archive carrying `LICENSE`,
`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `THIRD_PARTY_NOTICES` and
`THIRD_PARTY_LICENSES`.
