# F-239 colour observation, working review pass 2

Reviewed immutable tree `30473ff7493739545f56276016d7968a525f2c57`,
including the complete pass 1 delta and the six-path remediation against
`f1d94095059834a1c9271d63460fc4f1986b3a87`. Author: root Codex.
Reviewer: independent Codex participant `glm_codec_merge`, who authored no
production or remediation in this observation delta.

**Result: 0 defects, 0 smells, 0 nitpicks on the bounded reviewed material.**

## D1, default inversion preserved and explicit colour still bound

The actual page calls the shared `viewportProperties` builder and passes its
result to the pinned viewport. Only `explicit-colour` supplies `invert`.
File and none sources omit that property. The builder still resets the VOI
function and delegates range conversion to the pinned utility, without a
second numerical formula.

The permanent Node regression passes. Restoring the unconditional false
assignment in the private copy makes that same regression fail, exit 1.
Restoring reviewed source passes, exit 0. Independent controls invoke the
actual pinned `StackViewport.setProperties` and `setInvertColorGPU` methods
with the production builder: file and none preserve MONOCHROME1's initial
inversion and both explicit colour values take effect, four cases, exit 0.

The reviewer independently hashes the final actual full reference output
against the prior F-244 AC22 output: all 86 shared monochrome frames,
including the MONOCHROME1 frame, are byte-identical. All 111 shared frames
also equal the corrected reference before the builder extraction. Both final
explicit partial references equal their earlier explicit outputs byte for
byte, record the declared window and inversion, and remain marked partial.
The source binding therefore has actual reference evidence beyond a helper
unit test. The reviewer did not launch those browsers.

## D2, measured gaps and source arithmetic remain separate

The plan now has explicit floor rows for the bounded JPEG decoder RGB gap,
measured GPU colour-window quantisation and the native observation producer's
fractional quad coverage. The LLD records all 13 renderable initial colour
rows plus both explicit observations, retaining full-canvas maximum and
pixel counts and recording that alpha agrees. It names the native
YBR_FULL_422 reference refusal instead of counting it as drawn.

The reviewer independently recomputes every canvas and source-centre aggregate
for all 13 rows from the final reference and unchanged native candidate.
Every recorded number agrees. The two explicit cases retain their 9,948
canvas differences, maximum 64, with 853 fractional-edge pixels above one
code. All 720 candidate source-centre channels still equal the hand rule.
No scored colour comparison, new tolerance, general JPEG error bound or
whole-canvas parity claim is made. The lead has explicitly named the
projection gap and its reason, as AC20 requires for attribution.

Pass 1's actual JPEG control remains applicable: all 6,144 candidate centres
are the decoder's reported RGB, proving that the bounded maximum-two gap is
before the colour stage. The revised prose attributes no specific IDCT or
chroma algorithm. Pass 1's native producer tests, public lifecycle controls
and two semantic mutants are carried evidence because the producer's blob
is byte-identical across these snapshots, verified mechanically.

## Other changed material and executed checks

- Node params and args: exit 0, 69 declared, 65 passed and 4 skipped. The
  unchanged skips are not described as exercised evidence.
- Metadata helper self-test: exit 0. Full actual output cross-read: exit 0,
  102 stack sidecars, 9 volume reformats, 18 hand fixture rows and 2 resolved
  metadata rows. The four new metadata declarations assert eight literal
  fields each, matching the approved 64 by 96 unsigned RGB8 specification
  and actual pydicom readings. Reversible and irreversible photometric names
  match PS3.5 8.2.4. No expected value was extracted from a reference sidecar.
- Independently downloaded the official pinned dicom-image-loader tarball,
  verified its SHA-512 against the lockfile and compared all 248 regular
  installed files. No difference, including createImage.js. Archive identity
  and the exact verified tarball are preserved in review evidence.
- Initial reviewer metadata-check script guessed the truth-table key as
  `cases` and failed before its fixture check. It was corrected to the actual
  `entries` schema and rerun successfully. This was reviewer setup, not a
  product failure or a passing check.

Evidence: `/private/tmp/ocelli-s46-codex/ac20-review-p2`, with direct command
receipts using the `ac20-review-p2-` prefix in the parent directory.
`identity.json`, `observations.json`, `default-metrics.json`,
`final-partial-binding.json`, `invert-mutant.json` and `package-integrity.json`
record exact inputs and results. Pass 1 retains the original failures.

This clean review covers the observation implementation, its remediations
and the factual claims above. It is not a feature profile, sprint profile,
GPU-tier proof, performance measurement or approval to publish. No source
edits or commits were made in the author worktree by this reviewer.
