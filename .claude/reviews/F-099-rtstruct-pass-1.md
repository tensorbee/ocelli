# F-099 review, pass 1

**Reviewed**: uncommitted F-099 sources on `sprint/s35`. HEAD is `bec00725e711527a711bc3c578b8f2e8da3f56fb`. This pass staged nothing. `git diff --cached --name-only` was empty. The reviewed sources are `crates/ocelli-seg/src/contour.rs` (sha256 `997c27cb9a7ad6476ee60bd24acc744df42cf27191a500b7a7fe314889877ab3`), `crates/ocelli-seg/src/lib.rs` (sha256 `21550c8a1f52f692535423e060ddfa02f57e1928d8851096129082bfeac2950c`), and `crates/ocelli-seg/tests/rtstruct_import.rs` (sha256 `98df20eb09ea506f3c0f785c932d8493779f23cfb2dc543ec6f66b50bf237326`). The test file is untracked. The contour and lib edits are unstaged.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19ec4-73ab-7bc0-b08a-4cd3715a9849` and did not write the plan or the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. The HLD does not give the spacing fraction. The half-spacing rule is the local plan. This pass did not invent another fraction.

AC1. A closed planar triangle at z 10, slice z 10, spacing 2, segment 1, returns those three points unchanged. The shipped expect is equality of the contour points with the caller slice, and equality of the caller with a clone taken before the call. The green run passed.

AC2. A point at z 10.9 with spacing 2 is inside, because half spacing is 1. A point at z 11.1 is `OutOfPlane`. The shipped test also keeps z 11.0, which is the equal boundary of `abs(point.z - slice_z) <= spacing / 2`. Widening the divisor from `2.0` to `1.0` made the z 11.1 call return `Ok` with caller z 11.1. The expect at `crates/ocelli-seg/tests/rtstruct_import.rs:50` wanted `Err(OutOfPlane)`. The z 10.9 and z 11.0 expects in that same test were not the failing assertion. They passed on the restored bytes.

AC3. Spacing 0 is `OutOfPlane`. A non-finite x is `Frame`. The shipped expects are those two errors. The green run passed. The mutated half did not reach those checks, and that test stayed green during the mutation.

AC4. One closed planar point is `Length`. Segment 0 is `MissingColour`. The shipped expects are those two errors. Both run before the half calculation. They stayed green during the mutation and passed on the restored bytes.

AC5. The returned z for an in-slab point is the caller z, not the slice z. The shipped expect compares a z 10.9 contour with the caller points and refuses equality with a triangle at the slice z 10. The green run passed. The widened slab still stored z 10.9, so this row stayed green during the mutation.

The plan says one point outside the slab refuses the whole contour. The shipped AC2 fixture places every point at the same z. A mixed contour is not a separate executed row. The source returns `Err` on the first out-of-slab point and does not return `Ok` with the earlier points. That reading is source inspection. It was not a second test command.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test rtstruct_import` | Restored divisor `2.0`. Source sha256 `997c27cb9a7ad6476ee60bd24acc744df42cf27191a500b7a7fe314889877ab3` | executed now, before the mutation | Exit 0. 5 passed and 0 failed. |
| Same command after `spacing / 2.0` became `spacing / 1.0` | One constant in `crates/ocelli-seg/src/contour.rs`. Mutated sha256 `624f6a83c1561c3d374645329c64c7e1a835fa084db185395f6278bd9c8732ee`. The test file and `lib.rs` hashes did not change. | executed now | Exit 101. 4 passed and 1 failed. `half_spacing_keeps_10_9_and_refuses_11_1` received `Ok` for z 11.1 and expected `Err(OutOfPlane)`. |
| `cmp` of the saved original against `crates/ocelli-seg/src/contour.rs`, then the same test | Bytes copied back. sha256 again `997c27cb9a7ad6476ee60bd24acc744df42cf27191a500b7a7fe314889877ab3` | executed now | `cmp` exit 0. Test exit 0. 5 passed and 0 failed. |

rustc 1.97.1. No wasm-pack, no second cargo, and no `bin/ocelli.sh gate`.

The restored comparison is:

```rust
let half = spacing / 2.0;
```

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`import_rtstruct_contour` is public in `contour.rs` and re-exported from `lib.rs`. `kind_len` stays private. The import calls it. The only geometric-type length match is that helper. `present_contour` and `project` are not in the contour diff. The diff adds the import after `rounded_i32` and does not change the projection. The import does not call `present_contour`.

Segment 0 returns `ContourError::MissingColour` before the length helper. A refused length returns `ContourError::Length`. Spacing that is not finite or not greater than 0 returns `ContourError::OutOfPlane`. A non-finite `slice_z` uses the same error, because the slab comparison cannot hold. A non-finite x, y, or z returns `ContourError::Frame` before the distance test. The distance is `(point.z - slice_z).abs()`. A distance greater than `spacing / 2.0` returns `ContourError::OutOfPlane`. An equal distance is kept. The stored point uses the caller x, y, and z. It does not replace z with `slice_z`.

A refusal returns `Err` and drops the local vector. The argument is a shared slice. The AC1 and AC5 tests compare the caller with a pre-call clone. `ContourError` has no new variant in the diff. There is no `unsafe` and no `wasm-bindgen` in `ocelli-seg`. There is no `as` cast in the import. Tiers A, B, and C are not branched. The plan marks each tier n/a.

No TypeScript name for this import was found. `packages/core` and `seg_roundtrip.rs` were not in the diff. `git status` did not show them.

## Limitations and handoff

The mutation is not left in place. `cmp` against the saved original was exit 0, and the sha256 is `997c27cb9a7ad6476ee60bd24acc744df42cf27191a500b7a7fe314889877ab3`.

The shipped out-of-slab fixture uses the same z on every point. The negative side of the slab is the same absolute distance and is not a separate fixture. Spacing below 0, a non-finite spacing, and a non-finite y or z are refused by the same checks and are not fixtures. Open and point kinds are not fixtures. They use `kind_len`. This pass does not count those missing rows.

The plan's write list includes an LLD sentence. `docs/lld/README.md` and `docs/lld/public-api.md` were not reviewed and the absence is not counted. Clippy was not run. No feature profile was run. No browser and no pixel oracle were used. This story does not draw and has no rendered parity view.

No patient data was read.
dwv, Horos, and the medical viewer Grok were not opened.
