# F-038 review, pass 3

**Reviewed**: working tree diff for F-038. New graph module and graph tests plus the extended recovery value in the GPU module, the crate exports, and the backlog status word. This pass first verified every pass 2 remediation in the current tree and then reviewed the whole diff again in microscope order.
**Result**: 0 defects, 1 smell, 1 nitpick

## Defects

None found in this pass.

## Smells

### S1, The chain validation in set_voi still runs before the registry checks and the error documentation still lists one variant

**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:641) and [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:630)
**What**: `set_voi` converts the chain through `VoiParams::from_chain` before it looks at the registry. The registered check, the stack check and the rebound slot check then run in the right order. A chain with no section 18.4 uniform therefore answers `Voi` for an id this graph never issued, for a registered volume viewport and for a stack viewport that has not been rebound. The `Errors` documentation on the method still names only the `Voi` variant and names none of `UnknownViewport`, `PassHasNoUniform` or `ViewportNotRebound`.
**Why it is wrong**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:185) defines `UnknownViewport` as an id this graph never issued and the other two variants carry their own definitions at the same site. Pass 2 smell S2 recorded the same precedence concern for the pass and slot edges. The state to state order is now correct and every case still returns an error so no wrong frame renders. A caller that matches on the variant to choose between reporting an unknown id, reporting a pass with no uniform and driving a rebind will still take the `Voi` branch for a sequence chain in all three states, and the documentation gives that caller no warning. That keeps this a smell rather than a defect.
**Evidence**: Reading the body from line 641 shows the chain conversion with its early return ahead of the registry lookup at line 650, the stack check at line 653 and the slot check at line 660. The three state assertions in [graph.rs tests](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/tests/graph.rs:489) all drive the method with a valid window chain built by the local `chain` helper, so no test separates chain precedence from state precedence. The pass 2 record of this ordering is in [F-038-graph-pass-2.md](/Users/atulsharma/Documents/projects/tensorbee/ocelli/.claude/reviews/F-038-graph-pass-2.md:20).

## Nitpicks

### N1, The submitted outcome counts passes by copying the viewport count

**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:799)
**What**: One pass lives in each slot, so `Submitted` carries one fact twice as `viewports` and `passes`. This is pass 1 nitpick N1 and pass 2 nitpick N1 unchanged. The field earns its place when a viewport holds more than one pass.
**Why it is wrong**: It is not wrong today. It reads as measured detail until a viewport can hold a second pass.

## Verified clean

Pass 2 defect D1 is remediated. The device test `set_voi_is_the_single_uniform_route_and_rejects_in_order` in [graph.rs tests](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/tests/graph.rs:480) asserts `UnknownViewport` for id 99 at line 489 and asserts `PassHasNoUniform` for a volume viewport at line 523. It then executes `set_voi` positively at line 538 with centre 240. Input 40 renders as 128 under centre 40 and width 400 and renders as 0 after the centre move, which matches DICOM PS3.3 C.11.2.1.2 where the lower bound under centre 240 is 40 exactly and the comparison at that bound clamps to the minimum. The recovery test `recovery_drops_viewport_objects_until_they_are_rebound` asserts `ViewportNotRebound` before any rebind at [graph.rs tests](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/tests/graph.rs:428). Both tests passed under the GPU gate in this pass.

Pass 2 smell S1 is remediated. [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:674) `install_recovered` has no early return on a missing graph value. It adopts the recovered tier at line 681, recompiles layouts and pipelines, drops every per viewport slot at line 694 and marks every registered viewport fully dirty. A `None` graph value and a `Some` graph value now share the one install path by construction because the body never branches on that field.

Pass 2 smell S2 is remediated in its state ordering. [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:644) checks registered state first and answers `UnknownViewport`, then checks the stack pass and answers `PassHasNoUniform` at line 654, then checks the rebound slot and answers the new `ViewportNotRebound` variant at line 660. The remaining chain precedence and documentation gap is smell S1 above rather than a failure of that ordering.

The mutation this pass was asked to prove went red and was reverted exactly. Deleting the `PassHasNoUniform` guard at the stack check left `bin/ocelli.sh test ocelli-render --test graph -- --ignored --test-threads=1` at exit 101 with `set_voi_is_the_single_uniform_route_and_rejects_in_order` failing on its `PassHasNoUniform` assertion at [graph.rs tests](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/tests/graph.rs:523) and the other four graph device tests passing. The source was then restored from the pre mutation copy. The hash before and after is `b4fe687dfdd48365b07d054398f51abefe0c4c7f149e577bbdc65386d3d1c5a0` and a byte comparison reports no difference. A restored run of the same graph device command exited 0 with all five tests passing.

Arithmetic was checked first in microscope order. The graph Rust contains no `as` cast. Every conversion uses a checked conversion with a named fallback that indexes outside the vectors or reports capacity, and the pipeline count fallback reports zero for a count of one in this story. No LUT formula is reimplemented in Rust. The composed shader reuses the shared `VOI_WGSL` text and calls its chain function, and that text was left untouched by this diff. Its LINEAR branch uses centre minus 0.5 and width minus 1, its LINEAR EXACT branch uses neither, its boundary operators are low end at or below and high end above, and its inversion reflects about the midpoint of the output range. The graph fragment entry normalises by the declared output range and writes black when that range is not positive, with the reason stated at the site. The GPU gate shader rows and the half float graph VOI rows passed on a real adapter in this pass.

Prose claims were executed where this pass could execute them. One `queue.submit` call site exists in the graph tick and nowhere else in the module, and the `submissions` counter beside it is asserted by the two viewport device test. The tick creates no vector and no formatted string, so the no allocation claim holds for Rust heap allocation in the frame path with the command encoder as the one driver handle the design allows. Shader and pipeline creation appear only inside the compile helper called from creation and from recovery install, never from the tick. The backlog status word reads in-progress.

Boundary and tier were checked by reading and by gate. No `wasm-bindgen` and no new `unsafe` appear in the new module. No pixel data crosses the boundary because frames render into caller supplied texture views and readback stays inside native test files. Tier A ran in full under the GPU gate. Tier B remains legal by construction through fragment pipelines, uniform buffers and `textureLoad` with no compute and no storage buffer, and no tier B execution is claimed. Tier C refuses at graph creation in code and the CPU LUT path in `ocelli-pixel` remains the tier C answer with no second copy behind a tier check.

Structure was checked against the standing rules. The diff adds no trait, no generic parameter and no dynamic dispatch. The `ViewportId` newtype removes a transposition case in the same style as the existing probe newtypes. The recovery value dropping from `Copy` to `Clone` touches no other workspace caller.

Commands were run with exit codes read from the command itself and never from the end of a pipe. Final runs on the restored tree gave `bin/ocelli.sh test ocelli-render` exit 0, `bin/ocelli.sh clippy ocelli-render` exit 0 and `bin/ocelli.sh gate gpu` exit 0 with ALL GREEN. The GPU gate passed the five graph device rows including the `set_voi` row, the two half float VOI rows, the device lifecycle rows and the nine shared shader rows on this machine adapter.
