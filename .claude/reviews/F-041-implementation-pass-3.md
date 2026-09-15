# F-041 review, pass 3

**Reviewed**: working tree, staged set `git write-tree` =
`dd8ffac6996c272ce50a7811391dfcca0f598204`, remediation of pass 2.
Independent review. Reviewer did not write the code.
**Machine**: `arm64` Darwin, `Apple M4 Max`, Metal, resolved tier **A**.
**Result**: 1 defect, 1 smell, 5 nitpicks

**This pass is NOT clean**, and it is one deleted paragraph and one rename away
from being clean.

Pass 2's D2 and D3 are fixed and the fixes are exactly right, verified by
measurement rather than by reading. S1 and S2 are fixed and the new assertion is
live. All four nitpicks are fixed. Fifteen mutations confirm no regression.

**What blocks is that pass 2's D1 named two locations and only one was fixed.**
The false paragraph survives verbatim in the other.

---

## Defects

### D1, the deleted observability claim is still in the tree, in the second place pass 2 named

**Where**: `crates/ocelli-render/tests/voi_shader.rs:683-688`, the doc comment of
`voi_linear_at_width_one_pins_the_lower_boundary_operator`.

**What is still there, unchanged from the pass-2 tree**:

> **It does NOT pin the upper operator, and nothing does.** `>` to `>=` is
> green at every legal width: for `w > 1` the body at the upper breakpoint is
> exactly `ymax` in `f32`, and for `w == 1` the lower check returns first, so
> the upper comparison is never reached. That is recorded at the shader rather
> than left for a later reviewer to rediscover, and this test's name says
> `lower` for the same reason.

**Why it is wrong**: it is verbatim the claim pass 2's D1 falsified and pass 3
deleted from the shader. Pass 2 recorded the location explicitly, under
"**Where**: `crates/ocelli-render/shaders/voi.wgsl` ... and
`crates/ocelli-render/tests/voi_shader.rs`, the doc of
`voi_linear_at_width_one_pins_the_lower_boundary_operator`. **Both say it.**"
The shader was fixed. This was not. The measurement that falsifies it stands
unchanged: at `c = 1024.5`, `w = 1.0003662109375`, range `[0, 255]`, input
`1024.000244140625`, the shader returns **297.50003** with `>` and **255.0**
with `>=`, a difference of 42.5 of 255, and `VoiTransform::new` accepts that
window.

**And it now carries a dangling reference on top of the false claim.** "That is
recorded at the shader rather than left for a later reviewer to rediscover" is
no longer true of anything: the pass-3 remediation removed the shader paragraph
it points at, on purpose. A reader following the pointer finds the opposite
statement, which is the shader's new text saying that three versions of exactly
this argument were wrong.

**Why this matters more than one stale paragraph.** The microscope's guidance is
to report when the same finding survives passes. This is its third: pass 1
raised the operator claim, pass 2 falsified it with measurements and named both
sites, pass 3 fixed one. The remaining copy is the more dangerous one, because
it sits on the test whose job is the operators and reads as that test's own
account of its coverage.

**The fix is a deletion, not a fourth rewrite**, and the shader already models
it. Nothing depends on the paragraph.

**Evidence**:

```
$ grep -rn -e "observab" -e "pin the upper" -e "at every legal width" crates docs
crates/ocelli-render/shaders/voi.wgsl:98:   OBSERVABLE AND ALL THREE WERE WRONG.**  (the correct, new text)
crates/ocelli-render/tests/voi_shader.rs:683:/// **It does NOT pin the upper operator, and nothing does.** `>` to `>=` is
crates/ocelli-render/tests/voi_shader.rs:684:/// green at every legal width: for `w > 1` the body at the upper breakpoint is
crates/ocelli-render/tests/voi_shader.rs:685:/// exactly `ymax` in `f32`, and for `w == 1` the lower check returns first, so
```

`git diff 9bce4392 dd8ffac6 -- crates/ocelli-render/tests/voi_shader.rs` touches
the sweep assertion, the drag doc, the stage-one doc, the vertex shader and
`drop(pipeline)`. It does not touch lines 679 to 688.

---

## Smells

### S1, the test named for the asymmetric comparisons is green under all four of them

**Where**: `crates/ocelli-render/tests/voi_shader.rs:337-350`,
`the_voi_boundaries_are_lower_inclusive_and_upper_strict_on_the_gpu`, headed
"**The boundaries, where the asymmetric comparisons live.**"

Measured on this tree, all four operator mutations leave it green:

| Mutation | This test |
|---|---|
| LINEAR `<=` to `<` | green (only `voi_linear_at_width_one...` is red) |
| LINEAR `>` to `>=` | green |
| LINEAR_EXACT `<=` to `<` | green |
| LINEAR_EXACT `>` to `>=` | green |

It is not a dead test. It is red under the `c - 0.5` and `w - 1` mutation, so it
pins the boundary *values* and the LINEAR against LINEAR_EXACT split, which is
real coverage. What it does not pin is either operator, which is what its name
and its heading say it does.

**Why a smell and not a defect**: the assertions are load bearing and the values
are hand-computed correctly. Only the label is wrong. It is a smell rather than
a nitpick because a name is how a later reader decides what is already covered,
and with D1 removed this name becomes the last surviving statement in the tree
that the operators are pinned here. Renaming to something like
`the_voi_boundary_values_on_the_gpu` closes it in one line, and the `<=` is now
genuinely pinned by `voi_linear_at_width_one_pins_the_lower_boundary_operator`.

---

## Nitpicks

### N1, "panics through wgpu's error scope" names a mechanism the test does not use

`tests/voi_shader.rs`, the closing comment of
`the_shader_composes_into_a_render_pipeline_not_only_a_compute_one`: "the
pipeline exists, which is the assertion: `create_render_pipeline` validates the
composed module and panics through wgpu's error scope if it does not." No error
scope is pushed anywhere in the test. The mechanism is wgpu's default uncaptured
error handler. The test is genuinely load bearing, measured below, so this is
wording rather than coverage.

### N2, "only SIGMOID gives one" is true at `w = 0` and not beyond it

`src/voi.rs`, the `VoiParams` hazard paragraph. As scoped to `w = 0` it is
exactly right and I measured it. The paragraph's subject, though, is the public
field hazard in general, and a reader who carries the sentence past `w = 0` gets
it wrong. Measured on the adapter, same soft-tissue chain, centre 40:

```
width NaN Linear:      [NaN, NaN]        (inputs 40, 100)
width NaN LinearExact: [NaN, NaN]
width NaN Sigmoid:     [NaN, NaN]
width -1  Linear:      [0.0, 255.0]
width -1  LinearExact: [0.0, 255.0]
width -1  Sigmoid:     [127.5, 0.0]
```

`width: -1.0` under SIGMOID returning a plausible mid-grey 127.5 at the centre
is a better example of the paragraph's own point than the NaN it mentions.

### N3, the in-range assertion hardcodes the range instead of reading it

`tests/voi_shader.rs`, the sweep: `(0.0..=255.0).contains(&got)`. Correct today
because `chain()` fixes the range at `[0, 255]`, and the comment says the bound
is scoped to this sweep's parameters. Written as
`(params.ymin..=params.ymax).contains(&got)` it would stay correct if `chain()`
ever changed, and it would still be scoped rather than an invariant claim. As
written, a change to `chain()`'s range loosens the guard silently instead of
failing.

### N4, the rewrapped stage-one doc line is 94 characters

`tests/voi_shader.rs`: "/// `(ymax - ymin) / 2 + ymin`, and here `ymin` is zero.
Hand-computed: stored 2106, modality". The fix to pass 2's N2 is correct, the
rewrap was not carried through, and rustfmt does not reflow doc comments so
nothing catches it.

### N5, "LINEAR clamps its upper bound at `c' + w'/2 = 239`"

`tests/voi_shader.rs:340`. At exactly 239 nothing clamps, because the comparison
is `x > 239`. The body runs and returns 255 by arithmetic. Pass 2's N1 corrected
the two assertion messages to "reaches ymax" and this sentence, which says the
same wrong thing, was not corrected with them.

---

## Verified clean

### The D1 deletion in the shader is right, and its replacement checks out

`shaders/voi.wgsl` lost thirty-four lines of argument and gained eleven. What
remains is checkable and checks out:

- "the operators ... are what the standard says and what `ocelli-pixel` does" is
  true against PS3.3 C.11.2.1.2, C.11.2.1.3.2 and
  `ocelli-pixel/src/lut.rs::apply_window`.
- "At `w = 1`, which PS3.3 permits, `w' = 0` and the body is `0 / 0`, so `<`
  instead of `<=` lets `x == c'` reach the division and the shader returns NaN"
  is the pass-1 measurement and still reproduces.
- "THREE EARLIER VERSIONS ... ALL THREE WERE WRONG" is accurate: version one
  claimed both operators unobservable and was wrong about the lower, version two
  claimed both load bearing and was wrong about the upper, version three claimed
  an asymmetry and was wrong about the upper.
- "legal windows where the upper operator changes the output by 42.5 of 255" is
  pass 2's case A, reproduced exactly.
- Nothing in the shader now asserts observability. The only remaining instance
  anywhere in `crates/` or `docs/` is D1's.

### D2's new sentences are exactly right, measured

`VoiParams { width: 0.0, ..from_chain(chain) }` on the adapter, centre 40, range
`[0, 255]`, inputs `-1000, 39, 39.999996, 40, 40.000004, 41, 1000`:

```
W0 Linear:      [0.0, 0.0, 0.0, 0.0, 255.0, 255.0, 255.0]
W0 LinearExact: [0.0, 0.0, 0.0, 0.0, 255.0, 255.0, 255.0]
W0 Sigmoid:     [0.0, 0.0, 0.0, NaN, 255.0, 255.0, 255.0]
```

Every clause holds. No division by zero under LINEAR, because `w' = -1`. The
bounds invert, `lower = c' + 0.5 = c` and `upper = c' - 0.5 = c - 1`, so
`lower > upper`. Every input clamps. `ymin` at or below the centre, which the
column at index 3, x = 40 = the centre, shows returning 0. `ymax` above it,
index 4 at x = 40.000004 returning 255. A clean two-tone threshold image. And
only SIGMOID gives a NaN, at exactly `x == c`, index 3 and nowhere else.
LINEAR_EXACT is covered by the same sentence and behaves identically, for the
different reason that `lower == upper == c`.

### The in-range assertion is correctly scoped and is evaluated

The comment scopes it to this sweep's parameters, names pass 2's case A as the
reason it is not claimed as an invariant, and says the CPU reproduces the
overshoot and it is not this story's to change. All three are accurate. "Here
the range is `[0, 255]` with a width of 400" matches `chain()`, and the sweep
passes, so the overshoot does not arise at these parameters.

It is not decorative. Tightening the literal to `(0.0..=100.0)` turns the sweep
**RED**, so the assertion runs on all 4096 values times three functions times
two photometric interpretations.

### The vertex shader really is a valid fullscreen triangle

Not read, rasterised. I took the exact `vs_main` from the file, rendered
`draw(0..3, 0..1)` into a 64 by 4 `Rgba8Unorm` target cleared to opaque red with
a fragment writing blue, copied the texture back and counted texels whose red
channel was still 255.

```
TRIANGLE: 64x4 target, covered=256 uncovered=0
```

Every texel was rasterised. By hand the three clip-space vertices are index 0
`(-1,-1)`, index 1 `(3,-1)`, index 2 `(-1,3)`, which is what the new comment
says, from `u = f32((index << 1u) & 2u)` giving 0, 2, 0 and
`v = f32(index & 2u)` giving 0, 0, 2.

### The D3 fix keeps the test load bearing

`get_bind_group_layout` is gone and `drop(pipeline)` replaces it. The comment's
claims are accurate: asking for the layout confirms nothing, which is pass 2's
measurement, and
`voi::tests::the_shader_is_composable_and_reserves_only_binding_zero` is what
watches the binding set and does run on the floor. The test still fails for the
right reasons:

```
X2: VOI_WGSL syntactically broken                 | render-pipeline test=RED | wgpu error: Validation Error
X3: VOI_WGSL uses workgroupBarrier() in a helper  | render-pipeline test=RED | wgpu error: Validation Error
```

### The remaining nitpick fixes

The ULP restatement is exact, computed rather than accepted:
`ulp(255) = 1.52587890625e-05` so the measurement is `2.0000` of it, and
`ulp(256) = 3.0517578125e-05` so it is `1.0000` of that. `docs/lld/pixel-pipeline.md`
now says "two `f32` ULP at 255, or one at 256", which is right in both halves.
`(ymax - ymin) / 2 + ymin` is the correct expression. `docs/lld/gpu-ownership.md`
is now two paragraphs, the `ocelli-pixel` edge and the `ocelli-core` dev edge,
and the spliced 99-character line is gone.

### No regression, fifteen mutations

Every mutation that was red in pass 1 or pass 2 is still red, and two are now red
on more tests than before.

| Mutation | Floor | GPU |
|---|---|---|
| LINEAR lower `<=` to `<` | green | **RED** `voi_linear_at_width_one...` |
| LINEAR loses `- 0.5` and `- 1.0` | green | **RED**, 4 tests |
| inversion `voi.ymax - d` | green | **RED**, the non-zero-range row |
| `voi.invert` ignored | green | **RED**, inversion row and sweep |
| SIGMOID `-4.0` to `-2.0` | green | **RED**, the `255/(1+e)` row and sweep |
| delete LINEAR lower clamp | green | **RED**, sweep and width-one |
| delete LINEAR upper clamp | green | **RED**, 18.3 rows, sweep and width-one |
| stage 1 to `let m = stored;` | green | **RED** `stage_one_applies...` |
| WGSL struct `ymin`/`ymax` swapped | **RED** | not needed |
| WGSL struct `slope`/`intercept` swapped | **RED** | not needed |
| WGSL struct `center`/`width` swapped | **RED** | not needed |
| WGSL struct `fn_kind`/`invert` swapped | **RED** | not needed |
| `fn_kind` swaps 0 and 1 | **RED** | not needed |
| `from_chain` `ymin`/`ymax` hardcoded | **RED** | not needed |
| `VoiParams` field pairs reordered, three ways | **RED** | not needed |
| drag test narrow centre 40 to 50 | green | **RED**, the equality assertion is live |

### Unchanged from earlier passes, re-checked

The three window formulas against PS3.3 C.11.2.1.2, C.11.2.1.3.2 and
C.11.2.1.3.1. `c - 0.5` and `w - 1` in LINEAR and neither in LINEAR_EXACT, in
separate functions. SIGMOID's `-4`. Inversion `voi.ymin + voi.ymax - d`. Stage
order 1, 2, 3, applied once each. The four section 18.3 rows with D-13's
correction, the boundary rows, `255/(1+e)` and the `[16, 235]` reflection to
70.75, all recomputed as exact rationals. The 32-byte layout and its eight
offsets. `from_chain`'s two sequence refusals. No route found by which the
shader could double-invert, re-select a window or apply a sequence. Measured
sweep divergence reproduces at `0.000030517578`.

### Lints, policy and gates

No `as` cast, no `.unwrap()`, no `.expect(`, no `panic!` and no `unreachable!`
added by this delta. No em-dash. Gates green on this tree: `fmt`, `clippy`,
`prose` (304 files), `unsafe`, `device`, and `gate gpu` ALL GREEN with 9 shader
tests, 9 device tests and the lib's.

### Tree

`git write-tree` = `dd8ffac6996c272ce50a7811391dfcca0f598204`, identical to the
starting hash, with `git diff --stat` empty so the working tree matches the
index. `git status` was checked after every mutation batch, as instructed, and
no mutation leaked this time. The temporary probe file is deleted. Nothing was
committed and nothing was fixed.

---

**F-041 pass 3: 1 defect, 1 smell, 5 nitpicks. NOT clean.**

The defect is a paragraph that pass 2 asked to be deleted from two files and
which was deleted from one. The smell is the test name that says the same thing
in four words. Neither needs new reasoning, and a pass 4 should be short.
