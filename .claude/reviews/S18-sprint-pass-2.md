# S18 review, pass 2

**Reviewed**: uncommitted staged correction on `sprint/s18`. HEAD is `2af862735ee9fb10b9ee2812b56944fcb679845b`, tree `f258a04046ad788f3bb8f6ff4e5ce28cb8d45b7a`. Staged tree `102b72c725a5abeaaab03b522f640867bab84772`, from `git write-tree` before this file and again after the mutation was restored. `git diff` was empty both times. Staged paths are `.claude/plans/F-051-design.md`, `crates/ocelli-volume/src/lib.rs`, `crates/ocelli-volume/tests/construct.rs`, `docs/lld/volumes.md` and `docs/sprints/AS_BUILT.md`. This review file is not in that tree.
**Author / reviewer**: The staged correction was already in the index. This pass did not write it and did not write the F-051 implementation. Independent subagent.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1, AC2 and AC10. These are the three success tests that now read `volume.order`. The new sentences say `place` writes at sorted position `n` and that `order[n]` is that slice's arrival index.

AC1. Arrival 0 is z = 6 with `[1,1,1,1]`. Arrival 1 is z = 0 with `[2,2,2,2]`. Arrival 2 is z = 3 with `[3,3,3,3]`. Row `(1,0,0)` crossed with column `(0,1,0)` is normal `(0,0,1)`, so the projection is z. Ascending position is arrival 1, then 2, then 0. `order` is `[1, 2, 0]`. The inverse, the sorted position of each arrival, is `[2, 0, 1]`. The assertion is `[1, 2, 0]`. It rejects a wipe and it rejects the swapped direction. Slice 0 stays `[2,2,2,2]`, which is arrival 1, not arrival 0.

AC2. Python f64 projections of `(0, 4.8, -3.6)`, `(0, 2.4, -1.8)` and `(0, 0, 0)` onto `(0, 0.8, -0.6)` are 6, 3 and 0. Sorted arrivals are 2, 1, 0. The assertion is `[2, 1, 0]`. That vector is its own inverse, so this test alone does not separate the two directions. AC1 and AC10 do. Slice 0 is `[4,4,4,4]`, arrival 2. A z sort would put `[6,6,6,6]` first, and that assertion is unchanged.

AC10. Arrivals at z = 10, 2 and 6 sort as arrival 1, then 2, then 0. The assertion is `[1, 2, 0]`, not the inverse `[2, 0, 1]`. Slice 0 is `[1,2,3,4]`, the z = 2 frame.

`construct` clones `order` and then calls `place` with the sorted index. `volume.order.clear()` before `Ok(volume)` leaves the bytes in place and returns an empty vector. AC1 failed at `construct.rs:108` with `left: []` and `right: [1, 2, 0]`. AC2 failed at `construct.rs:158` with `right: [2, 1, 0]`. AC10 failed at `construct.rs:241` with `right: [1, 2, 0]`. Seven tests stayed green, including `construct_spacing_is_the_projected_gap` and `assemble_stays_cleared_when_construct_fills`. Those two do not read `order`. The suite still fails, so a correct placement with a wiped `order` cannot pass.

PS3.3 C.7.6.2.1.1 maps `(i, j, 0, 1)`. `i` is the column index and `j` is the row index. The third component is a literal zero, so the section does not define a slice index. Sorted position `n` is this crate's z. It is Image Position Patient projected on `X cross Y`, low to high. The new sentences use `n` as that position. They do not call `order[n]` a DICOM column index. `order[n]` is the candidate arrival that landed there.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git rev-parse HEAD^{tree}`, `git write-tree` | `sprint/s18` staged correction, before this file | executed now | HEAD `2af862735ee9fb10b9ee2812b56944fcb679845b`, HEAD tree `f258a04046ad788f3bb8f6ff4e5ce28cb8d45b7a`, staged tree `102b72c725a5abeaaab03b522f640867bab84772` |
| `git diff --cached --stat` | same index | executed now | 5 files, 15 insertions, 9 deletions. No unstaged diff |
| `bin/ocelli.sh test ocelli-volume --test construct` | staged correction, `order.clear` absent | executed now | exit 0, 10 passed |
| Same command after `volume.order.clear()` before `Ok(volume)` | `lib.rs` `construct`, then restored | executed now | exit 101. The three order assertions failed with `left: []`. 7 passed |
| `rg -n 'order.clear' crates/ocelli-volume/src/lib.rs` | restored `construct` | executed now | no match |
| `git diff --exit-code` and `git write-tree` | restored worktree | executed now | diff exit 0. Staged tree still `102b72c725a5abeaaab03b522f640867bab84772` |
| Same construct command after the restore | restored staged tree | executed now | exit 0, 10 passed |
| Python f64 of the AC1, AC2 and AC10 positions | fixture literals | executed now | orders `[1, 2, 0]`, `[2, 1, 0]`, `[1, 2, 0]`. AC1 inverse is `[2, 0, 1]` |
| `rg` for `sorted index \`order\`` and `index \`order\` names` | tracked tree including `.claude` | executed now | only the AS_BUILT note that names the old false wording and then corrects it |
| Pass 1 arrival-index `place` mutation | `construct` place call, unchanged in this diff | prior receipt, not re-run | exit 101. AC1 slice 0 `[1,1,1,1]` against `[2,2,2,2]`. Recorded in `.claude/reviews/S18-sprint-pass-1.md` |
| `rustc --version` | host toolchain | executed now | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `Volume::construct` still clones `order` and loops `(sorted, arrival)`. `place` receives `u32::try_from(sorted)`. The samples come from `candidate.slices.get(arrival)`. `arrival` is `order[sorted]`. The returned `volume.order` is the vector `assemble_from` built. Nothing clears it. The field doc at `lib.rs` lines 378 to 382 already said `order[n]` is the candidate index of sorted position `n`.
- The replaced doc comment, the LLD Construction sentences, and the design-plan signature comment now say the same thing. `place` writes at sorted position `n`. `order[n]` is the arrival index. The returned volume has every slice present, which is the `Ok` path after one `place` per sorted index.
- The plan's mutation sentence now says place at the arrival index instead of the sorted index, and that AC1 slice 0 becomes `[1,1,1,1]`. That is the pass 1 result. The old wording said "instead of `order[n]`", which named the arrival index as the destination. That wording is gone. The approach paragraph was already right: `place` uses `candidate.slices[arrival]` at sorted index `n`. It was not rewritten.
- `docs/lld/volumes.md` lines 73 to 76 were already right. `order[n]` is the arrival index of sorted position `n`, and `construct` copies samples to that position. The Construction section no longer says the samples are written at the sorted index `order` names.
- The state-table sentence "bytes at the sorted index" is the destination. It does not say `order` names that index. `order` still names arrival indexes in the invariant paragraph. Both match the loop.
- The LLD impact sentence now says samples are copied to sorted position `n` where `order[n]` is the arrival index, and that `assemble` stays cleared. `assemble` is still the one-line forwarder. It does not call `place`.
- The AS_BUILT note names the old false claim, states the arrival and sorted-position reading, and says the success tests lock `order`. The construct suite does lock it. A wipe fails AC1, AC2 and AC10. The note does not say the design plan was a third copy. The plan signature was that copy, and it is corrected in this same diff. The note is not false by leaving it out.
- Hand-computed orders match the three assertions. AC1 and AC10 are not the inverse permutation. The clear mutation shows those assertions run. The green rerun shows they match what `construct` returns.
- No new `as` cast, no `unsafe`, no `wasm-bindgen`, no cache insert, and no second projection. The behaviour of `construct` is unchanged apart from the doc comment. New Markdown and the new Rust doc comments have no em-dash and no prose semicolon.
- `volume.order.clear()` is absent. `git diff` is empty. The staged tree is still `102b72c725a5abeaaab03b522f640867bab84772`.

## Limitations and handoff

This pass did not re-run `bin/ocelli.sh gate --sprint`, clippy, wasm, the oracle or a GPU. The place-at-arrival-index mutation was not repeated. The `place` call is not in the staged behaviour diff, and pass 1 already recorded AC1 slice 0 becoming `[1,1,1,1]`. The new plan sentence matches that receipt.

AC3 and AC8 do not read `volume.order`. AC3's arrivals are already in projected order, so its vector would be `[0, 1, 2]`. A wipe is already red on the other three success tests. The `SliceOutOfRange` arms were not executed. They are unchanged and are not what this correction claims.

This file is the pass record. It was not staged, and `scripts/sprint_workflow.py record-sprint-review` was not run. The reviewed tree remains `102b72c725a5abeaaab03b522f640867bab84772`. Recording this file changes that tree and needs its own verification. This pass does not merge and does not close the sprint.
