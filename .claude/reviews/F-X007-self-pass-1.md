# F-X007 review, pass 1

**Reviewed**: the working tree of `work/f-x007-claude` against base `9df8539`
**Result**: 0 defects, 0 smells, 3 nitpicks

This is the implementer's own pass, so it is not independent and does not
substitute for `/microscope F-X007 --working`. It records what was checked and
what was found and fixed during implementation, so an independent pass can see
where this one already looked.

## Defects found and fixed during implementation

Recorded rather than omitted, because each was found by executing a claim
rather than by reading the code, and the next reviewer should know which of
those habits paid.

### F1, a failed reformat reported the run green

**Where**: `tools/oracle/page/volume.mjs`, the `failure` helper.
**What**: `failure(boundary, error, extra)` built `{ok: false, boundary, stage,
error, ...extra}`. `extra` is the partial SUCCESS record, which carries
`ok: true` and `boundary: null`, so the spread overwrote both. The driver then
counted a subject that had failed at `reformat-presented` as one that succeeded
with no frames.
**Why it is wrong**: this is the quietly-wrong class the whole harness exists to
refuse, reached through an object spread. A guard fired, produced the right
message, and the message never left the page.
**Evidence**: five of the six reformat faults reported "the injected fault did
not break anything". After spreading `extra` FIRST, every one is red at its own
boundary. Both states observed.

### F2, the OK line claimed twelve reference frames where nine were written

**Where**: `tools/oracle/run.mjs`, `volumeCounts`.
**What**: `reformatsReadBack` counts what the PAGE read back. `real/ct_cmb_mml`
renders three reformats and is then refused at `volume-geometry`, which is the
DRIVER's boundary, so twelve frames are read back and nine become reference
output. The OK line reported twelve.
**Why it is wrong**: a false claim in output. F-011 reads that directory.
**Evidence**: `ls tools/oracle/out/volume__*.raw | wc -l` gave 9 against an OK
line saying 12. Fixed by adding `reformatsWritten` with an accounting identity
(`volumesBuilt * orientations.length`), and the identity is asserted rather than
assumed.

### F3, a real corpus row's slice position reached an error message

**Where**: `tools/oracle/src/geometry.mjs`, the same-position refusal.
**What**: the message named both member paths AND their projected positions.
`real/ct_cmb_mml` reaches it, so a real study's coordinate went into stdout on a
red run and into `run.json` on a green one.
**Why it is wrong**: `corpus_check.py`'s convention, which
`check_sidecars.py` follows and states, is that a real row reports the relative
path and the attribute name and never the value. CLAUDE.md's first rule.
**Evidence**: the message is now the two paths and the tolerance, and the
refusal was re-observed on `--rows real/ct_cmb_mml/`.

### F4, `volume-repeat-slice` hung the load instead of reaching its guard

**Where**: the fault's injection point.
**What**: handing `createAndCacheVolume` a repeated image id does not produce a
volume with a repeated member. `getImageIdIndex` resolves a repeat to the FIRST
index, `callLoadImage` early-returns for it, `framesProcessed` never reaches
`totalNumFrames`, and the run hangs until the declared five-minute timeout and
fails THERE.
**Why it is wrong**: the fault would have proved the timeout guard a second time
and the membership guard not at all, while costing five minutes on every gate.
**Evidence**: measured, a 300 second hang. The injection now corrupts the BUILT
volume's `imageIds`, which is the state the guard reads, and it is red in
seconds.

### F5, a fixture that could not tell a mean from a median

**Where**: `tools/oracle/tests/geometry_test.mjs`.
**What**: `voxelAxes.sliceStepMm` is the MEAN gap. On both corpus series the
mean and the median are exactly 2.5, so no fixture built from the corpus could
distinguish them.
**Evidence**: swapping `meanGapMm` for `median(gapsMm)` left the whole geometry
suite green. A four-slice series where the two differ was added, and the same
mutation is now red.

## Nitpicks

### N1, the volume page duplicates three small helpers

`base64ToBytes`, `bytesToBase64`, `sha256Hex`, `uniformValue`, `isSentinel` and
`frameStatistics` are the same in `page/app.mjs` and `page/volume.mjs`. Sharing
them would mean editing `app.mjs`, which this story is forbidden to touch
because `renderParamsSha256` and the eighty-nine stack digests are F-011's
evidence that nothing moved. Worth folding into a shared module by whoever next
has a reason to change `app.mjs`.

### N2, `check_sidecars.py` reads each volume member once per subject

Cached within a run, so each file is read once rather than once per orientation,
but a member of a subject is still read separately from its stack-partition
read of the same file. Two reads of twenty files.

### N3, `InstanceNumber` is not carried on a volume sidecar's members

The design plan's sketch listed it. `page/app.mjs`'s attribute reader does not
read it and this story may not change that file. It is Type 2 and a label rather
than a geometry, and the sort is by projected position and by nothing else, so
nothing depends on it.

## Verified clean

**Arithmetic, against the cited section rather than the comment.**

- The slice normal is `row x col` and not `col x row`, asserted in both
  directions in `geometry_test.mjs` (PS3.3 C.7.6.2.1.1).
- `PixelSpacing[0]` multiplies the COLUMN direction cosine and `[1]` the ROW
  one, asserted on the deliberately non-square `[0.5, 0.25]`, and the
  transposed answers are asserted NOT to have come back. Mutation: swapping the
  two indices in `voxelAxes` goes red on exactly that fixture.
- Slice spacing is the difference of projected `ImagePositionPatient` values.
  Neither `SpacingBetweenSlices` nor `SliceThickness` is read anywhere in
  `geometry.mjs`, and `check_sidecars.py` asserts on the files that the first is
  absent and the second is the nominal 2.5 on both series, which is what would
  make a tag reader answer 2.5 for a series whose gaps are 2.5, 3.75 and 1.25.
- The z profile's expected values are derived from PS3.3 C.7.6.3.1.4 and PS3.3
  C.11.1 in `volume-truth.json` and come out 1056 to 1200 in steps of 16,
  exactly as computed.
- No `as` casts, no `unsafe`, no Rust. This story writes none.
- Tolerance: HLD 25.1's 1e-6 mm is the only one used, it is asserted as a
  constant, and `volume_test.mjs` shows 2e-6 red and 5e-7 green, so the number
  decided something.

**Would the tests fail if the code were wrong.** Six mutations run, each
reverted:

| Mutation | Result |
|---|---|
| swap `PixelSpacing` indices in `voxelAxes` | red, on the transposition fixture |
| `meanGapMm` to `median(gapsMm)` in `sliceStepMm` | red, after N1's fixture was added |
| fixture's expected `gapsMm` 3.75 to 2.5 | red |
| disable `checkZProfile`'s first-value branch | red |
| disable `checkZProfile`'s step branch | red, nine cases |
| `comparePairs` `identical` always true | red |
| `compareGeometry` tolerance 1e-6 to 2 | red, two cases |
| `measureSubject` same-position refusal disabled | red |
| `check_sidecars.py` truth 18.75 to 17.5 | red, three sidecars |
| a volume sidecar's member `imagePositionPatient` perturbed | red |

**Things that exist and that nothing executes.** Eleven new faults exist and
every one has been observed red at its own boundary for its own reason.

**This paragraph claimed more than that and the S03 sprint review corrected it.**
It said every new refusal in the volume page and the volume half of the driver
is aimed at by exactly one of those faults. Four are not, and all four are
declared-parameter guards written in the same shape and with the same reasoning
as the orientation guard, which does have `bad-orientation`: the z-profile
voxel-range refusal, the blend-mode refusal, the interpolation refusal and the
camera-mode refusal. `mutateVolumeRequest` touches only `loadTimeoutMs`,
`orientations` and `params.canvas`, and `mutateParams` and `pageFault` are
stack-only by design, so no fault can reach them. Being page code, no unit test
reaches them either. They are carried to F-X009, which is the story that gives
every guard a standing probe. The three refusals that are pure functions rather than browser
state (`checkZProfile`'s first-value branch, `validateVolumeParams`'s ten key
refusals, `validateVolumeTruth`'s self-consistency checks) are covered by unit
tests that were watched go red.

**Boundary and tier.** No Rust, no `wasm-bindgen`, no pixels across an Ocelli
boundary, no render loop, no `queue.submit`. All three tiers are n/a and are
declared n/a in the design plan rather than omitted. This story runs somebody
else's renderer and declares no tier of its own.

**The property F-011 depends on.** `tools/oracle/render-params.json`,
`page/app.mjs`, `page/index.html`, `src/params.mjs`'s resolution rules,
`src/voi.mjs`, `Cargo.toml`, `src/lib.rs`, `bin/ocelli.sh`, `corpus/manifest.tsv`
and `scripts/corpus_synth.py` are all untouched, confirmed by
`git diff --cached --name-only`. All eighty-nine stack digests are byte-identical
to the recorded baseline.

---

# F-X007 review, pass 2, the integrator's finding

**Reviewed**: `run.json`'s volume counters on head 0a5267b
**Result**: 1 defect, found by the integrator and not by pass 1. Fixed.

### D1, `reformatsPresented` and `reformatsReadBack` reported the declared total

**Where**: `tools/oracle/run.mjs`, the volume loop, where both were accumulated
before the refusal check.
**What**: both counted 12 where 9 reformats became reference output, so one
`boundaries` object used `readBack` to mean "achieved and written" for stacks,
where `readBack: 89` equals the 89 files on disk, and "attempted and discarded"
for reformats.
**Why it is wrong**: `docs/lld/oracle.md`'s own rule is that covered is not the
same as measured, and the boundary counts exist so that a count cannot claim
more than occurred. The cause is that the volume boundaries do not run in the
order they are listed: `volume-geometry` is the driver's and runs LAST, so a
subject refused there has already rendered its reformats.
**Why pass 1 missed it**: pass 1 found the same shape one level up, in the OK
line, and fixed THAT by adding `reformatsWritten` with an identity on it. The
identity was written for one counter of a trio, so the other two were left free
to count something else, and they did. An identity over part of a set is not an
identity.
**Evidence**: `reformatsPresented`, `reformatsReadBack` and `reformatsWritten`
are now 9, 9 and 9 against 9 `volume__*.raw` files on disk, beside `readBack`
89 against 89 stack files, both asserted against `ls`. The declared total is
published as `reformatsDeclared: 12`. What a refused subject reached is recorded
per subject in `volumes[]`, so `volume__real__ct_cmb_mml` still shows
`reformatsPresented: 3` with `frames: []` and nothing is erased.
**The identity, observed red**: reintroducing the exact defect, accumulating for
every subject rather than only for those that survived every boundary, on
`--rows real/`, which is one built subject and one refused:

```
volumes  applicable 2, built 1, refused 1, reformats declared 6,
         presented 6, read back 6, written 3
FAIL: oracle
  accounting: 1 volume(s) built at 3 orientation(s) each is 3 reference
  reformat(s), and reformatsPresented is 6.
  accounting: 1 volume(s) built at 3 orientation(s) each is 3 reference
  reformat(s), and reformatsReadBack is 6.
```

Real exit code 1 mutated, 0 reverted, read from the run and not from a pipe.
The identity is now written over all three counters rather than over one.
