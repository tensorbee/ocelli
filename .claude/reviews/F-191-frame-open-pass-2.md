# F-191 review, pass 2

**Reviewed**: uncommitted working tree on `sprint/s42` at `c6eec1f`, after the rescale, descriptor, creator and post-chain fill remediation, and before `fill_padding` took `&LutChain`.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell. The host did not treat this as a self-review.
**Result**: 0 defects, 2 smells, 5 nitpicks

## Acceptance challenged

AC5 padding fill, the presentation matrix, and the rescale and descriptor repairs from pass 1. The reviewer checked the current functions against PS3.3 C.11.1, C.11.2.1.1 and C.11.6 from memory of the standard.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-dicom --test frame_open` | remediated tree before this pass | prior receipt, not re-run by the reviewer | 20 passed |
| `bin/ocelli.sh test ocelli-example-server` | same tree | prior receipt | 5 passed |
| Read of `padding_black` against `LutChain::inverts` | `frame_open.rs` and `lut.rs` | executed as a read | the two inversion rules agreed, and they were separate copies |

## Defects

None.

## Smells

### S1, the inversion decision is made twice

**Where**: `crates/ocelli-dicom/src/frame_open.rs` `padding_black`
**What**: `padding_black` recomputed inversion from photometric interpretation and presentation evidence. `PresentationTransform::new` already makes that decision, and `LutChain::inverts` exposes it. The two copies agreed on every input that can reach them. A later change to the chain rule would fill the wrong end while these tests stayed green. Callers also passed `ymin` and `ymax` as literals `0.0` and `255.0`.
**Why it is wrong**: inversion is resolved once, in the chain. A second copy is the defect class CLAUDE.md names for the LUT chain.
**Evidence**: read-only comparison of `padding_black` with `lut.rs` `PresentationTransform::new`.
**Scope and regression**: present on the tree this pass reviewed. The following edit makes `fill_padding` read `chain.inverts()` and `chain.voi().output_range()`.

### S2, two cells of the padding matrix are not pinned

**Where**: `crates/ocelli-dicom/tests/frame_open.rs`
**What**: fixtures covered absent shape on both photometric interpretations and explicit IDENTITY on MONOCHROME1. Explicit INVERSE on either interpretation was untested. Changing the inverse arm to false would leave the suite green.
**Why it is wrong**: a green suite that cannot see the wrong black end will be counted as coverage.
**Evidence**: the reviewer listed the three existing tests and the missing INVERSE cells. No mutation was run.
**Scope and regression**: present on the tree this pass reviewed. The following edit adds the two INVERSE fixtures.

## Nitpicks

### N1, the pixel-data skip is not a blocking smell

Dropping only the tag skip copies bytes into a `MetadataSet` that `open_frame` drops. Changing only the pixel-sequence arm is still caught by the tag skip. A later failure needs both changes, and no fixture carries pixel data. Coverage nitpick.

### N2, the approved plan still says MONOCHROME1 black is ymax

`.claude/plans/F-191-design.md` was not rewritten. `docs/lld/frame-open.md` already stated the post-chain rule.

### N3, one-sided rescale beside a modality LUT is accepted

`rescale_or_identity` passes a half pair through when a modality LUT is present. The LUT still wins. Only slope-without-intercept has a refusal test.

### N4, the SS descriptor fixture uses small positive first and third values

`[1, -2, 8]` does not show that the first and third values stay unsigned. An entry count of `0x8000` or above would.

### N5, a multi-valued text creator still refuses the frame

`one_code` returns an attribute error for more than one creator value. That refuses the object over malformed private data. Posted `padding_count` still counts ranges, while the synthetic body counts samples.

## Verified clean

Absent rescale with no modality LUT is slope 1 and intercept 0. One side alone names the missing attribute. A modality LUT still wins. A modality LUT forces the VOI descriptor's second value unsigned. Intercept `-1024` with unsigned pixels reads `0xFC00` as `-1024`. An SS descriptor opens, and only the second value is signed. `fill_padding` writes one constant and is not a window formula. The hand-checked matrix matches the chain for absent, IDENTITY and INVERSE on both monochrome interpretations. `creator_for(0x0009, 0x1001)` returns the creator. A non-text creator does not refuse the frame. The helper builds the chain from `output_range` and keeps posted pixels off the wire.

## Limitations and handoff

Read only. No mutation was executed in this pass. The 20 passed and 5 passed counts are earlier receipts. Implicit VR creator VR mapping was not opened. Remediation of S1 and S2 is a later tree and needs its own pass. Do not commit and do not start F-192 until a pass reports 0 defects and 0 smells.
