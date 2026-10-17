# F-038 review, pass 1

**Reviewed**: working tree diff for F-038. New graph module and graph tests plus the extended recovery value in the GPU module, the crate exports, and the backlog status word.
**Result**: 4 defects, 6 smells, 2 nitpicks

## Defects

### D1, Recovery reinstalls pipelines and leaves every per viewport GPU object on the dead device
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:526)
**What**: `install_recovered` recompiles the module, layouts and pipelines on the new device and marks every viewport fully dirty. It never touches the per viewport slot vector. Each slot still holds the uniform buffer, both bind groups and the target view created on the old device at registration. The next tick binds those old device objects into a pass on the new device encoder.
**Why it is wrong**: HLD section 22 requires rebuild of the device and all resources. The approved design section 7 returns the rebuilt resources in `Recovered` for the caller to install so nothing is rebuilt twice and the following tick re-renders every viewport. A tick after this install uses handles the new device does not own, which is a validation failure or a lost frame on the exact path device loss handling exists to save.
**Evidence**: Reading the body at lines 526 to 539 shows assignments to module, layouts and pipelines only, with no write to the slot vector. A workspace search for `install_recovered` finds no caller and no test outside its definition, so no gate executes this path. The design device row for loss observed through the graph is absent from the graph tests.

### D2, The one submission rule has no test that can fail
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/tests/graph.rs:222) and [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:630)
**What**: The production tick has exactly one submit call site and that half is sound. No test observes a submission count. The graph tests register one viewport per graph in each device test, so the two viewport frame the design requires does not exist.
**Why it is wrong**: HLD section 22 and section 7 both state one `queue.submit()` per frame across all viewports. The microscope rule is that a test passing both ways is a defect because it will be counted as coverage forever. The design mutation list predicts the per viewport submit mutation goes red on the two viewport row. That row is missing, so the prediction is false today.
**Evidence**: Mutation M3 moved encoder creation and submit inside the per viewport loop and removed the single submit after the loop, then reverted exactly. `bin/ocelli.sh test ocelli-render --test graph -- --ignored --test-threads=1` exited 0 with both device tests passing, and the floor run of the same file also exited 0. Search counts two `graph.register` calls in the test file, one in each separate device test.

### D3, Tier isolation in the pipeline lookup has no test that can fail
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/tests/graph.rs:37) and [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:400)
**What**: The unit test asserts a tier B key is not equal to a tier A key as a value. It never performs a lookup. The helper that reports whether a key is compiled also ignores tier. Dropping tier from the lookup comparison changes no test result.
**Why it is wrong**: HLD section 22 keys pipelines by pass kind, blend mode and tier. The design states tier is in the key so a rebuild cannot hand back a pipeline for a tier the session no longer runs and so later tier B variants need no key change. The design mutation list predicts this mutation goes red because a tier B lookup returns the tier A pipeline. Value inequality does not prove that.
**Evidence**: Mutation M4 changed the lookup to compare pass and blend only, then reverted exactly. The floor graph run exited 0, the ignored device graph run exited 0, and the full `bin/ocelli.sh test ocelli-render` run exited 0 with 92 passed and 20 ignored. No test failed.

### D4, The graph fixture set cannot distinguish the VOI functions it claims to render
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/tests/graph.rs:4) and the missing file `crates/ocelli-render/tests/graph_voi.rs` named by the design
**What**: The only rendering fixture drives LINEAR and asserts 8 bit values 0, 128, 255 and 64. Hand computed LINEAR EXACT values for the same inputs quantise to the same four bytes. Row one is 0.000 for both functions once deviation D-13 is applied, the centre values 127.819 and 127.500 both quantise to 128, the upper row is 255 for both, and 63.910 and 63.750 both quantise to 64. A graph that always evaluated LINEAR would pass this fixture with LINEAR EXACT selected. The design fixture file covering the 32 byte uniform readback, LINEAR EXACT, SIGMOID and inversion through the graph does not exist.
**Why it is wrong**: DICOM PS3.3 C.11.2.1.2 against C.11.2.1.3.2 is the arithmetic that reaches patients, and the centre divergence of 0.32 in 255 is invisible after quantisation. The shader text itself is guarded at f32 precision by the older shader tests. The new production composition, its uniform plumbing through registration and its function selection are not guarded at any precision that separates the functions.
**Evidence**: File listing shows no `graph_voi.rs` beside `graph.rs` in the render tests directory. The graph test function list is two unit tests and two device tests only. Baseline `bin/ocelli.sh gate gpu` exited 0, and its shader rows at f32 precision passed, which locates the gap in the graph composition rather than in the shared shader text.

## Smells

### S1, Two sources of truth for whether a pipeline exists
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:285) and [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:400)
**What**: A public helper answers compiled from pass and blend alone while the tick answers by searching the compiled vector with the full key. Only tests call the helper. When a later story adds a shader body, both places must change and nothing fails if one is missed.

### S2, The single VOI route is a convention the public API breaks
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:478)
**What**: `mark_dirty` accepts a full dirty value including the VOI flag, so a caller can schedule a window frame without writing any uniform bytes. The tick then renders the old window and clears the flag. The design section 3 rule is that VOI marks are set by the uniform write path and by no other route. The same method also lets the write path mark VOI dirt on volume and overlay passes whose uniform is never bound.

### S3, The output normalisation changes arithmetic for small ranges without a stated reason
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:214)
**What**: The fragment entry divides by the larger of the output range and 1.0. For every range the fixtures use this is the plain range and the result is right. For an output range below 1 the frame is silently dimmed instead of normalised, and no comment or test records that choice. The quietly wrong pixel is this project stated defect class, so an unstated arithmetic branch on the pixel path belongs in the open.

### S4, Viewport slots can never be released
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:238)
**What**: The registry can register and mutate slots and has no removal path. The design section 3 rule that removal clears the slot has no method and no test. A long session that registers viewports against the fixed capacity exhausts it permanently, and the refusal is correct while the leak is not.

### S5, A mixed frame hides the unavailable viewport and pass order is asserted in prose only
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:551)
**What**: The tick records the first unavailable viewport, then returns the submitted outcome whenever any viewport encoded, discarding that record until a later tick. A slot also holds exactly one pass, so the overlay ordering sentence on the pass type has no ordering code behind it beyond registration order. Both behaviours are defensible readings of the design and neither is pinned by a mixed frame test.

### S6, Creation reports errors it cannot produce and builds an encoder it may throw away
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:289)
**What**: Creation returns a result whose error arm its own documentation says never fires, and tier C receives no named unavailable answer at creation. The safety of tier C rests on no context existing, while the context constructor is public and accepts any capabilities value. In the tick, an encoder is created before the loop even when every dirty viewport turns out unavailable, then dropped unfinished. Neither costs correctness today and both blur the contract the next story will build on.

## Nitpicks

### N1, The submitted pass count can only ever repeat the viewport count
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:631)
**What**: One pass lives in each slot, so the two counts in the submitted outcome carry one fact twice. The field earns its place when a viewport holds more than one pass and reads as measured detail until then.

### N2, The shader module handle is stored and never read
**Where**: [graph.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-render/src/graph.rs:269)
**What**: The module is kept in the graph and replaced on recovery, but no code reads it after pipeline creation. Keeping it alive is harmless and unexplained.

## Verified clean

Arithmetic and casts were checked first. The graph Rust contains no `as` cast. Every conversion is a checked conversion with a named fallback, covering slot index to viewport id, viewport id back to an index, pipeline count to `u32`, and loop index to viewport id in the unavailable record. The fallbacks index outside the vectors or report capacity, and the only lossy one reports zero for a pipeline count that is one in this story. The WGSL entry converts fragment position to texel coordinates by truncation, which is the intended pixel centre mapping and is exercised by the rendering fixture.

No VOI formula is reimplemented. The Rust computes no window value and builds the uniform only through the existing chain reader. The composed shader reuses the shared shader text and calls its chain function, and that text was left untouched by this diff. Its three formulas, boundary operators and inversion reflection were verified in place against the design transcription. Deviation D-13 row one at 0.000 for LINEAR EXACT is honoured by the shared shader tests, which passed on a real adapter in the GPU gate. The gap in D4 is the graph level fixture precision, not a second formula.

Two mutations went red as required. M1 made the lookup fall back to the first compiled pipeline on a miss. The unavailable blend device test failed with a submitted outcome where unavailable was required, exit 101, then reverted exactly. M2 cleared dirt per viewport before the pipeline lookup instead of after encode. The same device test failed on its second tick with a no-op outcome where unavailable was required, exit 101, then reverted exactly. M3 and M4 stayed green and are recorded as D2 and D3. All four mutations were reverted and the restored graph source matches its pre mutation copy byte for byte.

Prose claims were executed. One submit call site exists in the graph tick and nowhere else in the module. The tick body contains no vector creation, formatting, push, collect or clone, so the no allocation claim holds for Rust heap allocation in the frame path, with the command encoder as the one driver handle the design allows. Pipeline and shader creation appear only inside the compile helper, called from creation and from recovery reinstall, never from the tick, so no mid frame compile path exists. The backlog gate confirms the status word change to in-progress agrees with the sprint records.

Boundary and tier were checked by gate and by reading. The bindgen gate exited 0 with wasm-bindgen confined to the wasm crate, and the unsafe gate exited 0 with no new unsafe outside the allowlist. No pixel data crosses the boundary in the new code. Frames render into caller supplied texture views and readback exists only inside the native test file. Tier A runs in full and its device tests passed on this machine adapter. Tier B is legal by construction only, through fragment pipelines and uniform buffers with no compute and no storage buffer, and no execution is claimed for it. Tier C constructs no graph on the sanctioned path because no context can be opened for it.

Structure was checked against the standing rules. The diff adds no trait, no generic parameter and no dynamic dispatch. The viewport newtype removes a transposition case in the same style as the existing probe newtypes. The recovery value dropping from copy to clone touches no other workspace caller, verified by a workspace search that finds the type only inside this crate.

Commands run with exit codes read from the command itself. `bin/ocelli.sh test ocelli-render` exited 0 with 92 passed, 0 failed and 20 ignored across the crate. `bin/ocelli.sh clippy ocelli-render` exited 0. `bin/ocelli.sh gate gpu` exited 0 with 20 device tests passing, including both graph device tests and the nine shared shader tests. The format, backlog, bindgen and unsafe gates each exited 0.
