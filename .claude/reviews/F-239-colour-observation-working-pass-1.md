# F-239 colour observation, working review pass 1

Reviewed tree `f1d94095059834a1c9271d63460fc4f1986b3a87` against
`06cc8a7908659a0ef906cc9ecba874f0522684a8`. Twelve changed paths.
Author: root Codex. Reviewer: independent Codex participant `glm_codec_merge`.
The reviewer authored none of this production delta. Review used an immutable
archive at `/private/tmp/ocelli-f239-ac20-review-p1`, separate build output,
and read-only synthetic corpus input. No reviewer browser or GPU run.

**Result: 2 defects, 0 smells, 0 nitpicks.**

## D1, the default monochrome reference inversion is changed

`tools/oracle/page/app.mjs:579` assigns `properties.invert = voi.invert ?? false`
for every frame. File VOI does not supply invert. The pinned 5.8.2 provider's
`StackViewport` sets `initialInvert` for MONOCHROME1 and applies it when the
actor is installed. Its actual `setProperties` forwards the new false value
to `setInvertColorGPU`, reversing that existing transfer function again.

Independent executable control invokes the pinned provider's actual methods
with a MONOCHROME1 transfer-function state. Without the new property, invert
stays true and the first RGB node stays white. With the reviewed page's actual
assignment, invert becomes false and that node becomes black. This violates
the amendment's unchanged-default claim and affects scored monochrome rows.
Smallest repair: bind invert only for the explicitly declared colour source,
then preserve default monochrome output through the real reference route.

Receipt: `ac20-review-default-invert-defect`, exit 0 demonstrating the defect.
The lead's later conditional fix is outside this reviewed snapshot.

## D2, measured differences need explicit floor attribution

AC20 requires per-row largest difference and differing-pixel count, with every
difference attributed to a floor-table row. The reviewed table does not name
fractional image-quad coverage or GPU colour-window quantisation. The actual
new observations differ at 9,948 canvas pixels, maximum 64, in each of the
window and inverted cases. Calling them full-canvas agreement would be false.

Independent physical-camera sampling checks every original source-pixel
centre, all 240 pixels and 720 RGB channels. Candidate output agrees exactly
with hand LINEAR centre 128, width 100 and the approved final ties-even byte
rule. Reference output differs at 10 channels by one code, five each way.
Every canvas difference above one code is on columns 42 or 469, 853 pixels,
where the reference has fractional edge coverage and the candidate has black
background. Quarter-intensity edge values are consistent with the reference's
multisample coverage, rather than wrong source colour arithmetic.

Smallest repair is an explicit lead attribution in the floor table and final
per-row observation record, retaining the full-canvas figures and naming the
projection gap and its reason. Source-centre evidence supplements those
figures and cannot replace them. No threshold or scored-colour rule is added.
A canvas renderer change is not required by the recorded-observation contract
if this measured gap is explicitly accepted and explained by the lead.

## Executed checks and bounded clean coverage

- Node params and args suites: exit 0, 68 declared, 64 passed and 4 skipped.
  Missing values, separate subset output, closed observation selection,
  unchanged default resolution, malformed pairs and non-LINEAR refusal pass.
- Named native `colour_observation`: exit 0, 2 selected. Other test binaries
  selected zero, recorded in the raw output. Window-bypass and invert-bypass
  mutants each fail semantically with exit 101. Restored source passes.
- Actual public `ocelli-compare produce`: six calls, each exit 0. Changing the
  window or invert changes emitted pixels. Incompatible camera placement and
  invalid colour metadata omit output. A following valid call recovers the
  exact original digest. A zero process status with an omitted view is not
  called rendered output.
- Independent native JPEG control: registry reports RGB and all 6,144 source
  centres of the actual candidate equal the decoder's RGB bytes. There is no
  second YBR inverse. Against reference centres those decoded bytes differ
  at 3,857 pixels, maximum 2, with 27 source pixels above one code. This is a
  measured decoder-output gap, without attribution to a specific IDCT or
  chroma algorithm. An external imagecodecs decode is also not byte-identical
  to reference, so it does not establish a stronger cause.
- No new Rust `as` cast is introduced. Colour conversion, display scaling,
  window and byte rounding reuse library owners. The native geometric raster
  uses the executed camera, or refuses unsupported placement. Whole-image
  scratch is observation memory and is not a performance claim.

Evidence directory: `/private/tmp/ocelli-s46-codex/ac20-review-p1`.
Direct command JSON/log receipts are in its parent directory with the
`ac20-review-` prefix. `source-centres.json`, `edge-localization.json`,
`jpeg-comparison.json`, `native-mutants.json` and `public/results.json` retain
actual inputs, outcomes and limitations. The independent JPEG test exists
only in the private review copy. The root's browser observations are separate
receipts, read as inputs rather than attributed to the reviewer.

All-colour final attribution, default MONOCHROME1 preservation after D1 repair,
and the lead's subsequent metadata-truth additions remain for pass 2.
