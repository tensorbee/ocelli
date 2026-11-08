# F-243 review, pass 2

**Reviewed**: `work/f-243-xai`, range `6e8784c5..2a9b62a4`, worktree `ocelli-wt-f-243`, clean against `2a9b62a4` when the pass began (`git diff --quiet 2a9b62a4`). Untracked `.venv` and `tools/oracle/out` only. The original tree `aba1cb09`, the pass 1 base `6e8784c5` and the tip `2a9b62a4` were each extracted with `git archive` into scratch, each with its own `CARGO_TARGET_DIR`. Mutations ran in a fourth extracted copy with two further target directories.
**Author / reviewer**: written by a Claude implementer under the lead's rulings. Reviewed by an independent Claude reviewer that wrote none of the diff, the plan, the rulings or the pass 1 review. The rulings were taken as given except where a measurement below contradicts the premise one rests on.
**Result**: 5 defects, 0 smells, 6 nitpicks

## Acceptance challenged

- **AC1 to AC3, AC6, AC17, AC20, AC21 on undamaged streams.** My own generator, independent of `generate_jpegls.py`, encoded 6,160 monochrome images with CharLS 2.4.4 through imagecodecs 2026.10.10: P 8, 12 and 16 at NEAR 0, 2 and 3 (1,980 streams), then P 2, 5, 8, 10, 12, 14 and 16 at NEAR 1, 3, 4, 5, 7, 9, 20, 34, 85, 127 and 255 (4,180 streams). Sixteen contents (zero, maximum, flat, two bands, vertical and horizontal stripes, blocks, checkerboard, two ramps, a diagonal ramp, uniform noise, low-amplitude noise, sparse spikes, smooth texture and a CT-like body with fat, bone ring and noise) over twelve small shapes from 1 by 1 to 100 by 37, including 1 by 2, 2 by 1, 13 by 1, 2 by 31 and 5 by 129, and seven contents over 64 by 64, 256 by 256, 301 by 300 and 512 by 512. Plus 420 pyjpegls 1.5.1 streams and 144 streams made in scratch from 24 real corpus CT and MR slices at P 12 and 16, NEAR 0, 2 and 3. For lossless streams CharLS's decode was asserted equal to the image and for near-lossless inside NEAR, before it was used as the reference. **Tip: 6,724 of 6,724 exact against CharLS**, release and debug builds, no refusal. The `6e8784c5` base on the first set: 27 wrong and 156 refused, and all 144 real-slice streams refused, so the sweep discriminates.
- **AC5, AC21 on damaged streams.** 483,889 damaged streams built from 1,053 of my CharLS streams: the codestream cut at each of its last 8 byte positions, 1 to 8 scan bytes dropped with EOI kept, every single-bit flip across the whole scan (sampled to 1,200 on longer scans), EOI, DNL, RST0, LSE, SOS, SOI, `FF FF`, `FF 00`, `00`, `00 00` and `80` inserted at the start, a third, two thirds, before the last byte and before EOI, bytes after EOI, every LSE byte edited and nine LSE segments inserted. Each compared with CharLS's verdict and samples, and with the stream's own undamaged samples. D1 and D2.
- **The 21 named divergences and the 61 accepted refusals.** Re-derived from the tracked TSVs with my harness and a fresh CharLS run: 13 are `ins:00` that CharLS refuses and Ocelli opens to the stream's own samples, 8 are `drop:1` that CharLS opens to its own samples and Ocelli refuses, 54 flips and 7 drops CharLS opens to changed samples and Ocelli refuses. Exactly as recorded. CharLS opens `ins:00` on 136 of the 150 streams, as the test comment says.
- **The three further vendored rules** against T.87 as I hold it and against CharLS, by fuzzing 105,000 random scans over seven tiny shapes at six precision and NEAR pairs and by constructing the limited-length code by hand. D3.
- **AC9.** Every refusal of the 483,889 left the caller's `0xa5` buffer untouched.
- **AC14.** Not met. See limitations.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-codec -p ocelli-dicom` | worktree at `2a9b62a4` | now | 0, `tests/jpegls.rs` 31 passed, no failure anywhere |
| `cargo test -p ocelli-codec -p ocelli-dicom -- --ignored` | worktree, `corpus/data` symlinked to the canonical corpus | now | 0, the three `ocelli-dicom` corpus tests pass |
| vendored `cargo test --lib` | scratch `aba1cb09` and `2a9b62a4` | now | 0, 320 passed on each |
| `bin/ocelli.sh gate pins` | worktree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate wasm` | worktree | now | 0, wasm 2,202,907 bytes, decode wasm 16,455, both equal to their recorded figures |
| `unittest scripts.tests.test_pin_and_size_check` | worktree, venv interpreter | now | OK, 18 tests, including the planted byte in `tests/conformance.rs` |
| SHA-256 of the six patched files and `PATCH-PROVENANCE.md` | worktree | now | all equal `RITK_PATCHED_HASHES`, `RITK_PROVENANCE_SHA256` and the provenance text |
| `generate_jpegls.py` check mode | scratch tip, corpus symlinked | now | 0, all 37 artefacts `ok`, both TSVs reproduced byte for byte |
| corpus references against the images | the generator's own functions | now | lossless CharLS digest equals the image on 72 of 72, near streams inside NEAR on 72 of 72 |
| `deviation_check.py`, `prose_check.py`, `source_provenance_check.py`, `unsafe_allowlist_check.py` | worktree | now | all OK, `ritk-codecs-0.6.0` unsafe count 0 |
| `git diff aba1cb09..2a9b62a4 -- vendor crates`, added `unsafe` | range | now | none, only `src/jpeg_ls/*` touched in the vendored source |
| published test edit | `conformance.rs` against `aba1cb09` | now | only `0xF8` to `0xF0` and a comment. CharLS refuses `F8`, decodes `F0` to four zeros, and writes `F0` itself for a 2 by 2 zero frame |
| undamaged sweeps, 6,724 streams | my generators, imagecodecs and pyjpegls | now | tip all exact. Base `6e8784c5`: 27 wrong, 156 refused on 1,980, 144 of 144 real-slice streams refused |
| damaged sweep, 483,889 cases | 1,053 of my CharLS streams | now | **71 `Ok` with samples neither CharLS's nor the stream's own**, 1,965 refusals CharLS opens to the stream's own samples outside the LSE gap, no refusal touched `out` |
| the same 483,889 through the debug build | tip | now | 0, no panic, every case answered |
| fuzz, 105,000 random scans plus constructed codes | 1 by 1 to 4 by 4, P 2, 5, 8, 12, 16 | now | no case where both open with different samples, 0 refused streams that re-encode byte for byte to themselves under CharLS |
| `decode_jpegls` release runner, four rows | scratch tip | now | medians 0.1134, 0.1119, 0.0103 and 0.0104 ms |
| mutation catalogue, 24 mutants | scratch copy, `target-mut` and `target-mutv` | now | table under Verified clean, each file restored and compared with `diff -r` |

## Defects

### D1, a damaged scan one byte shorter than its data opens to wrong samples, through the 15-bit allowance

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs:318`, `if end.unread_bits > 15`, in `verify_scan_end`.
**What**: 71 of my 483,889 damaged streams return `Ok` from `JpegLsDecoder::decode` with samples that differ from the stream's own undamaged decode, while CharLS refuses every one with "the decoding process is ready but the source buffer still contains encoded data". 56 are bit flips and 15 are `0x80` inserted before the last scan byte, over 18 base streams at P 8, 12 and 16 and NEAR 0, 2 and 3. In all 71 the damaged stream is byte for byte CharLS's own encoding of the samples Ocelli returned, followed by one `0x00` byte before EOI: the damage turned the scan into a valid scan of another image that ends one byte earlier, and the original final zero byte is left over. That leftover is at most 15 zero bits, which the allowance accepts. Smallest case, P 8, 3 by 3, NEAR 0: the undamaged stream `ffd8fff7000b080003000301011100ffda00080101000000007ed00000076bcb00ffd9` decodes to `2 4 1 2 114 113 2 115 114` in both decoders. With `80` inserted before its last scan byte, `...076bcb8000ffd9`, CharLS refuses and Ocelli returns `Ok` with the last sample 113. A p16 NEAR 2 3 by 3 ramp with its first scan bit flipped returns `Ok` with 8 of 9 samples wrong, `0 62335 40065 320 32385 32015 4160 32364 34580` where the image is `0 32765 65535 0 32765 65535 0 32765 65535`.
**Why it is wrong**: the lead's ruling D3 and the plan's invariant, "a returned Ok means every sample was decoded from bits the encoder wrote", and AC21's "never opens a stream to samples that are neither CharLS's nor the stream's own". cornerstone3D 5.8.2 shows no image for these 71 and Ocelli shows a wrong one, which is the quietly wrong pixel. The generated corpus cannot see it, because it only flips the last three scan bytes and inserts after the final byte, where this shape does not arise.
**Evidence**: the damaged sweep and `analyse2.py` in scratch, which re-encoded each of Ocelli's 71 outputs with CharLS and found the damaged stream equal to that encoding plus `00`. A build with the allowance at 7 bits refuses all 71, still decodes all 6,724 undamaged streams of this pass exactly, and on the generated corpus moves the 136 `ins:00` cases CharLS opens to "CharLS opens to its own samples, Ocelli refuses" while the 13 named `ins:00` divergences become agreement.
**Scope and regression**: any damaged or mis-sliced `.80` or `.81` frame whose damage leaves a valid shorter scan, measured here only on frames up to 7 by 17 (the 64 by 64 flips were sampled). Present on the tip. Consumers are the folder viewer preview and value endpoints and the oracle producer. Smallest regression: the 3 by 3 stream above, asserted refused. The fix is the lead's, because the measured trade is between this and the ruling's same-verdict rule on `ins:00`, which CharLS opens on most streams.

### D2, unnamed divergences in the direction the ruling forbids, on damaged streams CharLS opens to their own samples

**Where**: `bitstream.rs:172` (EOI only after the first marker prefix), `bitstream.rs:128` (the limited-length prefix), `scan.rs:227` and `:269` (the coded range), and `tests/jpegls.rs:1232` `OPEN_VERDICT_DIVERGENCES`, which names 21.
**What**: excluding the operator's LSE gap and the adapter's existing even-length padding rule, 1,965 of my damaged streams are opened by CharLS to exactly the stream's own samples and refused by Ocelli. 1,053 of 1,053 are `FF FF` inserted before EOI, refused as "not followed by the EOI marker". 683 are a zero byte or two inserted inside the scan, refused as run out, and 64 more refused as an error outside the coded range. 22 are bit flips refused as an error outside the coded range. 126 are the final byte dropped or an EOI inserted before it, the mechanism of the 8 named `drop:1` cases. No Ocelli pixel is wrong in any of them.
**Why it is wrong**: the lead's ruling D3 says "A stream CharLS opens and Ocelli refuses is a defect unless CharLS's samples differ from the encoded image, and any such exception is listed by name." These are neither listed nor of the excepted kind. `FF FF` before a marker is fill, which ITU-T T.81 B.1.1.2 permits. Whether T.87 carries that permission over was not checked against its text, so that sub-case is a plausible conformance gap, labelled unconfirmed.
**Evidence**: the damaged sweep grouped by mutation and refusal text. With the prefix rule reverted, 33 of the 683 inserted-zero cases open.
**Scope and regression**: refusals only, the safe side, which is why this is for the lead to name under D14 or to change. If fill bytes are conforming, a stream ending `FF FF FF D9` is a conforming stream CharLS decodes correctly and Ocelli refuses, the one outcome priority 3 of this pass rules out.

### D3, three sentences about CharLS's behaviour are false as measured

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/bitstream.rs:126` to `:127`, "CharLS 2.4.4 refuses a 0 here, which only a damaged scan carries". `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md:50` to `:51`, "which CharLS also requires". `docs/lld/codecs.md:124`, "CharLS refuses both". `PATCH-PROVENANCE.md:43` to `:45`, "`verify_scan_end` accepts what CharLS 2.4.4 accepts on the generated agreement corpus".
**What**: a hand-built P 8, 1 by 1 frame whose run interruption carries the limited-length code with 1, 2, 5, 10, 30 or 60 extra zeros before its terminating 1 is opened by CharLS every time, to the sample the unextended code gives (1, 51 and 101 for remainders 0, 100 and 200), so CharLS does not refuse a 0 after the prefix, it keeps counting zeros. Ocelli refuses all of them. The constructed escapes with remainders 254 and 255 at the same position, whose errors fall outside the coded range of RANGE 256, are opened by CharLS and refused by Ocelli, and on the random scans CharLS opened 563 that Ocelli refused for an error outside the coded range. So CharLS refuses neither. On the generated corpus `verify_scan_end` refuses 8 cases CharLS accepts and accepts 13 CharLS refuses, which the same file's test names.
**Why it is wrong**: a claim in prose that is false, `.claude/commands/microscope.md` section 3. The refusals themselves follow T.87 A.5.3 and A.4.5 as I hold them, because no encoder writes more than `LIMIT - qbpp - 1` zeros or an error outside the reduced range, and none of the 563 nor the over-long codes re-encodes to itself under CharLS. Only the sentences are wrong.
**Evidence**: `p3.py` and `p3b.py` in scratch.
**Scope and regression**: prose in three files. Each sentence should say that Ocelli refuses these per T.87 where CharLS opens them, or be deleted. The fourth should say what the generated corpus measures, with the 21 named.

### D4, three counts in the LLD, the test and the plan do not reproduce

**Where**: `docs/lld/codecs.md:116` and `crates/ocelli-codec/tests/jpegls.rs:1350` to `:1351`, "3,679 samples wrong" and "NEAR 2 3,855, both refused". `docs/lld/codecs.md:133` and the plan's AC21, "41 mutations". `docs/lld/codecs.md:132`, "150 CharLS streams", and `examples/folder-viewer/src/use.html:631`, "on every undamaged stream of a generated corpus of 150".
**What**: the CT-like fixtures decode through the public adapter on the original tree `aba1cb09` to `Ok` with **1,596** of 4,096 samples wrong and, at NEAR 2, **2,035** wrong against CharLS's decode. The `6e8784c5` base with its refusals disabled gives the same two numbers, and the tip with only the RESET line reverted refuses both, at lines 39 and 41. I found no tree that gives 3,679 or 3,855. `MUTATIONS` in `generate_jpegls.py` has 40 entries including `none`, so 39 mutations. Of the 150 streams, 2 are hand-built rather than written by CharLS (`jpegls_run_partial_hit_u8` and `jpegls_final_ff_one_padded_u8`), and the second is refused by both decoders, so it does not decode to CharLS's samples.
**Why it is wrong**: a number in prose that the tree does not produce, section 3, and HLD 27.2 R2 for the test comment.
**Evidence**: `ctcount.rs` in three scratch trees and `len(g.MUTATIONS)`.
**Scope and regression**: prose and one test comment. Re-measure and state the tree, or delete the counts.

### D5, README, D-20 and the LLD say damaged scans are refused, and the tip opens some to wrong samples

**Where**: `README.md:188` to `:190`, "so is a JPEG-LS scan that is truncated, damaged or carries more than its padding after the last sample". `docs/hld/DEVIATIONS.md:37` D-20, "a damaged or truncated scan refused". `docs/lld/codecs.md:142` to `:143`, "Ocelli never opens a stream to samples that are neither CharLS's nor the stream's own", stated without the corpus it was measured on.
**What**: D1's 71 are damaged and open to wrong samples. One whole `0x00` byte after the padding is more than padding and opens on most streams, 149 of the 150 generated ones, 13 of them where CharLS refuses. 71,849 of my flipped streams open, to CharLS's samples.
**Why it is wrong**: a false sentence in README is a defect under section 3, and D-20 is the register.
**Scope and regression**: prose. Each sentence should state what is measured, which streams are refused and on which corpus, and the LLD's invariant becomes true only if D1 is fixed.

## Smells

None.

## Nitpicks

- **N1, carried from pass 1 N4.** `scan.rs:311` to `:313`, the exhaustion check in `verify_scan_end`, cannot fire, because `decode_scan` checks after every line including the last and `verify_scan_end` runs only after it succeeds. Mutant X1 removing it leaves both suites green.
- **N2.** The limited-length prefix refusal reuses the `exhausted` flag, so it reports "ran out of entropy-coded data" for a scan that did not run out. The adapter maps both to `DecoderFailure`, so nothing a caller sees changes.
- **N3.** `generate_jpegls.py` records CharLS's undamaged digest as the reference without asserting it equals the encoded image for NEAR 0 or lies inside NEAR otherwise. Both hold on all 144 synthetic streams, measured.
- **N4.** The refusal table row "Bytes after a complete image, `TrailingData`" in `docs/lld/codecs.md` does not cover `EOI junk EOI`, which the adapter accepts and the tip now decodes, as CharLS does. Pre-existing in the adapter. `6e8784c5` refused it and the tip does not.
- **N5, carried from pass 1 N2.** Plan item 8 still says imagecodecs cannot write P 12. It wrote P 12 for every 12-bit stream of this pass.
- **N6.** The `docs/lld/benchmarks.md` Tip column names no commit. Re-measured on `2a9b62a4` it is the same order, 0.1134, 0.1119, 0.0103 and 0.0104 ms.

## Verified clean

- **Pass 1 findings.** D1 closed: the floor at RESET is in `context/mod.rs:171`, every undamaged stream of this pass agrees with CharLS, and mutant R1 fails the RESET and corpus tests. D2's non-zero padding now refused, mutants Z1 to Z4 each red, though D1 above is the adjacent case it left. D3's prose rewritten, with new claims checked under D3 and D4. D4 closed: M4e is gone by design, the stuffed byte cannot change the verdict because an `0xFF` data byte is non-zero, M4f is killed as K2, and M7e and M7f are killed as P1 and P2. D5 closed by deleting the count. N1 closed by the prefix rule. N3 closed.
- **Mutation catalogue on the tip.** Red: R1 reset divides (2 tests), G1 prefix 0 accepted, E1 and E2 range checks removed, E3 upper bound inclusive, E4 lower bound one wider, E5 upper bound one narrower (2 plus 2 vendored round trips), Z1 non-zero padding accepted, Z2 buffered bits unchecked, Z3 unbuffered bytes unchecked, Z4 mask one bit short, L1 to L4 allowance 16, 14, 7 and 23, K1 EOI not required, K2 EOI required at the end of the data, X2 both exhaustion checks removed, U1 RUNindex raised on the partial hit (9 plus 2 vendored), U2 miss clamped (2), C1 saturating CLAMP, P1 T3 above MAXVAL, P2 T1 equal to NEAR. Survives: X1 only, N1.
- **The coded range, T.87 A.4.5 as I hold it.** Reduction modulo RANGE gives `[-floor(RANGE / 2), ceil(RANGE / 2) - 1]`, which `in_coded_range` states, checked on the error before the sign is applied in regular mode and before `signum(rb - ra)` at a run interruption. The error-correction flip `x` to `-(x + 1)` maps the interval to itself because NEAR 0 with the default MAXVAL makes RANGE `2^P`, which is even.
- **The limited-length code, A.5.3 as I hold it.** `LIMIT - qbpp - 1` zeros, a 1, then `MErrval - 1` in `qbpp` bits. My constructed codes with exactly that many zeros open in both decoders to the same samples for 254 of the 256 remainders. The other two are the out-of-range errors under D3, and by A.7.2.1 as I hold it the encoder writes at most EMErrval 254 there (k 2, map 1 for an error of -128), so neither is writable.
- **The first EOI.** `segment_end` stops at the first marker prefix and `self.data[offset + 1]` is in bounds because a prefix is recorded only when a next byte exists. `EOI EOI`, `EOI junk EOI` and a padded fragment decode to CharLS's samples.
- **`segment_end` arithmetic.** `bits` is at most 31 after `refill`, so `1u64 << self.bits` cannot overflow, confirmed by 483,889 debug-build decodes with no panic.
- **The published test edit, the pins and the provenance.** Every digest, the planted-byte test, the six files named, 320 of 320 on both trees, no `unsafe`, no JPEG 2000 file touched, which keeps D-31's clause true.
- **The generator and both TSVs** reproduce in check mode, CharLS and pyjpegls versions pinned, and their expected values come from CharLS, never from Ocelli.
- **use.html's** other sentences and D-20's file count, test edit and pins sentence, executed. No CharLS, dwv, Horos or Grok source was opened. CharLS was only run through imagecodecs and pyjpegls.

## Limitations and handoff

- **AC14 is not met.** No published T.87 text was obtained for this pass either. Every clause reading above is my own knowledge of the standard, corroborated or contradicted by CharLS's behaviour, and the fill-byte question in D2 is open on it.
- No oracle run, no GPU, no browser, so AC18 stays open, and `bin/ocelli.sh bench --compare` was not run, so AC13's comparison stays owed.
- The 64 by 64 and larger streams had their flips sampled, so D1's rate on realistic frames is not measured. The real-slice streams were made in scratch and never written to the repository.
- The adapter's even-length rule refused 519 `stream + 00` cases CharLS opens. It is outside this story and is recorded here only so a later pass does not count it as missed.
- D1 and D2 go to the lead, because D1's measured fix conflicts with the same-verdict rule of ruling D3 on `ins:00`. D3, D4 and D5 are for the implementer. A fresh independent pass reviews the remediation.
- The scratch trees, target directories and streams of this pass were deleted with Python `shutil.rmtree` at the end of the pass.
