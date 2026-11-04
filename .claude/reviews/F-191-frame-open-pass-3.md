# F-191 review, pass 3

**Reviewed**: uncommitted working tree on `sprint/s42` at `c6eec1f`, after `fill_padding` took `&LutChain`, before the 16-bit range fixture.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 2 smells, 3 nitpicks

## Acceptance challenged

Pass 2 S1 and S2. The reviewer checked `padding_black` against `LutChain::inverts` and the inverse fixtures against a flipped comparison, using the prior exit 101 receipt rather than a new run.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Read of `padding_black` | `frame_open.rs` and `lut.rs` | executed as a read | inversion comes from `chain.inverts` |
| Inverse fixture reasoning | flipped `==` to `!=` | prior receipt of exit 101, confirmed by reading | both inverse tests go red |
| rustfmt trailing comma | `tests/frame_open.rs` lines 94 and 132 | not executed | reviewer expected `cargo fmt --check` to fail |

## Defects

None.

## Smells

### S1, nothing tests that the fill uses the chain output range

**Where**: `crates/ocelli-dicom/tests/frame_open.rs` and `padding_black`
**What**: every padding fixture used a window whose range is 0 to 255. Replacing `output_range` with those literals would leave the suite green. A 16-bit VOI LUT range is 0 to 65535, so a fill of 255 is the wrong black end.
**Why it is wrong**: the range half of pass 2 S1 was closed in code and not pinned.
**Evidence**: read of `VoiTransform::output_range`. The reviewer suggested 12 bits. `LutDescriptor::new` accepts only 8 or 16, so the following fixture uses 16.
**Scope and regression**: present on the tree this pass reviewed.

### S2, two fill calls may not be rustfmt clean

**Where**: `crates/ocelli-dicom/tests/frame_open.rs` lines 94 and 132
**What**: those calls sit on one line with a trailing comma. The reviewer expected the fmt gate to reject them. This was not executed.
**Why it is wrong**: a fmt failure blocks the verify receipt.
**Evidence**: a reading of rustfmt's default trailing-comma rule, not a run.
**Scope and regression**: to be checked with `bin/ocelli.sh gate fmt` on the following tree.

## Nitpicks

### N1, photometric is still a separate argument

`LutChain` does not store photometric interpretation. A mismatched argument would pick the wrong end. Current callers pass the same value the chain was built from. Colour cannot build a chain, so the colour arm is only reached by a mismatch.

### N2, an intercept without a slope is not tested

The slope-only refusal is tested. The intercept-only tag is not.

### N3, earlier nitpicks stay

One-sided rescale beside a modality LUT, the small SS descriptor ends, and a multi-valued creator still match pass 2.

## Verified clean

Pass 2 S1's second inversion rule is gone. `padding_black` reads `chain.inverts` and `chain.voi().output_range`. The monochrome matrix matches `PresentationTransform::new`. Both explicit INVERSE tests expect the end that a flipped comparison would miss. Absent rescale is the identity. One-sided rescale names the missing tag. VOI descriptor signedness still follows C.11.2.1.1. `creator_for` resolves the private element. The helper uses the new signature. The LLD states the chain rule.

## Limitations and handoff

Read only. No fmt command and no mutation were run in this pass. The 12-bit range in the review does not exist in `LutDescriptor`. Remediation of S1 is a later tree. S2 needs an executed fmt check. Do not commit and do not start F-192 until a pass reports 0 defects and 0 smells.
