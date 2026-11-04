# S42 review, pass 2

**Reviewed**: the staged remediation on `sprint/s42` against HEAD `fb1381d`, read from `/tmp/ocelli-s42-remediation.diff`. I opened the current source for `crates/ocelli-dicom/src/series_open.rs` (the sharing key and `object_key`), `crates/ocelli-render/src/cpu.rs` (`reserve_source`, `fill_display`, `fill_rgb_planes`), `crates/ocelli-render/src/graph.rs` (`set_stack_interpolation`, `show_ortho`, the camera write sites), `crates/ocelli-render/tests/ortho.rs` (the GPU helpers), `tools/oracle/src/produce_current.rs` (`raster_colour`), `crates/ocelli-wasm/src/session.rs` (`draw_cpu_rgb`), `scripts/gen_sprint_plan.py` (the preamble) and the headers of `docs/lld/{render-graph,feature-availability}.md`. I did not need the full sprint diff, because no remediated contract has a consumer outside the files above.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **S3 in both call orders.** The fix refuses `show_ortho` when the stack mode is already linear. I checked the other order, where a plane is uploaded under nearest and linear is selected afterwards. That order is not guarded. This is the smell below.
- **S2 against every `reserve_source` caller.** The oracle's `raster_colour` (`produce_current.rs:2154`) and the wasm `draw_cpu_rgb` (`session.rs:595`) both reserve while nearest is selected and draw nearest. Under the new rule neither one grows the display plane, and neither one draws linear. The oracle receipt of 0 omitted fits this. Mono linear reads the same plane through `fill_display` (`cpu.rs:754`), and it still needs a reserve made while linear is selected. `linear_without_a_reserve_refuses_and_does_not_allocate` covers that.
- **S1 sharing key lifetime.** `object_key` is the `ParsedDicom` address (`series_open.rs:994`). The map lives inside one `assemble_display` call while every `ParsedFrame` still borrows its object. Two live objects therefore cannot share an address, and the key is the same one `uid_identity` already uses for absent UIDs. The first frame of an object does the projecting, so a projection error still carries that frame's input index, as it did before.
- **Whether the new tests would go red.** Remove the map and the projection test counts 2 instead of 1. Grow the plane on every reserve and the nearest colour test fails its first `allocations() == 1`. Remove the CPU guard and the plane test returns `Ok` and writes `out`. Remove the GPU guard and `show_ortho` uploads, or else returns `PlaneIncomplete`, and both are a mismatch with `SecondInterpolant`.
- **D2 ranges everywhere.** I grepped `docs/`, `scripts/`, `README.md`, `CLAUDE.md` and `WORKFLOW.md` for any remaining S01 to S41 or S42 to S72 range. Only the pass 1 report has one.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read the remediation diff | `/tmp/ocelli-s42-remediation.diff` | executed now, read only | findings below |
| Grep for `reserve_source(`, `set_interpolation(`, `set_stack_interpolation(`, `show_ortho(`, `present_cpu_plane(` and `OrthoError::` | worktree, excluding `target` | executed now | two production reserve callers, both nearest. No production caller selects linear |
| Grep for `ortho` in `graph.rs` | `crates/ocelli-render/src/graph.rs` | executed now | no per-viewport record of an orthographic source |
| Grep for `cpu_allocations()` and `allocations(), 2` | worktree | executed now | the only remaining `2` is `tests/cpu.rs:502`, which is the second reserve and is intended |
| Grep for the phase ranges | `docs/`, `scripts/`, top-level docs | executed now | `SPRINT_PLAN.md:7-8` and `gen_sprint_plan.py:44-46` agree on S01 to S44 and S45 to S75 |
| Grep for `F-IDs that contributed` | `docs/lld/*.md` and `scripts/` | executed now | `render-graph.md:3` lists F-194. `feature-availability.md:3` does not. No script checks this |
| `gate --sprint` ALL GREEN, 31 gates | host | prior receipt | not re-run |
| `compare produce`: 102 written, 0 omitted, 2 unsupported | host | prior receipt | not re-run |
| `compare gate`: 102 views, 71 pass, 0 fail, 31 unmeasured, verdict pass | host | prior receipt | not re-run |

## Defects

None.

## Smells

### S1, a linear mode selected after an orthographic upload still blends that plane a second time

**Where**: `crates/ocelli-render/src/graph.rs:3401` (the new guard in `show_ortho`) and `graph.rs:2445-2483` (`set_stack_interpolation`).
**Mechanism**: the guard runs only when the plane is uploaded. The sequence is `show_ortho` while nearest is selected, which uploads the plane, then `set_stack_interpolation(Linear)`. `set_stack_interpolation` checks only that the pass is a stack and that `available(mode, tier)` holds. It then writes the lane and marks the camera dirty. Nothing in the graph records that the stored texture holds a host reslice, and grepping `graph.rs` for `ortho` finds only the import and `show_ortho`. The next `tick` therefore draws the resliced plane through the linear branch. That is the second interpolant the remediation exists to refuse. It falsifies the new `render-graph.md:41` sentence and the "There is no second interpolant" clause in `feature-availability.md:35` for tiers A and B. Tier C is not affected, because `present_cpu_plane` checks the mode on every call and the raster holds no orthographic source between calls. No production code calls `set_stack_interpolation` today, so this is a smell and not a defect, for the same reason pass 1 gave for S3.
**Smallest regression**: `show_ortho` records on the viewport state that its stored texture is an orthographic plane, and the next stack upload clears that record. `set_stack_interpolation` refuses `Linear` while the record is set, leaves the mode at nearest and writes no lane. A test then calls `show_ortho` under nearest, then `set_stack_interpolation(Linear)`, and expects the refusal with `uploads() == 1`. Ignore that test without an adapter, like the others in `tests/ortho.rs`.

## Nitpicks

1. `docs/lld/feature-availability.md:3`. The `F-IDs that contributed` line does not list F-194, although the follow-up edited row `:35` and the AS_BUILT follow-up names this file under LLD updated. `render-graph.md:3` does list F-194.
2. `crates/ocelli-render/src/cpu.rs:294`. The summary line now says "Reserve colour scratch for one source grid". A mono linear draw also depends on this call for the display plane (`fill_display` at `:754`). The paragraph below the summary says so, but the summary alone reads as colour only.
3. `crates/ocelli-dicom/src/series_open.rs:297`. The doc comment breaks after "A missing Image Position" and continues with "Patient is unpositioned" on the next line. The edit was not reflowed.
4. `docs/sprints/AS_BUILT.md:4831`. The follow-up heading is F-194, but its first paragraph also records the `series_open.rs` sharing change, and that code is F-192. The F-192 entry has no pointer to this change.

## Verified clean

- **D1.** All three copies of the M19 goal (`allocation.json:119`, `BACKLOG.md:325` and `SPRINT_PLAN.md:611`) now name the example viewer. The folder viewer is not touched.
- **D2.** Phase 1 is S01 to S44 and Phase 1.5 is S45 to S75 in both `SPRINT_PLAN.md` and the generator preamble. The totals are still 118 and 397, and 39 and 352. The edited paragraph places F-191 to F-201 in Phase 1 and does not contradict the range.
- **S1 from pass 1.** Each distinct `ParsedDicom` is projected once in `assemble_display`. Two distinct objects are projected twice, and the test asserts both counts. `declared_frame_count` still projects on its own call (`:285`). Grouping still keys absent UIDs by `object_key`, so the result is unchanged. `read_cine` reads the shared `Arc` through deref (`:703`). The counter is `cfg(test)` and thread local, so it adds nothing to the release build.
- **S2 from pass 1.** A nearest reserve increments allocations once. A linear draw after a nearest reserve returns `Extent`, leaves `out` alone and does not allocate. Only a reserve made while linear is selected grows the plane. `set_interpolation` still does not allocate. Both wasm session colour tests expect 1, and the settled-idle colour test expects 1.
- **S3 from pass 1, in the order it names.** `present_cpu_plane` and `show_ortho` return `SecondInterpolant` before the image is read. They do not change the mode, do not write `out` and do not upload. There is no substitution with nearest. The GPU test carries the same `#[ignore]` reason as its neighbours. An unknown id in `show_ortho` falls through to the existing error path.
- **One LUT chain and no new pixel arithmetic.** The remediation adds no mapping and does not touch the 0 and 138.75 values at `(0.5, 0.5)`.
- **Boundaries.** There is no new `unsafe`, no `wasm-bindgen` outside `ocelli-wasm`, and no new `as` cast in the remediation.
- **Prose rules.** I found no em dash and no prose semicolon in the new docs, the AS_BUILT entry, the test ignore strings or the code comments.
- **The F-194 AS_BUILT entry** is unchanged, and the follow-up entry carries the correction, as the brief requires.

## Limitations and handoff

- I ran no test, no gate, no mutation and no shader compiler. The 31-gate and compare receipts belong to the host. The new GPU test is ignored on the floor, and I cannot confirm whether the sprint profile ran it on an adapter.
- The smell above is reasoned from the code. I did not execute that call order.
- I did not reopen pass 1's four nitpicks. The remediation did not make any of them false. Pass 1's nitpick 3 still holds, because `feature-availability.md` still has no row for `available(mode, tier)`.
- Next: the host addresses S1, and a fresh pass 3 reviews that change. This pass is not sprint closure.

S42, pass 2: 0 defects, 1 smell, 4 nitpicks.
