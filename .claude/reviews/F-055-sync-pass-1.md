# F-055 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s20`, base `672277e6355f9b1917df0b4b3a6f47cb7ef649bc` (F-054). Nothing is staged. Tracked edits are `Cargo.lock`, `crates/ocelli-viewport/Cargo.toml`, `crates/ocelli-viewport/src/lib.rs`, `crates/ocelli-volume/src/lib.rs`, `docs/lld/README.md`, `docs/lld/viewport.md`, `docs/lld/volumes.md`, `docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md`. Untracked product files are `crates/ocelli-volume/src/sync.rs`, `crates/ocelli-volume/tests/sync.rs`, `crates/ocelli-viewport/src/sync.rs` and `crates/ocelli-viewport/tests/sync.rs`. This review file is the only path this pass adds.
**Author / reviewer**: The product diff is uncommitted, so a commit does not name its author. This pass is an independent reviewer and did not write that diff. The reviewer mutated `index_through_point` twice, ran temporary probes, restored every product and test edit, and then re-ran the suite.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. Anchor `(0, 0, 10)`, normal `(0, 0, 1)`, spacing `1`, index `0`. PS3.3 C.7.6.2.1.1 with `k = 0` puts both `(0, 0, 0)` and `(4, 7, 0)` on `z = 0`. The offset along `(0, 0, 1)` is `-10`, so the index is `0 + (-10 / 1) = -10`. World z of that index is `10 + (-10) * 1 = 0`. A sign error is `10`. Adding `x` for `(4, 7, 0)` is `-6`. The stored plane is not written.

AC7. Anchor origin, normal `(1, 0, 0)`, spacing `1`, index `3`, point `(0, 0, 5)`. Plane x is `3`, the offset is `-3`, and the index is `0`. Substituting `(0, 0, 1)` for the stored normal yields `5`. The in-plane `5` mm is not the offset.

Decision 2, against a normal the permanent fixtures cannot tell from a unit vector. Normal `(2, 0, 0)`, spacing `1`, index `3`, point `(0, 0, 5)`. Origin x is `6`, offset is `(0 - 6) * 2 = -12`, and the index is `-9`. Normalising to `(1, 0, 0)` yields `0`. Applying the unit simplification `((point - anchor) dot normal) / spacing` also yields `0`. The same normal at index `0` and point `(4, 0, 0)` is `8`, not `4`.

Decision 1 and AC6, against the stale approach sentence that names only later slots. Registration order is id `7` at anchor z `10`, id `8` at anchor z `0`, id `9` at anchor z `20`, one UID. `set_point` of `(4, 7, 0)` from id `8` must change id `7` to `-10` and id `9` to `-20`, leave id `8` at `0`, and list `7` then `9`. A loop from the source index forward would leave id `7` at `0`.

AC5's result check, which the permanent refusal fixture does not reach. Index `1e308` and spacing `1e308` are finite, the quotient is not, and the stored index must stay `1e308`. Normal `(1e308, 1e308, 0)` has finite components and a non-finite length, so it is refused before the division. An earlier same-UID peer with spacing `0` refuses the later source and writes neither slot.

AC2, AC3, AC4, AC6, AC8 and the scene half of AC5 are the permanent viewport tests. They reject a trimmed UID, a shared default group, a peer without a UID joining or failing the call, a second apply of the offset, a cleared slot after refusal, and a write of the valid peer before a later spacing of `0` is seen.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume -- sync` | Restored tree after both mutations and after the probes were removed | Executed now | Exit 0. `sync::matching_world_z`, `sync::sagittal_uses_its_own_normal`, `sync::refuses_before_a_quotient` passed. Other volume binaries reported 0 matching tests |
| `bin/ocelli.sh test ocelli-volume -- sync::matching_world_z` | Temporary `self.index - offset / self.spacing_mm`, then restored to `+` | Executed now | Exit 101. `left: 4621819117588971520` (`10`) `right: 13845191154443747328` (`-10`) at `tests/sync.rs:21` |
| `bin/ocelli.sh test ocelli-volume -- sync::sagittal_uses_its_own_normal` | Temporary `DVec3::Z` in the origin and the offset, length check left on `self.normal`, then restored | Executed now | Exit 101. `left: 4617315517961601024` (`5`) `right: 0` at `tests/sync.rs:43` |
| `bin/ocelli.sh test ocelli-viewport -- sync` | Restored tree | Executed now | Exit 0. 7 tests passed: `same_uid_moves_other_uid_stays`, `missing_source_uid_moves_nobody`, `missing_peer_uid_does_not_join`, `stages_then_refuses`, `repeat_is_empty_then_a_new_point_moves`, `signed_zero_is_the_same_index`, `refusal_and_register_preserve_then_retry` |
| Temporary `probe_non_unit_keeps_the_stored_normal_and_index` and `probe_non_finite_result_and_length_refuse` | Added to `crates/ocelli-volume/tests/sync.rs`, run, then deleted | Executed now | Exit 0. Index `-9` for the length-2 normal, `8` for the flat case, and `Unavailable` for the overflowing quotient, the non-finite length and infinite spacing. The held index bits were unchanged |
| Temporary `probe_earlier_peer_moves_and_a_bad_earlier_peer_refuses` and `probe_signed_zero_still_reports_a_real_move` | Added to `crates/ocelli-viewport/tests/sync.rs`, run, then deleted | Executed now | Exit 0. Change list was id `7` at `-10` then id `9` at `-20`. The spacing-0 earlier peer left indexes `4` and `0`. A stored `-0` moved to `5` and was reported |
| `bin/ocelli.sh clippy ocelli-volume` and `bin/ocelli.sh clippy ocelli-viewport` | Restored tree. `cargo clippy -p <crate> --all-targets -- -D warnings` | Executed now | Exit 0 for each |
| `cargo fmt -p ocelli-volume -p ocelli-viewport -- --check` | Restored tree | Executed now | Exit 0 |
| `cargo tree -p ocelli-viewport --depth 1` and `cargo tree -p ocelli-volume --depth 1` | Restored manifests | Executed now | Exit 0. Viewport depends on `ocelli-volume`. Volume's direct dependencies are `glam`, `ocelli-cache`, `ocelli-core`, `ocelli-pixel` and `thiserror` |
| `git diff --name-only` plus a search for `DVec3::Z`, `index - offset` and `probe_` | Restored tree | Executed now | Forbidden paths are absent. The product sources do not contain those mutation or probe strings. `DVec3::ZERO` in the volume refusal test is the original fixture |

Host is macOS, zsh, `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Exit codes are the cargo status, not the status of a pipe.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The quotient is the formula in decision 1, not the stale "every later slot" sentence in the design approach. `crates/ocelli-volume/src/sync.rs` lines 57 to 64 compute `anchor + index * spacing_mm * normal`, dot the point offset with that same `self.normal`, and return `index + offset / spacing_mm`. There is no division by `normal.length()`. The call takes `&self`. AC1's test compares both results with the bits of `-10` and checks that index `0` and anchor z `10` are still stored. Replacing `+` with `-` made that test report `10`. Restoring `+` made it pass.

AC7 uses the row cosine. Replacing `self.normal` with `DVec3::Z` only in the origin and the offset made the sagittal test report `5`. The length gate still saw `(1, 0, 0)`, so the failure is the substituted axis and not an early refusal. The normal was restored and the test passed.

Refusal happens before the division. Lines 42 to 56 return `Unavailable` when any anchor, normal, index or point component is non-finite, when spacing is non-finite or not greater than `0`, or when the length is non-finite or not greater than `0`. There is no epsilon. Lines 65 to 67 also refuse a non-finite quotient. The permanent fixture covers NaN point, infinite anchor, NaN normal, NaN index, spacing `0`, spacing `-1` and a zero normal, and it still gets `-10` from the original plane afterwards. The deleted probe covered the overflowing quotient and a finite-but-overflowing length.

`SyncScene::set_point` in `crates/ocelli-viewport/src/sync.rs` walks every slot. A missing peer UID is skipped. A different string, including `1.2.3 ` with a trailing space, is skipped. `FrameOfReferenceUid` is `PartialEq` on the owned `String` (`crates/ocelli-volume/src/lib.rs` lines 38 to 43). There is no trim. The source is included. The deleted probe showed an earlier peer move to `-10` and a later peer move to `-20` in registration order while the source stayed `0`. That is the state decision 1, the state table and AC6 require, and it is the state the stale approach sentence would miss.

Staging finishes before any index write. `PlaneSyncError` returns at line 96, before the change list is built and before lines 112 to 118 assign `slot.plane.index`. `stages_then_refuses` keeps the valid peer at `0` when the later same-UID spacing is `0`. The deleted probe did the same when the bad spacing was the earlier peer. Unknown id `9` stays absent. A missing source UID returns `Unavailable` and leaves the peer at `0`. After a move to `-10`, a `None` source refusal and a register of id `9` leave that peer at `-10`, and a later point then sets the source to `10` and the peer to `0`.

`same_index` treats a zero difference, including `-0`, as equal, and the slot is neither listed nor rewritten. `signed_zero_is_the_same_index` keeps the bits of `-0`. A stored `-0` against point z `5` was reported as index `5` in the deleted probe, so a real move is not swallowed. For finite values the predicate matches `==`, which is what AC6 names. The two signed zeros were checked explicitly. `register` replaces one id and returns without touching another slot. Only `plane.index` is assigned on a successful set, so the anchor stays the world point of index `0`.

`Volume` at `crates/ocelli-volume/src/lib.rs` lines 430 to 452 has no `FrameOfReferenceUid` and no `HeldPlane`. `group_volumes` still compares the UID only while grouping. `sync.rs` does not call `reslice`, `assemble` or `voxel_world`. `ImagePlane::slice_normal` normalises the cross product, and `Volume::assemble` stores that vector as the third direction column, which is the LLD sentence about the slice column. The new LLD sections in `docs/lld/volumes.md` and `docs/lld/viewport.md` match these paths, including every same-UID slot, no second normalise, no clamp, and the same quotient on tier A, tier B and tier C because there is no tier argument.

No new shader. No `wgpu` device in either crate. `git diff --name-only` does not contain `ortho.rs`, `graph.rs`, `raycast.wgsl` or `tools/oracle/volume-params.json`. The new modules have no `unsafe`, no `as` cast, and no `unwrap` or `expect`. Clippy denies those and `float_cmp`, and both crates passed `--all-targets`. `ocelli-viewport` depends on `ocelli-volume`. The volume crate does not depend on the viewport crate, so the edge does not cycle. `ocelli-cache` does not depend on the viewport crate either.

The ledger edits mark F-055 `in-progress`. This pass did not edit them and does not treat that status as completion.

## Limitations and handoff

`bin/ocelli.sh check ocelli-wasm`, `bin/ocelli.sh wasm`, the oracle, the corpus and `bin/ocelli.sh gate` were not run. There is no adapter result here, and AC9 does not ask for one. Native tests and clippy are the evidence for the quotient.

The permanent suite always calls `set_point` on the earliest same-UID id. The earlier-peer move, the earlier spacing-0 refusal, the length-2 normal and the non-finite quotient were run as temporary tests and then removed. They are not protection in the tree that remains. Replacing an already registered id was read at `register` and was not given its own run. The function returns after writing that one slot.

This pass found no product defect to remediate. It is not story completion and it is not a sprint-gate receipt. The next owner can take the restored tree. No mutation remains in the product sources.
