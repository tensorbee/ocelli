# F-195 review, pass 3

**Reviewed**: uncommitted working tree on sprint/s43 for F-195 after the pass 2 remediation
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **AC1**: `crates/ocelli-geom/tests/calibration.rs:64` fills every later source, and line 76 asserts `PixelSpacing`. Lines 102 to 124 show that a zero, NaN or negative zero pixel spacing does not fall through to the imager pair.
- **AC2**: `crates/ocelli-geom/src/calibration.rs:153` and `:154` divide by the factor. The fixture at `tests/calibration.rs:83` uses 0.2 and 0.4 against factor 2, so 0.1 and 0.2 also tell row from column. Multiplying gives 0.4 and 0.8, and the bit comparison at lines 97 and 98 goes red. This matches PS3.3 C.8.7.5, where the factor is SID over SOD, so the detector spacing is the larger one.
- **AC3**: `tests/calibration.rs:127` covers ultrasound winning over manual. Lines 143 to 197 cover the imager halves blocking ultrasound and manual.
- **AC4**: `tests/calibration.rs:200` asserts that manual is last, that an empty request is `Unavailable`, and that a negative manual value is `Unavailable`.
- **AC5, AC6**: `tests/calibration.rs:228` and `:229` compare bits against 350 and 90. `stats.rs:340` and `:356` pass scales of 1000 and 10000 into the shared `scaled_suv`. Using weight, or leaving out the 10000, would change those numbers.
- **AC7**: `suv_bw` at `stats.rs:323` now goes through `activity_from` and `scaled_suv`. The F-075 fixtures in `tests/stats.rs:166` to `:210` (140, 280, and negative zero elapsed giving 140) still apply. `stats.rs:391` keeps `elapsed_seconds < 0.0` rather than `total_cmp`, so negative zero elapsed is still accepted. That is what `tests/stats.rs:169` needs.
- **AC8**: `tests/calibration.rs:246` to `:250` cover 2, the sign of -2, positive zero, negative zero and NaN.
- **AC9**: `tests/calibration.rs:256` gives 150. The mean would be 75.
- **AC10**: `examples/example-server/src/http.rs:519` checks every JSON field. `App.tsx:929` mounts `CalibrationPanel`, and `vite.config.ts:23` proxies the route.

## Evidence

Taken from the host and not re-run here. After the tests were added, `bin/ocelli.sh test ocelli-geom --test calibration` passed 9 tests. `Ok(ratio.abs())` made `length_ratio_four_over_two_is_two` fail. Multiplying by the factor made `imager_spacing_divided_by_the_magnification_factor_is_one_tenth` fail after the guard restructure. Both changes were reverted. From reading the code, each mutation should fail exactly as reported.

Pass 2 smells, each confirmed closed by reading:

1. The inline comment at `tests/calibration.rs:84` now cites C.8.7.5, and so does the module header at line 4.
2. `tests/calibration.rs:190` to `:196` pins a lone NaN factor, with ultrasound and manual also present, to `Frame`. If the finiteness check at `calibration.rs:111` were removed, the result would be `Unavailable`. If the outer condition at `:99` checked only `imager_spacing`, the result would be `Ok(Ultrasound)`. Either change turns the test red.
3. `tests/calibration.rs:247` asserts that the bits of -2 come back. `calibration.rs:190` returns `Ok(ratio)`.

## Defects

none

## Smells

none

## Nitpicks

1. `crates/ocelli-geom/src/calibration.rs:183` writes out the two-sign zero check, while `stats.rs:112` already has `exact_zero` for the same test. It is not wrong. It is a second copy of a small predicate.
2. The `negative_factor` case at `crates/ocelli-geom/tests/calibration.rs:163` provides no ultrasound or manual source. If the code fell through, it would also return `Unavailable`, so this case cannot detect a fall-through. Only the `zero_factor` case at line 156 can. Adding `ultrasound_delta` here would make it test fall-through too.
3. `examples/example-server/src/calibration.rs:21` uses a symmetric 0.2 by 0.2 imager pair, so a row and column swap would not show at the route level. The unit fixture at `tests/calibration.rs:87` covers this, so it does not block.

## Verified clean

- The order in `select_calibration`, `calibration.rs:95` to `:131`, matches the design plan's scope list and the rule at plan line 40. A present but invalid earlier source returns an error and does not fall through. Either imager half counts as present. A non-finite half is checked before incompleteness.
- `accept` at `calibration.rs:135` checks again after dividing. An overflowing quotient is `Frame` and an underflow to zero is `Unavailable`, which agrees with the error documentation at lines 89 to 94.
- `contour_volume` at `calibration.rs:204` is a sum times spacing, not a trapezoid. Error mapping matches `docs/lld/calibration.md:14`.
- The error codes quoted in the LLD (400, 401, 402, 403) match `measure.rs:32`, `:34` and `stats.rs:96`, `:98`.
- No `unsafe`, no `as` cast and no rounding decision in the new arithmetic. `known` in the example server compares bits and does not format floats.
- The design plan, `docs/lld/calibration.md`, `docs/lld/geometry.md:109` to `:116`, the README rows at lines 23 and 24, and `CHANGELOG.md:17` agree with the code and with each other. In the prose I read I found no em dash and no prose semicolon.
- `wasm-bindgen` is untouched and no pixels cross the boundary. Tier rows are n/a, which fits host arithmetic.

## Limitations and handoff

I could not run the gates, so the test count, the mutation results, rustfmt and clippy all come from the host. Line 183 of `calibration.rs` is exactly 100 columns by my count, which is rustfmt's default limit. Without a shell I could not diff `stats.rs` against HEAD, so the claim that `suv_bw` is unchanged rests on the F-075 fixtures in `tests/stats.rs` and not on a diff. I did not open PS3.3 itself. The section numbers were checked against the plan's transcription and against what I know of C.8.7.5 and C.8.11.4. This pass has zero defects and zero smells, so under the microscope rule the story can go to `/complete-feature`. The three nitpicks do not block.
