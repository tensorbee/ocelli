# F-041 review, pass 2

**Reviewed**: working tree, staged set `git write-tree` =
`9bce439264d0470ca509c6e4820ad71574af3102`, remediation of pass 1.
Independent review. Reviewer did not write the code.
**Machine**: `arm64` Darwin, adapter `Apple M4 Max`, backend `Metal`, resolved
tier **A**, `compute: true`, `max_tex_3d: 2048`.
**Result**: 3 defects, 2 smells, 5 nitpicks

**This pass is NOT clean.**

Every pass-1 finding was fixed and every fix was verified by mutation. What
blocks is new: the third version of the operator argument is still false, and
two of the explanatory sentences added by this remediation are false about
their own subject matter.

---

## Defects

### D1, the third version of the operator argument is still false, in the same direction as the first

**Where**: `crates/ocelli-render/shaders/voi.wgsl`, the block above
`fn voi_window`, and `crates/ocelli-render/tests/voi_shader.rs`, the doc comment
of `voi_linear_at_width_one_pins_the_lower_boundary_operator`. Both say it.

**What the shader claims**:

> THE UPPER OPERATOR IS NOT OBSERVABLE AT ANY LEGAL WIDTH, and `>` to `>=` is
> green. Two separate reasons, measured: for `w > 1` the body at the upper
> breakpoint is `(0.5 + 0.5) * range + ymin`, which is exactly `ymax`, and the
> division `(w'/2) / w'` is exact in `f32`, so the clamp and the body return the
> same bits.

The universal claim is false and both stated reasons are false. Measured on the
Metal adapter, through `VoiTransform::new` and `VoiParams::from_chain`, with the
shader as written against the shader with `>` changed to `>=`:

| Case | function | c | w | range | x | `>` gives | `>=` gives |
|---|---|---|---|---|---|---|---|
| A | LINEAR | 1024.5 | 1.0003662109375 | [0, 255] | 1024.000244140625 | **297.50003** | **255.0** |
| B | LINEAR | 40 | 400 | [-1134.1293, 3889.3992] | 239 | **3889.399** | **3889.3992** |
| C | LINEAR_EXACT | 2048 | 0.000732421875 | [0, 255] | 2048.00048828125 | **297.50003** | **255.0** |

Case A is a divergence of **42.5 of 255**, 16.7 percent of full scale and 132
times the 0.32-of-255 divergence this project exists to catch.
`ocelli-pixel` accepted the window (the probe prints
`HOLE A: ocelli-pixel ACCEPTED the window`), so it is a legal chain, not a
hand-built `VoiParams`.

**Why reason one is wrong.** `(w'/2) / w'` is indeed exact, but the body never
computes it. It computes `(x - c') / w'` where `x` is the *rounded* `f32` value
of `c' + w'/2`. When `c'` is large relative to `w'/2` the addition drops low
bits of `w'/2`, the subtraction `x - c'` recovers a different number, and the
quotient is not 0.5. In case A it is **0.6666667**, because `c' = 1024`,
`w' = 3 * 2^-13`, and `fl(1024 + 1.5 * 2^-13)` rounds to even at
`1024 + 2 * 2^-13`, so `(x - c')/w' = 2/3`.

**Why reason two is wrong, independently.** Even when the quotient is exactly
0.5, `(0.5 + 0.5) * range + ymin` is `fl(fl(ymax - ymin) + ymin)`, and that is
not `ymax` whenever the subtraction loses bits. Case B is a legal non-zero
`ymin` where it does not round-trip: `range = 5023.5283`,
`range + ymin = 3889.399`, `ymax = 3889.3992`.

**Why it is not a corner case.** Over 800,000 random legal LINEAR parameter
pairs (`c` in +/- 30000, `w` in [1, 5000], range [0, 255]), **519,795 of them,
about 65 percent, have `body(upper) != ymax`**, so the two operators return
different values at the upper breakpoint. The magnitude falls with width: about
1e-4 of 255 at w = 4000, about 0.002 at w = 100, and up to 9.3 in the random
sample and 42.5 in the constructed case as `w` approaches 1 with a large centre.

**Why it is wrong under the microscope's own rules**: "a claim in prose that is
false" is a defect, and this one is load bearing. It is the stated reason no
test pins the upper operator, exactly as the pass-1 version was the stated
reason no test pinned the lower one. The shader's history paragraph says "the
first version ... was wrong about the lower one, the second claimed both were
load bearing and was wrong about the upper one. The asymmetry is the truth and
neither summary was." **The asymmetry is not the truth either.** Both operators
are observable. The lower one is observable at `w = 1`, which the new test now
pins. The upper one is observable for most legal parameter pairs at exactly one
input, which nothing pins.

**Evidence**: probe on the real adapter, both baseline and with
`if (x > cp + wp / 2.0)` mutated to `>=`, reverted after.

```
HOLE A: c=1024.5 w=1.0003662 cp=1024 wp=0.00036621094 lower=1023.9998 upper=1024.0002
HOLE A: lower < upper ? true
HOLE A: (upper - cp)/wp = 0.6666667
HOLE A: ocelli-pixel ACCEPTED the window
HOLE A: gpu at x == upper = [297.50003]   (clamp would give 255.0)
HOLE A: cpu at x == upper = 297.50003
MUT >= (LINEAR): HOLE A: gpu at x == upper = [255.0]

HOLE B: range=5023.5283   (range + ymin) == ymax ? 3889.399   ymax = 3889.3992
HOLE B: (upper - cp)/wp = 0.5
HOLE B: gpu at x == upper = [3889.399]   differs from ymax by -0.00024414063
MUT >= (LINEAR): HOLE B: gpu at x == upper = [3889.3992]

EXACT: c=2048 w=0.0007324219 upper=2048.0005 lower<upper? true
EXACT: (upper - c)/w = 0.6666667
EXACT: gpu at x == upper = [297.50003]   (clamp would give 255.0)
MUT >= (EXACT): EXACT: gpu at x == upper = [255.0]
```

**Scope note, so this is not over-read.** The CPU returns the identical
297.50003, so this is **not** a shader-versus-`ocelli-pixel` divergence and the
sweep could not have caught it. The arithmetic is `ocelli-pixel`'s, inherited
from PS3.3's literal formula, and F-041 should not change it. What F-041 owns is
the sentence, and the right remediation is probably to delete the
unobservability argument rather than to attempt a fourth version of it. The
operator is kept as PS3.3 writes it regardless, which the block already says, so
the argument carries no decision.

---

### D2, "width 0 divides by zero and returns NaN" is false for LINEAR, which is the case both sentences cite

**Where**: `crates/ocelli-render/src/voi.rs`, the `VoiParams` doc comment, and
`crates/ocelli-render/tests/voi_shader.rs`, the doc of
`changing_the_window_changes_the_output_through_thirty_two_bytes`. Both say it.

**What**:

> The cost is that `VoiParams { width: 0.0, ..built }` compiles and hands the
> shader a width PS3.3 C.11.2.1.2 forbids, which divides by zero and returns
> NaN.

PS3.3 C.11.2.1.2 is the **LINEAR** section, and `built` and `soft` are both
LINEAR chains. For LINEAR, `w = 0` makes `w' = w - 1 = -1`, not zero. Nothing
divides by zero and nothing returns NaN. Both comparisons cover the whole line
(`lower = c' + 0.5`, `upper = c' - 0.5`, an inverted pair), so every input takes
an early return and the shader emits a clean hard threshold at `x = c`.
LINEAR_EXACT at `w = 0` is the same: `lower == upper == c`, no division. NaN
occurs only under **SIGMOID**, and only at exactly `x == c`, where
`-4 * 0 / 0` is NaN.

**Why it is wrong**: it is a false statement about the arithmetic in the file
that owns the arithmetic, and it weakens the very point it is making. A NaN is a
loud failure that a downstream texture write or a comparison would expose. A
silent, plausible-looking threshold image is the quietly-wrong-pixel class this
project names as its dangerous defect. The true behaviour is a *stronger*
argument for the public-field hazard than the stated one.

**Evidence**: `VoiParams { width: 0.0, ..from_chain(soft) }` on the real adapter,
inputs `[-160, 39.5, 40, 40.5, 240]`, centre 40, range [0, 255]:

```
WIDTH ZERO Linear:      [0.0, 0.0, 0.0, 255.0, 255.0]  any_nan=false
WIDTH ZERO LinearExact: [0.0, 0.0, 0.0, 255.0, 255.0]  any_nan=false
WIDTH ZERO Sigmoid:     [0.0, 0.0, NaN, 255.0, 255.0]  any_nan=true
```

---

### D3, `get_bind_group_layout(0)` confirms nothing, and the comment says it confirms something

**Where**: `crates/ocelli-render/tests/voi_shader.rs`, the last two lines of
`the_shader_composes_into_a_render_pipeline_not_only_a_compute_one`.

**What**:

> // The bind group layout the pipeline derived confirms binding 0 is the only
> // one `VOI_WGSL` brought with it: the fragment harness declares none.
> let _layout = pipeline.get_bind_group_layout(0),

Fetching the auto-derived layout for group 0 succeeds whether or not group 0
holds other bindings, and it says nothing about groups above 0. The line is a
no-op with a claim attached.

**Why it is wrong**: microscope class 4, a thing that is present, looks
authoritative and checks nothing, with class 3 on top, a prose claim about what
a guard does. The property really is guarded, by
`voi::tests::the_shader_is_composable_and_reserves_only_binding_zero`, which is
a different test in a different file, so the comment also misattributes the
coverage.

**Evidence**: adding a used `@group(0) @binding(1) var<uniform> extra` to
`VOI_WGSL`, then adding a used `@group(1) @binding(0) var<storage, read>`:

```
T: VOI_WGSL grows @binding(1) uniform (used)     | render-pipeline test=GREEN | lib text tests=RED the_shader_is_composable_and_reserves_only_binding_zero
U: VOI_WGSL grows a storage buffer               | render-pipeline test=GREEN | lib text tests=RED the_shader_uses_nothing_tier_b_lacks, the_shader_is_composable_and_reserves_only_binding_zero
V: VOI_WGSL syntactically broken                 | render-pipeline test=RED   | lib text tests=GREEN
```

Row V is the good news and is recorded under Verified clean: the test is load
bearing for what its name says, that the composed text is accepted as a vertex
and fragment pair. It is only the trailing comment that is false.

---

## Smells

### S1, nothing asserts the shader's output stays inside `[ymin, ymax]`

The clamps exist so that stages 1 to 3 land in the declared output range, and
the shader says so at `voi_sigmoid`: "No clamps. The sigmoid is asymptotic and
never leaves the open range, which is why the other two have boundary
comparisons and this does not." No test checks the property for the other two.
The sweep asserts agreement with `ocelli-pixel`, so both sides leaving the range
together is invisible, and the fixture rows check four named inputs. D1's case A
is a legal window and a legal input where the shader returns **297.50003 against
a declared `ymax` of 255**, with the clamp present.

A single `assert!(v >= ymin && v <= ymax)` inside the existing sweep loop costs
one line, runs on inputs already being dispatched, and turns a property
currently asserted only in prose into one the `gpu` gate checks. It would not
have caught D1 by itself, because the sweep does not visit the breakpoint
exactly, but it is the assertion the sentence is making.

### S2, the operator block is now mostly meta-commentary, and it has been wrong three times

`shaders/voi.wgsl`'s block above `fn voi_window` has grown from roughly fifteen
lines to roughly thirty-five across three attempts, and the growth is entirely
argument about what a mutation does rather than statement of what the code
computes. Three of those versions have shipped, and all three have been false.
The microscope's own guidance is the fix: "Prefer deleting a wrong sentence to
explaining it", and "a remediation that corrects one sentence and adds three
explaining it ships three new claims for the next pass to falsify."

Nothing depends on the argument. The operators are transcribed from PS3.3 and
kept whatever the mutation result is, which the block already states. What the
file needs is the two sentences that are checkable: the operators are PS3.3's,
and `voi_linear_at_width_one_pins_the_lower_boundary_operator` is the test.
Everything else belongs in the review record, where being wrong costs a
correction rather than a false comment beside the highest-risk arithmetic in the
project.

---

## Nitpicks

### N1, "about one `f32` ULP at a magnitude of 255"

`docs/lld/pixel-pipeline.md`. 0.000030517578 is `2^-15`. `ulp(255)` is `2^-16`,
so the measurement is **two** ULP at 255, or exactly one ULP at 256. "About"
carries it, but the file is stating a measured number precisely and then
describing it imprecisely.

### N2, `(ymax - ymin) / 2` is missing its `+ ymin`

`tests/voi_shader.rs`, `stage_one_applies_the_rescale_slope_and_intercept_on_the_gpu`:
"the expected display value is the window centre under `LINEAR_EXACT`, which is
exactly `(ymax - ymin) / 2`". It is `(ymax - ymin) / 2 + ymin`. The two coincide
here only because `ymin` is 0, which is the same conflation that makes
`ymax - y` look like a correct inversion.

### N3, the render-pipeline harness's vertex shader is not a fullscreen triangle

`f32(i32(index) * 4 - 1)` gives x of -1, 3, 7 for the three vertices and
`f32(i32(index & 2u) * 4 - 1)` gives y of -1, -1, 7. That is not the standard
oversized triangle and does not cover the viewport. It is never drawn, so
nothing notices, but it reads as purposeful geometry that is wrong and F-038 may
copy it.

### N4, the `gpu-ownership.md` insertion splices a paragraph

The new `ocelli-core` dev-dependency sentence is inserted mid-sentence, leaving
"...does not name it. HLD section 18 says to implement the LUT" as a
99-character line in a paragraph that otherwise wraps at 79.

### N5, the history paragraph cites a version that never landed

`shaders/voi.wgsl`: "the second claimed both were load bearing and was wrong
about the upper one". The first version is in the repository's history and can
be read. The second was never committed, so a later reader cannot check the
claim about it. Given D1 the paragraph needs a fourth revision anyway, which is
S2's argument for deleting it rather than extending it.

---

## Verified clean

### Every pass-1 finding is fixed, and each fix is verified by mutation

| Pass-1 finding | Fix | Mutation evidence |
|---|---|---|
| D1, lower operator unpinned | `voi_linear_at_width_one_pins_the_lower_boundary_operator` | `<=` to `<` now **RED** on that test alone. Was GREEN |
| D2, stage 1 unexecuted | `stage_one_applies_the_rescale_slope_and_intercept_on_the_gpu` | `let m = stored;` **RED**, `stored + intercept` **RED**, `stored * slope` **RED**, `stored * intercept + slope` **RED**. All were GREEN or partly GREEN |
| D3, field-order scan defeated by prose | search confined to the struct body | WGSL struct swaps of ymin/ymax, slope/intercept, center/width and fn_kind/invert are all **RED on the floor**. All four were GREEN |
| D4, false clamp-deletion sentence | the sentence is deleted, not rewritten | `grep -n "sweep\|deletion\|load bearing"` finds no such claim in the shader. This is the better fix and matches the microscope's "prefer deleting a wrong sentence" |
| D5, divergence in no tracked file | new section in `docs/lld/pixel-pipeline.md` | the number, the bound, the machine, the D14 framing and the not-an-oracle caveat are all present |
| S1, drag test bypassed CPU validation | narrow window built through `VoiTransform::new`, plus an equality assertion | changing the narrow chain's centre from 40 to 50 is **RED**, so `VoiParams { width: soft.width, ..narrow } == soft` is live |
| S2, tier B text-only | `the_shader_composes_into_a_render_pipeline_not_only_a_compute_one` | a syntactically broken `VOI_WGSL` is **RED** on that test and GREEN on the text tests, so it adds evidence the greps cannot |
| S3, drag test overclaimed | doc now states only what it asserts and names F-038 as the owner of the no-re-upload claim | read and agreed, and it explicitly retracts the earlier sentence |
| S4, `from_chain` range unguarded on the floor | `ymin_and_ymax_come_from_the_voi_stages_declared_range`, range [16, 235], compared by `to_bits` | hardcoding `(0.0, 255.0)` is **RED on the floor**, and swapping `ymin`/`ymax` in `from_chain` is **RED on the floor** on two tests. Both were GREEN |
| N1 to N4 | "reaches ymax", the `w - 1` correction, `.get()` instead of a raw slice, both crate edges named | read and correct. The corrected `w - 1` sentence now says the validation "does NOT stop `w - 1` being zero ... what keeps the division unreachable there is the pair of boundary comparisons", which is exactly right |

### No regression: every previously-red mutation is still red

| Mutation | Floor | GPU |
|---|---|---|
| LINEAR loses `- 0.5` and `- 1.0` | green | **RED**, 4 tests |
| inversion `voi.ymax - d` | green | **RED**, the non-zero-range row |
| `voi.invert` ignored | green | **RED**, inversion row and sweep |
| SIGMOID `-4.0` to `-2.0` | green | **RED**, the `255/(1+e)` row and the sweep |
| delete LINEAR lower clamp | green | **RED**, sweep and width-one (was sweep only, so the new test widened it) |
| delete LINEAR upper clamp | green | **RED**, 18.3 rows, sweep and width-one |
| `fn_kind` swaps 0 and 1 | **RED** | **RED**, 5 tests |
| `VoiParams` ymin/ymax reordered | **RED** | not run, floor is enough |
| `VoiParams` slope/intercept reordered | **RED** | not run |
| `VoiParams` center/width reordered | **RED** | not run |
| `from_chain` ymin/ymax hardcoded | **RED** | (was GPU-only, now floor) |

### The three new GPU tests and the one new floor test are sound

`voi_linear_at_width_one_pins_the_lower_boundary_operator`: centre 0, width 1,
`c' = -0.5`. Inputs -0.5 and -0.499 are the two sides of the collapsed
breakpoint and are the rows `ocelli-pixel/tests/voi.rs`'s `width_one` block
already pins, derived from PS3.3 C.11.2.1.2's threshold behaviour at `w = 1`
rather than from any implementation output. The `is_finite` assertion before the
value assertion is right, because the failure mode is NaN and
`(NaN - 0.0).abs() < 0.001` is false but reports the wrong thing. Its claim that
"every other GPU test here uses a width of 400, 200 or 80" is true: 400 in
`chain`, 200 in the inversion row, 400 and 80 in the drag test, 400 in the
stage-one test. Its closing paragraph is D1.

`stage_one_applies_the_rescale_slope_and_intercept_on_the_gpu`: slope 2,
intercept -1024, stored 2106, so modality is 3188, the window is centred there
and `LINEAR_EXACT` gives 127.5. Both stated counterfactuals check out by hand,
1082 with the slope dropped and 4212 with the intercept dropped, and both are
further than `w/2 = 200` from the centre so both clamp. Both mutations measured
red separately, so slope and intercept are individually pinned rather than
jointly.

`the_shader_composes_into_a_render_pipeline_not_only_a_compute_one`: builds a
real `create_render_pipeline` over `VOI_WGSL` plus a vertex and fragment pair.
Red on a broken shader, so it is load bearing. Its doc's refusal to claim tier B
is correct and is the right shape: "This runs on whatever tier the machine
resolved, which for every machine this project has is tier A, and a downlevel
adapter can refuse a module a tier A adapter accepts." `F-042` and `F-X002` both
check out against `docs/sprints/BACKLOG.md` lines 168 and 171, the WebGL2 story
and the lavapipe-plus-SwiftShader story.

`ymin_and_ymax_come_from_the_voi_stages_declared_range`: range [16, 235],
compared with `to_bits()` so there is no float comparison in a layout-and-
provenance test. Red under two different wrong sourcings.

`the_shader_declares_section_18_4s_uniform`: the struct body is found by
`struct VoiParams {` and the first `}` after it, which is unambiguous because the
body has no nested braces and the phrase does not appear in the prose header.
Red on all four adjacent-pair swaps.

### The remediation's numeric prose, executed

`0.000030517578` reproduces exactly on the pass-2 tree
(`the_shader_agrees_with_ocelli_pixel_over_a_sweep`, `--nocapture`).
`aarch64-apple-darwin` is right (`uname -m` is `arm64`), and `Metal`, tier `A`
and `Apple M4 Max` are confirmed from the resolver's own output rather than
assumed. `1e-4` is the asserted bound. "4096 stored values spanning both clamps,
all three VOI functions, inverted and not" matches the test. The
not-an-oracle paragraph is accurate, and its list of hand-computed rows (18.3,
boundary, width-one, SIGMOID) is correct, though it omits the new stage-one row,
which is also hand-computed.

`the shader ... agreeing with the CPU on every finite value` survives D1: the
CPU returns the identical 297.50003 for case A, so the shader and
`ocelli-pixel` still agree bit for bit at the counterexample.

### Unchanged from pass 1, re-checked

The three window formulas against PS3.3 C.11.2.1.2, C.11.2.1.3.2 and
C.11.2.1.3.1 character by character. `c - 0.5` and `w - 1` in LINEAR and neither
in LINEAR_EXACT, in separate functions with no shared branch. SIGMOID's `-4`.
Inversion `voi.ymin + voi.ymax - d`. Stage order 1, 2, 3, each applied once. The
section 18.3 rows and D-13's correction of row one, recomputed as exact
rationals. The boundary rows. `255/(1+e)`. The `[16, 235]` reflection to 70.75.
The 32-byte layout and its eight offsets. `from_chain`'s two sequence refusals.
No route found by which the shader could double-invert, re-select a window or
apply a sequence.

### Lints, policy and gates

No `as` cast, no `.unwrap()`, no `.expect(`, no `panic!` and no raw index or
slice in the pass-2 delta. No em-dash and no prose semicolon. Gates run green on
this tree: `fmt`, `clippy`, `prose` (303 files), `unsafe`, `device`, `nostd`,
`deviations`, `backlog`, and `gate gpu` (ALL GREEN, 9 shader tests plus 9 device
tests plus the lib's).

### Tree

`git write-tree` = `9bce439264d0470ca509c6e4820ad71574af3102`, identical to the
starting hash, with `git diff --stat` empty so the working tree matches the
index. One mutation run timed out mid-mutation and left `shaders/voi.wgsl`
dirty. It was detected by `git status`, inspected, reverted with
`git checkout --`, and the two measurements taken while it was dirty were
discarded and re-run clean. Every other mutation was reverted by its own
handler, the temporary probe file is deleted, nothing was committed and nothing
was fixed.

---

**F-041 pass 2: 3 defects, 2 smells, 5 nitpicks. NOT clean.** Defects and smells
both block completion. All ten pass-1 findings are closed and verified, and the
three defects here are new prose introduced by the remediation, two of them in
sentences added to explain a correction.
