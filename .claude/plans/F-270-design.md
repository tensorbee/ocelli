# F-270, irreversible JPEG 2000 decode within a measured bound of OpenJPEG

**Status**: draft
**Epic ref**: E4.15
**Sprint**: S48
**Estimate**: 3w

Filed at the S46 close from the recovered close-backlog draft, original item
6. The estimate is inherited and is provisional, lead-suggested and not
capacity-approved. This draft claims no implementation, no measurement, no
review cleanliness and no completion.

## Normative source, transcribed

`docs/hld/11-decision-log.md` D14: "Attestation claims measured divergence"
rather than claiming bit-exact reproducibility. The remaining difference after
the fix is recorded as a measured divergence and never scored.

ISO/IEC 15444-1 is the investigation's standard. The official
[ITU-T T.800 (07/2024) table of contents](https://www.itu.int/dms_pubrec/itu-t/rec/t/T-REC-T.800-202407-I%21%21TOC-HTM-E.htm)
names Annex E Quantization, with inverse reconstruction in E.1, and Annex F
Discrete wavelet transformation of tile-components, with inverse filtering
in F.3. Annex D covers coefficient bit modelling and Annex A codestream
syntax. These are investigation references, not a diagnosed cause. The
investigation reads the standard and measures, and reads no OpenJPEG source.

`docs/hld/22-testing-and-tolerance.md` 25.1: "Write it down once and hold
it." No acceptance tolerance is created or widened by this story. A measured
bound is recorded as a D14 record, and the corpus row's existing one-code
agreement is left exactly as it is.

## What the specification does not cover

The bound itself. No HLD or standard text gives a number for how close an
irreversible decoder must sit to OpenJPEG. The story measures and records, and
D14 says what kind of claim that is. The cause is also open until the story
finds it.

## Problem and evidence

F-239 codec review pass 3 measured Ocelli through `Registry::decode` against
pylibjpeg-openjpeg 2.5.0 on synthetic 112 by 80 RGB images encoded by
`opj_compress` 2.5.4. Per-channel maximum absolute difference, from its table:

| Case | 8-bit | 12-bit | 16-bit |
|------|-------|--------|--------|
| J2K lossy ICT `.91`, `-q 80` | `[1, 1, 1]` | `[1, 1, 1]` | `[11, 10, 15]` |
| J2K lossy ICT `.91`, `-r 8` | `[34, 29, 26]` | `[278, 276, 402]` | `[2012, 1676, 4596]` |
| J2K lossy `-r 8`, MCT 0 control | `[10, 5, 15]` | not run | `[4917, 4559, 4627]` |

Every lossless case was byte exact. The review's handoff adds that at
`-n 1 -r 8`, 16 bits, MCT 0, two channels differ by exactly 2,048, and there
ritk's error against the source image is 6,143 where OpenJPEG's is 4,095. So
ritk is further from the source than OpenJPEG, not only different from it,
and the MCT 0 rows show the inverse ICT is not the cause.

Pass 3's limits, carried into the story: a monochrome 16-bit `.91` control was
not run because the adapter refused that pass's `opj_compress` PGM output with
`UnsupportedPixelFormat`, and ISO/IEC 15444-1 Annex G and OpenJPEG's `mct.c`
were not read. The story keeps to the standard and to measurement, and reads
no OpenJPEG or Grok source. The monochrome 16-bit control is built by the
story rather than claimed.

Starting points in the vendored decoder, read at 31323a79 and not a finding:
`vendor/ritk-codecs-0.6.0/src/jpeg_2000/quantization.rs:154` (`step_size`),
`:198` (`dequantize`) and `wavelet_9_7.rs:63` (`inverse_dwt_9_7`). These source positions
are starting points only. The story re-baselines every number at its own
tip rather than assuming that a historical location or measurement is current.

## Scope, API and compatibility

In scope: the rate-truncated monochrome and colour `.91` comparison set at 8,
12 and 16 bits, the cause found against Annex E and F, the fix in the vendored
decoder with the pins record refreshed, and the residual recorded as a D14
divergence with its measurement.

Non-goals: lossless paths, HTJ2K, OpenJPEG source, encoder changes, tolerance
changes, and any public API change.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Registry::decode` on `.91` | The vendored ritk decoder | Unchanged API | Samples closer to OpenJPEG by the measured bound |
| Folder viewer and the showcase decode consumer | The same registry path | Colour `.91` files through the front door | The corrected pixels on screen |
| The recorded divergence | `docs/lld/codecs.md` | D14's record shape | The bound, the method and the versions |

## State transitions and invariants

Not applicable. The decoder is stateless per frame and its refusal policy is
unchanged.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | The comparison set exists: rate-truncated 8, 12 and 16-bit monochrome and colour `.91` codestreams with their OpenJPEG decodes, the MCT 0 controls and the monochrome 16-bit control pass 3 could not run | The generated set and its provenance, tool versions recorded | A set that only repeats pass 3's images | implementation |
| AC2 | Base and tip per-channel maxima recorded for every case, with the method | The recorded table | A bound claimed with no number | implementation |
| AC3 | The 16-bit rate-truncated difference's cause is named against Annex E or F, with the clause | The finding and its citation | A fix without a cause | implementation, reviewer |
| AC4 | The fix lands in the vendored decoder with the pins record refreshed and the decode graph unchanged | The pins gate and the decode graph check | A fix outside the vendor record | implementation |
| AC5 | Every lossless case stays byte exact | The existing suites | A change that perturbs lossless | implementation |
| AC6 | The residual is recorded as a D14 divergence in `docs/lld/codecs.md`, with units and both versions | The recorded entry | Silence, or a tolerance created instead of a record | implementation, reviewer |
| AC7 | Negative control. Reverting the fix returns AC2's 16-bit maxima | Recorded in the review | A test insensitive to the fix | implementation, reviewer |
| AC8 | The visible result is through the front door. A colour `.91` file draws in the single viewer with the corrected decode | A browser row on the usual host | A Rust-only improvement counted as end-user acceptance | integrated browser |

Unrun ownership. No decode, comparison or browser row ran for this filing.
AC1, AC2 and AC7 run at the story tree, and AC8 needs the browser host,
staying open with that owner named if it is unavailable.

## Approach

1. Rebuild pass 3's set plus the missing monochrome control, recording
   `opj_compress` and the adapter route per case.
2. Measure base per-channel maxima.
3. Read Annex E and F against `step_size`, `dequantize` and the 9/7
   reconstruction, and derive the expected arithmetic by hand for a small
   case before changing anything.
4. Fix the vendored decoder, refresh the pin record, and re-measure.

Rejected: replacing the vendored decoder, matching OpenJPEG by copying its
constants without the clause, and absorbing the difference into a tolerance.

## Dependencies, write ownership and resources

Depends on F-026, done, and F-239, in-progress in S46, as the recovered draft
had. Both dependencies must be DONE before this consumer story starts
implementation. A staged F-239 union does not satisfy that condition. If
F-239 carries beyond S46, schedule this story after its actual completion,
preserving dependency order. Writes
`vendor/ritk-codecs-0.6.0/src/jpeg_2000/**` and the pins record,
`docs/lld/codecs.md`, and synthetic generators. The vendored `jpeg_2000`
files are not shared with F-269's `jpeg_ls` audit, and the pins record is
refreshed once per story in sequence. OpenJPEG runs as a host tool on the
operator's machine.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-codec` while iterating, the feature profile at
the tree, and AC2's table as the evidence, with tool versions recorded beside
every number. Evidence invalidates on any later vendored change, which
re-runs AC2.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Decode is outside the render loop
- unsafe: none
- Tier A (WebGPU): n/a. Decode is CPU arithmetic in the core
- Tier B (WebGL2): n/a
- Tier C (CPU): full. The decode is the CPU path and the fix applies to it

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | The hand-derived step-size or reconstruction case, clause cited | `crates/ocelli-codec/tests/jpeg2000.rs` |
| conformance | The comparison set against OpenJPEG's decodes | AC1, AC2, recorded |
| unit | Lossless unchanged | The existing suites, AC5 |

## Parity surface covered

None new. The corpus `.91` row's one-code agreement is F-026's and stays.

## Deviations

None new. D14 already governs the claim's shape.

## LLD impact

`docs/lld/codecs.md`, the D14 record and the measured bound.

## User-facing documentation impact

`examples/folder-viewer/src/more.html`, if it names `.91` fidelity. Else no
change.

## Open questions

1. The comparison set's exact sizes and rates, fixed at design time. Affects
   AC1. Owner: the design round.
2. Whether the recorded bound is one number or a per-case table. Affects AC6.
   Owner: the design round, beside D14's record shape.
