# F-041 review, pass 1

**Reviewed**: working tree, staged set `git write-tree` =
`2cdd887e80180c47b01308b7b20102ab890de281`, against base `d33ed44` (F-037).
Independent implementation review. Reviewer did not write the code.
**Machine**: resolves tier A, real adapter present, `bin/ocelli.sh gate gpu`
runs.
**Result**: 5 defects, 4 smells, 4 nitpicks

---

## Defects

### D1, the `<=` continuity claim is false at the legal LINEAR width of 1, and the operator mutation IS catchable

**Where**: `crates/ocelli-render/shaders/voi.wgsl`, the comment block above
`fn voi_window`, and the same argument in `.claude/scratch/F-041-progress.md`
under "Mutations 1 and 2 are green and that is arithmetic rather than a gap".

**What**: the shader states, as a general mathematical fact, that

> The linear formula is CONTINUOUS at its own breakpoints. At `x = c' - w'/2`
> the body evaluates to `(-0.5 + 0.5) * range + ymin`, which is `ymin` ... So at
> the one input where the two operators disagree, the clamp and the formula
> return the same number

and concludes that no test can catch `<=` becoming `<`, so none is written.

**Why it is wrong**: the algebra `(-w'/2)/w' = -0.5` requires `w' != 0`.
`w' = w - 1`, and PS3.3 C.11.2.1.2 and `VoiTransform::new`
(`crates/ocelli-pixel/src/lut.rs:277`, `VoiFunction::Linear => *width >= 1.0`)
both accept `w = 1`. At `w = 1` the two breakpoints collapse onto `c'` and the
body is `0 / 0`. With `<=` the clamp fires and returns `ymin`. With `<` the
division is reached and WGSL returns **NaN**. The shader's own header
acknowledges this width in the paragraph explaining why `select` is not used
("`wp` is zero at the legal width of 1"), so the file contradicts itself.
`crates/ocelli-pixel/tests/voi.rs:85-97` already pins `w = 1` on the CPU
(`width_one.apply(Modality(-0.5)) == 0.0`). **No GPU test uses a width other
than 400, 200 or 80**, so the one input that separates the operators is absent
from the GPU suite.

Consequences, all three of which block:
1. A false factual claim in a tracked source file and in the progress note.
2. The conclusion drawn from it ("that is arithmetic rather than a gap")
   suppressed the test that would have made `<=` load bearing.
3. `the_voi_boundaries_are_lower_inclusive_and_upper_strict_on_the_gpu` claims
   in its name and its doc comment to pin the asymmetry. It pins neither
   operator: both mutations leave it green.

**Evidence**:

```
# baseline, width 1, LINEAR, c = 0, range [0,255], inputs [-0.5, -0.499, -0.501, 0.0]
WIDTH ONE LINEAR gpu = [0.0, 255.0, 0.0, 255.0]

# mutation: `if (x <= cp - wp / 2.0)` -> `if (x < cp - wp / 2.0)`
WIDTH ONE LINEAR gpu = [NaN, 255.0, 0.0, 255.0]
```

Full suite under each of the four operator mutations (`cargo test -p
ocelli-render -- --ignored --test-threads=1`):

| Mutation | Result |
|---|---|
| LINEAR lower `<=` -> `<` | GREEN, exit 0 |
| LINEAR upper `>` -> `>=` | GREEN, exit 0 |
| LINEAR_EXACT lower `<=` -> `<` | GREEN, exit 0 |
| LINEAR_EXACT upper `>` -> `>=` | GREEN, exit 0 |

The narrower true statement is that three of those four really are
unobservable on finite inputs (LINEAR_EXACT has `w > 0` so its denominator is
never zero, and the upper operator is shadowed by the lower clamp at
`w = 1`). The fourth is observable and is the one the standard's `<=` exists
for.

---

### D2, the shader's stage 1 is executed by no test

**Where**: `crates/ocelli-render/shaders/voi.wgsl`, `fn lut_chain`, line
`let m = stored * voi.slope + voi.intercept;`

**What**: deleting stage 1 entirely leaves the whole GPU suite green.

**Why it is wrong**: HLD section 18.1 and PS3.3 C.11.1 are the shader's own
stage 1, and the plan's item 2 under "What the specification does not cover"
makes it explicit that "the shader's input is a **Stored** value and the shader
runs stage 1 as well as stages 2 and 3". Every chain built in
`tests/voi_shader.rs` uses `ModalityTransform::new(None, Some(1.0), Some(0.0))`
(the `chain()` helper) or the same identity inline in
`inversion_reflects_about_the_midpoint_of_a_non_zero_range`, so `slope = 1` and
`intercept = 0` in every dispatch and the stage is the identity in every
comparison, including the 4096-value sweep. This is microscope class 4, a
branch that is present, looks authoritative, and is never meaningfully reached.
`tests/voi_params.rs` proves only that the host-side struct carries the numbers,
not that the shader consumes them.

**Evidence**: mutation `let m = stored * voi.slope + voi.intercept;` ->
`let m = stored;`

```
=== K: stage 1 ignores slope/intercept: GREEN (exit 0)
```

A probe with `slope: 2.0, intercept: -1024.0` and input 532 returns 127.5 on
the real device, so the code is correct today. Nothing would report it if it
stopped being.

---

### D3, the only floor-runnable WGSL check does not check what its own comment says it checks

**Where**: `crates/ocelli-render/tests/voi_params.rs`,
`the_shader_declares_section_18_4s_uniform`.

**What**: the test comments say

> The eight fields, in section 18.4's order. `find` from a moving cursor, so a
> shader declaring them in a different order fails even though every name is
> present.

It does not fail. Swapping any adjacent pair in the WGSL `struct VoiParams`
leaves the no-GPU suite green, because every field name already appears in
`voi.wgsl`'s prose header ahead of the struct declaration and the moving cursor
is consumed there.

**Why it is wrong**: this is a false prose claim, and it is the *only* check on
the WGSL that runs without an adapter. Deviation D-04 leaves CI without one, so
on the CI floor the section 18.4 layout is guarded by nothing.

**Evidence**: `cargo test -p ocelli-render` (no `--ignored`, i.e. what the floor
runs) under each WGSL struct mutation:

```
=== P: WGSL struct ymin/ymax swapped (NO GPU suite): GREEN
=== Q: WGSL struct slope/intercept swapped (NO GPU suite): GREEN
=== R: WGSL struct center/width swapped (NO GPU suite): GREEN
```

The same mutation with `--ignored` on this machine's adapter is red on five of
six tests, so the property is covered by the `gpu` gate and only by it.

---

### D4, "Both deletions are red, on the section 18.3 rows and on the sweep" is false for the lower clamp

**Where**: `crates/ocelli-render/shaders/voi.wgsl`, the `voi_window` header
block, sentence beginning "What the comparisons are load bearing FOR".

**What**: deleting the LINEAR lower clamp fails the sweep only. The section
18.3 rows and the boundary rows both stay green, because at `x = -160` the
formula body evaluates to exactly `ymin`, which is the same continuity the
paragraph above it relies on.

**Why it is wrong**: a tracked source file states a mutation result that the
mutation does not produce. `.claude/scratch/F-041-progress.md`'s own table gets
it right ("1b | REMOVE the LINEAR lower clamp | RED, the sweep"). The sentence
that was carried into the shader compressed the two rows into one and became
false.

**Evidence**:

```
=== I: delete LINEAR lower clamp: RED (exit 101)
    test the_shader_agrees_with_ocelli_pixel_over_a_sweep ... FAILED
    test result: FAILED. 5 passed; 1 failed
=== J: delete LINEAR upper clamp: RED (exit 101)
    test hld_section_18_3_rows_on_the_gpu_apply_d_13 ... FAILED
    test the_shader_agrees_with_ocelli_pixel_over_a_sweep ... FAILED
```

---

### D5, the measured divergence is recorded in no tracked file

**Where**: `docs/lld/pixel-pipeline.md`, the F-041 block added by this diff.

**What**: the approved plan commits twice to recording the number:

> The measured divergence across the sweep is recorded in
> `docs/lld/pixel-pipeline.md` when the story lands, as a number rather than a
> claim.

It is not there. `grep -rn "divergence" docs/lld/pixel-pipeline.md` returns
nothing. The only place the number exists is
`.claude/scratch/F-041-progress.md`, which `.gitignore:93` ignores
(`git check-ignore -v` confirms), so it leaves no trace in the repository.

**Why it is wrong**: decision D14 is to claim MEASURED divergence rather than
bit-exactness, and the plan made the LLD the place the measurement lands. The
LLD was edited by this diff without it. Satisfiable at `/complete-feature`, but
it is missing from the tree under review.

**Evidence**: the number reproduces exactly as reported.

```
$ cargo test -p ocelli-render --test voi_shader -- --ignored --test-threads=1 --nocapture
measured maximum divergence over the sweep: 0.000030517578
```

3.0517578e-5 is 2^-15, one `f32` ULP at magnitude 256, against a
`SWEEP_TOLERANCE` of 1e-4. The bound and the rationale are sound.

---

## Smells

### S1, `VoiParams` is publicly constructible and the story ships the counter-example to its own safety argument

**Where**: `crates/ocelli-render/src/voi.rs`, all eight fields `pub`.
`crates/ocelli-render/tests/voi_shader.rs`,
`changing_the_window_changes_the_output_through_thirty_two_bytes`.

The shader's contract rests on "a `width` already validated against each
function's domain, `w >= 1` for LINEAR and `w > 0` for the other two". That
validation lives in `VoiTransform::new`. The test demonstrates the intended
window-level drag as `VoiParams { width: 80.0, ..soft }`, which reaches the
uniform without passing through it. A drag to `width: 0.0` under SIGMOID gives
`exp(-4 * 0 / 0)` at `x == c`, hence NaN. Under LINEAR it inverts the clamp
bounds (`w' = -1`) and produces a silently reversed threshold. Nothing in the
crate refuses either. The claim "every decision is resolved on the CPU" is true
only of values that came through `from_chain`, and the only documented update
path in the story does not.

### S2, the tier B claim is text-only, and a stronger check was available and unused

**Where**: `crates/ocelli-render/src/voi.rs`,
`the_shader_uses_nothing_tier_b_lacks`.
`crates/ocelli-render/tests/voi_shader.rs` module header.

The plan said "This plan writes a fragment shader, so one shader serves both GPU
tiers". The implementation ships no entry point and the only consumer is a
tier-A compute harness over two storage buffers, which is honestly scoped in
prose and honestly recorded as a plan deviation in the progress note. What is
left is a text assertion. I confirmed on this machine that `VOI_WGSL` plus a
trivial `@vertex`/`@fragment` pair compiles into a real `create_render_pipeline`
with no validation error, which is a cheap, available, and materially stronger
check than `!VOI_WGSL.contains("@compute")` and would defend the fragment
composability F-042 will depend on. It was not taken.

### S3, the HLD section 26 test proves nothing about upload volume

**Where**: `crates/ocelli-render/tests/voi_shader.rs`,
`changing_the_window_changes_the_output_through_thirty_two_bytes`.

Its doc comment is headed "**A window-level change is thirty-two bytes and
creates no texture**" and cites section 26. Nothing in it asserts the absence of
a texture, and `on_gpu` recreates the shader module, the compute pipeline, four
buffers and the bind group on every call, so the two "window changes" are two
complete rebuilds rather than one uniform write. The plan's test row asked for
"the absence of a texture write in the recorded sequence". What the test
actually proves is that `width` affects the output, which the section 18.3 test
already proves. `assert_eq!(core::mem::size_of::<VoiParams>(), 32)` duplicates
`voi_params.rs`.

### S4, `from_chain`'s range sourcing is guarded only by a GPU test

**Where**: `crates/ocelli-render/src/voi.rs`, `from_chain`.
`crates/ocelli-render/tests/voi_params.rs`.

Replacing `let (ymin, ymax) = chain.voi().output_range();` with a hardcoded
`(0.0, 255.0)` is GREEN across the entire no-GPU suite and red only on
`inversion_reflects_about_the_midpoint_of_a_non_zero_range`. `voi_params.rs` is
the host-side suite for `from_chain` and never constructs a chain with a range
other than `[0, 255]`, where the correct and the wrong sourcing are
indistinguishable. Under D-04 the CI floor has no adapter, so on the floor a
`from_chain` that invented a range ships green. One `[16, 235]` case in
`voi_params.rs` costs nothing and closes it.

```
=== M no-GPU: ymin/ymax hardcoded: GREEN (exit 0)
=== M (with --ignored): RED, inversion_reflects_about_the_midpoint_of_a_non_zero_range
```

---

## Nitpicks

### N1, "LINEAR at 239 clamps" is wrong three times

`tests/voi_shader.rs`, the module doc of
`the_voi_boundaries_are_lower_inclusive_and_upper_strict_on_the_gpu` ("LINEAR
clamps its upper bound at `c' + w'/2 = 239`") and two assertion messages
("LINEAR at 239 clamps"). At exactly 239 the comparison is `x > 239`, which is
false, so the body runs and returns 255 by arithmetic. The clamp starts strictly
above 239. The asserted value is right, the description of why is not.

### N2, "never divides by a zero `w - 1`" is attributed to the wrong mechanism

`shaders/voi.wgsl` header: "It receives a `width` already validated against each
function's domain, `w >= 1` for LINEAR ... so it never divides by a zero
`w - 1`." At `w = 1`, `w - 1` is exactly zero. What prevents the division is the
two clamps, not the validation. See D1.

### N3, `VOI_WGSL[cursor..]`

`tests/voi_params.rs` slices a `&str` by a byte offset. Clippy's
`indexing_slicing` exempts `str`, so it does not trip the workspace lint, and the
file is ASCII so it cannot panic today. It is the same construct that
`tests/voi_shader.rs::value` goes out of its way to avoid, with a comment
explaining why.

### N4, `gpu-ownership.md` says "a second edge" and this diff adds two

`ocelli-core` arrives as a dev-dependency of `ocelli-render` in the same diff.
The progress note records it as plan deviation 3, the LLD does not mention it.

---

## Verified clean

Everything below was executed, not read.

**The formulas, against PS3.3 rather than against the comment above them.**
`voi_linear` is `c - 0.5`, `w - 1`, lower `<=`, upper `>`, body
`((x - c')/w' + 0.5)*(ymax - ymin) + ymin`: C.11.2.1.2, exact.
`voi_linear_exact` has neither the half nor the one and the same asymmetric
operators: C.11.2.1.3.2, exact. `voi_sigmoid` is
`(ymax - ymin) / (1 + exp(-4*(x - c)/w)) + ymin` with the constant `-4` and no
clamps: C.11.2.1.3.1, exact. The two linear functions are separate WGSL
functions with no shared branch. Inversion is `voi.ymin + voi.ymax - d`, PS3.3
C.11.6, not `ymax - d` and not `1 - d`. Stage order in `lut_chain` is 1, 2, 3,
each applied once.

**The section 18.3 fixtures, recomputed independently as exact rationals**, not
taken from any implementation. `c' = 39.5`, `w' = 399`, bounds -160 and 239.
LINEAR: -160 clamps to 0, at 40, `(0.5/399 + 0.5)*255 = 127.819549`, 240 > 239
so 255, at -60, `(-99.5/399 + 0.5)*255 = 63.9097744`. LINEAR_EXACT: bounds -160
and 240, -160 clamps to 0 (**deviation D-13's correction of the HLD's 1.594,
independently confirmed: `-160 <= -160` holds and the body also evaluates to
zero there, so both routes agree**), 127.5, 255, 63.75. Centre divergence
`127.819549 - 127.5 = 0.319549`. The test asserts 127.819_55, 127.5, 255.0,
63.909_775, 63.75 and 0.319_549. All match.

**The boundary rows, recomputed.** LINEAR at -159 is `255/399` because
`(0.5*399 - 198.5)/399 = 1/399`, at 238 it is `255*398/399`, at 239 it is 255.
LINEAR_EXACT at -159 is `255/400`, at 239 `255*399/400 = 254.3625`, at 240 255.
The `linear[3] > exact[3]` assertion is 255 > 254.3625. All correct.

**SIGMOID at -60**: `-4*(-100)/400 = 1` exactly, so `255/(1+e)`, written from
`core::f32::consts::E` and independent of both `exp` implementations. Correct.

**Inversion row**: LINEAR_EXACT c=100 w=200 range [16,235] at x=150 gives
`(0.25 + 0.5)*219 + 16 = 180.25`, reflected `16 + 235 - 180.25 = 70.75`,
`ymax - y` would give 54.75 and the test separates them by > 1.0. Correct, and
the choice of 150 rather than the midpoint is right, since at the midpoint the
two forms coincide.

**The central design claim, attacked.** The uniform carries no Photometric
Interpretation, no window multiplicity, no sequence and no presentation
evidence, so the shader has nothing to re-decide from. `LutChain::new` refuses a
Presentation LUT Sequence and refuses every colour or palette photometric
interpretation, so `from_chain` can never see one. `VoiTransform::window`
returns `None` for a sequence and `ModalityTransform::rescale` returns `None`
for one, and both refusals are separately asserted and separately named. `ymin`
and `ymax` come from `VoiTransform::output_range`, which is literally the value
`LutChain::new` passed to `PresentationTransform::new`, so the range the
reflection uses cannot drift from the range the inversion was resolved against
while `VoiTransform` stays immutable. `invert` is `u32::from(chain.inverts())`
and the shader tests `!= 0u` once. I found no path to a double inversion, a
re-selected window or an applied sequence.

**The four accessors add no arithmetic.** Read line by line:
`ModalityTransform::rescale`, `VoiTransform::window`, `LutChain::modality`,
`LutChain::voi` are `const fn` matches returning held state or shared borrows.
Confirmed against `crates/ocelli-pixel/src/lut.rs`.

**Mutations run, with results.** Every one reverted, final tree hash confirmed
below.

| # | Mutation | Observed |
|---|---|---|
| A | LINEAR lower `<=` -> `<` | GREEN (see D1) |
| B | LINEAR upper `>` -> `>=` | GREEN |
| C | LINEAR_EXACT lower `<=` -> `<` | GREEN |
| D | LINEAR_EXACT upper `>` -> `>=` | GREEN |
| E | LINEAR loses `- 0.5` and `- 1.0` | RED, 4 tests including the 18.3 rows |
| F | inversion `voi.ymax - d` | RED, the non-zero-range row |
| G | `voi.invert` ignored | RED, inversion row and sweep |
| H | SIGMOID `-4.0` -> `-2.0` | RED, the `255/(1+e)` row and the sweep |
| I | delete LINEAR lower clamp | RED, the sweep only (see D4) |
| J | delete LINEAR upper clamp | RED, 18.3 rows and sweep |
| K | stage 1 -> `let m = stored;` | GREEN (see D2) |
| L | `fn_kind` swaps 0 and 1 | RED, 5 GPU tests and the no-GPU `fn_kind` unit test |
| M | `ymin`/`ymax` hardcoded, not `output_range()` | RED on GPU only (see S4) |
| N1 | `VoiParams` `ymin`/`ymax` reordered | RED, GPU 5/6 and the offsets test |
| N2 | `VoiParams` `slope`/`intercept` reordered | RED, GPU 5/6 and the offsets test |
| O | `VoiParams` `center`/`width` reordered | RED, GPU 6/6 and the offsets test |
| P/Q/R | WGSL struct field pairs reordered | GREEN on the floor, RED on GPU (see D3) |
| probe | `w = 1` LINEAR on device, baseline and under A | `[0.0, ...]` vs `[NaN, ...]` |
| probe | `slope 2.0 intercept -1024` at 532 | 127.5, so stage 1 is correct today |
| probe | `VOI_WGSL` + `@vertex`/`@fragment` -> `create_render_pipeline` | builds, no validation error |

**Prose claims executed.**
`0.001` is byte-identical to `crates/ocelli-pixel/tests/voi.rs:6`'s `TOLERANCE`:
true. `VoiParams` is 32 bytes with fields at 0/4/8/12/16/20/24/28: true, and the
offsets test is red under all three field reorders. `bytemuck` had no prior
consumer: true, `grep -rn bytemuck --include=Cargo.toml` finds only the
pre-existing workspace entry (unchanged in this diff) and `ocelli-render`.
`ocelli-pixel` stays `no_std`: `bin/ocelli.sh gate nostd` green. Measured
divergence 3.05e-5: reproduced exactly.
`crates/ocelli-render/src/voi.rs`'s `fn_kind` is a total match with no wildcard:
true. `LutChain::new` builds the presentation stage from `voi.output_range()`:
true.

**Lints and policy.** No `as` cast anywhere in the diff. No `.unwrap()`, no
`.expect(`, no `panic!`. `unreachable!` appears in tests only and is not among
the denied lints. No em-dash and no prose semicolon in the added markdown or
source. Only `ocelli-render` creates a device. No `wasm-bindgen`, no pixels
across the wasm boundary (the readback is host-side in a native test, D3
untouched), one `queue.submit` per `on_gpu` call, pipeline created before
dispatch.

**Gates run green on the reviewed tree**: `fmt`, `clippy`, `test`, `nostd`,
`prose`, `deviations`, `backlog`, `unsafe`, `device`, `pins`, `ci`, `errors`,
and `cargo test -p ocelli-render -- --ignored --test-threads=1` (18 tests over
3 binaries, exit 0).

**Tree restored.** `git write-tree` =
`2cdd887e80180c47b01308b7b20102ab890de281`, identical to the starting hash.
`git status --porcelain` shows the same twelve staged paths, plus this review
file. Every mutation was reverted, the temporary probe test file was deleted,
nothing was committed and nothing was fixed.

---

**This pass is NOT clean.** F-041 pass 1: 5 defects, 4 smells, 4 nitpicks.
Defects and smells both block completion.
