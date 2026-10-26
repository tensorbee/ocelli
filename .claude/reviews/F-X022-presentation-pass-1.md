# F-X022 presentation review, pass 1

**Reviewed**: staged tree `74de1e694d6c55bce512b3913bcfa7a74765c9fc`
**Base**: preparation tree `d013d266b4cb45e5e614ef46b3059d16b6b47971`
**Author / reviewer**: xAI model per operator handoff / Codex
**Result**: 0 defects, 0 smells, 0 nitpicks in the scoped repair
**Date**: 2026-10-18

## Acceptance challenged

AC1 requires selected VOI-sequence refusal through both window routes and
preservation of the entire presentation. AC2 permits a manual window to
retain a modality sequence while named presets still refuse it. AC3 retains
ordinary window, invert and colormap behavior.

The independent composition case first changes explicit inversion and the
colormap, then attempts all three manual functions with valid, zero and NaN
widths. Sequence refusal must occur before window construction and leave the
modified presentation unchanged. A named Bone preset is refused too. This
rejects a guard confined to the original initial state or to valid windows.
The contract is the approved F-X022 selected-chain policy, not a claim that
DICOM forbids alternative views.

## Evidence

All commands ran natively on the current macOS host in dev test configuration.
The implementation and committed test paths were byte-compared with the
supplied tree. The index also matches that tree. Unstaged instruction changes
are excluded from this repair approval.

| Command / selection | Executed now / prior | Exit and observation |
|---------------------|----------------------|----------------------|
| `bin/ocelli.sh test ocelli-viewport` | now, supplied implementation | 0, 33 tests passed, 0 ignored, plus 0 doc tests |
| `bin/ocelli.sh test ocelli-viewport --test presentation independent_refusal_after_invert_preserves_state_even_for_invalid_width -- --exact` | now, isolated export plus independent test | 0, 1 passed, 9 filtered |
| Same independent selection with the shared VOI guard removed | now, isolated mutation | 101, semantic assertion saw `Ok(())` instead of `Err(Sequence)` |
| Same selection after restoration | now, isolated restored source | 0, 1 passed, 9 filtered |
| Integrated sprint gate and old-source RED | implementer receipt only | reported in ignored audit-verification JSON, not rerun as part of this pass |

The isolated export is `/private/tmp/ocelli-independent-6kkmmfma`. Its
`tests/presentation.rs` contains the independent case. Individual logs are
`/private/tmp/ocelli-independent-presentation-green.log`,
`/private/tmp/ocelli-independent-presentation-mutation.log` and
`/private/tmp/ocelli-independent-presentation-restored.log`. The source was
restored. No mutation was applied to the canonical checkout.

## Defects

None found in this scoped repair.

## Smells

None found in this scoped repair.

## Nitpicks

None recorded.

## Verified clean

Reviewed both public window routes, shared replacement, atomic rebuild,
inversion, retained modality, and complete presentation equality assertions.
The new refusal precedes construction and mutation. Manual modality handling
still clones its provider. No formula, tolerance, public signature, shader or
upload behavior changes. LLD wording matches the implemented distinction.

Inspected the unchanged `VoiParams::from_chain` consumer. Its sequence
expressibility refusals remain present. There is no new browser SetWindow
consumer to claim exercised. This pass does not assert rendered parity.

## Limitations and handoff

The two repair tests and independent composition case support acceptance of
this repair. This is not a whole-sprint review or a fresh sprint verification.
GPU/browser paths were not executed in this pass. New instruction refinements
and this report require inclusion in the later reviewed and verified union.
No story status, receipt or commit was written by this review.
