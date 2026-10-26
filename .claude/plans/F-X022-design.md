# F-X022, preserve the selected VOI sequence on manual window changes

**Status**: approved
**Epic ref**: Y1.17
**Sprint**: S16
**Estimate**: 1w
**Owner**: xAI model implementation, independent reviewer unassigned

## Normative source, transcribed

HLD section 18's stage-2 source is "Window centre/width with a function, or
a VOI LUT Sequence". Section 18 instructs: "Implement it once, in
ocelli-pixel, and let the shader read the parameters".

F-046's approved sequence policy: "A preset does not replace either sequence
with the linear numbers it overrode." S15 CURRENT_SPRINT's done contract:
"A chain driven by a VOI LUT sequence stays unavailable rather than being
replaced by the linear values the sequence overrode."

PS3.3 C.11.2.1.2.2 allows alternative VOI views. Ocelli's selected-chain
refusal policy is stricter. This plan does not claim the standard prohibits
explicit alternative selection.

## Problem, scope and consumer map

Audit D1 constructs a three-entry VOI sequence `[5, 15, 25]`. Manual window
40/400 returns success and replaces it, while the named equivalent refuses.
`Presentation::apply_preset` and `set_window` both reach `replace_window`,
then `VoiTransform::new` and `LutChain::new`. `VoiParams::from_chain` must
continue to refuse sequence-driven chains. CPU callers read `chain()`.
Future SetWindow boundary commands must get the same result as Rust callers.

## What the specification does not cover

Keep the existing preset refusal of modality sequences. A manual window may
preserve a modality sequence, since rebuild already clones it unchanged.
Refuse only a VOI sequence for that manual operation. Do not unnecessarily
narrow the API by refusing a modality sequence which is retained.

## Approach

Move the VOI-sequence check into `replace_window`, before construction or
mutation. Leave the modality-only preset check in `apply_preset`. Reuse
`PresentationError::Sequence`. No signature, formula or tolerance changes.
A new policy abstraction or alternate-view API is unnecessary.

## Acceptance and tests

1. Named and manual windows on a VOI sequence return Sequence and preserve
   the entire Presentation, including colormap and evidence. Exercise all
   three manual VOI functions. Old source must fail semantically.
2. A manual window on a modality sequence preserves that modality and succeeds.
   A named preset retains its current modality-sequence refusal.
3. Ordinary windows, invert and presets retain existing tests. No stored
   samples or colour textures are uploaded by this repair.

Tests live in `crates/ocelli-viewport/tests/presentation.rs`. Sequence fixture
lookups cite PS3.3 C.11.2.1.1. Test 1 rejects a manual caller bypassing policy,
test 2 rejects an overly broad refusal. Capture individual RED/GREEN results.
Run viewport check, tests and clippy. Run final integrated sprint gates on the
actual final staged tree, with independent review still required.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: no new allocation
- unsafe: none
- Tier A (WebGPU): sequence remains unavailable to the existing uniform
- Tier B (WebGL2): same refusal
- Tier C (CPU): same selected-chain policy, no new raster

## Parity surface covered

None newly claimed. This repairs selection policy and does not prove parity.

## Deviations

None.

## LLD impact

Update `docs/lld/viewport.md` and its index contribution list. Preserve F-046's
historical plan, review and AS_BUILT entry. Add completion only after evidence.

## Open questions

None for this repair. Explicit alternative-view selection would be new work.
