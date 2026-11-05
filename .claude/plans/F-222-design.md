# F-222, Pyramid means skip non-finite F32 samples

**Status**: approved
**Epic ref**: E30.7
**Sprint**: S46
**Estimate**: 1w

Story row, `docs/sprints/BACKLOG.md`: "Pyramid means skip non-finite F32 samples: a coarser level averages only the finite samples of its box by their overlap weight, a box with no finite sample stays NaN, and negative zero is written as positive zero". Layer Rust, depends on F-119, milestone M14.

`allocation.json` notes: "The operator answered F-119's open question on 2026-11-05. Due before F-120 makes the pyramid reachable."

## Normative source, transcribed

### The operator's answer, `docs/sprints/CURRENT_SPRINT.md`, "Questions for the operator"

The question, verbatim: "F-119's plan does not say what a NaN, an infinity or a negative zero does to a coarser level. Today the mean passes NaN and infinity through unchanged, so one NaN voxel spreads to eight times its volume at each coarser level, and an all `-0.0` box is written as `+0.0`. No render reaches this yet, because a volume refuses 32-bit samples and nothing calls `generate_pyramid` until F-120. The choices are to let NaN through and have every tier draw it as no value, to leave non-finite samples out of the mean and re-weight, or to refuse to generate. The same answer covers non-finite input to `level_voxel_to_level0`."

The answer, verbatim: "**Answered by the operator on 2026-11-05:** leave non-finite samples out of each mean and divide by the finite weight only, keep a box with no finite sample as NaN, and write negative zero as positive zero. F-222 implements it."

### F-119 decisions this story amends or relies on, `.claude/plans/F-119-design.md`

Decision 1, verbatim: "**Filter: an exact-overlap box filter, computed from level 0.** On each axis, with parent dimension `D` and child dimension `d`, child voxel `c` covers the level 0 continuous interval `[c * D / d, (c + 1) * D / d)`. Multiplying by `d` makes every boundary an integer, `[c * D, (c + 1) * D)` against parent voxel `p` at `[p * d, (p + 1) * d)`. The axis weight is the integer overlap `w(c, p) = max(0, min((c + 1) * D, (p + 1) * d) - max(c * D, p * d))`, and the weights of one child sum to `D`. The 3D weight is the product of the three axis weights and the child value is `sum(w * v) / (Dx * Dy * Dz)`."

Decision 5, verbatim: "**Float: weights as `f64` from `u32`, the mean in `f64`, narrowed by the existing ties-to-even `narrow_f32`.** There is no `as` cast. The weight product is exact while `Dx * Dy * Dz < 2^53`."

Decision 12, verbatim: "**Level geometry: one mapping from a level voxel to level 0 continuous index.** `Level` has no origin. Under the extent-preserving convention of decision 1, level voxel `c` sits at level 0 index `u = (c + 0.5) * D / d - 0.5` on each axis."

Decision 2, which this story keeps, verbatim: "**Every level is computed from level 0, never from the level above it.**"

### HLD section 19, `docs/hld/16-volume-representation.md`

```rust
pub voxel: VoxelKind, // U8 | I16 | U16 | F32
```

### HLD section 30, `docs/hld/26-differentiating-capabilities.md`

Section 30 names bricks, a level axis and level selection, transcribed in full in F-119's plan. It names no filter and says nothing about non-finite samples.

### The code this story changes, read from the tree

`crates/ocelli-volume/src/levels.rs`, `mean_f32` today:

```rust
let sum = fold_parents(full, dims, voxel, 0.0_f64, |sum, weights, parent| {
    let [wx, wy, wz] = weights;
    let factor = weight_f64(wx)? * weight_f64(wy)? * weight_f64(wz)?;
    let sample = read_f32(source, full, parent)?;
    Ok(sum + factor * f64::from(sample))
})?;
let [fx, fy, fz] = full;
let divisor = f64::from(fx) * f64::from(fy) * f64::from(fz);
Ok(narrow_f32(sum / divisor))
```

`crates/ocelli-volume/src/ortho.rs`, `narrow_f32`, the branches that matter here: `if value.is_nan() { return f32::from_bits(0x7fc0_0000); }`, an f64 exponent of `0x7ff` returns the signed infinity, an f64 exponent of `0` returns `f32::from_bits(sign << 31)`, and a subnormal target rounds ties-to-even through `rounded_shift` and keeps the sign, so a negative value that rounds to zero is written as `0x8000_0000`.

`round_ties_away` and `mean_i128` serve the integer kinds, which have no non-finite value. This story does not touch them.

`crates/ocelli-volume/tests/pyramid.rs`, `non_finite_floats_keep_the_current_bits`, whose doc comment reads "This test pins the current behaviour only. The decision is open." It pins a payload NaN with `1.0` to `0x7fc0_0000`, `+inf` with `1.0` to `+inf`, two `-inf` to `-inf`, `+inf` with `-inf` to `0x7fc0_0000`, and the non-finite centre inputs of `level_voxel_to_level0`.

### External definitions the fixtures rely on, cited and not quoted

The standards text is not in the repository, so these are citations a reviewer checks at the source.

- DICOM PS3.5 section 6.2, VR `OF`, a stream of 32-bit IEEE 754 floating point words. PS3.3 C.7.6.24, the Floating Point Image Pixel Module that carries Float Pixel Data (7FE0,0008). Neither gives NaN or infinity a meaning in pixel data.
- IEEE 754-2019 section 6.3, the sign of a NaN is not interpreted, and section 6.2.3, payload propagation is a recommendation. An arithmetic NaN's bits are therefore implementation-defined.
- The WebAssembly core specification, NaN propagation for floating-point instructions, allows a nondeterministic sign and payload in an arithmetic NaN result. Its canonical f32 NaN has payload `2^22`, which with a positive sign is `0x7fc0_0000`.

## What the specification does not cover

No DICOM section and no HLD section specifies a pyramid or a non-finite sample's effect on one. The operator's answer fixes the rule. Every item below is this plan's reading of it, decided by the lead under the operator's delegation of 2026-11-05.

1. **Finite means `f32::is_finite` on the stored sample, tested before widening to `f64`.** NaN of any payload or sign, `+inf` and `-inf` are all non-finite and all skipped. Infinity is skipped because it is non-finite, which is the operator's word, and because a weighted mean with one infinite term is that infinity, which is the same spreading the question describes for NaN. The test is on the `f32` bits, so a signalling NaN is never converted.
2. **The finite weight is the sum of the same `w(c, p)` products decision 1 already computes, over finite samples only.** The mean is `sum(w * v) / sum(w)` over finite `v`. It is accumulated in `f64` from the factor `mean_f32` already forms, and it is exact for decision 5's reason, since every partial sum is an integer no larger than `Dx * Dy * Dz < 2^53`. When every sample is finite the finite weight equals `Dx * Dy * Dz` exactly, so every existing finite fixture keeps its bits.
3. **"No finite sample" is decided by a boolean set when a finite sample is folded, never by comparing a float.** The workspace denies `clippy::float_cmp`, and a flag cannot be confused by a weight that underflows, which this one cannot, but the flag makes the intent plain.
4. **The NaN written for an empty box is the canonical quiet NaN `0x7fc0_0000`, from a named constant.** Positive sign, quiet bit set, zero payload beyond it. It is written from bits, never produced by arithmetic and never copied from an input sample. Justification: an arithmetic NaN's sign and payload differ by target (IEEE 754 6.3, and x86-64 SSE's default NaN carries the sign bit, `0xffc0_0000`), so arithmetic would break "the same bytes on every target". `0x7fc0_0000` is WebAssembly's canonical f32 NaN, it is what `narrow_f32` already writes for a NaN, so the crate writes one NaN pattern on every path, and it is quiet, so no consumer that loads it raises an invalid-operation signal. Rust's `f32::NAN` is not used, because its bit pattern is not guaranteed by the language. The constant lives in `levels.rs`. `narrow_f32` in `ortho.rs` is not edited, because it is the oblique sampler's provider too.
5. **A box with no finite sample is NaN even when every sample is the same infinity.** An all `+inf` box is `0x7fc0_0000`, not `+inf`. The operator's sentence is "a box with no finite sample stays NaN", and an all-infinite box has no finite sample. This changes today's behaviour, where two `-inf` stay `-inf`.
6. **Negative zero is normalised after narrowing, on the `f32` bits.** `0x8000_0000` becomes `0x0000_0000`. After narrowing, because `narrow_f32` can produce `-0.0` from a non-zero negative `f64` mean that underflows, for example `-2^-150`. Normalising the `f64` before narrowing would miss that case. Only exact zero changes. A negative subnormal result such as `0x8000_0001` is kept, because it is not zero.
7. **Integer kinds are unchanged.** `U8`, `I16` and `U16` have no non-finite value and no negative zero.
8. **`level_voxel_to_level0` does not change.** It maps a coordinate through the affine formula of decision 12, and there is no box and no mean in it, so "skip and re-weight" has nothing to act on. The CURRENT_SPRINT question said the same answer covers its non-finite input, and the faithful reading is that its behaviour is settled rather than open: level 0 returns the input unchanged, payload included, and a coarser axis propagates by IEEE arithmetic to NaN or the same-signed infinity. A refusal would need a new `VolumeError` variant, which F-119 declined, and the caller that would produce a non-finite coordinate is F-120's camera, which owns validating it. The coordinate half of the old test is kept as a regression under a name that no longer says the decision is open.
9. **Every level is still computed from level 0** (decision 2). A coarse level's NaN is never an input to another level. AC6 shows that this also changes the answer under re-weighting, not only under rounding.

## Problem and reproducer

Trigger: an `F32` volume `[2, 2, 2]` with samples `1` to `7` and one NaN. `downsample_level([1, 1, 1])`. Expected under the operator's answer: `4.0`. Current behaviour: `0x7fc0_0000`, so the NaN covers the whole child, eight times its volume. A `[2, 1, 1]` box of `+inf` and `1.0` gives `+inf` today and must give `1.0`. A `[2, 1, 1]` box of `0x8000_0001` and `+0.0` gives `0x8000_0000` today and must give `0x0000_0000`.

```bash
bin/ocelli.sh test ocelli-volume --test pyramid non_finite
```

This is a story because F-120 makes the pyramid reachable, and a coarse level that quietly spreads one invalid voxel over a block of tissue is the defect class this repository exists to prevent.

## Scope, API and compatibility

In scope: `mean_f32` in `crates/ocelli-volume/src/levels.rs`, one named constant, and the fixtures in `crates/ocelli-volume/tests/pyramid.rs`.

No public signature changes. `downsample_level` and `generate_pyramid` keep their signatures, errors and doc comments, which gain one sentence each naming the non-finite rule. No new `VolumeError` variant. No new dependency.

Non-goals:

- How a tier draws a NaN voxel. Nothing renders a coarse level until F-120, and F-120 declares its three tier rows. The WGSL specification lets an implementation assume NaN and infinity are absent during shader execution, so F-120 should not rely on a shader NaN test. Recorded here as a hand-off, not as a decision for F-120.
- Float Pixel Padding Value (0028,0122). F-119 Q11 left padding out of scope and this story does not revisit it.
- Constructing an `F32` volume from DICOM. `describe` in `lib.rs` still refuses 32-bit frames.
- `narrow_f32` and the oblique sampler in `ortho.rs`.

Compatibility: level bytes change only for `F32` boxes containing a non-finite sample, and for `F32` boxes whose narrowed mean is `-0.0`. Every integer level and every all-finite `F32` level whose mean is not `-0.0` keeps its bits, which AC7 and AC8 prove.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Volume::downsample_level`, `F32` | `mean_f32`, then `put` | Load time. Owns the finite test, the finite weight, the empty-box NaN and the zero sign | New level bytes |
| `Volume::generate_pyramid`, `F32` | `downsample_level` per level, each from level 0 | Load time, not reachable in production this sprint (F-119 Q10) | Levels whose `F32` boxes follow this rule |
| `Volume::downsample_level`, integer kinds | `mean_i128`, `round_ties_away` | Unchanged | Unchanged bytes |
| `Volume::level_voxel_to_level0` | `level_centre` | Unchanged, decision 8 | Unchanged values |
| `narrow_f32` | `ortho.rs` | Shared with the oblique sampler, unchanged | The narrowed `f32`, whose sign `mean_f32` then normalises at zero |
| `cut_level_brick`, `BrickCache` | `ocelli-cache` | Unchanged, copies whatever bytes the level holds | Unchanged |
| F-120, F-121, F-122 | This story's levels | Later sprints | Not consumers in S46 |

Owner of the finite test, the finite weight, the NaN pattern and the zero sign: `mean_f32` alone. Owner of narrowing: `narrow_f32`, unchanged.

## State transitions and invariants

Not applicable as a stateful change. `mean_f32` is a pure function of level 0 bytes and dims. F-119's state table (completeness refusal, replace on regenerate, drop on `place`) is unchanged and its tests stay green. Invariants added: a child with at least one finite parent is finite, because the mean of finite values lies between their minimum and maximum, and `f64` rounding error is far below an `f32` half-ULP. A child with no finite parent is exactly `0x7fc0_0000`. No child is `0x8000_0000`.

## Ownership, commit and recovery

Not applicable. No resource changes. F-119's ownership table is unchanged, because the level buffer, its commit through `append_level` and the refusals are untouched.

## Numbered acceptance inventory

Every value is hand-computed from F-119 decision 1 and this plan's decisions 1 to 6. Samples are IEEE 754 binary32 words, PS3.5 section 6.2 VR `OF`, in the Float Pixel Data layout of PS3.3 C.7.6.24. Every comparison is by bits.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | One NaN among finite values, re-weighted. `F32` `[2, 2, 2]`, x fastest, `1, 2, 3, 4, 5, 6, 7` and `0x7fc0_0001`. Every weight is 1. `downsample_level([1, 1, 1])` is `28 / 7 = 4.0`, bits `0x4080_0000` | `fixture_one_nan_is_left_out_and_reweighted` | Dividing by the full weight 8 gives `3.5`. Treating NaN as zero gives `3.5`. Propagating gives NaN. RED on today's source, which writes `0x7fc0_0000` | Implementation, `ocelli-volume` |
| AC2 | Infinity is non-finite and skipped. `[2, 1, 1]` `+inf, 1.0` gives `1.0`, `0x3f80_0000`. `[4, 1, 1]` `-inf, -3.0, 6.0, +inf` to `[1, 1, 1]` gives `3 / 2 = 1.5`, `0x3fc0_0000` | `fixture_infinity_is_left_out_and_reweighted` | Keeping infinity in the mean gives `+inf` and NaN. Skipping NaN only, by `is_nan`, gives the same. RED today | Implementation |
| AC3 | A box with no finite sample is the canonical quiet NaN. `[2, 2, 1]` of `0x7fc0_0001`, `0xffc0_0000`, `0x7f80_0001` (signalling) and `0x7fff_ffff` gives `0x7fc0_0000`. `[2, 1, 1]` `+inf, +inf` gives `0x7fc0_0000`. `[2, 1, 1]` `-inf, -inf` gives `0x7fc0_0000`. `[2, 1, 1]` `+inf, -inf` gives `0x7fc0_0000` | `fixture_a_box_with_no_finite_sample_is_canonical_nan` | Copying the first input NaN gives `0x7fc0_0001`. Keeping a same-signed infinity gives `+inf` and `-inf`, today's behaviour, so RED today. Dividing a zero sum by a zero weight gives an arithmetic NaN whose bits are target-dependent before narrowing | Implementation |
| AC4 | Negative zero is written as positive zero. `[2, 1, 1]` `-0.0, -0.0` gives `0x0000_0000`. `-0.0, +0.0` gives `0x0000_0000`. `3.0, -3.0` gives `0x0000_0000`. `0x8000_0001, +0.0` has mean `-2^-150`, which ties-to-even narrows to `0x8000_0000` today and must give `0x0000_0000`. Control: `0x8000_0001, 0x8000_0001` gives `0x8000_0001` | `fixture_negative_zero_is_written_as_positive_zero` | Normalising the `f64` mean before narrowing passes the first three and gives `0x8000_0000` for the fourth. Flushing subnormals or taking `abs` breaks the control. The fourth case is RED today | Implementation |
| AC5 | Mixed weights on an odd axis. `[5, 1, 1]` `10, NaN, 30, 40, +inf` to `[3, 1, 1]`. Child 0: parents 0 and 1 at weights 3 and 2, finite `3 * 10 / 3 = 10.0`, `0x4120_0000`. Child 1: parents 1, 2 and 3 at weights 1, 3 and 1, finite `(3 * 30 + 1 * 40) / 4 = 32.5`, `0x4202_0000`. Child 2: parents 3 and 4 at weights 2 and 3, finite `2 * 40 / 2 = 40.0`, `0x4220_0000` | `fixture_odd_axis_reweights_by_finite_overlap` | Dividing by the full weight 5 gives `6, 26, 16`. An unweighted mean of the finite samples gives `10, 35, 40`. Propagating gives `NaN, NaN, +inf`. RED today | Implementation |
| AC6 | Every level from level 0 under re-weighting. `[4, 1, 1]` `NaN, 1.0, 3.0, 5.0`. `downsample_level([2, 1, 1])` gives `1.0, 4.0`. Then `downsample_level([1, 1, 1])` gives `(1 + 3 + 5) / 3 = 3.0`, `0x4040_0000` | `fixture_reweighting_reads_level_zero` | Downsampling level 1 instead gives `2.5` | Implementation |
| AC7 | Integer kinds unchanged | F-119's `fixture_even_box_rounds_ties_away_from_zero`, `fixture_signed_tie_rounds_away_from_zero`, `fixture_odd_axis_uses_exact_overlap`, `fixture_extremes_do_not_overflow`, unchanged and green | A change that reaches `mean_i128` | Implementation |
| AC8 | All-finite `F32` boxes keep their bits, including `lost_units` at `1_252_698_796` | F-119's `fixture_float_mean`, unchanged and green | Dividing by a finite weight accumulated in a different order or type, which can move `lost_units` by one ULP | Implementation |
| AC9 | `level_voxel_to_level0` is unchanged. The coordinate half of the old test is kept: level 0 returns a payload NaN, `+inf` and `-inf` bit-identical, and a coarser axis returns NaN, `+inf` and `-inf` | `non_finite_centres_follow_ieee_arithmetic` | Applying the sample rule to coordinates | Implementation |
| AC10 | The old pin is gone. `grep -n non_finite_floats_keep_the_current_bits crates/` prints nothing, no comment in `tests/pyramid.rs` or `levels.rs` calls the decision open, and the `docs/lld/volumes.md` sentence "is an open operator question" is replaced by the rule | grep, review | A replaced test whose comment still says the behaviour is provisional | Implementation, then `/complete-feature` |
| AC11 | Generation is the same bytes on tiers A, B and C. No tier participates, and every operation on the finite path is IEEE 754 `+`, `*` and `/` on finite `f64` followed by `narrow_f32`, which is bit-exact integer code. The only NaN is a constant | review against decision 4, `bin/ocelli.sh native` builds the crate for both targets | An arithmetic NaN reaching the output, whose sign differs between x86-64 and AArch64 or wasm | Implementation and review. No wasm32 execution of this test exists, so byte identity on wasm32 is argued, not observed |

Semantic RED, recorded in the progress note: AC1 to AC5 run against today's `mean_f32` and fail with the values named above. Mutations after GREEN, each run and seen red: dividing by `Dx * Dy * Dz` instead of the finite weight (AC1, AC5), testing `is_nan` instead of `!is_finite` (AC2), returning the first NaN's bits (AC3), normalising before narrowing (AC4 fourth case), and folding from level 1 (AC6).

## Approach

1. Add `const CANONICAL_NAN_BITS: u32 = 0x7fc0_0000;` to `levels.rs` with a comment naming decision 4.
2. In `mean_f32`, fold a state of `(sum: f64, weight: f64, any: bool)` through the existing `fold_parents`. For each parent, read the `f32`, and when `sample.is_finite()` add `factor * f64::from(sample)` to `sum`, `factor` to `weight`, and set `any`. Non-finite samples change nothing.
3. If `!any`, return `f32::from_bits(CANONICAL_NAN_BITS)`. Otherwise narrow `sum / weight` with `narrow_f32`, and if the result's bits are `0x8000_0000` return `f32::from_bits(0)`.
4. Replace `non_finite_floats_keep_the_current_bits` with AC1 to AC6 and AC9 in `tests/pyramid.rs`, each with a doc comment citing PS3.5 section 6.2, PS3.3 C.7.6.24 and the operator's answer of 2026-11-05.
5. One sentence each in the doc comments of `downsample_level` and `generate_pyramid`.

Rejected alternatives:

- Propagating NaN and drawing it as no value on every tier. The operator did not choose it, and one invalid voxel would cover 8, 64 and 512 level 0 voxels at levels 1 to 3.
- Refusing to generate. The operator did not choose it, and one bad voxel would deny a series its pyramid.
- Counting finite samples instead of weighting them. It disagrees with decision 1's footprint on an odd axis, which is AC5's mutation.
- Editing `narrow_f32` to drop the negative zero. It is the oblique sampler's provider, and that would change oblique output in a story about the pyramid.
- `f32::NAN`. Its bits are not guaranteed by the language.

## Dependencies, write ownership and resources

Declared dependency F-119 is done. Write set:

- `crates/ocelli-volume/src/levels.rs`, `mean_f32`, the constant and two doc comments
- `crates/ocelli-volume/tests/pyramid.rs`, one test replaced, six added
- `docs/lld/volumes.md`, section "The level axis", at `/complete-feature`

No file is shared with F-220 or F-221, and none with F-208 to F-216. This story may run in parallel with any of them. No GPU, browser, corpus or network resource.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume --test pyramid
bin/ocelli.sh test ocelli-volume --test levels
bin/ocelli.sh clippy ocelli-volume
bin/ocelli.sh native
```

Final profile: `bin/ocelli.sh gate --sprint` at sprint verification. Evidence is invalidated by any change to `mean_f32`, `fold_parents`, `axis_weight`, `narrow_f32` or the constant. No performance claim. The finite test adds one branch per parent sample.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. Levels are host bytes in the worker's store
- Render-loop allocation: none. Generation is a load-time call and its level buffer is F-119's local `Vec`
- unsafe: none
- Tier A (WebGPU): full for generation, which is host CPU work. The same bytes as tier C. Drawing a coarse level is F-120
- Tier B (WebGL2): full for generation, the same bytes. Drawing is F-120
- Tier C (CPU): full for generation, the same bytes. No tier C consumer reads a coarse level in this story

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 to AC6, hand-computed by bits, citing PS3.5 section 6.2 VR `OF` and PS3.3 C.7.6.24 for the sample format, and the operator's answer for the rule | `crates/ocelli-volume/tests/pyramid.rs` |
| fixture | AC7 and AC8 regression, F-119's existing fixtures unchanged | `crates/ocelli-volume/tests/pyramid.rs` |
| unit | AC9 coordinate regression | `crates/ocelli-volume/tests/pyramid.rs` |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no pyramid row, and cornerstone3D 5.8.2 does not build a pyramid.

## Deviations

None. The rule fills a gap the HLD leaves, and the operator chose it.

## LLD impact

`docs/lld/volumes.md`, section "The level axis": replace "What a non-finite F32 sample does to a coarser level is an open operator question, recorded in `docs/sprints/CURRENT_SPRINT.md`" with the rule, the canonical NaN, the zero sign, and that `level_voxel_to_level0` follows IEEE arithmetic.

## User-facing documentation impact

No change to `README.md`, `examples/folder-viewer/src/use.html` or `examples/folder-viewer/src/more.html`. Under F-119 Q10 nothing in production calls `generate_pyramid`, and a volume refuses 32-bit frames, so no page or control reaches an `F32` coarse level. F-120 updates both pages when a level becomes reachable.

## Open questions

None open. The rule is the operator's answer of 2026-11-05, quoted above. The readings of it are lead decisions.

- Q1, all-infinite box. Answered: lead decision under the operator's delegation of 2026-11-05. It is NaN, decision 5, because it has no finite sample.
- Q2, NaN bit pattern. Answered: lead decision under the operator's delegation of 2026-11-05. `0x7fc0_0000` from a constant, decision 4.
- Q3, where the zero sign is fixed. Answered: lead decision under the operator's delegation of 2026-11-05. After narrowing, decision 6.
- Q4, `level_voxel_to_level0`. Answered: lead decision under the operator's delegation of 2026-11-05. No change, decision 8.
