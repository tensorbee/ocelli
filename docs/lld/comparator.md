# The comparator, the oracle's judging half

**F-IDs that contributed:** F-011
**Last updated:** 2026-09-05

HLD section 11 says the harness "pushes the same study through both stacks and
compares frames within a written per-modality tolerance, with metadata diffed
alongside pixels because a wrong rescale slope can still produce a plausible
image". `docs/lld/oracle.md` is the half that renders. This is the half that
judges. Read them together, because they are one instrument.

The tolerance policy is `docs/hld/22-testing-and-tolerance.md` section 25.1 and
it is written down once and held. **A tolerance change is a pull request with a
rationale, reviewed like code.** Every number this file gates on lives as a
constant in `tools/oracle/src/tolerance.rs` with 25.1 quoted above it, and not
in a configuration file, precisely so that changing one is a diff a reviewer
sees.

## What it compares today, stated plainly

**No port code exists.** Decision D7 is that the validation oracle exists before
the port code, and F-011 did not break it. There is no Ocelli renderer, so
there is no Ocelli frame, so the comparator has no second side of the corpus to
compare against. A stub renderer was refused in the design round: it would be
port code written before it was designed, and its frames would prove only that
the stub and the comparator agree.

So the comparator is a function of two directories, and today those two
directories are filled by three things, none of which is a renderer.

1. **Hand-constructed fixture frame pairs**, in `tools/oracle/tests/`, whose
   divergence is known by construction because both sides' bytes are written
   out by hand. They run under `cargo test --workspace`, which is the `test`
   gate, which is in `--floor`, so the comparator's own arithmetic is proved in
   CI with no GPU, no browser and no corpus. Under deviation D-04 that is a
   real strengthening of what CI covers.
2. **Identity over the real reference output**, `ocelli-compare identity`. It
   proves the loader, the identifier mapping, the tolerance-class resolution,
   the sidecar contract and the report shape over all ninety-eight views. **It
   proves nothing whatever about detection**, which is why it is never allowed
   to be the only corpus-scale exercise.
3. **A declared mutation catalogue**, `tools/oracle/src/mutations.rs`, applied
   in memory to real reference frames with the verdict each entry must produce
   written down beside it. That is what proves detection at corpus scale. It
   follows `src/faults.mjs`'s split: the catalogue is production data in
   `src/`, the runner is separate, and every entry is replayed on every oracle
   gate rather than trusted from a note saying somebody once watched it fail.

**The candidate side is a directory contract and not a call into a renderer.**
When the port lands, the Ocelli half writes `<id>.raw` and `<id>.json` in the
shape `docs/lld/oracle.md` already specifies, points the comparator at it with
`--candidate`, and nothing here changes. That is also what makes deviation
D-07's requirement that "the divergence bound has to cover tier A against tier
C" this same binary with two candidate directories and no reference, at no
additional cost and with no code change when F-X001 to F-X004 land.

## Where it runs

```bash
bin/ocelli.sh compare                       # identity, then the catalogue
bin/ocelli.sh compare identity --candidate DIR
bin/ocelli.sh gate oracle                   # renders, then compares
```

`bin/ocelli.sh gate oracle` is `"$0" oracle && "$0" compare`, chained with `&&`
for the reason that file already comments on: a case arm returns the status of
its LAST command, so an unchained render could fail and be reported green by a
passing comparison. **No new gate name was added.** The comparator is part of
what `oracle` means.

The binary is built in release. A debug comparison over ninety-eight frames
plus twenty mutation replays is minutes rather than seconds, and a check nobody
wants to wait for is a check that stops being run.

The corpus-scale exercises are subcommands of a binary and **not** `#[ignore]`
tests. An ignored test that needs `tools/oracle/out/` reads as a pass on the day
it did not run, and this repository refuses a skip that reports as a success.

## The input contract, asserted on load

`tools/oracle/src/sidecar.rs` refuses rather than assumes. Each of these is a
place a later story could otherwise break the comparator silently.

- Every top-level `run.json` key ending in `Sha256` is present on both sides and
  equal. **Collected by suffix and not from a fixed list**, because F-X007
  added `volumeParamsSha256` and `volumeTruthSha256` and a fixed list of two
  would have quietly stopped covering the inputs that decide the frames. A
  disagreement is a problem before a single pixel is read, because
  `render-params.json`'s own note says a change there changes every reference
  frame, and two frames produced under different parameters are two correct
  frames that differ.
- **The view list is the union of the declared frame lists, and a `.raw` that no
  declared list names fails the run.** `run.json`'s `rows[]` stays stack-only on
  purpose, so that the accounting identity `readBack + unsupported ==
  applicable` means something exact, and F-X007 put its reformats in
  `volumes[].frames[]`. A comparator reading `rows[]` alone would compare 89 of
  98 views and report success. The orphan refusal is what turns that into a
  failure, and it keeps working when a later story adds a third list.
- **Each sidecar carries a top-level `kind` and the comparator switches on it.**
  An unknown value is refused and never defaulted to `stack`, because defaulting
  is how a new view kind gets compared under the wrong rules. The kind is
  re-parsed from the sidecar JSON during structural verification rather than
  read from a field cached at load, so a sidecar edited after load is refused by
  the same code that refuses one edited before it.
- The identifier is **opaque**. It is never reconstructed from a manifest path
  and the work list is never a glob, so a later story may mint a new identifier
  scheme and nothing here needs to know it.
- **The mapping from a manifest path to view identifiers is one to many.**
  `real/mr_eay131/00000001.dcm` is one stack view and a member of a volume whose
  three reformats are three more, so that path resolves to four identifiers.
  Which of them a census entry means is decided by the entry's own `kind`, and a
  `path`-keyed entry is filtered by it. Without that filter a saturated stack
  row that happened to be a series member would silently mark three reformats
  as weak too.
- Each `.raw` is `width * height * 4` bytes and hashes to its sidecar's
  `frame.sha256`. **The reference half hashes each frame twice already**, in the
  page and in the driver. The comparator hashes a third time at read, so a file
  edited after the run is refused rather than compared as though somebody had
  rendered it.
- The identifier sets on the two sides are equal. An identifier on one side only
  is a run failure and never a skip.
- The frame dimensions are read from each sidecar rather than from
  `render-params.json`, because a later story may render a view at a size the
  base canvas does not declare.

## Tolerance class resolution

From the manifest category tokens the sidecar carries, matching `CLASS_TOKENS`
in `scripts/corpus_check.py`: `mono16` is class one, `colour` or `us` is class
two.

**Never from the modality.** Modality does not resolve: four corpus rows are
`OT` and one is `DX`, and 25.1's first bullet names neither, while
`corpus_check.py --coverage` already fails a row that declares no class token.
So the token is a checked declaration and the modality is not. One row,
`synthetic/us_ybr_full_422.dcm`, carries both `us` and `colour`, and both are
class two, so it resolves rather than conflicting. A row carrying `mono16`
alongside either of them is refused.

**A volume sidecar carries no `row` block at all**, which F-011's design plan
assumed it would. A reformat's class is resolved from its members' own stack
sidecars instead, through `volume.members[].stackSidecar`. The class token is a
property of the pixel data and every member of a series carries the same one.

Measured on the current corpus: 84 class one and 5 class two among the stack
views, and all 9 volume reformats class one.

## The per-view record, and what gates

Four components, evaluated in order, and the first that answers attributes the
divergence. That order is the concrete answer to "can attribute a divergence to
a side".

### Component 1, input identity

The `*Sha256` digests above, plus `row.sha256` per view, which is in the
compared field list. A mismatch attributes the divergence to the inputs and not
to either renderer.

The comparator does not read `corpus/manifest.tsv` itself, deliberately: it must
run against two output directories on a machine that has no `corpus/data`. Two
sides agreeing on `manifestSha256` and on each view's `row.sha256` is the same
claim reached without that coupling.

### Component 2, parameters

Every declared field that decides the compared pixels and the compared camera,
compared for exact equality. This is HLD section 11's "metadata diffed alongside
pixels", and it runs BEFORE the pixels rather than beside them, because a
parameter divergence explains a pixel divergence and the reverse is not true.

The two field lists are `STACK_PARAMETER_FIELDS` and
`VOLUME_PARAMETER_FIELDS` in `tools/oracle/src/attribution.rs`.

Floating-point values here are DICOM-declared numbers read from a file, not
computed results, so the comparison is exact equality implemented as
`f64::to_bits` equality. That satisfies the denied `float_cmp` lint honestly
rather than by an allow, it makes the intent visible, and it is correct: two
readers of one `DS` value that disagree is a finding and not a tolerance. A
`NaN` in any of these fields is refused outright rather than compared, because
every comparison with a `NaN` is false and a comparator that tested one would
report agreement. Two consequences are worth knowing. JSON `1` and `1.0` compare
EQUAL, because a representation difference between two writers of the same
number is not a divergence in what the file declared. Positive and negative zero
compare UNEQUAL, because a sign flip on a rescale intercept is a finding.

**This is not the full metadata harness. That is F-013, E2.5, S04.** F-011 diffs
only what decides the compared pixels and the compared camera, and says so.

### Component 3, geometry

25.1's third bullet. `position`, `focalPoint`, `viewUp` and `parallelScale` are
compared within 1e-6 mm, and `viewPlaneNormal` too where a reformat carries one.
A view plane normal present on one side only is a divergence rather than a field
quietly skipped.

The canvas half is applied to the derived image extent, within a quarter pixel,
and the offsets are compared as well as the sizes, because two extents of the
same size in different places are two different pictures.

**The canvas scale is read and never re-derived.** F-X007 publishes
`canvasPixelsPerSourcePixel` on every stack sidecar rather than only on the two
rows listed under `downsampled`, which was decision 13 of F-011's design round,
taken so this crate would not hold a second copy of `canvasScale` from
`tools/oracle/src/params.mjs`. HLD section 18's rule that a piece of arithmetic
exists exactly once is about the LUT chain, and the reasoning generalises.

### Component 4, pixels

Alpha is asserted to be 255 on every pixel of both sides and is never included
in a difference, because a difference in alpha is a difference in the canvas and
not in the image. A class-one view is asserted to be monochrome on both sides,
red equal to green equal to blue, since a `mono16` token over a frame that is
not monochrome means the token is lying. The comparison is then over one lane
for class one and three for class two.

Per view: the sample count, `maxAbsDiff`, the counts at difference 0, 1 and 2
and over 2, `fractionWithinOneLsb`, `signedMeanDiff` as candidate minus
reference, `differingFraction`, the 99.9th percentile of the absolute
difference, `rowsTouched` and `columnsTouched`, and all of it four times over:
the full frame, the image rectangle, the letterbox, and the informative subset.
Every one of those falls out of one 256-bucket histogram per lane, computed in a
single pass, so no two regions can be taken over different readings of the same
buffers.

**The gating predicate for class one, and only this:**

> `fractionWithinOneLsb >= 0.999` and `countOverTwo == 0`, over the full frame

At least 99.9% of pixels within 1, and no pixel exceeding 2. A pixel differing
by exactly 2 is permitted, for up to 0.1% of the frame. A pixel differing by 3
is never permitted, at any count.

Plus 25.1's bias bullet, over the informative region:

> `|signedMeanDiff| <= 0.1`

The full frame is the region 25.1's first bullet is evaluated over, because it
says "of pixels" and excludes nothing. The letterbox agrees by construction on
both sides, so counting it inside the denominator inflates the pass rate by up
to a quarter. The same statistics restricted to the image rectangle are reported
alongside, so the inflation is visible rather than absorbed.

**There is no gating predicate for class two.** See deviation D-16 below.

### The image rectangle

For a stack view: `columns` and `rows` times the published canvas scale, centred
in the canvas. A canvas pixel is inside when its CENTRE is, so pixel `i` is
inside when `i + 0.5 >= x0` and `i + 0.5 < x0 + width`, and the result is
clamped to the canvas. The alternatives are wrong in a way that is invisible.
Flooring the offset admits a letterbox column and ceiling the far edge admits
one on the other side, and at 512 rows either is 512 background pixels inside
the region the bias bullet averages over.

`docs/lld/oracle.md`'s worked case is the fixture:
`syntax/reference_mono12.dcm` at 64 by 96 with a published scale of 8 vertical
and 4 horizontal gives a 384 by 512 rectangle at offset 64, so the letterbox is
512 rows by 128 columns, which is 65536 pixels, which is 0.25 of the frame
exactly. The reference measured 65536 black pixels on that frame and recorded
`blackFraction: 0.25`, so the derivation is checked against a number the
instrument produced.

**For a volume reformat the image rectangle is the whole frame**, and that
narrowing is stated rather than left to be noticed. A reformat plane is a cut
through a volume and has no source pixel grid to be a magnification of, which is
why `docs/lld/oracle.md` deliberately does not publish
`canvasPixelsPerSourcePixel` for one. Deriving a rectangle from the volume
bounding box would be a second copy of a derivation the reference did not
publish, which is the thing decision 13 exists to avoid. The consequence is that
a reformat's letterbox counts as image, so its bias bullet is averaged over a
region containing background that agrees by construction, which makes the bound
slightly LOOSER there than on a stack view. Worth a story if a reformat ever
gates on bias in anger.

## The verdict vocabulary

`pass`, `fail`, `unmeasured` and `absent`, plus a qualifier set. 25.1 gives a
predicate and does not say what a comparator reports, so this vocabulary is
F-011's construction.

**`unmeasured` is a third outcome and not a synonym for `pass`.** A run that
reports "70 pass, 0 fail, 28 unmeasured" over 98 views is saying something a run
that reported "98 compared" would not. The number is uncomfortable on purpose:
better than a quarter of the corpus is covered and not measured, and that was
true before F-011 and invisible.

| Qualifier | Meaning |
|-----------|---------|
| `weak` | Listed under `lowInformation` and below the informative floor |
| `decimated` | Listed under `downsampled`, so the pixel statistics do not gate |
| `unstated-threshold` | Class two, deviation D-16 |
| `parameter-divergence` | The two sides declare different pixel-deciding values |
| `geometry-divergence` | Outside 25.1's geometry bound |
| `reference-divergence` | Explained by the register or by F-X007's measurement |
| `bias` | Outside 25.1's bias bound |
| `letterbox-only` | The difference is in the fit and not in the picture |
| `divergent-while-unmeasured` | The predicate failed on an unmeasured view |

The last one is the second half of the mechanism that stops `unmeasured` being a
place things go to be forgotten. The first half is the census. If a view's
outcome is `unmeasured` and the pixel predicate nonetheless FAILED, the
instrument has answered, and reporting that as "cannot tell" would absorb a
measured divergence, which is the exact defect this sprint names. Such a view
carries `divergent-while-unmeasured` and fails the run on its own.

## The attribution ladder, in order

1. **Inputs disagree**, by digest. Attributed to the inputs, not to either
   renderer.
2. **Parameters disagree.** Attributed to the side that disagrees with the
   INDEPENDENT reading of the bytes. That third reading exists already: the
   sidecar's `attributes` block is read straight from the file by
   `dicom-parser` in the page, independently of the render path, and
   `check_sidecars.py` cross-reads it under pydicom. So if one side's
   `/image/slope` agrees with its own `/attributes/rescaleSlope` and the other
   side's does not, the second side is at fault, and the record says which and
   why. Where a field has no independent counterpart, or where neither side
   agrees with its own, the divergence is reported unattributed rather than
   assigned by default.
3. **A register entry matches, or F-X007's own `referenceDivergence` is set** on
   the subject and the pixels diverge with parameters agreeing. Attributed to
   the reference, with a PS3.3 citation where the register supplies one.
4. **Geometry is outside 25.1's bound, or the difference is confined to the
   letterbox.** Attributed to the fit rather than to the LUT chain.
5. **Pixels diverge with parameters and geometry agreeing.** **Attributed to
   ours**, by default. HLD section 11 makes cornerstone3D the reference and
   deviation D-11 makes the pin the definition of correct until an entry in the
   register says otherwise. That default direction is the conservative one, and
   it is what makes the instrument useful under D7: the burden is on us to show
   the reference is wrong, not on the reference to show it is right.

`rowsTouched` and `columnsTouched` are PUBLISHED and do not automatically
attribute. A resampling phase error touches whole rows and columns while a LUT
error is scattered, and that distinction is the useful signal, but the threshold
that separates them is a number nobody has stated. Rung 4 covers the two cases
that need no threshold, the geometry bound and the letterbox, and the spatial
statistics are in the record so a human can raise the third.

## What the bias bound is, and what it is not

25.1 gained a fourth bullet in S03, by operator decision through F-011's design
plan: signed mean difference over the **informative region** within 0.1 of one
display code, evaluated only where input identity, declared parameters and
geometry already agree.

**It said "image rectangle" until the sprint review's second pass, and over that
region it detected nothing.** The per-pixel divergence is exactly `u / w`, where
`u` is the LINEAR display value, so the mean over a region is `mean(u) / w`. A
pixel clipped to black or white on both sides differs by nothing whatever the
arithmetic underneath says, so averaging over the whole rectangle divides the
divergence the unclipped pixels do show by a denominator full of pixels that
structurally cannot show one. Measured over all 71 gating class-one views on
this corpus, the largest observable bias over the image rectangle was **0.0825**
and a 0.1 bound caught **none of them**. Over the informative region the same
soft-tissue CT rows run to about 0.28.

It exists because of a finding this story made. At the soft-tissue window,

```text
LINEAR(x) - LINEAR_EXACT(x) = 255 * (x + 160) / 159600
```

which is 0 at LINEAR's lower clamp of -160 and 0.6375 at its upper clamp of 239,
and never exceeds 0.639 over the whole window. **A difference bounded by 0.639
of a display code cannot survive quantisation to eight bits as more than one
code under any monotone quantiser.** So a whole-frame swap between the two
window functions always yields `maxAbsDiff <= 1`, `countOverTwo == 0` and
`fractionWithinOneLsb == 1.0`, and passes 25.1's maximum-difference rule by a
wide margin. The divergence HLD 18.3 calls "the entire argument for building the
oracle before writing the code it validates" could not be failed by the
tolerance the HLD wrote down. `tools/oracle/tests/voi_divergence_fixture.rs`
asserts exactly that, and asserting it is the finding rather than a bug in the
test.

**What the bound is proven to do.** It detects the actual swap, and the
mutation that proves it is `the-actual-linear-exact-swap`, which applies
`round(u - u/w)` to every pixel using the view's own declared window. That is
the divergence rather than a stand-in for it.

**`plus-one-on-two-fifths-of-the-image` is kept and is not that proof.** It
moves 40 per cent of the image by a whole code where the real divergence moves
each pixel by a sub-code amount that only sometimes crosses a rounding
boundary, so it clears the bound several times over. It proves the bound catches
a large one-sided difference. It says nothing about whether the bound catches
the divergence HLD 18.3 is about, and reading it as that proof is what let the
bound ship over a region where it detected nothing.

**The region is load-bearing and is proved so.** Reverting the evaluation to the
image rectangle makes `the-actual-linear-exact-swap` come back `NOT DETECTED`
with the view passing, and the run exits 1 on the undetected mutation. Restoring
the informative region detects all 21.

**A structural limit, stated because no region choice removes it.** The largest
divergence any view can show is `255 / w`, so **a view whose window is wider
than 2550 cannot reach this bound at all**. The corpus already carries rows at
`w = 4096`, including `real/dx_varepop/00000001.dcm` and
`synthetic/cr_monochrome1.dcm`. Those views are not protected by this bullet and
nothing here pretends they are.

**What the bound is not.** It is NOT calibrated against measured divergence.
**Its false-positive rate is unmeasured and unmeasurable in this sprint**,
because measuring it needs two independent renderers and only one exists. An
identity comparison of the reference against itself returns exactly zero bias
and therefore proves nothing whatever about the bound. The specific untested
risk is a rounding-convention mismatch on pixels whose display value lands
exactly on a half, which is rare for continuous output and not rare for a
synthetic ramp, and all fifteen saturated `syntax/` rows are ramps. The sixteenth saturated
STACK row is `synthetic/ct_unsigned_16`, which is a ramp too but is not under
`syntax/`.
Those rows do not gate, for other reasons. If a real divergence is later
measured that this bound misclassifies, widening it is a reviewed change and not
a fix.

The bound is 0.1 and the derivation is on the record rather than chosen for
roundness. It sits roughly three times below the 0.32 mean the section 18.3
worked example produces at the window centre, and far above the zero expected
when two implementations agree, because the divergence is one-sided and rounding
noise is not. That is why it is evaluated only after the parameter rung has
already agreed, and why the rounding convention is part of what the parameter
rung compares.

## The low-information views

Twenty-two of the ninety-eight, sixteen stack and six volume reformats. The
comparator reads `run.json`'s `lowInformation.rows` rather than rederiving
saturation from the frames, because the reference half already computes it
against a declared threshold in `render-params.json` and a second derivation
would be free to drift from the first.

For each listed view it measures `informativeFraction`, the fraction of image
rectangle pixels not clipped to the same extreme on both sides, and marks the
view `weak` and `unmeasured` when that is below `INFORMATIVE_FRACTION_FLOOR`.

**The floor is 0.10 and it is declared in `tools/oracle/src/tolerance.rs`**
beside the 25.1 constants, not in `render-params.json`, because the reference
half's configuration must not decide a comparator verdict. **It is not from 25.1
and is marked as such at the site.** The reasoning: a pixel clipped to the same
extreme on both sides agrees by construction, so the largest differing fraction
a view can ever produce is its informative fraction, and 25.1 permits 0.1% of
the frame to differ by more than one code. A view whose informative subset is
smaller than that budget could not fail the predicate even if every informative
pixel were wrong. That is the necessary condition, and 0.10 keeps roughly two
orders of magnitude above it, so a defect touching one percent of the informative
pixels can still fail.

Measured on today's corpus the two mechanisms agree without being made to:
every one of the twenty-two views the reference lists has an informative
fraction at or below 0.0583, and the lowest fraction among the other
seventy-six is 0.1007. Both AXIAL synthetic reformats are at exactly zero, every
pixel being black or white. That separation is an observation recorded after the
fact and not the reason for the number.

A zero-information view stays `unmeasured` rather than `fail`, because it is a
corpus problem and failing it would pressure somebody to widen a window.

**What F-011 deliberately did not do.** It did not add a second render at a
wider window, and it did not touch `render-params.json` or
`scripts/corpus_synth.py`. A comparator that widened a window to make its own
numbers look better would be the tolerance-widening failure in a different coat.

## The two decimated views

`real/dx_varepop/00000001.dcm` renders at 0.438 canvas pixels per source pixel
and `real/us_cmb_crc/00000001.dcm` at 0.625. Under `NEAREST` a sub-source-pixel
difference in the fit selects a different source pixel, so the difference at a
canvas pixel becomes the difference between two unrelated stored values, which
is unbounded. A perfect LUT chain can fail 25.1 by hundreds of codes on these
two views for a reason that has nothing to do with the LUT chain.

**25.1's own geometry bound does not rescue them.** A quarter of a canvas pixel
is 0.570 source pixels at the DX row's scale, which is more than half a source
pixel, so two cameras agreeing inside 25.1's written canvas tolerance can still
land on different source pixels. No written tolerance covers a decimated frame.

So they are `unmeasured` with the qualifier `decimated`, their geometry
component still gates at 25.1's written bound, and their pixel statistics are
computed and reported in full. Giving those two rows a magnified render is a
change to `render-params.json` and therefore to every reference frame, and it
belongs in its own reviewed story.

## Class two, and deviation D-16

25.1 requires class two to be "below a stated threshold" and states no threshold
and names no metric. The comparator measures per-channel `maxAbsDiff`, the
99.9th percentile of the absolute difference, `differingFraction` and
`signedMeanDiff`, and returns `unmeasured` with the qualifier
`unstated-threshold`. **It never returns `pass` for a class-two view**, because
a pass would be a claim against a bound nobody wrote, and decision D14 says to
claim measured divergence rather than a bound we invented. That is deviation
**D-16**.

It implements no perceptual metric. Choosing CIEDE2000 without a threshold
produces a number nobody can act on, and the metric and the threshold are one
decision rather than two.

**The known gap, recorded here so the next 8-bit greyscale row does not
re-derive it.** `real/us_cmb_crc/00000001.dcm` is an 8-bit MONOCHROME2
ultrasound. HLD 25.1 has no class for 8-bit monochrome at all, and
`docs/lld/corpus.md` absorbs it into class two by modality, which under D-16
leaves it with no evaluable bound whatever. It is also one of the two decimated
views, so it collects both `unstated-threshold` and `decimated`. The operator
did not add a fourth class to 25.1 for it in S03. Every record carries a
`monochromeFrame` flag for exactly this reason, so a class-two view whose frame
is grey on every pixel is visible in the report rather than inferred.

That flag also caught something real. The first class-two view in identifier
order is that greyscale ultrasound, so a red-and-blue channel swap aimed at "the
first class-two view" changes NOTHING, and the catalogue would have reported a
guard as watched while measuring a frame the mutation could not damage. The
catalogue now has two class-two targets and two entries.

## The reference-divergence register

`tools/oracle/reference-divergence.json`, committed. It exists because D-11
makes the pin "the definition of correct for this project", so a place where the
pin is wrong about PS3.3 has to be recorded or our correct arithmetic is
reported as a defect.

Each entry carries an identifier, a match condition over the reference
sidecar's fields, a PS3.3 citation, what 5.8.2 does, what the standard requires,
the expected shape of the pixel effect, a `reachable` flag with its reason, and
the F-ID that raised it. When a view's divergence is explained by a matching
entry, the outcome is `unmeasured` with the qualifier `reference-divergence` and
the record names the entry, rather than `fail`.

The match language is deliberately small: an RFC 6901 pointer mapped to exactly
one of `equals`, `lessThan` or `greaterThan`, and every condition must hold. **A
test the comparator cannot evaluate is refused at load rather than read as
unsatisfied**, because a condition nobody evaluates is an entry that never fires
and nobody notices.

**The strictness is asymmetric, and that is deliberate.** An entry marked
`reachable: false` that FIRES is a run failure, because the claim was wrong.
The opposite direction, an entry no row exercises failing the run, is
`unsupported.json`'s other half and is not applied while the only entry is
unreachable by today's corpus, since it would fail every run from the first one.

**A new entry is a reviewed change with a rationale, exactly like a tolerance
change.** The register is the one mechanism in this design that can turn a
failure into a non-failure, so it gets the same handling as the other one.

Its only entry on day one is the SIGMOID width case F-010 recorded, carrying
`reachable: false` with the reason that all eighty-five windowed corpus rows
resolve LINEAR. Resolving the case itself is F-X012, S04.

### Two mechanisms for "the reference is wrong", and why both

F-X007 gives each `volumes[]` entry a `referenceDivergence` field and a
committed `volume-truth.json`. That is a per-run MEASUREMENT of one specific
thing, the reference's volume geometry against a truth the corpus generator
knows by construction, written into the run record. The register here is a
declared EXPECTATION with a match condition and a PS3.3 citation, committed, and
it is the thing that converts a pixel divergence into an attributed
non-failure. An observation and a policy.

Both are kept and they are connected: rung 3 reads F-X007's field, so a volume
view whose geometry the reference got wrong is attributed to the reference
without a hand-written register entry. **Do not unify them by accident.**

That connection is coarse, and saying so is better than discovering it later. It
applies to a whole subject rather than to the pixels the divergence explains,
and it fires only when the pixels diverge with parameters and geometry agreeing.
Today no view reaches it, because the identity comparison has no pixel
divergence anywhere.

## The census, and the run-level rule

A comparison run is green when every view has outcome `pass` or `unmeasured`,
there are zero `absent` views, no view carries `divergent-while-unmeasured`, no
register entry marked unreachable fired, and the census of `unmeasured` views
and their qualifiers matches `tools/oracle/compare-expectations.json` exactly,
**in both directions**.

A view that JOINS the census is a coverage loss that has to be explained. A view
that LEAVES it is a coverage gain that has to be recorded in the same change.
That is `unsupported.json`'s discipline, and it is the first half of what stops
`unmeasured` becoming a place things go to be forgotten.

The census is committed and hand-maintained. It was seeded from the first
identity run, which is the only honest way to start one, and from here a change
to it is reviewed like a tolerance change. A qualifier label the comparator does
not emit is refused at load, so a stale census cannot read as an empty set.

## Today's numbers

`bin/ocelli.sh compare` over `tools/oracle/out/`, all ninety-eight views:

```text
98 views: 70 pass, 0 fail, 28 unmeasured, 0 absent
  decimated: 2
  unstated-threshold: 5
  weak: 22
compare: 20 mutations, 0 not detected
```

Twenty-eight and not twenty-nine, because `real/us_cmb_crc/00000001.dcm` is
class two and decimated and is counted once with both qualifiers.

The seventy pass numbers mean nothing on their own. **The identity run compares
the reference against itself**, so a comparator that always answered zero would
produce the same seventy passes. The mutation line is what makes the seventy
worth reading.

## Output

`tools/oracle/compare-out/`, emptied at the start of each run.

- `compare.json`, the whole report: every view's outcome, qualifiers, attributed
  side, ladder rung, parameter and geometry divergences, and all four regions of
  statistics per lane.
- `<id>.diff.raw` for every view carrying a difference, in the same RGBA8 shape
  as the frames it came from, beside the reference's own `<id>.png` pair. It is
  the per-lane ABSOLUTE difference, **unamplified**: a scale factor is a number
  nobody stated, and the statistics are the evidence. The image is for locating
  a difference and not for judging one. No PNG encoder is pulled in for it.

**Nothing under `tools/oracle/compare-out/` is ever committed.** A difference
image of a real corpus row is derived from that row exactly as a reference frame
is, and every real row carries `burned-in-unchecked` because HLD story E22.3 is
not built. `.gitignore` covers the path and that is not enough on its own,
because `git add -f` exists, so `COMPARE_OUTPUT_PREFIXES` in
`scripts/staged_content_check.py` refuses it by path with its own message.

## The mutation catalogue

Twenty entries in `tools/oracle/src/mutations.rs`, every one replayed on every
oracle gate. Each declares a target, an effect and the verdict it must produce,
and the runner fails the gate when an entry is not detected.

The catalogue reaches all four kinds of answer the comparator can give: a
structural refusal, a comparison refusal, a run-level problem, and a per-view
outcome. Six entries are worth naming.

- `plus-one-on-two-fifths-of-the-image` is the LINEAR against LINEAR_EXACT
  signature at corpus scale, and the bias bound is the only thing that fails it.
- `plus-two-at-the-fraction-budget` and `plus-two-one-pixel-over-the-budget` are
  25.1's fraction boundary from both sides, one pixel apart. The budget is
  computed in integers as `pixels - ceil(999 * pixels / 1000)`, which is 262 on
  the declared 512 by 512 canvas, and 262.144 being fractional is exactly why it
  is computed rather than written down beside a canvas size a later story could
  change.
- `candidate-image-slope-changed` and `reference-image-slope-changed` are the
  same damage on the two sides, and the pair is what proves rung 2's attribution
  is a measurement rather than a constant. Without the second one, "attributed
  to ours" would be indistinguishable from a default.
- `an-undeclared-raw-in-the-directory` is the guard that stops a partial
  comparison reporting as a complete one.
- `plus-three-on-one-pixel-of-a-reformat` damages a view that `rows[]` does not
  name, so a catalogue that only ever damaged a stack row could not have caught
  a comparator reading one list.

Per `docs/sprints/CURRENT_SPRINT.md`, the mutation that proves a guard must not
be run in the same command that adds the guard, which is why the catalogue is
standing production data and the gate re-runs all of it every time rather than
trusting a note.

## What F-011 did not build

Named, because each is somebody's story.

- Any Ocelli renderer, decoder, LUT chain or port code. Decision D7.
- A perceptual colour metric, and any CIEDE2000 implementation. Metric and
  threshold are one decision and belong together.
- The full three-way metadata diff harness. F-013, E2.5, S04.
- Re-rendering the saturated rows at a wider window, or a magnified render for
  the two decimated rows. Both change `render-params.json` and therefore every
  reference frame.
- Resolving the reference's SIGMOID divergence. F-X012, S04. F-011 built the
  register that holds it.
- Stable render-hash emission, F-015, E2.7, S04, and the CI gate that renders
  the corpus per pull request, F-012, E2.4, S04.
