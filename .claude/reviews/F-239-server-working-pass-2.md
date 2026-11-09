# F-239 server review, pass 2

**Reviewed:** repair tree `b5ae6518c1ea001ae1d3f279583a3dd9a61725a3`
against pass 1 tree `3f1f607d4d526c6b7f63292e6a88afaedba2c584`.
The immutable source copy is `/private/tmp/ocelli-f239-server-review-p2`.

**Author / reviewer:** repair and permanent fixture adaptation by Codex
participant `/root/glm_codec_merge`. Independent reviewer `/root/f244_review`
authored neither the production repair nor its Rust fixture adapter. The
reviewer authored the original independent lifecycle reproduction. Its logic
is disclosed, not counted as independently reviewed test authorship. The
reviewer's browser harness and untracked GLM test file remain excluded.

**Result:** 1 new defect, 0 smells, 0 nitpicks. Prior D1 and D3 are closed
within this repair scope. The lead's D2 policy correction was read separately
in the existing F239 plan and matches measured retained storage.

## Acceptance challenged

The six-path repair delta prepares palette ownership before sample acquisition,
passes its actual persistent overhead to every cache acquisition caller,
reserves fresh sample capacity exactly, and updates public field comments.
Review challenged the original tight-budget held-frame case, an empty-acquire
callback, the fixture adapter, first/refused/retry lifecycle, and geometry
independence when only colour rendering is unavailable.

Existing independent pixel and raw-value controls were replayed. The source
repair was tested using a separate Cargo target directory after detecting that
Cargo could reuse an artefact across two review workspace roots when they
shared one target directory. The fresh-target receipts below identify the
actual repaired source.

## Evidence

Direct-exit JSON receipts and logs remain under
`/private/tmp/ocelli-s46-codex`. Commands used two Cargo jobs. The corrected
P2 controls use `/private/tmp/f239-review-p2-target`, distinct from P1.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `--lib palette_admission` | Actual repair, two permanent controls and original independent lifecycle with pydicom input | Now, `f239-server-review-repair-admission` | 0, three tests pass |
| `--lib fresh_palette_claim` | Empty acquisition callback, independent 256-entry palette | Now, `f239-server-review-repair-fresh-reserve` | 0, actual charge equals predecode claim |
| `--test review_colour` | Same independent ten public controls as pass 1, fresh P2 target | Now, `f239-server-review-repair-public-fresh-target` | 101, nine pass and calibrated missing-palette length refuses |
| Refused-recipe cache control | Original source, actual cache presence and decode ticks | Now, `f239-server-review-refused-cache-before-corrected` | 0, refusal is held and retry does not decode |
| Same cache control | Repaired source, freshly built in separate P2 target | Now, `f239-server-review-refused-cache-after-corrected` | 101, no held recipe remains after refusal |
| Remove colour recipe overhead from claim | Independent empty-acquire control | Now, `f239-server-review-repair-undercharge-mutant` | 101, actual 6818 versus claim 458 |
| Restore that overhead | Same control | Now, `f239-server-review-repair-undercharge-restored` | 0 |
| Remove exact fresh sample reservation | Same control | Now, `f239-server-review-repair-reserve-mutant` | 101, actual 6824 versus claim 6818 |
| Restore exact reservation | Same control | Now, `f239-server-review-repair-reserve-restored` | 0 |

Setup limitations were kept separate. The first cache control incorrectly
expected a Source header on errors, which existing error replies omit. It was
replaced by actual `PreviewSource::Hit` and decode-count checks. Its failures
are not production findings. The first P2 cache attempt after switching roots
shared a target with P1 and is not used as repaired-source evidence. A new
separate target was built and the corrected control failed at the actual
missing-held assertion. The original public length regression also reproduced
in the separate target.

## Defects

### D4, pre-acquire preparation returns recipe refusals too early

**Where:** `examples/folder-viewer/src/preview.rs`, `finish_decode`, the new
`ColourRecipe::new(...)?` before `prepare_samples`.

**What:** the repair correctly resolves the palette once before admission,
but now returns a colour recipe error before constructing a held frame. This
changes a rendering refusal into a refusal of the entire decoded-frame
consumer path.

An independently generated two-pixel PALETTE COLOR file has valid unsigned
samples, Pixel Spacing `1\1`, position and orientation, but no palette tables.
Before the repair, a calibrated length from `(0.5, 0.5)` to `(1.5, 0.5)` is
1 mm even though rendering is unavailable. After the repair, public
`measure_frame` returns `ColourUnavailable(Palette)` before geometry runs.
The same input's preview refusal was previously cached in a held recipe. The
repair leaves no held frame after its first refusal.

**Why it is wrong:** F211 keeps geometric measurement independent of a scalar
or colour mapping refusal. F239's ownership table explicitly says a refused
resolve is held as a recipe so a retry returns the same refusal without a
second decode. The repair needed earlier ownership accounting, not an early
change to these consumers' failure policy.

**Scope and regression:** missing, segmented or otherwise unusable palettes
and other failures returned by `ColourRecipe::new` can take this path. Carry
`Result<ColourRecipe, PreviewError>` with the prepared metadata, reserve actual
persistent colour heap on success, and preserve the typed held refusal on
error. That is a lead-directed remediation, not code authored by this reviewer.
Preserve both the 1 mm public length case and actual held-refusal/retry control.

The lead accepted this finding and assigned the author a repair. Freeze6
`cc62cdb4` is new material requiring a separate pass.

## Smells

None reported.

## Nitpicks

None reported.

## Verified clean

The D1 repair counts the same prepared box and both vectors of each owned LUT
descriptor before the cache chooses room. That prepared recipe moves into the
held frame rather than being rebuilt. Every production cache acquisition
caller carries the overhead, and reused sample buffers reserve their actual
capacity. The independent empty-acquire case rejects both a missing palette
charge and Vec's minimum fresh-capacity growth. Original and adapted lifecycle
controls preserve the pinned A after B is admitted.

The updated public comments distinguish float colour inversion, colour's
LINEAR default and its absence of selectable file windows. Their behavior
matches the inverse-tie and ignored-file-VOI cases. The lead's ownership ruling
now describes retained conversion storage per workspace, with separate 16-bit
and supported 32-bit container bounds and actual capacity counting.

Nine unchanged public composition controls still pass, covering packed and
planar layouts, byte order, sample scaling, reported RGB, raw values, ROI
statistics, multiframe selection and window/inversion arithmetic. D4 is an
additional state/refusal regression, not a rejection of the ownership repair.

## Limitations and handoff

No browser, GPU, oracle, full workspace/profile or overall feature approval.
The full viewer union, README and Use remain separate root work. GLM's new
untracked tests are outside this source. Native reference equality at a codec
boundary is composition evidence, not a new codec arithmetic qualification.

Use freeze6 in a fresh bounded review. Keep the original D4 RED receipts and
replay public geometry and actual cached refusal as well as palette admission.
No source remediation or commit was made by this reviewer.
