# F-030 review, pass 1

**Reviewed**: the staged working tree, `bin/ocelli.sh gate --sprint` green at
tree `bb39be3a942c` before this pass's remediations.
**Result**: 1 defect, 3 smells, 0 nitpicks

## Defects

### D1, two wildcard match arms would absorb a new colour space as RGB

**Where**: `crates/ocelli-pixel/src/color.rs`, `ColorTransform::pixel`, the
`_ => Rgb { .. }` arm inside the `Rgb | YbrFull` branch and the
`_ => &FULL_RANGE` arm inside the 4:2:2 branch.

**What**: both outer arms matched two variants and then re-matched `self.space`
with a wildcard. Adding a sixth `ColorSpace` variant and routing it into either
outer arm would compile, and the new space would be treated as pass-through RGB
in the first case and as full-range YCbCr in the second.

**Why it is wrong**: HLD section 31's rule, as `CLAUDE.md` generalises it. A
feature that cannot run reports unavailable and never quietly produces a
different result. A wildcard arm converts "this space is unhandled" into "this
space is RGB", which is a plausible image in the wrong colours, and no test can
exist for a variant nobody has added yet. The compiler refusing to build is the
only guard that works here.

**Evidence**: `grep -n "_ =>" crates/ocelli-pixel/src/color.rs` returned two
hits before the fix and none after. `pixel` now matches `self.space` once,
exhaustively, with the shared index arithmetic hoisted into two closures.

**Remediated in this pass.** The five mutations below were re-run against the
restructured function rather than assumed to still hold.

## Smells

### S1, "F-030 is their first consumer" was false as written

**Where**: `docs/lld/pixel-pipeline.md`, colour stage section.

**What**: the sentence claimed F-030 is the first consumer of
`DecodePhotometricInterpretation` and `DecodeSampleLayout`. Those types are in
`ocelli-codec`, and `ocelli-pixel` does not depend on it, so F-030 consumes
neither type. It consumes the evidence, through mirrored types.

**Evidence**:
`grep -rln "decode_photometric_interpretation\|decode_sample_layout" crates/*/src/`
returns four files, all in `ocelli-codec`, and all of them are decoders
declaring their own behaviour rather than readers deciding anything.

**Remediated**: the sentence now says what is true, that until F-030 nothing
read them to decide anything, and that F-030 reads the evidence through
mirrored types.

### S2, "every quirk case" where there is exactly one quirk

**Where**: `docs/lld/corpus.md`, the case-selector section.

**What**: "reported every quirk case as unreachable" is a plural claim over a
set of size one, which reads as broader evidence than was gathered.

**Evidence**: `corpus/quirks.json` holds one quirk, `sigmoid-width-below-one`.

**Remediated**: the sentence names the one case, and now also records the
mutation that proves the loosened reachability rule still discriminates, which
the original sentence did not claim at all.

### S3, `first_input_bits` justified bit equality without covering negative zero

**Where**: `crates/ocelli-pixel/src/lut.rs`.

**What**: the doc comment said the value is "never a NaN, so bit equality is
value equality". NaN is not the only way that inference fails. `-0.0` and `0.0`
compare equal as values and differ in bits, and the comment did not say why
that cannot arise. The code was correct and the justification was incomplete,
which is the shape that lets a later edit break the invariant while the comment
still looks like it covers the case.

**Evidence**: `descriptor_input_to_f32` reaches its negating branch only when
`value < 0`, so the magnitude it negates is at least one and `-0.0` is
unreachable. Read from the function rather than inferred.

**Remediated**: the comment now names both failure modes and why each is
unreachable.

## Nitpicks

None.

## Verified clean

**Arithmetic.**

- `git diff --cached -- '*.rs' | grep -E "^\+.* as (u|i|f)[0-9]"` returns
  nothing. **F-030 adds no `as` cast.** Conversions use `try_from`,
  `f32::from` or `usize::from`.
- The same grep for `.unwrap(` and `.expect(` returns nothing in `src/`. The
  `unwrap_or` calls in tests are total and cannot panic.
- The two inverse matrices were re-derived in exact rational arithmetic from
  the transcribed forward equations, independently of the implementation.
- The claimed divergence from BT.601 was **proved rather than sampled**. The
  difference of the two inverses is a linear map, so its maximum over
  `[0,255] x [-128,127]^2` is at a vertex. Evaluated at all eight vertices in
  exact arithmetic the maximum is `0.020027735`, at `(Y, Cb, Cr) = (0, 0, 0)`,
  which rounds to the `0.020028` the code and the LLD state. The fixture
  tolerance of `0.001` is `20.03` times tighter, so the BT.601 substitution
  cannot hide inside it. The earlier figure came from a step-5 sweep and is now
  exact.
- Boundary arithmetic in the 4:2:2 path was checked for out-of-range indexing
  by hand. With `pixels` even and `source.len() == 2 * pixels`, the largest
  index reached is `4 * (pixels / 2 - 1) + 3 = 2 * pixels - 1`. Interleaved
  reaches `3 * pixels - 1` and planar reaches `index + 2 * pixels` at most
  `3 * pixels - 1`. All inside the length `map_into` checked.
- Stage 2's `<=` low and `>` high, and `LINEAR`'s `c - 0.5` and `w - 1`, are
  untouched by this story. Confirmed by diff: `lut.rs` gains three accessors
  and one doc comment and no arithmetic.

**Would the tests fail if the code were wrong.** Eight mutations, each applied
to the source, run, and reverted. Every one turned a test red and the tree was
confirmed green again after each revert.

| Mutation | Result |
|---|---|
| `1.401_987_6` to BT.601's `1.402` | red |
| `PARTIAL_RANGE` offset `16.0` to `0.0` | red |
| 4:2:2 counted as three samples in `color.rs` | red |
| 4:2:2 counted as three samples in `stored_pixel.rs` | red |
| decoder `Rgb` evidence ignored, so conversion happens twice | red |
| `YbrPartial422` routed to `FULL_RANGE` | red |
| interleaved indices replaced with planar ones | red |
| palette first-mapped-input offset ignored | red |
| palette descriptor cross-check removed | red |
| JPEG-LS codestream header check removed | red, both colour tests |
| `case_sigmoid_width_half` dropped from `case_callables` | `quirk_check` red |

The first is the strongest evidence in the story: it moves one coefficient by
`0.0000124` and the fixture still fails.

**Fixture provenance.** Every expected value in `tests/color.rs` and
`tests/palette.rs` was computed in a scratch Python script from PS3.3's stated
equations and descriptor rules, before the Rust existed. None was read from
Ocelli's output. The fixtures state YBR and assert RGB, never the reverse,
because the standard's rounded forward matrix sends a saturated primary to
`Cb = 255.5`, which the wire cannot hold.

**Things that exist and that nothing executes.** The new corpus rows are not in
that class: the palette rows are asserted by three Python tests and the
multi-component row by a Rust test that goes red when the header check is
removed. `stored_samples_per_pixel` is public with no caller outside the crate,
which was weighed rather than left: the alternative leaves `samples_per_pixel`
as the only public answer to a question it answers wrongly for two
interpretations. The doc comment now records that reasoning.

**Prose claims executed.**

- `ocelli-codec` carries five codec dependencies. Counted from its manifest.
- `crates/ocelli-codec/src/jpegls.rs` refuses `samples_per_pixel != 1` at line
  129. Read.
- `crates/ocelli-codec/src/jpeg2000.rs` refuses `components != 1 || mct != 0`
  at line 285. Read.
- `docs/lld/errors.md` does not enumerate `PixelError`, so the six new variants
  need no entry and no `ci/error-codes.json` row. The plan's LLD impact list
  said "to be checked" and now records the answer.
- The manifest gained exactly three rows and no existing digest moved. Verified
  by diffing the manifest against a copy taken before the change.
- The multi-component JPEG-LS refusal already had a codestream behind it, which
  the plan did not know. Recorded in the progress note, and the new row is
  justified on what it actually adds: manifest backing and `ILV = 2` against
  the existing fixture's `ILV = 1`.

**Boundary and tier.** No `wasm-bindgen`, no pixels crossing the boundary, no
render-loop allocation, no `unsafe`. `bin/ocelli.sh gate nostd` green, so
`ocelli-pixel` keeps its `no_std` posture with the new module. No shader and no
second copy of the LUT arithmetic: `PaletteColorLut` delegates to
`LutDescriptor::lookup` and adds none.

**Structure.** No new trait, generic or `Box<dyn>`. `PixelDataEncoding` is a
two-variant enum rather than a `bool` because the call site carries three
two-state arguments and positional booleans there are unreadable.
