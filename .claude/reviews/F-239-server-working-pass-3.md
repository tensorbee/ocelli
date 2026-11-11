# F-239 server review, pass 3

**Reviewed:** staged repair tree `cc62cdb4ab9e9ca9cecbf43b0836c48e0ba4fcc1`
against `b5ae6518c1ea001ae1d3f279583a3dd9a61725a3`. This is the four-path
D4 repair. Earlier production coverage and findings are in server passes 1 and 2.

**Author / reviewer:** Codex participant `/root/glm_codec_merge` authored the
production repair and adapted permanent fixture. Reviewer `/root/f244_review`
authored neither. The original independent admission reproduction was written
by this reviewer and imported by the author. That ancestry is disclosed rather
than counted as an independent review of the reviewer's own test logic. The
reviewer's browser harness and untracked GLM test file remain excluded.

**Result:** 0 defects, 0 smells, 0 nitpicks in this bounded server repair.
D4 is closed. D1 and D3 remain closed, and the lead's D2 correction describes
the retained, counted storage actually observed.

## Acceptance challenged

Replay of D4 through public calibrated length and actual held-refusal/retry.
Adjacent cases use both missing and segmented palettes, point and ROI refusals
followed by a calibrated line on the same held frame, and non-LINEAR, file
window index and preset refusals before and after valid colour windows.
The original pinned-A/admit-B admission case and an empty-acquire callback
were retained. Valid palette preparation still carries its exact ownership
before cache admission.

The ten independent native pixel/value/ROI/window cases were rerun unchanged,
including native packed 4:2:2, planar band boundaries, both byte orders, masked
12-bit samples, 32-bit containers, RLE layout and codec-reported RGB. Their
normative provenance and literal expectations are recorded in pass 1.

## Evidence

Fresh immutable source: `/private/tmp/ocelli-f239-server-review-p3`.
Separate Cargo target: `/private/tmp/f239-review-p3-target`, two jobs.
All ten authored paths were checked against the supplied tree after mutation
restoration, byte-identical or exact production prefixes followed only by
reviewer controls. Inventory: `/private/tmp/f239-server-review-p3-source.json`.
No author worktree production source was changed by the reviewer.

These commands were executed now. Exact arguments, direct exits and logs are
under `/private/tmp/ocelli-s46-codex`.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `--test review_colour` | Fresh repair and original independent fixtures | Now, `f239-server-review-p3-public` | 0, ten tests pass, including the original calibrated length regression |
| `--lib f239` | Permanent controls, original admission, empty-acquire and refusal/geometry adjacency | Now, `f239-server-review-p3-native-controls` | 0, eight tests pass |
| `--lib f239_review_modes` | First and settled window refusals on one actual held frame | Now, `f239-server-review-p3-held-modes-corrected` | 0, valid window and initial pixels recover without another decode |
| Independent pydicom fixture cross-read | Actual bytes emitted by the author's adapted Rust fixture | Now, `f239-server-review-p3-fixture-crossread` | 0, exact tables, indices, spacing, missing-table variant and no patient fields |
| Restore early return while preserving Result type | Same actual cached-refusal control | Now, `f239-server-review-p3-early-refusal-mutant` | 101 at missing held recipe |
| Restore original production source | Same native controls plus held modes | Now, `f239-server-review-p3-restored-native` | 0, nine tests pass |
| Original public refusal/length case after restoration | Independent missing-palette fixture | Now, `f239-server-review-p3-restored-geometry` | 0 |

The held-mode control first used a Result-returning preset token parser as an
Option and did not compile. The reviewer corrected the test driver with an
explicit successful token parse before execution. That failure is retained as
setup evidence, not a product finding or mutation result.

## Defects

None remain in the reviewed repair scope.

## Smells

None reported.

## Nitpicks

None reported.

## Verified clean

D4 preparation carries `Result<ColourRecipe, PreviewError>` with the same
metadata through held-frame preparation. A successful recipe contributes its
actual persistent box and palette capacity to admission. A failed recipe has
no persistent colour heap and retains its typed refusal in the held frame.
It is not converted into a monochrome recipe or an early decode refusal.

For missing and segmented palettes, point and ROI reads return the expected
colour refusal, then a calibrated line on that same held frame still returns
1 mm. A later preview returns the same refusal with unchanged decode ticks.
The direct early-return mutation breaks the actual held assertion, and the
restored source passes the same consumer checks.

The D1 admission invariant survives this change. Permanent and independent
palette controls keep the pinned A available after B is admitted. The
independent empty-acquire control still covers exact fresh capacity, so a tiny
new sample allocation does not exceed its claim. The adapted fixture was
cross-read independently: two indices 0 and 255, three 256-entry descriptors
with 16-bit entries, exact red `i * 257`, green `(255 - i) * 257`, blue
`128 * 257`, and 1 mm pixel spacing. Its missing variant omits all palette data.

A non-LINEAR first request still creates a held frame. Numeric window success,
file-index and preset refusals, then a valid initial request preserve that
frame, expected channel bytes and decode count. The original ten public
controls establish unchanged conversion, window, inversion, readout, ROI and
native multiframe behavior over their declared cases.

The prior one-walk, three-byte admission and retained-capacity evidence remains
bounded to the unchanged code reviewed in pass 1. The D2 lead amendment was
read in the existing root F239 plan: retained storage is per conversion
workspace, with the grey over-twice policy separate, and actual capacities
counted. The measured 16- and supported 32-bit container cases remain distinct.

## Limitations and handoff

This pass approves the bounded server production repair, not the entire F239
story, browser harness, GLM test file, full viewer union or sprint readiness.
The old page-sentence expectation is the root's separate page integration.
README, Use and More in that union still require their composed review.

No browser, GPU, oracle, full workspace, clippy sweep, feature/sprint profile or
RSS/benchmark measurement was run by this reviewer in this pass. Author check
and clippy receipts were read as prior evidence, not relabelled as new checks.
Actual codec arithmetic remains with its earlier codec reviews. The JPEG
consumer equality check establishes layout/composition only.

Root can compose this exact source with the independently reviewed page and
then run the required combined checks and remaining external measurements.
A later implementation or GLM test delta needs its own source identity and
appropriate review. No commit or feature-completion claim was made here.
