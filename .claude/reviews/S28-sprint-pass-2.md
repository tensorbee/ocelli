# S28 sprint review, pass 2

**Reviewed**: `27c227ebe8e16a5b52ae0058686f8ed103cc0f54...e759c40dd3e4de3c72c78f57cdf633a87ef3bbb7` on `sprint/s28`. HEAD is `e759c40`. HEAD tree is `08ae88391dc96a399e14d9ed3f263a66aac4f443`. The uncommitted remediation is `.claude/plans/F-073-design.md`, `.claude/plans/F-074-design.md`, and the doc comment on `Volume::sample_world` in `crates/ocelli-volume/src/ortho.rs`. Pass 1 at `.claude/reviews/S28-sprint-pass-1.md` was already untracked and was not edited. The only product edit in this pass was `sum.abs() * 0.5` changed to `* 1.0` in `crates/ocelli-geom/src/measure.rs`, then restored. SHA-1 before and after is `479befbd9f0bde427206f850e5099ff158986fd2`.
**Author / reviewer**: Pass 1 recorded story trailers as `Ocelli-Generated-By: grok`. A generated trailer does not name a person. Those trailers were not re-read here. This reviewer did not write the sprint and did not write the remediation. This reviewer did not resume `01a19be4`, `01a19bcc`, `01a19b80`, `01a19b8d`, `01a19b98`, `01a19bac`, or `01a19bba`.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 1 D1 is closed. `.claude/plans/F-073-design.md` line 22 no longer says fewer than two points returns 0. It says one finite point is 0 and a non-finite coordinate is `Frame`. Line 30 says fewer than two points is `Degenerate` before the `polyline_length` call. One finite point is 0 from `polyline_length`. One non-finite point is `Frame` from `polyline_length` and `Degenerate` from `length`.

`polyline_length` checks `finite_point` on every point before any sum, at `crates/ocelli-geom/src/measure.rs:121`. There is no earlier `Ok(0)`. An empty list and one finite point reach `Ok(0)` because `windows(2)` adds nothing. One non-finite point returns `Frame` from that check. `length` returns `Degenerate` when `points.len() < 2` at line 324 and only then calls `polyline_length`. A one-point list never reaches the finite check through `length`.

Pass 1 D2 is closed. `.claude/plans/F-074-design.md` line 12 and the `sample_world` comment at `crates/ocelli-volume/src/ortho.rs:98` say `spacing[0]` divides the row cosine and `spacing[1]` divides the column cosine. Neither sentence says `spacing[0]` divides the column cosine.

`Volume` stores `spacing[0]` from `column_mm()` and `spacing[1]` from `row_mm()` at `crates/ocelli-volume/src/lib.rs:584`. `direction` is `from_cols(orientation.row(), orientation.column(), normal)` at line 595. `direction.col(0)` is the row cosine. `direction.col(1)` is the column cosine. `voxel_world` multiplies column 0 by `spacing[0]` and column 1 by `spacing[1]`. `sample_world` divides `col(0).dot(delta)` by `spacing[0]` and `col(1).dot(delta)` by `spacing[1]` at `ortho.rs:108`. `docs/lld/volumes.md` already says `spacing[0]` is column millimetres along the row cosine, and that `PixelSpacing[0]` is `spacing[1]`. DICOM PS3.3 C.7.6.2 puts the between-rows value first. That value is `row_mm`, stored as `spacing[1]`, and it scales the column cosine. The comment names the cosine in the division. It does not swap that DICOM order.

`git diff -U0` of `ortho.rs` changes only that comment line. The function body is the HEAD body. Commit `da6047e` does not touch `ocelli-volume`. The committed sprint range does not either.

The three hunks do not say every short polyline is 0. They do not say closing S28 completes M8. They do not change the workspace version, a tracker days cell, or the F-X003 status.

The area half is still guarded. `sum.abs() * 0.5` became `* 1.0`. `the_unit_square_has_area_one` went red. The left bits `4611686018427387904` are 2. The right bits `4607182418800017408` are 1. The restored line is `sum.abs() * 0.5` and the SHA-1 matches the pre-edit hash.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `polyline_length` and `length` | `measure.rs` lines 111 to 141 and 314 to 328. SHA-1 `479befbd` before the mutation | read now | Finite check is first in `polyline_length`. Count check is first in `length`. One finite point is 0. One non-finite point is `Frame` from `polyline_length` and `Degenerate` from `length` |
| `sample_world`, `voxel_world`, spacing array | `ortho.rs` lines 88 to 112. `lib.rs` lines 584 to 595. `docs/lld/volumes.md` lines 143 to 149 and 246 to 251 | read now | `spacing[0]` is `column_mm` and divides the row-cosine dot. `spacing[1]` is `row_mm` and divides the column-cosine dot |
| `git diff -U0 -- crates/ocelli-volume/src/ortho.rs` | Working tree against HEAD `e759c40` | executed now | One comment line. Column cosine and row cosine are swapped to row cosine then column cosine. No body line |
| `git diff --name-only 27c227e...e759c40` and `git show --stat da6047e` | Committed sprint range. F-074 commit | executed now | `ocelli-volume` is absent. F-074 writes geom, plans, reviews, changelog and sprint ledgers |
| `bin/ocelli.sh test ocelli-geom -- the_unit_square_has_area_one` after `* 0.5` became `* 1.0` | `measure.rs` only. Restored after. One cargo process | executed now | Exit 101. Failed at `crates/ocelli-geom/tests/measure.rs:426`. Left bits `4611686018427387904`. Right bits `4607182418800017408`. Lib filter ran 0 tests |
| `python3` `struct` pack of 2.0 and 1.0 | Not the Rust function | executed now | `4611686018427387904` is 2. `4607182418800017408` is 1 |
| `shasum` of `measure.rs` before the edit and after `git checkout` | Same path | executed now | Both `479befbd9f0bde427206f850e5099ff158986fd2`. `ortho.rs` stayed `ff87538dae86a735b5633d12042f8fe65654d89a`. The two plans stayed `e1bd46ca254dfb6bcb0ecb01bd799194aff1462d` and `9b028d710f7b804992364eeb0dba67d72aed8734` |
| Backlog, plan, tracker, `Cargo.toml` | F-073 through F-079, F-X003, workspace version | executed now | S28 rows are `done`. S29 rows F-075, F-077, F-078 and F-079 are `pending`. Estimates 5, 5, 4, 5, 5, 3 and 3 sum to 30. Tracker days cells for F-073, F-074 and F-076 start `not measured`. Version is `0.1.0`. F-X003 is `in-progress` |
| `git rev-parse HEAD^{tree}` and `git write-tree` | Clean index | executed now | Both `08ae88391dc96a399e14d9ed3f263a66aac4f443`. That is the pass 1 recorded tree. It does not contain the three unstaged hunks |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14). The test command's own status was `code=$?` and was 101. wasm-pack was not run. No patient data was used.

## Defects

None. Pass 1 D1 is closed. Pass 1 D2 is closed.

## Smells

None.

## Nitpicks

None.

## Verified clean

Line 22 of the F-073 plan now says one finite point is 0 and a non-finite coordinate is `Frame`, and that this story does not change `polyline_length`. Line 30 separates the two functions for one non-finite point. The module comment on `polyline_length` still says zero points or one finite point have length 0, and a non-finite coordinate refuses the sum. `docs/lld/geometry.md` and `docs/lld/public-api.md` keep the same finite qualifier. No sentence in the three hunks says every short polyline is 0.

The F-074 plan line and the `sample_world` comment agree with the division. `spacing[0]` divides the dot with the row cosine. `spacing[1]` divides the dot with the column cosine. `voxel_world` is the matching multiply. The assembly test `row_spacing_multiplies_the_column_direction_cosine` states the same pairing for `PixelSpacing` `[0.5, 1.0]`: stored spacing `[1.0, 0.5, 2.0]`, and column 0 times `spacing[0]` is `(1, 0, 0)`. That test was read and not run.

`CURRENT_SPRINT.md` says closing this sprint does not complete M8. The plan says M8 is S28 to S29, 7 stories and 30 engineer-weeks. Three of those stories are `done` and four are `pending`. The workspace version in `Cargo.toml` is `0.1.0`, and the mutated test compiled `ocelli-geom v0.1.0`. The three tracker days cells start with `not measured`. F-X003 stays `in-progress`. The three hunks do not touch those files.

The restored area line is the half. Dropping it makes the unit-square test compare 2 with 1. The shifted square is in that same test, so this run also did not accept an open ring, but the failure was the first expect and the later expects were not reached.

## Limitations and handoff

This pass is a review of the remediation. It is not a sprint close and it does not complete M8. Nothing was staged, committed, pushed, or recorded with `record-sprint-review`. The index still matches HEAD tree `08ae88391dc96a399e14d9ed3f263a66aac4f443`. A receipt on that tree would miss the three hunks. This file is not a sprint-profile record. A later edit to the reviewed tree makes this pass stale.

One cargo process ran. The full `ocelli-geom` suite was not repeated. One `NaN` point was not executed. The longer non-finite polyline in `polyline_length_sums_segments` was not re-run. A zero rectangle side, Cobb of opposite parallel segments, and a volume sample were not executed. Clippy was not run.

The F-074 plan still says the story does not write `ocelli-volume`. That sentence was not part of the hunk. It is true of `da6047e`. The uncommitted volume change is the comment only. It is not scored as a new claim from the hunk.

The straight-line sentence in `.claude/plans/F-076-design.md` was not edited. Pass 1 left it as a limitation. The numeric points are not colinear. That sentence is not re-scored here.

The next owner can stage this tree and record the pass. Do not record the HEAD tree as if it contained these hunks. Do not close M8 from this sprint.
