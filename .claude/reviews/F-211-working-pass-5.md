# F-211 review, pass 5

**Reviewed**: `git diff d8034cd HEAD` on `work/f-211-xai` in `ocelli-wt-f-211`, HEAD `f17fe9297cad4afb5b894ee2410918b552fddf0b` ("F-211, review pass 4 fixes"). 5 files, +67 -9. Pass 4 swept the whole story, so this pass sweeps only this diff. The tree was clean at the start and at the end, apart from the untracked pass 1 to 4 reports, which are not under review.
**Author / reviewer**: A Claude implementer agent wrote f17fe92, per the lead. The trailer alone cannot establish that. This pass is by an independent Claude reviewer subagent that wrote none of the F-211 code, plan, rulings or fixes and did none of passes 1 to 4.
**Result**: 0 defects, 0 smells, 1 nitpick

## What the diff is

- `crates/ocelli-geom/src/lib.rs:26`, one crate doc sentence (pass 4 D2).
- `crates/ocelli-geom/tests/calibration.rs`, four rows added to `an_invalid_pixel_spacing_is_refused_beside_an_agreeing_imager_pair`, with the loop now carrying the imager pair per row (pass 4 S1).
- `examples/folder-viewer/tests/measure.rs`, four IPS-alone refusal rows in `measure_calibration_provenance` (pass 4 D1).
- `examples/folder-viewer/src/measure.rs`, a new `#[cfg(test)] mod tests` appended after `geom`, holding `a_non_finite_imager_pair_alone_refuses` (pass 4 D1, the finiteness half).
- `docs/lld/dicom-ingest.md:189`, "only" deleted (pass 4 N1).

**No product code changed.** `git diff -U0 d8034cd HEAD -- examples/folder-viewer/src/measure.rs` is one hunk at `@@ -590,0 +591,24 @@`, entirely inside the new `#[cfg(test)]` module. The `lib.rs` change is a `//!` line. The other three files are tests and the LLD.

## Mutation replay

Scratch `git archive HEAD` copy under the session scratchpad, `CARGO_TARGET_DIR=<wt>/target/p5-scratch`, `cargo test --no-fail-fast -p ocelli-geom -p ocelli-folder-viewer` for geom mutations and `-p ocelli-folder-viewer` for viewer ones. An unmutated baseline in the copy exited 0. After each mutation the file was restored from the in-memory original and touched, and at the end `git show HEAD:<path> | cmp` showed both mutated files identical to HEAD.

| Mutation (pass 4 name) | Result, with the row that went red |
|---|---|
| M1, drop the Pixel Spacing `accept` on the equal branch | red: `an_invalid_pixel_spacing_is_refused_beside_an_agreeing_imager_pair`, `measure_calibration_provenance` |
| M2, imager path first, then Pixel Spacing | **red** (green in pass 4): `0\0` beside `5e-10\5e-10` with factor NaN gave `Err(Frame)` |
| M3, fall back to Pixel Spacing when the imager path refuses | **red** (green in pass 4): the new test, `equal_pixel_and_imager_spacing_is_the_detector_plane`, `measure_calibration_provenance`. Only the `5e-10\5e-10` beside `0\0` row can fail here, since every other row's Pixel Spacing refuses first |
| M4, row half only | **red** (green in pass 4): `0.2\0` beside `0.2\5e-10` gave `Ok(Detector 0.2\5e-10)` |
| M5, column half only | **red** (green in pass 4): `0\0.2` beside `5e-10\0.2` gave `Ok(Detector 5e-10\0.2)` |
| M7, viewer IPS-alone positivity removed | **red** (green in pass 4): `measure_calibration_provenance`, "Imager Pixel Spacing alone, zero" answered `0 mm at the detector` |
| M8, viewer IPS-alone finiteness removed | **red** (green in pass 4): `measure::tests::a_non_finite_imager_pair_alone_refuses` |
| M9, viewer unparseable imager pair read as absent | **red** (green in pass 4): "Imager Pixel Spacing alone, unparseable" answered `4.0 px`, `uncalibrated` |

Pass 4 D1 and S1 are closed. Each new row fails for the reason its comment gives.

## Claims executed

- **Geom test comment** (`tests/calibration.rs:181-189`).
  - "0\0 with a NaN factor is Pixel Spacing's `Unavailable`, not the factor's `Frame`": M2 shows `Frame` once the order flips.
  - "0\0.2 beside 5e-10\0.2, and 0.2\0 beside 0.2\5e-10, agree within the tolerance": `SPACING_EQUALITY_MM` is `1e-9` (`ocelli-core/src/spacing.rs:19`), and each difference is `5e-10`. M4 and M5 return `Detector`, which only the equal branch yields, so they did agree.
  - "A valid 5e-10\5e-10 beside ... 0\0 takes the imager path ... and does not fall back": M3 is red.
- **The rustdoc sentence pass 4 S1 was about** (`calibration.rs:124-125`, "both ... are checked, in that order") is now pinned by M2, M3, M4 and M5.
- **Viewer test comment** (`tests/measure.rs:659-663`). Zero goes red under M7. `x\y` goes red under M9. "One value is not a pair" matches `top_level_pair` (`preview.rs:1777-1786`), which needs exactly two parts, so `0.2` becomes `Unusable`. `-0.2\-0.2` passes `clean_decimal` and reaches the positivity check, so it is a second M7 witness, but `expect_refusal` stops at the first failure, so the zero row is what reported.
- **Viewer unit test doc** (`measure.rs:596-599`). "A Decimal String never parses to a non-finite number": `decimal_pair` calls `ocelli_dicom::parse_decimal_string` (`preview.rs:32`, `:1843`), which ends `value.is_finite().then_some(value)` (`frame_geometry.rs:732`). So `1e999` is `Unusable`, and the finiteness check really is unreachable from a file. Driving `choose_spacing` directly is justified. The test asserts what its name says: IPS alone, no Pixel Spacing, no factor, an infinite row or column half, and `Err(PreviewError::Calibration)` for both.
- **`lib.rs:26`**, "finds where a ring crosses or touches itself", agrees with `region.rs:233-245`. `grep -rn "proper crossing" crates docs examples README.md` leaves only `region.rs:241` and `geometry.md:154`, and both list a proper crossing as one case among several, which is correct. Pass 4 D2 is closed.
- **`dicom-ingest.md:189`**, "PS3.3 10.7.1.1 says that an uncorrected Pixel Spacing has the same values as Imager Pixel Spacing and a corrected one differs". This matches the text pass 4 fetched, and with "only" gone it no longer claims to be the whole of the section. The second and third table rows (`:178-179`) are the equal and different cases, so "which the second and third rows record" holds. Pass 4 N1 is resolved.

## Test layout

The viewer crate already holds inline unit modules: `#[cfg(test)] mod tests` in `held.rs:1851`, `schedule.rs:557`, `serve.rs:2377`, `preview.rs:2778` and `value.rs:358`. `WORKFLOW.md:300` puts pure-logic unit tests under `#[cfg(test)]` in `src`. `choose_spacing` and `CalibrationInputs` are private to the module, so a `tests/` file could not reach them. The new module uses `use super::*`, as `held.rs` does, and clippy with `--all-targets` is clean on it.

## Evidence

| Command | Source | Executed now | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at f17fe92, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-core -p ocelli-folder-viewer -p ocelli-geom -p ocelli-dicom` | same | now | 0. 525 passed, 0 failed, 3 ignored. The three named tests above ran and passed |
| `npm run lint` | same | now | 0 |
| `bin/ocelli.sh gate guards` | same, `OCELLI_PORT=0` | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | same | now | 0, "0 watched by nothing" |
| `python3 scripts/prose_check.py` | same | now | 0, 883 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1744 files |
| Mutation catalogue | scratch copy, `target/p5-scratch` | now | table above, all red |
| Cleanup | scratch copy and `target/p5-scratch` deleted with Python `shutil` | now | `git status` shows only the four earlier reports and this one. HEAD is f17fe92 |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1.** Pass 4 N3 carries over: `.claude/plans/F-020-design.md` and `docs/sprints/AS_BUILT.md:2615` cite C.7.6.1.1.5 for spacing calibration. Historical ledgers outside this diff, for the lead.

## Limitations and handoff

- **Bounded to this diff.** Pass 4 swept the whole story, and this pass did not repeat that sweep. PS3.3 was not re-fetched, and the 10.7.1.1 check relies on the text pass 4 quoted.
- **Not run.** No browser, no live viewer, no Playwright, so AC18 stays external. No oracle or GPU job. The wider crate set pass 4 tested (`ocelli-pixel`, `ocelli-seg`, `ocelli-example-server`) was not re-run, since the diff touches none of them.
- **Scratchpad.** `scratchpad/p5/` holds `main.txt`, `test.txt`, `baseline.txt`, `guards.txt`, `census.txt`, `mut.txt`, `mut2.txt`, `mut.py` and `mut2.py`.
- This is a clean pass of f17fe92's material. It is not acceptance and not sprint readiness.
