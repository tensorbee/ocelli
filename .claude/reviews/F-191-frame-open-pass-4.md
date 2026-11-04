# F-191 review, pass 4

**Reviewed**: uncommitted working tree on `sprint/s42` at `c6eec1f`, after the 16-bit VOI LUT padding fixture. The production fill still reads `chain.voi().output_range()`.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

Pass 3 S1, the chain output range, and pass 3 S2, the fmt reading. The 16-bit fixture was checked by hand against `LutDescriptor::new`, `lookup_index` and the inverse reflection.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `a_16_bit_voi_lut_padding_fill_uses_the_chain_range` | 16-bit VOI LUT, entry 100, INVERSE, stored 50 | prior receipt after the revert | passed. Pre-fill 65435. Fill 65535 |
| Literal `DISPLAY_MIN` and `DISPLAY_MAX` in `padding_black` | same fixture | prior receipt, mutation reverted | exit 101 at `tests/frame_open.rs:437`. Left bits 1132396544. Right bits 1199570688 |
| `bin/ocelli.sh test ocelli-dicom --test frame_open` | full suite after the revert | prior receipt | 23 passed |
| `rustfmt --edition 2024 --check` on the three F-191 Rust files | frame open, its test, `reply.rs` | prior receipt | exit 0 |
| `bin/ocelli.sh gate fmt` | workspace | prior receipt | failed only in committed `crates/ocelli-codec/tests/jpegls.rs` |

## Defects

None.

## Smells

None. Pass 3 S1 is closed by the 16-bit fixture. A fill of 255 fails it. Pass 3 S2 is closed by the rustfmt receipt. The trailing commas inside the macro match this rustfmt. The `jpegls.rs` fmt failure is a committed file this story does not modify.

## Nitpicks

### N1, trailing commas differ between fill calls

Lines 94 and 132 keep a trailing comma on one line. Later single-line calls do not. rustfmt accepts both.

### N2, a 12-bit VOI LUT is refused by the existing descriptor

`LutDescriptor::new` accepts 8 or 16 bits. The fixture comment says 12 is not a range this chain has. That limit is in `ocelli-pixel` and this story did not change it. A conformant 12-bit VOI LUT, if the standard allows one, is a later pixel-crate question.

Pass 3's separate-photometric argument, the untested intercept-only refusal, and the earlier descriptor nits are unchanged.

## Verified clean

The 16-bit numbers match the chain. Count 1, first input 0, 16 bits, largest value 65535. Stored 50 with slope 1 and intercept 0 looks up entry 100. Explicit INVERSE reflects that to 65435. MONOCHROME2 with inversion fills `ymax`, which is 65535. The mutation receipt is the bits of 255 against the bits of 65535. Window fixtures still use 0 to 255. An oversized bits word is an attribute error rather than a truncated `u8`. Inversion is still only `chain.inverts`.

## Limitations and handoff

Read only. The mutation, the 23 passed count and the rustfmt exit were receipts, not commands run in this pass. The reviewer did not reread the whole `frame_open.rs` diff. `gate fmt` stays red on the committed JPEG-LS test until that file is formatted or the baseline is otherwise accepted. This pass reports 0 defects and 0 smells, which is the microscope stop for F-191.
