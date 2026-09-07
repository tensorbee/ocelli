# The comparator, the oracle's judging half

**F-IDs that contributed:** F-011, F-012, F-013, F-015, F-X012
**Last updated:** 2026-09-06

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
   the sidecar contract and the report shape over all ninety-nine views. **It
   proves nothing whatever about detection**, which is why it is never allowed
   to be the only corpus-scale exercise.
3. **A declared mutation catalogue**, `tools/oracle/src/mutations.rs`, applied
   in memory to real reference frames with the verdict each entry must produce
   written down beside it. That is what proves detection at corpus scale. It
   follows `src/faults.mjs`'s split: the catalogue is production data in
   `src/`, the runner is separate, and every entry is replayed on every oracle
   gate rather than trusted from a note saying somebody once watched it fail.

F-012 adds a fourth operation without claiming a fourth producer. `gate`
requires both `--reference` and `--candidate`, resolves both paths, and refuses
when they name the same directory. It writes the same report as `identity` but
is the only command whose invocation is suitable for candidate evidence. A
copied reference directory is useful as controlled contract evidence. It is
not Ocelli evidence and is not made binding by this story.

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
bin/ocelli.sh compare gate --reference REF --candidate CANDIDATE --out OUT
bin/ocelli.sh compare identity --candidate DIR
bin/ocelli.sh gate oracle                   # renders, then compares
./target/release/ocelli-compare census      # a measurement, not a gate
```

`bin/ocelli.sh gate oracle` is `"$0" oracle && "$0" compare`, chained with `&&`
for the reason that file already comments on: a case arm returns the status of
its LAST command, so an unchained render could fail and be reported green by a
passing comparison. **No new gate name was added.** The comparator is part of
what `oracle` means.

The binary is built in release. A debug comparison over ninety-nine frames
plus twenty-nine mutation replays is minutes rather than seconds, and a check
nobody wants to wait for is a check that stops being run.

The corpus-scale exercises are subcommands of a binary and **not** `#[ignore]`
tests. An ignored test that needs `tools/oracle/out/` reads as a pass on the day
it did not run, and this repository refuses a skip that reports as a success.

**`census` is deliberately not in any gate.** It gates nothing, it exits 0
whatever it finds, and `bin/ocelli.sh compare` does not call it. It exists
because this document, `tolerance.rs`, `attribution.rs` and HLD 25.1 all carry
counts of which corpus views the bias bound can and cannot reach, and every one
of those counts was wrong at least once. It applies the catalogue's own swap to
every gating class-one view and prints the signed mean over both the informative
region and the image rectangle, so a number in prose has a command beside it
rather than a provenance. Rerun it after any change to the corpus, to
`render-params.json` or to the swap, and update the numbers those four files
carry in the same change.

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
  `volumes[].frames[]`. A comparator reading `rows[]` alone would compare 90 of
  99 views and report success. The orphan refusal is what turns that into a
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

Measured on the current corpus: 85 class one and 5 class two among the stack
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

F-013 adds committed truth for named synthetic cases around this side-to-side
comparison. Real rows retain the independent-reader comparison without
committing or printing their values.

### Component 3, committed metadata truth

`tools/oracle/metadata-truth.json` is the one hand-authored metadata and
display-value source used by both Rust and the pydicom checker. The existing
`volume-truth.json` is the sole owner of series geometry. It contains only
named synthetic and syntax rows. Each resolved field cites PS3.3 and binds its
scope to the exact `metadataSources` JSON pointer derived from the raw DICOM.
It covers pixel and photometric fields, modality and VOI parameters,
presentation inversion, IPP, IOP, Pixel Spacing, slice thickness and resolved
reference modules. It also carries literal PS3.3 C.11 display values, oblique
geometry samples. Volume dimensions, origin, direction, slice thickness and
projected gaps come only from `volume-truth.json`.

Declared numbers compare by `f64::to_bits` in Rust and packed IEEE-754 bytes in
Python. Arrays retain their order. A missing JSON member differs from explicit
null, null differs from an empty array, positive and negative zero differ, and
non-finite JSON numbers are refused. Derived geometry alone uses HLD
25.1's existing 1e-6 mm bound. The harness does not evaluate the LUT chain.
The four display values are literal fixture evidence, so LUT arithmetic still
has one implementation when `ocelli-pixel` lands.

Both sidecars are checked before either frame is opened. A metadata failure is
therefore reportable even when frame bytes are absent or malformed. If exactly
one side differs from truth, that side is named. Opposite one-sided findings,
mixed findings and any shared problem are unattributed. If both agree with each
other on a wrong value, the view still fails at the `metadata-truth` rung.

Real-row reports derive sensitivity from the sidecar path or volume series
directory, never from a mutable identifier. Parameter values are replaced by
`<withheld, real corpus row>` before JSON serialization.

### Component 4, geometry

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

**A volume's in-plane spacing is compared against the reference's own,
crosswise.** PS3.3 C.7.6.2.1.1 gives PixelSpacing (0028,0030) as [between rows,
between columns], and cornerstone3D 5.8.2 builds its volume spacing as
`[PixelSpacing[1], PixelSpacing[0], zSpacing]`, so the two arrays agree
crosswise and the reference's own locals for that pair are named `rowSpacing`
and `columnSpacing` in the reversed sense. **Until the S03 sprint review the
harness compared no in-plane spacing against the reference at all.** A
transposition is invisible on a square-pixel series and renders a plausible,
stably hashing frame on any other, which is what the corpus's non-square
`[0.5, 0.25]` row exists to catch. `compareGeometry` now compares both
components, above the uniformity early return so a real series is answered too,
and says so explicitly when the two agree in order rather than crosswise, which
is the transposition itself. Comparing them in order turns seven tests in
`tools/oracle/tests/volume_test.mjs` red.

**A reformat's scale is published unrounded.**
`reformat.millimetresPerCanvasPixel` is the only number the reformat rung
compares, and `reformat_scale_divergences` amplifies a difference in it by half
the canvas height before testing it against a quarter of a canvas pixel. It was
written through `toFixed(6)`, which discarded up to 5e-7 mm before the
comparator could see it. Determinism is measured on the frame digest and not on
this field, so the rounding was protecting nothing.

### Component 5, pixels

Alpha is asserted to be 255 on every pixel of both sides and is never included
in a difference, because a difference in alpha is a difference in the canvas and
not in the image. A class-one view is asserted to be monochrome on both sides,
red equal to green equal to blue, since a `mono16` token over a frame that is
not monochrome means the token is lying. The comparison is then over one lane
for class one and three for class two.

Per view: the sample count, `maxAbsDiff`, the counts at difference 0, 1 and 2
and over 2, `fractionWithinOneLsb`, `signedMeanDiff` as candidate minus
reference, `differingFraction`, the 99.9th percentile of the absolute
difference, the frame and image-rectangle dimensions, `rowsTouched` and
`columnsTouched`, and all of it four times over: the full frame, the image
rectangle, the letterbox, and the informative subset.
Every one of those falls out of one 256-bucket absolute histogram per lane,
computed in a single pass, so no two regions can be taken over different
readings of the same buffers. The JSON report also carries the same samples as
a sparse `signedHistogram` of `[difference, count]` pairs. The ledger derives
every published channel statistic from that one exact distribution instead of
trying to prove separately summarized values can share an unseen histogram.
Full histograms must be the exact sum of image and background. Every nonzero
image bin must also appear unchanged in the informative histogram, because a
nonzero lane difference cannot be clipped to the same extreme on both sides.
Only zero-difference samples may be omitted from the informative region.
`frameRows * frameColumns` must equal the full channel pixel count, and
`imageRows * imageColumns` must equal the image count at the reported
`imageX`, `imageY` origin within those bounds. The report preserves the exact
sorted row and column index sets touched in the image and background. Their
unions must reproduce `rowsTouched` and `columnsTouched`. Each region's lane
counts must fit jointly in the Cartesian cells named by its two sets, with the
image rectangle removed for the background. This prevents the separate row
and column marginals from describing a spatial shape the producer cannot
emit. A volume reformat must use the full frame as its image rectangle. When
`monochromeFrame` is true, every reported RGB channel distribution must be
identical because each source pixel has equal red, green and blue lanes. The
image difference union cannot exceed `informativePixels`, since every nonzero
image difference is informative. For a monochrome RGB frame the union is also
bounded by one lane's difference count because all three lanes have the same
pixel support. The false direction is checked where the distributions determine
it exactly. If every sample in every RGB lane has the same `+255` or `-255`
difference within each nonempty image or background partition, both byte
values are forced and both frames are necessarily monochrome. The two
partitions may force opposite signs, but `monochromeFrame` still cannot be
false.

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

**The rectangle is not the region the bias bullet averages over.** It BOUNDS
that region. 25.1 names the informative region, which is the subset of this
rectangle that is not clipped to the same display extreme on both sides, and
the rectangle's own job is to draw the line between the picture and the
letterbox and to be the denominator of `informativeFraction`.

For a stack view: `columns` and `rows` times the published canvas scale, centred
in the canvas. A canvas pixel is inside when its CENTRE is, so pixel `i` is
inside when `i + 0.5 >= x0` and `i + 0.5 < x0 + width`, and the result is
clamped to the canvas. The alternatives are wrong in a way that is invisible.
Flooring the offset admits a letterbox column and ceiling the far edge admits
one on the other side, and at 512 rows either is 512 background pixels counted
as picture.

**What that costs is not the bias bound, and this section said it was until the
sprint review's fifth pass.** Both sides paint the same clear colour in the
letterbox, so an admitted column agrees on both sides, and that colour is an
8-bit extreme, so the column is clipped to the same extreme and is
uninformative. It never enters the bias denominator at all.

**Both halves of that are load-bearing, and the sixth pass added the second.**
`clipped_to_the_same_extreme` in `tools/oracle/src/frame.rs` is
`reference == candidate && (reference == 0 || reference == u8::MAX)`, so
agreement alone does not make a pixel uninformative. The clear colour is
`base.background` in `tools/oracle/render-params.json`, today `[0, 0, 0]`. At
`[16, 16, 16]` every letterbox pixel becomes informative and this paragraph
inverts. That file's digest is compared between the two sides and never against
an expected value, so a change both halves agreed on would be silent. The
dependency is therefore asserted, by
`the_declared_background_is_an_eight_bit_extreme` in
`tools/oracle/tests/geometry_fixture.rs`, which checks the rule rather than
today's value: any 8-bit extreme passes and `[16, 16, 16]` does not.

Two other things break instead:

1. `informativeFraction` is informative pixels over image-rectangle pixels, so
   an admitted column inflates the denominator alone. **The move is
   `f * n / (P + n)` and not `n / P`, and this compared the second against the
   margin until the sixth pass.** The informative count does not change, so the
   fraction goes from `I / P` to `I / (P + n)`. On the worst view the identity
   run does not call weak, `synthetic/ct_series_nonuniform`, `I` is 21973 over a
   rectangle of `P = 218112` pixels, which is 0.10074 against a floor of 0.10,
   so the margin is 0.00074. One column of `n = 512` moves it to 0.100506, a
   move of 0.000236, and four columns cross the floor at 0.099805. 262144 is the
   canvas rather than that view's rectangle, which is where the old 0.00195
   came from. A wrong column can therefore turn a measured view into a `weak`
   one.
2. The `letterbox-only` qualifier fires only when the image region carries no
   difference at all and the letterbox carries one. A fit error in a column that
   should have been letterbox then lands inside the image region, the qualifier
   cannot fire, and a difference in the fit is reported as a difference in the
   picture and attributed to us.

`docs/lld/oracle.md`'s worked case is the fixture:
`syntax/reference_mono12.dcm` at 64 by 96 with a published scale of 8 vertical
and 4 horizontal gives a 384 by 512 rectangle at offset 64, so the letterbox is
512 rows by 128 columns, which is 65536 pixels, which is 0.25 of the frame
exactly. The reference measured 65536 black pixels on that frame and recorded
`blackFraction: 0.25`.

The fixture computes the rectangle and then READS those two numbers, from
`tools/oracle/out/syntax__reference_mono12.json`, where a complete run has left
one. It asserted only its own arithmetic until the eighth pass while this
paragraph and its own comment both said it was checked against the instrument.
`tools/oracle/out/` is gitignored, so the read is conditional, and the
condition is `run.json` present rather than the sidecar present, because
`run.mjs` writes `run.json` after every row sidecar and discards the whole
directory on any later problem, so it holds a complete run or holds nothing.
The guarantee is the discard rather than the write order, and a `console.log`
does follow the write. Under a complete run a missing worked case is a failure
and not a skip.

**For a volume reformat the image rectangle is the whole frame**, and that
narrowing is stated rather than left to be noticed. A reformat plane is a cut
through a volume and has no source pixel grid to be a magnification of, which is
why `docs/lld/oracle.md` deliberately does not publish
`canvasPixelsPerSourcePixel` for one. Deriving a rectangle from the volume
bounding box would be a second copy of a derivation the reference did not
publish, which is the thing decision 13 exists to avoid.

The consequence is that a reformat's black surround counts as image. **That does
not loosen the bias bound, which is what this said until the fifth pass.** The
surround is black on both sides, so it is uninformative and stays out of the
bias denominator exactly as a stack's letterbox does. What it does is put every
one of those pixels into `informativeFraction`'s denominator, so a reformat is
readier than a stack to fall under the informative floor and be reported `weak`.
On the identity run all six synthetic reformats are `weak`, at informative
fractions from 0.0 on the two AXIAL planes to 0.011 on the two CORONAL ones,
while the three real MR reformats run from 0.418 to 0.75 and pass. Reproduce
from `informativeFraction` in the `compare.json` that
`./target/release/ocelli-compare identity` writes. The second consequence is
that a reformat has no letterbox
region at all, so `letterbox-only` can never fire for one and a fit difference
there is always attributed to the picture. Worth a story if a reformat ever
gates on bias in anger.

## The verdict vocabulary

`pass`, `fail`, `unmeasured` and `absent`, plus a qualifier set. 25.1 gives a
predicate and does not say what a comparator reports, so this vocabulary is
F-011's construction.

**`unmeasured` is a third outcome and not a synonym for `pass`.** A run that
reports "71 pass, 0 fail, 28 unmeasured" over 99 views is saying something a run
that reported "99 compared" would not. The number is uncomfortable on purpose:
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

**"On its own" is now true, and it was not.** Four tracked places said so and
nothing pushed a run problem for it, so a view carrying the qualifier failed the
run only through the census, and `Qualifier::parse` accepts the label. Adding
`"divergent-while-unmeasured"` to a `compare-expectations.json` entry, which a
reviewer reads as documenting a known-unmeasured view, therefore made the run
green with a measured divergence absorbed. `RunReport::green` reads the
qualifier off the records itself, so no census entry and no forgetful caller can
switch it off. `report::tests::a_census_entry_naming_the_qualifier_does_not_buy_a_green_run`
builds exactly that bypass and asserts the run is red.

## The attribution ladder, in order

1. **Inputs disagree**, by digest. Attributed to the inputs, not to either
   renderer.
2. **Committed metadata truth disagrees.** The side that differs from truth is
   named. If both sides agree with each other and differ from truth, the run
   fails as an unattributed instrument or shared-reference problem.
3. **Parameters disagree.** Attributed to the side that disagrees with the
   INDEPENDENT reading of the bytes. That third reading exists already: the
   sidecar's `attributes` block is read straight from the file by
   `dicom-parser` in the page, independently of the render path, and
   `check_sidecars.py` cross-reads it under pydicom. So if one side's
   `/image/slope` agrees with its own `/attributes/rescaleSlope` and the other
   side's does not, the second side is at fault, and the record says which and
   why. Where a field has no independent counterpart, or where neither side
   agrees with its own, the divergence is reported unattributed rather than
   assigned by default.
4. **A register entry matches, or F-X007's own `referenceDivergence` is set** on
   the subject and the pixels diverge with parameters agreeing. Attributed to
   the reference, with a PS3.3 citation where the register supplies one. The
   `referenceDivergence` half additionally requires the geometry to have
   diverged, for the reason stated under the list.
5. **Geometry is outside 25.1's bound, or the difference is confined to the
   letterbox.** Attributed to the fit rather than to the LUT chain.
6. **Pixels diverge with parameters and geometry agreeing.** **Attributed to
   ours**, by default. HLD section 11 makes cornerstone3D the reference and
   deviation D-11 makes the pin the definition of correct until an entry in the
   register says otherwise. That default direction is the conservative one, and
   it is what makes the instrument useful under D7: the burden is on us to show
   the reference is wrong, not on the reference to show it is right.

**`referenceDivergence` explains a geometry difference and does not explain a
pixel difference on its own.** Every divergence a subject may declare names a
spacing component, and a wrong spacing shows up as a geometry difference.
Letting one absorb a pixel difference on a view whose geometry agrees would be
the comparator excusing our own defect with somebody else's, so rung 4 carries
`!geometry.is_empty()`.

The S03 sprint review found that narrowing watched by an accident.
`plus-three-on-one-pixel-of-a-reformat` lands on the one subject carrying a
declared divergence only because `real` sorts before `synthetic` in the
`BTreeSet` the records are iterated from, so a synthetic subject sorting first
would have retired the guard in silence. Three unit tests in `attribution.rs`
now build both sides themselves and hold the three cases apart: a declared
divergence with geometry agreeing is `fail` attributed to ours, the same
divergence with geometry diverging is `unmeasured` attributed to the reference,
and a geometry difference with nothing declared is `fail` attributed to the fit.
Removing the narrowing turns the first of the three red and leaves the other two
green, which is what says the pair is about the divergence rather than about the
geometry.

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
structurally cannot show one. Measured over all **70** gating class-one views on
this corpus, the largest bias over the image rectangle is **-0.0853**, on
`real/mr_eay131/00000008.dcm`, and a 0.1 bound catches **none of them**. The
sign is the census's own and is kept rather than dropped: the swap makes the
candidate darker, so every bias it produces is negative, and 25.1's bound is
two-sided. Over the informative region the same swap gives -0.269 to -0.284 on
the real soft-tissue CT rows and about -0.32 on the synthetic ones, and
**51 of the 70** exceed
the bound.

Reproduce with `./target/release/ocelli-compare census`, which applies the
catalogue's own swap to every gating view and prints the signed mean over both
regions. It gates nothing and it exists because every count in this section was
wrong at least once.

It said 71 views and 0.0825 until the sprint review's fourth pass, and both
errors are worth naming. **71** was `93 - 22 weak` and forgot that
`real/dx_varepop/00000001.dcm` is `mono16` and `unmeasured` for `decimated`, so
it gates nothing either. **0.0825** is a real number on this corpus and it is
not this one: it is what two of the `real/ct_cmb_mml` rows produce over the same
region.

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

**That fixture's own numbers are now assertable.** It carried HLD 18.3's four
worked rows as three-decimal literals and asserted only that they round to equal
display codes, so a mistyped value survived anywhere inside a half-code band.
Each display value is now written as the exact rational its formula produces,
`102000/798` at the window centre and `102000/800` beside it, and compared
against HLD 18.2's two formulas transcribed into exact integer arithmetic. The
comparison is by cross-multiplication and carries no tolerance at all, so a
numerator wrong by one fails it, and the thirty-two codes in the fixture's own
pixel tables are checked the same way. HLD 18.3 prints the centre row as
`127.819`, which is the exact value truncated rather than rounded, and its
`-60` row as `63.910`, which is rounded, so the rationals are the values and the
printed decimals are a rendering of them.

**What the bound is proven to do.** It detects the actual swap, and the
mutation that proves it is `the-actual-linear-exact-swap`. LINEAR_EXACT sits
`u / w` below LINEAR before the renderer quantises, so a pixel drops one display
code with probability `u / w` and the rest do not, and at `u > w` it drops more
than one. The mutation accumulates `u` across the image rectangle using the
view's own declared window, takes `accumulator / w` drops at each pixel and
keeps `accumulator % w`, which places exactly `floor(sum(u) / w)` drops in
proportion to `u`, in integers, with no rounding decision in it. That is the
divergence rather than a stand-in for it.

**It took at most one drop per pixel until the sprint review's fifth pass, and
the doc comment called the residue bound "by construction".** One subtraction
per pixel leaves the accumulator at or above `w` whenever `u >= 2w`, and `u`
runs to 254, so the bound held only for `w > 254`. Frame `[200, 200, 200]` at
`w = 100` came back `[199, 199, 199]`: three drops where `floor(600 / 100)` is
six, and a residue of 300. A drop of two codes at `w = 100` and `u = 200` is the
divergence itself, since `u / w` is 2 there, so the loop was corrected rather
than the domain narrowed to windows the corpus happens to have. The corpus's
narrowest window today is 256, on the class-two row
`real/us_cmb_crc/00000001.dcm`, which is why nothing measured it. A CT brain
window of 80, or any MR window under 255, reaches it. The delivered drop count
is now compared against `floor(sum(u) / w)` and the mutation refuses when they
disagree, where before the only thing asked of it was that it moved something.

**That comparison is a tripwire and not a measurement**, and saying so is the
point. The residue telescopes, so the loop applies `floor(sum(u) / w)` drops by
construction and the two sides cannot part company while the loop is the loop.
It fires only if a later edit breaks that, which is exactly what the single
`if` the fifth pass found had done.

**Both display extremes are excluded, and PS3.3 is why.** C.11.2.1.2 gives
LINEAR the clamps `c' - w'/2` and `c' + w'/2` on `c' = c - 0.5` and
`w' = w - 1`, and C.11.2.1.3.2 gives LINEAR_EXACT `c - w/2` and `c + w/2`.
Write `y_L` and `y_E` for the two display values at one stored value. Where
neither function clamps, the whole divergence is
`y_L - y_E = 255 * (x - c + w/2) / (w * w')`, which is `y_L / w`, because
`y_L = 255 * (x - c + w/2) / w'`.

**The two exclusions are not equally tight, and four review passes in a row got
the upper one wrong.** At 0 it is exact at every width. The lower clamps are
equal, since `(c - 0.5) - (w - 1)/2 = c - w/2`, and above them
`y_E = y_L * (w - 1) / w` lies in `[0, y_L]`, so `round(y_L) = 0` forces
`round(y_E) = 0`. The bracket is closed at the top, because `y_E = y_L` at
`y_L = 0` and the half-open form this used to be written in is the empty
interval at exactly the value the sentence is about. Coincident clamps alone
would not carry that either, which is what this section used to claim.

**At 255 the exclusion is conservative at every width and exact at none.** A
pixel the reference rendered 255 has `y_L >= 254.5`, and it moves when
`y_L - y_L / w < 254.5`, which is `y_L < 254.5 * w / (w - 1)`. A display value
cannot exceed 255, so the movable set in display-value space is `y_L` in
`[254.5, 254.5 * w / (w - 1))` intersected with `[0, 255]`, which is the
**closed** `[254.5, 255]` for `w < 510`, `[254.5, 255)` at `w = 510`, and
strictly inside `[254.5, 255)` above it. 254.5 is strictly below
`254.5 * w / (w - 1)` at every finite `w >= 2`, so the band never closes.

**The top is open in the formula and that is not a `min` with 255.** Below 510
the formula's top lies above 255, so no `y_L` reaches it and every display
value up to and including 255 moves. `y_L` is exactly 255 at
`x = c + w/2 - 1`, the last stored value LINEAR does not clamp, and at
`w = 100`, 256 and 400 that stored value is the only mover the fixture's table
carries, `w = 400, x = 239` being the row the table hand-works. This paragraph
wrote the top as `min(255, 254.5 * w / (w - 1))` with an open bracket, which
excluded it, and the clamped interval `(c + w/2 - 1, c + w/2]` named in the
next paragraph is open at its left, so that stored value fell into neither
stated region while the fixture's own table moved it. `254.5 / (w - 1)` capped
at `0.5` is the band's **measure**, and the cap is the only thing the `min`
ever meant.

**What `w >= 510` buys is only that a clamped pixel cannot move.**
`254.5 * w / (w - 1) >= 255` exactly when `w <= 510`, which is the only place
510 comes from. On `(c + w/2 - 1, c + w/2]`, where LINEAR clamps to 255 and
LINEAR_EXACT does not, the lowest `y_E` is `255 - 255/w`, which rounds back to
255 exactly when `w >= 510`. That says nothing about the pixels LINEAR rounded
up to 255 from below, so 510 is not a threshold separating two regimes and must
not be written as one. Measured over integer stored values at centre 40,
counting `x` where LINEAR rounds to 255 and LINEAR_EXACT does not. The fixture
tabulates twelve widths and these are seven of them, named because they bracket
the two numbers the old derivation treated as boundaries: `w = 400` gives 1,
`w = 510` gives 0, and 512, 600, 1000, 2048 and 4096 each give 1. The other
five rows are 100, 255, 256, 509 and 511, and 255 is the second and last width
in the table with no mover.

**The two zeroes, at 255 and at 510, are where a half rounds up, and not where
the integers happen to fall.** This paragraph said the second of those until
the eighth pass. The movable interval below is `509/510` of an input unit
wide, so it holds no integer exactly when its open endpoint `u_end = 254w/510`
IS one, which is exactly when 255 divides `w`. At both widths the sole
candidate stored value sits ON `u_end`, at `w = 255, x = 167` and at
`w = 510, x = 294`, where `y_L` is exactly 255 and `y_E` is exactly `509/2`.
The fixture's declared rounding rule, half away from zero, takes `y_E` up to
255, so neither pixel moves. Truncate instead and both rows carry a mover and
seven of the fixture's fourteen tests go red.
`the_two_empty_rows_are_empty_because_a_half_rounds_up` asserts every step of
that, including that those two are the ONLY display values in the section's
43212 that land on 254.5. So the emptiness is a property of the width and the
rounding rule together, and 510 still sorts nothing: 255 is below it and 510
is not.

**In stored-value units the movable set is one contiguous interval at every
width, with no case split at all.** Write `u = x - c`. `y_L` reaches 254.5 at
`u = (254w - 509)/510` and `y_E` reaches it at `u = 254w/510`, both solved from
the two formulas, and a stored value moves exactly on
`u` in `[ (254w - 509)/510, 254w/510 )`, which is `509/510` of one input unit
wide at every width and holds at most one integer and sometimes none. That one
interval covers the pixels LINEAR rounded up to 255 and the pixels it clamped
to 255 together, so 510 sorts nothing here either: it moves the clamp point
`u = w/2 - 1` across the interval and changes neither endpoint. Reproduce with
`cargo test -p ocelli-oracle --test voi_divergence_fixture`, which pins the
band, the counts and the clamped-pixel case, and which now evaluates both
transcribed formulas AT both endpoints rather than asserting a literal derived
from neither. Until the seventh pass this section's `509/510` was checked
entirely in closed form: mutating the C.11.2.1.2 transcription left the band
test green. Measured again after the eighth pass, on a file of fourteen tests, by
rewriting each formula's own `Exact::whole` binding, which is the symmetric
mutation and the one these counts belong to. Writing `w` where C.11.2.1.2 says
`w - 1` takes eight of the fourteen red, the band test among them. Writing
`w - 1` where C.11.2.1.3.2 says `w` takes nine, the band test failing on its
`254w/510` endpoint. **Name the site, because the other readings give other
numbers**: rewriting the `2 * w` denominator alone takes ten, and rewriting
both that and the binding takes seven. The ninth pass measured all three.

An 8-bit frame does not carry the stored value behind a 255, so there is no way
to tell a pixel inside the band from one outside it, and excluding the whole
population is the conservative reading at every width. It under-damages the
frame by whatever share of the 255s fell in the band and never over-damages it,
so no pixel is wrongly moved. It also keeps the mutation from perturbing the
informative region, which is what kept the numerator and the denominator
honest.

**Every direction above assumes `MONOCHROME2`, and under `MONOCHROME1` all of
them reverse.** PS3.3 C.7.6.3.1.2 defines `MONOCHROME1` so that the minimum
value is displayed as white, which is the `MONOCHROME2` ramp inverted, so the
byte in the rendered frame is `255 - y`. The accumulated `u` would have to be
`255 - byte` rather than `byte`, the drop would have to be an add, and the two
exclusions would swap ends, the exact one moving to the byte 255 and the
conservative one to the byte 0. Applied unchanged to an inverted frame the swap
still produces a one-sided difference the bound detects, so nothing goes red,
which is the same failure shape as `round(u - u/w)` below: detected, and not
the thing it claims to be.

So the swap takes a target of its own, `Target::MeasuredMonochrome2Stack`,
narrowed to a `MONOCHROME2` view by a positive predicate rather than by "not
`MONOCHROME1`", so a stack whose ramp direction is undeclared is not a target
either. Before that narrowing the only thing keeping the swap off
`synthetic__cr_monochrome1`, a `mono16` stack view that passes, was `real`
sorting before `synthetic` in the `BTreeSet` the runner iterates, which is the
accident smell S4 named on `MeasuredReformat` and here it was guarding
arithmetic rather than a ladder rung.
`the_monochrome2_target_skips_an_inverted_view` in `mutations.rs` puts the
inverted record first, which is the order the accident does not survive.

**A separate target and not a narrowing of the shared one, which is what the
eighth pass corrected.** The predicate sat on `MeasuredStack` itself for one
pass, where eighteen of the twenty-one catalogue entries resolve, so seventeen
entries that have nothing to do with ramp direction were narrowed by it. The
coupling showed the way couplings do: under an unrelated mutation of the
sidecar pointer that feeds it, the first entry to refuse was
`plus-one-on-a-twentieth-of-a-percent`, an `AddDelta`. `ColourClassTwo` is the
precedent and it was created as a separate target for exactly this reason.
`the_shared_stack_target_is_not_narrowed_by_ramp_direction` holds the shared
target open and
`one_entry_takes_the_monochrome2_target_and_the_rest_share_the_stack` holds the
split at one entry against seventeen.

**And the interpretation itself is now read by a unit test.** Nothing in
`cargo test -p ocelli-oracle` touched `/attributes/photometricInterpretation`
until the eighth pass: mutating that pointer string in `compare_view` left the
whole suite green and was caught only by `ocelli-compare mutations`, which
needs the rendered corpus, so it was `gate oracle` and never the floor. That is
the same shape as the bias region's DEFECT 2 and as `apply_to_frame` before the
fourth pass. `the_record_carries_the_declared_photometric_interpretation` and
`the_interpretation_is_the_references_and_a_disagreement_is_a_divergence` in
`attribution.rs` build a stack sidecar on each side and pin the value, the
absent case, and which side it is read from.

It said `round(u - u/w)` here and in the variant's own doc comment until the
sprint review's fourth pass, and `apply_to_frame`'s comment 600 lines below it
existed to repudiate exactly that. Three passes in a row accepted a mutation
because something went red rather than because the mutation was the thing it
claimed to be, and the third of those was the missing `grey == 255` exclusion.

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

**And it is proved without the corpus too, which it was not until the fifth
pass.** That proof above needs `tools/oracle/out/`, so on a machine with no
rendered run the substitution left `cargo test -p ocelli-oracle` fully green:
every bias fixture builds frames in which the two regions are the same pixels,
and `tolerance_fixture.rs` says so honestly at `verdict` without anything
elsewhere covering the gap.
`the_bias_bound_is_fed_the_informative_region_and_not_the_rectangle` in
`tools/oracle/src/attribution.rs` is the frame those fixtures cannot be: 216 of
its 256 pixels are white on both sides, 25 of the remaining 40 differ by one
code, so the bias is 25 / 40 = 0.625 over the informative region and
25 / 256 = 0.09765625 over the rectangle. The two regions disagree about the
verdict, and the substitution turns the test's `fail` into a `pass`.

**And the argument for it is now measured over the corpus rather than over one
view.** `ocelli-compare census` applies the swap to all 70 gating class-one
views and averages over the image rectangle instead: **0 of 70** exceed the
bound, the largest being -0.0853, while over the informative region 51 of 70 do.
The mutation's own target sits at -0.0773 over the rectangle, a 23 per cent
margin below the bound.

**The census is not narrowed to `MONOCHROME2` and one of its 70 rows is
inverted.** Its argument is about the whole gating population, so it applies
the same darkening accumulator to `synthetic__cr_monochrome1` and reports
`w=4096 bias=-0.0311`. That number has the **wrong sign** for an inverted ramp,
and its magnitude is near-right only because `mean(u)` and `255 - mean(u)` are
close on that frame. It moves no verdict, since the bound is two-sided and
0.0311 is far under 0.1, and the row is one of the 19 blind ones for a reason
that does not depend on the sign at all: `255 / 4096` is 0.062, so no
divergence on it can reach the bound in either direction. Reproduce with
`./target/release/ocelli-compare census`.

Until the fourth pass that argument rested on a modelling error and a margin of
half a per cent. The accumulator dropped white pixels, so the same measurement
gave **44 of 70** over the rectangle with a maximum of 0.4836, and the claim
held for the single view the mutation lands on and for no other, at -0.0995
against a bound of 0.1.

**A structural limit, stated because no region choice removes it.** The largest
divergence any view can show is `255 / w`, so **a view whose window is wider
than 2550 cannot reach this bound at all**. The corpus carries two rows at
`w = 4096`, `real/dx_varepop/00000001.dcm` and `synthetic/cr_monochrome1.dcm`,
and only the second of them gates at all: the first is `unmeasured` for
`decimated`. That view is not protected by this bullet and nothing here pretends
it is.

**The practical limit bites long before 2550**, and it is about content and not
only about width, because the mean over a region is `mean(u) / w`. Of the 70
gating views, 51 can fail this bound under the real divergence and **19 cannot**:
the fifteen `real/mr_eay131` stack rows and two of that subject's three
reformats, at windows from 678 to 881 and informative means from 0.058 to 0.087,
plus `synthetic/mr_nonsquare_spacing` at 2048 and `synthetic/cr_monochrome1` at
4096. **The smallest blind window is 678.** That subject's CORONAL reformat is
at -0.114 and CAN fail, so the blind pair is two of three reformats rather than
all of them.

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

Twenty-two of the ninety-nine, sixteen stack and six volume reformats. The
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
seventy-six is 0.10074. Both AXIAL synthetic reformats are at exactly zero,
every pixel being black or white. That separation is an observation recorded
after the fact and not the reason for the number.

**And it is now checked rather than relied on.** Until the sprint review's fifth
pass, `build_statistics` handled a view with no informative pixel at all by
inventing `signedMeanDiff: 0` and `biasPasses: true`, on the stated grounds that
such a view "is already `weak` and already `unmeasured`". Structurally it is
not.
`weak` needs the reference half to have listed the view under `lowInformation`
at `extremeFractionWarnAbove: 0.95`, and the floor here is 0.10, and nothing
asserts the two agree. The margin is 0.00074, the distance between 0.10074 and
0.10. A view the reference did not list would have taken an invented pass, which
is the reference half's configuration deciding a comparator verdict by the back
door. That case is now a refusal, `NoBiasDenominator`, whose message names both
numbers and both files. The listed case is unchanged: the bias is not evaluated,
and the view is `weak` and `unmeasured` as before. Two unit tests in
`tools/oracle/src/attribution.rs` hold the pair apart, and neither can be
satisfied by the corpus changing shape because both build their own frames.

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
the F-ID that raised it. A reachable entry also carries the valid F-ID that
resolved the evidence gap, and an `effect` naming the pixel relationship the
comparator can prove. F-IDs use the repository's canonical form: `F-` or
`F-X`, three digits, and an optional lowercase suffix. The current
`monochrome-inversion` effect requires geometry agreement, every
image-rectangle code to be the pointwise complement of the other side, and the
image to contain more than one code. Only then is the outcome `unmeasured` with
the qualifier `reference-divergence` and the record names the entry, rather
than `fail`. Matching sidecar conditions alone never change an identical view,
an arbitrary pixel defect, an unrelated parameter mismatch or an unrelated
geometry mismatch from its normal verdict.

The match language is deliberately small: an RFC 6901 pointer mapped to exactly
one of `equals`, `lessThan` or `greaterThan`, and every condition must hold. **A
test or effect the comparator cannot evaluate is refused at load rather than
read as unsatisfied**, because a declaration nobody evaluates is an entry that
never fires and nobody notices.

**The strictness is asymmetric, and that is deliberate.** An entry marked
`reachable: false` that FIRES is a run failure, because the claim was wrong.
The opposite direction, an entry no row exercises failing the run, is
`unsupported.json`'s other half and is not applied here.

**A new entry is a reviewed change with a rationale, exactly like a tolerance
change.** The register is the one mechanism in this design that can turn a
failure into a non-failure, so it gets the same handling as the other one.

Its only entry is the SIGMOID width case F-010 recorded. F-X012 preserves that
`raisedBy`, adds `resolvedBy: F-X012`, and makes it reachable with
`synthetic/ct_sigmoid_width_half.dcm`. The pinned helper returns lower 39.75 and
upper 39.25, but the observed presented pixels remain standard-correct and
monotonic. The register therefore acts only if a future candidate comparison
contains the declared pointwise monochrome inversion while geometry agrees. An
unrelated candidate pixel, rescale parameter or camera geometry change stays
attributed by its ordinary rung. This preserves D-11's attribution rule
without manufacturing a pixel divergence. D14's Ocelli bound remains against
the standard-correct presented output. The helper's separately measured
inverted range is not widened into that bound.

### Two mechanisms for "the reference is wrong", and why both

F-X007 gives each `volumes[]` entry a `referenceDivergence` field and a
committed `volume-truth.json`. That is a per-run MEASUREMENT of one specific
thing, the reference's volume geometry against a truth the corpus generator
knows by construction, written into the run record. The register here is a
declared EXPECTATION with a match condition and a PS3.3 citation, committed, and
it is the thing that converts a pixel divergence into an attributed
non-failure. An observation and a policy.

Both are kept and they are connected: rung 4 reads F-X007's field, so a volume
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

Structural and input refusals are `problems`. Census changes are
`coverageProblems`. Outcome counts and `divergent-while-unmeasured` are read
from the records inside `gate_verdict()` itself, so one category cannot be
reported as another. That split is not tidiness. The qualifier spent an earlier
sprint being described as failing a run on its own while nothing pushed a
problem for it.

A view that JOINS the census is a coverage loss that has to be explained. A view
that LEAVES it is a coverage gain that has to be recorded in the same change.
That is `unsupported.json`'s discipline, and it is the first half of what stops
`unmeasured` becoming a place things go to be forgotten.

The census is committed and hand-maintained. It was seeded from the first
identity run, which is the only honest way to start one, and from here a change
to it is reviewed like a tolerance change. A qualifier label the comparator does
not emit is refused at load, so a stale census cannot read as an empty set.

F-012 gives that rule a machine-readable run verdict. `claimedVerdictViews` is
exactly `pass + fail`. The `coverage` object names `unmeasured`, `absent`,
`unsupportedSourceRows` and `declaredVolumeRefusals` separately. A comparison
failure reports `comparison-failure`. A missing or newly unmeasured view
reports `coverage-loss`. An input or structural problem reports `refusal`.
Zero judged views is coverage loss, never a green empty claim.

## Today's numbers

`bin/ocelli.sh compare` over `tools/oracle/out/`, all ninety-nine views:

```text
99 views: 71 pass, 0 fail, 28 unmeasured, 0 absent
  decimated: 2
  unstated-threshold: 5
  weak: 22
compare: 29 mutations, 0 not detected
```

Twenty-eight and not twenty-nine, because `real/us_cmb_crc/00000001.dcm` is
class two and decimated and is counted once with both qualifiers.

The seventy-one pass numbers mean nothing on their own. **The identity run compares
the reference against itself**, so a comparator that always answered zero would
produce the same seventy-one passes. The mutation line is what makes the seventy-one
worth reading.

## Output

`tools/oracle/compare-out/`, emptied at the start of each run.

- `compare.json`, the whole report: every view's outcome, qualifiers, attributed
  side, ladder rung, parameter and geometry divergences, and all four regions of
  statistics per lane. It also carries the versioned reference and candidate
  render hash for every view, plus one aggregate render hash per side.
  `operation`, `gateVerdict`, `claimedVerdictViews` and the named `coverage`
  counts are the run-level candidate-evidence contract. Only `operation: gate`
  is accepted by the verification ledger.
- `<id>.diff.raw` for every view carrying a difference, in the same RGBA8 shape
  as the frames it came from, beside the reference's own `<id>.png` pair. It is
  the per-lane ABSOLUTE difference, **unamplified**: a scale factor is a number
  nobody stated, and the statistics are the evidence. The image is for locating
  a difference and not for judging one. No PNG encoder is pulled in for it.

The tracked `tools/oracle/report-contract.json` closes the Rust-to-Python
evidence boundary. It names the exact object schemas, serializer vocabularies,
green unmeasured class and qualifier states, tolerance constants and render
hash domains. Relative input directories are interpreted from the repository
root. The ledger rejects a contract with unknown or duplicate declarations and
rejects statistics whose fractions, signed sums, percentiles, regions or
informative state cannot be emitted by the comparator. Standing guard probes
exercise each refusal and also prove that a genuine relative-path report is
accepted when the ledger is launched from another directory. The production
run verdict consults the same green unmeasured state table, so a new emitted
class, qualifier and rung combination is red until the contract records it.

**Nothing under `tools/oracle/compare-out/` is ever committed.** A difference
image of a real corpus row is derived from that row exactly as a reference frame
is, and every real row carries `burned-in-unchecked` because HLD story E22.3 is
not built. `.gitignore` covers the path and that is not enough on its own,
because `git add -f` exists, so `COMPARE_OUTPUT_PREFIXES` in
`scripts/staged_content_check.py` refuses it by path with its own message.

## The mutation catalogue

Twenty-nine entries in `tools/oracle/src/mutations.rs`, every one replayed on every
oracle gate. Each declares a target, an effect and the verdict it must produce,
and the runner fails the gate when an entry is not detected.

The catalogue reaches all four kinds of answer the comparator can give: a
structural refusal, a comparison refusal, a run-level problem, and a per-view
outcome. The first eight are F-013's metadata-truth probes. They transpose
Pixel Spacing, reverse the IOP vectors, change Rescale Intercept, change VOI
LUT Function and substitute a top-level value where the per-frame functional
group wins. The other three change only the claimed scope, replace the
positive `INVERSE` declaration with `IDENTITY`, and combine that mismatch with
refusals on either input frame. The combined entry exercises the production
`compare_runs` path and turns red if frame I/O moves ahead of metadata truth.
Each must fail at `metadata-truth` and attribute the candidate side. Six
further entries are worth naming.

- `plus-one-on-two-fifths-of-the-image` is the LINEAR against LINEAR_EXACT
  signature at corpus scale, and the bias bound is the only thing that fails it.
- `plus-two-at-the-fraction-budget` and `plus-two-one-pixel-over-the-budget` are
  25.1's fraction boundary from both sides, one pixel apart. The budget is
  computed in integers as `pixels - ceil(999 * pixels / 1000)`, which is 262 on
  the declared 512 by 512 canvas, and 262.144 being fractional is exactly why it
  is computed rather than written down beside a canvas size a later story could
  change.
- `candidate-image-slope-changed` and `reference-image-slope-changed` are the
  same damage on the two sides, and the pair is what proves rung 3's attribution
  is a measurement rather than a constant. Without the second one, "attributed
  to ours" would be indistinguishable from a default.
- `an-undeclared-raw-in-the-directory` is the guard that stops a partial
  comparison reporting as a complete one.
- `plus-three-on-one-pixel-of-a-reformat` damages a view that `rows[]` does not
  name, so a catalogue that only ever damaged a stack row could not have caught
  a comparator reading one list. Its `Target::MeasuredReformat` is NOT pinned to
  a subject carrying a declared reference divergence, and nothing may assume it
  is. Rung 3's narrowing is watched by the three unit tests named above instead.

Per `docs/sprints/CURRENT_SPRINT.md`, the mutation that proves a guard must not
be run in the same command that adds the guard, which is why the catalogue is
standing production data and the gate re-runs all of it every time rather than
trusting a note.

**The effects themselves now have unit tests, and until the fourth pass they had
none at all.** `apply_to_frame` was reachable only from `bin/ocelli.sh compare`,
which needs the rendered corpus, so `cargo test -p ocelli-oracle` never executed
a line of it and nothing in `--floor` did either. That is how `round(u - u/w)`
shipped as "the real thing" through two review passes and how the missing white
exclusion survived a third. `mutations::tests` now carries an eight-pixel table
for the swap, with a 0 and a 255 in it, the drop set hand-computed from the
accumulator and the exclusions derived from PS3.3 rather than from the code.
Restoring the previous guard turns all four of those tests red.

The fifth pass added two more, at windows narrower than the display range, which
is the case the eight-pixel table at `w = 400` could not reach:
`[200, 200, 200]` at `w = 100` owes six drops and `[254, 3, 3]` at `w = 5` owes
fifty-two, fifty of them on the first pixel. Restoring the single subtraction
per pixel turns exactly those two red and leaves the four above green, which is
what says the mutation is the narrow-window defect and not something else.

## Stable render hashes

F-015 gives the exact RGBA8 output a versioned identity before F-151 builds an
attestation over presentation state plus that identity. The algorithm token is
`sha256-rgba8-v1`.

The comparator hashes the `Frame` it already loaded and validated. It never
opens the raw file a second time. The frame constructor has already required
the byte length to equal `width * height * 4`, so allocation padding cannot
enter the digest. PNG bytes, difference statistics, report key order, elapsed
time, environment identity and presentation parameters do not enter either.

### Per-view byte contract

The SHA-256 input starts with the fixed bytes `sha256-rgba8-v1\0`. Each field
after that is framed by an unsigned 64-bit little-endian byte length followed
by those bytes, in this order:

1. the view kind token
2. the opaque view identifier
3. width as unsigned 32-bit little-endian bytes
4. height as unsigned 32-bit little-endian bytes
5. the literal format token `RGBA8`
6. the exact tightly packed frame bytes

Binding the kind, identifier and dimensions means one byte buffer cannot be
silently reinterpreted as a different view or shape. The literal fixture is a
2 by 1 frame with bytes `[0, 0, 0, 255, 1, 2, 3, 255]`. Its independently
computed digest is:

```text
7ce1f3d20a7aa3652620049f76d3b99123acc1ccf35cd59649bef6a87c1785e0
```

`tools/oracle/tests/render_hash_fixture.rs` pins that value and proves that a
pixel byte, dimension, kind or identifier change moves it.

### Run byte contract

The run hash starts with `sha256-rgba8-run-v1\0`, followed by the number of
views as an unsigned 64-bit little-endian value. Per-view entries are sorted by
kind, identifier and digest. Each entry frames the kind, identifier and ASCII
per-view digest with the same length convention. Sorting makes report or input
iteration order irrelevant. The view count and framed entries mean an omitted
or duplicate view cannot have the same aggregate identity.

The comparator writes both levels under `renderHashes` in `compare.json` and
prints the two aggregate hashes on stdout. The identity run has equal reference
and candidate hashes. A candidate mutation changes its per-view and run hash.
Every catalogue effect that changes frame bytes must now move the damaged
side's hash as well as produce its declared comparator result, or the mutation
is reported as not detected.

An equal hash means exact equality under this byte contract. It is not a
tolerance and it is not a claim of cross-machine reproducibility. D14's
measured divergence remains the claim an attestation may make when hashes are
unequal.

## What the comparator still does not build

Named, because each is somebody's story.

- Any Ocelli renderer, decoder, LUT chain or port code. Decision D7.
- A perceptual colour metric, and any CIEDE2000 implementation. Metric and
  threshold are one decision and belong together.
- Re-rendering the saturated rows at a wider window, or a magnified render for
  the two decimated rows. Both change `render-params.json` and therefore every
  reference frame.
- The Ocelli producer for all current views. F-X021 activates the required
  candidate record after F-052 supplies the orthographic viewport. Under D-04,
  CI validates that local record and does not render the ignored corpus.

## Verification-ledger evidence

`scripts/verify_ledger.py record --comparison-report OUT/compare.json` accepts
only a green `gate` report with a positive judged-view count, zero absent views
and a complete record set whose paths, attribution states, sparse signed
histograms, derived statistics, predicates and aggregate hashes are internally
consistent. It records the report's exact SHA-256 digest, judged count and
verdict. The commit hook carries those values in `Ocelli-Verify` when present.

`tools/oracle/report-contract.json` is the cross-language contract for the
closed object schemas, vocabularies, hash domains and green control report.
The Rust report test rebuilds that control through `RunReport::to_json()` and
checks its schemas and constants against production types. The Python ledger
and guard harness read the same file. A serializer change therefore cannot be
copied into a hand-built Python control while leaving validation semantics
behind.

`assert --require-comparison` and `check-commit --require-comparison` are the
local and CI readers. F-012 deliberately leaves them dormant in the ordinary
profiles because no Ocelli producer exists. F-X021 activates the requirement.
