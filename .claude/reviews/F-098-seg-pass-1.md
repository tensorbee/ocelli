# F-098 review, pass 1

**Reviewed**: uncommitted F-098 sources on `sprint/s35`. HEAD is `32d7534cb906e855129b7efe985111f4c839406a`. This pass staged nothing. `git diff --cached --name-only` was empty. The reviewed sources are `crates/ocelli-seg/src/seg_roundtrip.rs` (sha256 `8dc88e3aed9b232509105b9c01aeb8a721d7a8c760fa8fa82fe01cd330848679`), `crates/ocelli-seg/tests/seg_roundtrip.rs` (sha256 `228296e28cb07a834a515f5c61bdacf60906f6ba5dc739436002206f951c57e5`), and the F-098 lines in `crates/ocelli-seg/src/lib.rs` (sha256 `21550c8a1f52f692535423e060ddfa02f57e1928d8851096129082bfeac2950c`). Those lines are the crate-doc sentence, `mod seg_roundtrip`, and the `seg_roundtrip` re-export. The same `lib.rs` also contains unreviewed F-099 exports. Those lines are not an F-098 finding.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19eac-192a-79c0-8484-8bb6daeab146` and did not write the plan or the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. The round trip is logical frames. It is not a Part 10 DICOM file. The HLD does not specify the file bytes. That absence is not a defect. Segment 0 is legal. Decode must not sort.

AC1. Two frames, SOP `1.2.9` then SOP `1.2.8`, each 2 by 1, samples `[0, 1]` and `[2, 0]`, round trip in that order. The shipped expect is equality with that input, and equality of the caller with a clone taken before the call. A UID sort would put `1.2.8` first and fail that expect. The green run passed.

AC2. Two frames with SOP `1.2.9`, samples `[0, 1]` then `[4, 0]`, both remain in that order. The shipped expect is equality with that input. Dropping either frame would fail it. The green run passed.

AC3. A sample of `-1`, a non-integer sample `1.5`, a length of 1 against 2 rows by 1 column, a row count of 0, and an empty UID are errors. The shipped expects are `SegError::Sample`, `SegError::Sample`, `SegError::Length`, `SegError::Side`, and `SegError::Uid`. Setting `EXACT_INT_LIMIT` to `0.0` made `encode_seg` return the `1.5` frame. The expect at `crates/ocelli-seg/tests/seg_roundtrip.rs:53` received `Ok` and wanted `Err(Sample)`. The `-1` expect above that line still passed. The length, zero-row, and empty-UID expects were not reached in that failed run. They passed on the restored bytes.

AC4. Segment 0 is preserved. The AC1 samples include `0.0` on both frames. A refusal of 0 would panic in `must_encode` before the equality expect. The green run passed.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test seg_roundtrip` | Restored limit `9007199254740992.0`. Source sha256 `8dc88e3aed9b232509105b9c01aeb8a721d7a8c760fa8fa82fe01cd330848679` | executed now, before the mutation | Exit 0. 4 passed and 0 failed. |
| Same command after `EXACT_INT_LIMIT` became `0.0` | One constant in `crates/ocelli-seg/src/seg_roundtrip.rs`. Mutated sha256 `7fcc7402fda1ea08f92cfa8c03e388737e4c8e606cce0c5a83cc2feb7326f9c2`. The test file and `lib.rs` hashes did not change. | executed now | Exit 101. 3 passed and 1 failed. `a_bad_frame_refuses_the_whole_call` received `Ok` for samples `[1.5, 0.0]` and expected `Err(Sample)`. |
| `cmp` of the saved original against `crates/ocelli-seg/src/seg_roundtrip.rs`, then the same test | Bytes copied back. sha256 again `8dc88e3aed9b232509105b9c01aeb8a721d7a8c760fa8fa82fe01cd330848679` | executed now | `cmp` exit 0. Test exit 0. 4 passed and 0 failed. |

rustc 1.97.1. No wasm-pack, no second cargo, and no `bin/ocelli.sh gate`.

The restored comparison is:

```rust
const EXACT_INT_LIMIT: f64 = 9_007_199_254_740_992.0;
```

A separate Python transcription of `integer_sample` was compared with `float.is_integer` for finite values of at least 0, and with refusal for negatives and non-finite values. The sweep covered the named fixtures, integers through 20000, halves and tenths, powers of two and their neighbors, and 200000 random bit patterns. Mismatches were 0. That script ran the transcription. It did not run the crate.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`encode_seg` and `decode_seg` both call `copy_frames`. The first loop checks every frame. The second loop clones the UID and the sample `Vec` in caller order. There is no sort. The same UID can occur twice because nothing collapses frames. A refusal returns `Err` before the copy loop. The input is a shared slice, so the caller samples are not written. `one_bad_frame_does_not_keep_an_earlier_good_frame` compares the caller with a pre-call clone after an empty UID on the second frame. The AC1 test does the same after a successful round trip.

`integer_sample` refuses a non-finite value and a value below 0. `f64::trunc` is not used. The bit test is the local substitute. At and above `2^53` every finite value is an integer, and the early return accepts those. Below `2^52` the low fraction bits must be 0. A negative exponent accepts only 0. The Python transcription matched whole-number acceptance on the sweep above. The plan allows every integer of at least 0, including segment 0. The bit test did not reject a whole number in that sweep and did not accept a non-integer.

Empty UID is `SegError::Uid`. A zero row or a zero column is `SegError::Side`, and that check is before the product. The sample count must equal rows times columns or the error is `SegError::Length`. The product uses `checked_mul`. A negative sample, a non-integer, and a non-finite sample are `SegError::Sample`.

The module is `mod seg_roundtrip`, not a public module. `check_frame`, `copy_frames`, and `integer_sample` stay private. `SegFrame` and `SegError` are public because the two public functions name them. Re-exporting those two types is what makes the private module usable from the integration test. No TypeScript name was found for this round trip. `seg_roundtrip.rs` names `present_labelmap` only in a comment and does not call it. It has no `unsafe` and no `wasm-bindgen`. There is no `as` cast. Tiers A, B, and C are not branched. The plan marks each tier n/a.

`git status` for `crates/ocelli-seg` and `crates/ocelli-render/shaders` did not show `labelmap.rs` or a shader. `contour.rs` and `tests/rtstruct_import.rs` are other uncommitted work and were not reviewed. `packages/core/src/annotation-state.ts` was not opened.

## Limitations and handoff

The mutation is not left in place. `cmp` against the saved original was exit 0, and the sha256 is `8dc88e3aed9b232509105b9c01aeb8a721d7a8c760fa8fa82fe01cd330848679`.

The shipped AC3 test sets rows to 0 and does not set columns to 0. The source refuses a zero column with the same `Side` check. This pass does not count the missing column row. The shipped refusal calls use `encode_seg` only. `decode_seg` uses the same `copy_frames`. This pass does not count a separate decode refusal. `NaN` and the infinities are refused by `is_finite` and are not fixtures in the shipped test.

The plan's write list includes an LLD sentence. `docs/lld/README.md` has no segmentation row for this round trip, and `docs/lld/public-api.md` has no F-098 sentence. Those files were not reviewed and the absence is not counted. Clippy was not run. No feature profile was run. No browser and no pixel oracle were used. This story has no rendered parity view.

No patient data was read.
dwv, Horos, and the medical viewer Grok were not opened.
