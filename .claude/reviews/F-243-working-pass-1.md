# F-243 review, pass 1

**Reviewed**: `work/f-243-xai`, range `aba1cb09..e2619832`, worktree `ocelli-wt-f-243`, clean against `e2619832` when the pass began (`git diff --quiet e2619832`). Untracked `.venv` and `tools/oracle/out` only. Base and tip were each extracted with `git archive` into a scratch copy with its own `CARGO_TARGET_DIR`.
**Author / reviewer**: written by a Claude implementer. Reviewed by an independent Claude reviewer that wrote none of the diff, the plan or the investigation. The lead's rulings in the plan were taken as given except where a measurement below contradicts the premise a ruling rests on.
**Result**: 5 defects, 0 smells, 4 nitpicks

## Acceptance challenged

- **AC1, AC2, AC3, AC17, AC20, run mode.** Derived the A.7.1 rule independently (below) and decoded 648 CharLS 2.4.4 streams of my own, 8, 12 and 16 bits, NEAR 0 and 3, twelve sizes from 1 by 1 to 64 by 129 including 8 by 4, 1 by 13 and widths that are not a multiple of any `2^J`, over flat, two-band, vertical bands, horizontal bands, 4 by 4 blocks, a disc mask, sparse, ramp and uniform random content. Then 81 smooth and noisy frames of 256 by 256, 512 by 512 and 100 by 333 at NEAR 0, 1 and 3, and 10 frames made from five real CT slices of the corpus at NEAR 0 and 2. The run-mode hunk is right on every one of them. The tip still decodes most realistic frames wrongly or refuses them, for a reason outside the hunks, D1.
- **AC5, end of scan.** Built 5,209 streams from 150 CharLS streams: the final 1, 2, 3 or 5 scan bytes dropped, seven insertions before EOI, every single-bit flip in the last three scan bytes that keeps the stuffing legal, and six cuts through the scan. Compared CharLS's verdict, Ocelli's verdict and Ocelli's samples. D2 and D3.
- **AC6, CLAMP.** Derived C.2.4.1.1.1 independently and decoded 168 8-bit CharLS streams at NEAR 33, 34, 35, 49, 50, 51, 60, 84, 85, 86, 100, 126 and 127, which cross every arm of CLAMP (T3 falling to T2 from NEAR 34, T2 falling to T1 from NEAR 50, T1 falling to NEAR + 1 from NEAR 85). Tip exact on all 168, base wrong on 151. CharLS 2.4.4 refuses NEAR above 255, so 12 and 16-bit streams cannot reach the CLAMP arm through it.
- **AC8, presets.** Mutated each branch of `check_preset`. D4.
- **AC4, AC7, AC9, AC10, AC11, AC12, AC15, AC16.** Mutation, plant and gate runs in the evidence table.
- **AC14, the standard.** Not met by this pass. See limitations.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-codec -p ocelli-dicom` | worktree at `e2619832` | now | 0, no failures, `tests/jpegls.rs` 30 passed |
| `cargo test -p ocelli-codec -p ocelli-dicom -- --ignored` | worktree, `corpus/data` symlinked to the canonical corpus | now | 0, the three ignored `ocelli-dicom` corpus tests pass |
| vendored `cargo test --lib` | `git archive` of base and of tip | now | 0, 320 passed on base and 320 on tip, no published test edited |
| `bin/ocelli.sh gate pins` | worktree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate wasm` | worktree | now | 0, wasm 2,202,907 bytes, the figure `docs/lld/build-targets.md` already records, decode wasm 16,455, equal to baseline |
| `python3 scripts/pin_and_size_check.py --with-size` | worktree | now | 0 |
| `cargo tree -p ocelli-wasm ... --target wasm32-unknown-unknown`, default and `decode-worker` | worktree | now | 0 lines naming `ocelli-codec` or `ritk-codecs` in either graph |
| SHA-256 of the five patched files | worktree | now | all five equal `PATCH-PROVENANCE.md` and `RITK_PATCHED_HASHES` |
| one planted newline in each patched file, then `pin_and_size_check.py` | scratch copy of tip | now | each fails naming its file, and a deleted `encoder.rs` fails as absent. Restored and re-checked with `cmp` |
| `unittest` of `test_pin_and_size_check.py` and `test_corpus_synth.py` with the venv interpreter | worktree | now | OK |
| `generate_jpegls.py` check mode | scratch copy of tip, corpus symlinked | now | 0, every fixture `ok` |
| `deviation_check.py`, `prose_check.py`, `source_provenance_check.py` | worktree | now | all OK |
| `git diff aba1cb09..e2619832 -- vendor` | range | now | no added `unsafe`, only `src/jpeg_ls/*` touched, none of F-239's `src/jpeg_2000/*` or `src/lib.rs` |
| decode sweep, 648 CharLS streams | imagecodecs 2026.10.10, CharLS 2.4.4, my generator | now | tip 632 exact, 12 refused, **4 `Ok` with wrong samples**. Base 376 exact, 272 wrong |
| realistic sweep, 81 CharLS streams | same | now | tip 17 exact, **63 refused, 1 `Ok` with 23 wrong samples**. Base 65 wrong |
| real CT, 10 CharLS streams | five slices of `real/ct_cmb_mml`, shifted to unsigned, P 13, scratch only | now | tip **all 10 refused**. Base all 10 wrong, 92,422 to 114,504 of 115,600 samples |
| the same three sweeps with `ctx.b /= 2` changed to `ctx.b >>= 1` in `context/mod.rs` | scratch copy of tip | now | **907 of 907 exact** |
| high-NEAR sweep, 168 streams | same | now | tip 168 exact, base 151 wrong |
| end-of-scan mutations, 5,209 streams | same | now | 3,621 both refuse, 609 both accept with equal samples, 742 Ocelli alone accepts with samples equal to the original, **220 Ocelli alone accepts with changed samples**, **16 CharLS alone accepts**, 1 both accept with different samples |
| mutation catalogue, 19 mutants plus one pair | scratch copy of tip, `target-mut` | now | table under D4 |
| `rescale_slope: 2.0` and `rescale_intercept: 5.0` | scratch copy of tip | now | 18 of 30 and 19 of 30 fail |
| `decode_jpegls` release runner, three rows | scratch copy of tip | now | medians 0.1178, 0.0104 and 0.0104 ms, the same order as `docs/lld/benchmarks.md` |
| T.87 from `itu.int` `dologin_pub.asp` | ITU | now | HTTP 500, the published text was not obtained |

### Run mode, derived

T.87 A.7.1, as I hold it independently of the plan. On the encoder side, A.15, a run of `RUNcnt` samples is emitted as one `1` bit per full segment while `RUNcnt >= 2^J[RUNindex]`, each subtracting `2^J[RUNindex]` and raising RUNindex when it is below 31. At the end of a line a non-zero remainder is one further `1` bit and RUNindex is left alone. Otherwise a `0` bit and `RUNcnt` in `J[RUNindex]` bits follow, then the interruption sample, after which RUNindex falls by one. The decoder mirrors it: each `1` adds `min(2^J[RUNindex], left on the line)` and raises RUNindex only when the addition was the full segment.

`scan.rs:183` to `:212` matches on every index and boundary. `count == full` at the line end with exactly a full segment left raises RUNindex, as A.15's loop does. The break at `run_len == remaining` can never see `remaining - run_len` negative, because the loop exits as soon as they meet. A miss reads `J[idx]` bits with `idx` taken before any change, and the glimit `limit - J[run_index] - 1` still uses the index before the decrement. The encoder at `encoder.rs:163` to `:195` raises on every full hit, including the one that fills the line, and not on the partial one. Mutating either back is caught (M1, M6).

### CLAMP, derived

For MAXVAL of 128 or more, `FACTOR = floor((min(MAXVAL, 4095) + 128) / 256)`, `T1 = CLAMP(FACTOR + 2 + 3 NEAR, NEAR + 1, MAXVAL)`, `T2 = CLAMP(4 FACTOR + 3 + 5 NEAR, T1, MAXVAL)`, `T3 = CLAMP(17 FACTOR + 4 + 7 NEAR, T2, MAXVAL)`, and `CLAMP(i, j, MAXVAL)` is `j` when `i > MAXVAL` or `i < j`. At P 8, FACTOR 1: NEAR 34 gives 105, 177, then 259 falls to 177. NEAR 50 gives 153, then 257 falls to 153, then 371 falls to 153. NEAR 85 gives 258, which falls to 86. The low arm takes `max(2, floor(3 / FACTOR) + 3 NEAR)` and the like. `clamp_threshold` and both arms of `default_thresholds` match, and the plan's hand values in AC6 are right.

## Defects

### D1, the vendored context reset rounds B toward zero, so most realistic JPEG-LS frames still do not decode as CharLS decodes them

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/context/mod.rs:167`, `ctx.b /= 2`, in `update_context`, which the decoder and the vendored encoder share.
**What**: when a regular context's N reaches RESET the published code halves B with Rust integer division, which truncates toward zero. For an odd negative B that is one above what CharLS computes, so the bias correction C drifts, the next prediction moves and the scan desynchronises. On the tip this turns into a refusal when the desynchronised scan happens to run out or overrun, and into `Ok` with wrong samples when it does not. Measured on the tip: 4 of 648 small CharLS streams and 1 of 81 realistic ones return `Ok` with wrong samples (for example `8_0_5x100_blocks`, two samples at row 4 decoded 126 where CharLS gives 127, and `12_3_100x333_noisy`, 23 wrong). 12 of 648, 63 of 81 and **all 10 frames made from real corpus CT slices are refused**. On the base the same 10 real frames returned `Ok` with up to 114,504 of 115,600 samples wrong. Changing the line to `ctx.b >>= 1`, which floors, makes 907 of 907 streams exact against CharLS on the tip.
**Why it is wrong**: T.87 A.6 halves A, B and N together at RESET, and the B halving is a floor for negative B, `-((1 - B) >> 1)` as I recall the code segment. I could not open the published text to cite the clause number, so the citation is my recollection, labelled as such. The behaviour is independently established by CharLS 2.4.4, which is the decoder cornerstone3D 5.8.2 uses and which the plan names as the parity floor, and by the single-line change above. The plan's Expected section says "The frame decodes exactly as CharLS decodes it", the story row says "JPEG-LS decoded per ITU-T T.87", and `examples/folder-viewer/src/use.html:631` says "JPEG-LS frames decode as CharLS decodes them". The last is false on the tip.
**Evidence**: the three sweeps and the real-CT run in the evidence table, with the experiment copy exact on all of them. The vendored round-trip suite cannot see this, because the encoder calls the same `update_context`.
**Scope and regression**: every `.80` and `.81` monochrome frame large enough for a context to reach N 64 with odd negative B, which is almost every clinical frame of 256 by 256 or more. Consumers are the folder viewer preview and value endpoints and the oracle producer. Present on base as wrong pixels, present on tip as refusals plus some wrong pixels. Smallest regression: a CharLS-encoded fixture that reaches a reset with odd negative B, such as my `8_0_5x100_blocks` shape (the 4 by 4 block formula over 5 by 100 at P 8, NEAR 0), with its formula raw, red on the tip. It is outside the hunks the plan lists, so the lead decides whether it joins this story's patch or a new story, but the tip cannot claim CharLS parity while it stands.

### D2, the padding-value divergence is not padding only, so corrupt scans decode to wrong samples that CharLS refuses

**Where**: `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs:297` `verify_scan_end`, and the sentences at `scan.rs:294` to `:296`, `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md:41` to `:42`, `docs/lld/codecs.md:118` to `:121`, `examples/folder-viewer/src/use.html:631` and the plan's floor table row "Measured divergence under D14".
**What**: of 5,209 mutated CharLS streams, 220 are refused by CharLS with "the decoding process is ready but the source buffer still contains encoded data" and accepted by Ocelli with samples that differ from the image the stream encodes. All 220 are single-bit flips in the last three scan bytes. Example: `8_0_8x4_twoband` with bit 4 from the end flipped is `Ok` on the tip with 3 bits unread and changed samples. A flipped data bit moves where the decode ends, and the non-zero bits it leaves behind are the only trace of the corruption. CharLS refuses on that trace. Ocelli does not look.
**Why it is wrong**: the ruling and every sentence listed rest on "no sample depends on them". That holds for the seven hand-built cases in `jpegls_end_of_scan_cases.tsv`, where only the padding was edited, and fails for a stream whose data bits were damaged. The plan's own invariant, "a returned Ok means every sample was decoded from bits the encoder wrote", is broken by these 220. cornerstone3D 5.8.2 shows no image for them and Ocelli shows a wrong one, which is below the floor the plan sets and is a quietly wrong pixel. D14 permits a measured divergence, but this one was measured on a class that does not include the cases it admits.
**Evidence**: the end-of-scan mutation run. 742 further Ocelli-only accepts decode to the original samples, so the sentence is true for most of them and false for 220.
**Scope and regression**: any damaged or mis-sliced `.80` or `.81` fragment whose damage lands near the end of the scan. This contradicts a lead ruling, so it goes to the lead. The smallest regression is one CharLS stream with a data bit flipped near its end that CharLS refuses and that decodes to other samples, asserted refused. If the ruling stands, each listed sentence needs to say that a damaged final data bit can decode to wrong samples.

### D3, "the same verdict as CharLS on every byte-count case" is false in both directions

**Where**: `docs/lld/codecs.md:116` "exactly as CharLS 2.4.4 does" and `:120` to `:121` "never the reverse", `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md:40` "which matches CharLS 2.4.4 as measured", `scan.rs:294` "These byte counts match CharLS 2.4.4 as measured", and the plan's floor table "Ocelli never refuses an ending CharLS opens".
**What**: 16 of my 5,209 streams are opened by CharLS and refused by Ocelli. Ten of them are a CharLS stream with its final scan byte dropped, for example `16_0_1x13_flat`, whose scan ends `40 40 40 00`. Instrumented, Ocelli's last read wants 7 bits and the segment holds 6, so it refuses, and CharLS decodes the cut stream to exactly the original image. One is a cut, and five are bit flips that CharLS opens and Ocelli refuses.
**Why it is wrong**: Ocelli's refusal of a stream that is one byte short is the defensible reading of C.2.1, because the bits it needs are absent. The defect is the prose, which claims byte-count parity with CharLS and that the divergence runs only one way. It does not, and AC5's 24 cases do not include a dropped all-zero final byte, which is where the two part.
**Evidence**: the end-of-scan run and the instrumented copy, which printed `EXHAUST want 7 have 6` for the cut stream and `END unread_bits 7 whole 0` for the original.
**Scope and regression**: prose only, in four places plus the plan. Each needs to say which direction it has measured, or the TSV needs a dropped-zero-byte case whose CharLS verdict is recorded.

### D4, four mutants of the patched refusal logic survive the suite

**Where**: `bitstream.rs:182`, `bitstream.rs:196` to `:198`, `crates/ocelli-codec/src/jpegls.rs:462`.

| Mutant | Suite |
|--------|-------|
| M1, raise RUNindex on the partial hit | 8 fail |
| M2, clamp the miss past the line | 2 fail |
| M3a and M3b, either exhaustion check alone removed | 0 fail, both removed together 3 fail |
| M4a, M4b, M4c, allowance 3 and 2, 1 and 1, 1 and 0 | 1 or 2 fail |
| M4d, final EOI unchecked | 1 fails |
| **M4e, a stuffed byte counted as 8 bits in `segment_end`** | **0 fail** |
| **M4f, EOI accepted anywhere, not only at the end** | **0 fail** |
| M5, saturating clamp | 1 fails |
| M6, encoder raises on the partial hit | 2 vendored round trips fail |
| M7a to M7d and M7g, preset checks removed | 1 or 2 fail |
| **M7e, `t3 <= default_maxval` removed** | **0 fail** |
| **M7f, `near < t1` widened to `near <= t1`** | **0 fail** |

**What**: M4e is the C.2.1 seven-bit rule itself. Built and run against my 5,209 streams it changes 32 verdicts, for example `FF 00` inserted before EOI on `12_0_32x32_vbands`, refused by CharLS and the tip and accepted by the mutant, and the suite stays green. M4f is reachable through the adapter, because `logical_codestream` accepts a fragment ending `FF D9 FF D9`. M7e and M7f are the two boundaries of C.2.4.1.1 that AC8 lists in its refusal and no case reaches: at P 16 a 16-bit T3 cannot pass MAXVAL, and no case puts T1 at NEAR.
**Why it is wrong**: `.claude/commands/microscope.md` section 2, a test that passes both ways is a defect.
**Evidence**: `mutate.py` in the scratch copy, each file restored and compared with `cmp` against the tip.
**Scope and regression**: add an end-of-scan case where the stuffed byte's seven bits decide the verdict, a fragment with a second EOI, and two AC8 cases at P 12, T3 above 4095 and T1 equal to NEAR.

### D5, the slope count in `docs/lld/codecs.md` is wrong

**Where**: `docs/lld/codecs.md:210`, "slope 2 fails 17 of the 30 JPEG-LS tests and intercept 5 fails 19".
**What**: on the tip slope 2.0 fails 18 of 30. Intercept 5.0 fails 19, as written.
**Evidence**: the two mutations in the evidence table.
**Scope and regression**: one number. Deleting the counts is also an option, because they go stale with every test added.

## Smells

None.

## Nitpicks

- **N1.** `bitstream.rs:129` discards the bit after the limited-length unary prefix without checking it is `1`. One flipped stream, `8_0_4x8_sparse` bit 9, is opened by both decoders with the last sample 63 from Ocelli, which is the original, and 62 from CharLS. Pre-existing, outside the diff, corrupt input only, and Ocelli's value is the encoder's.
- **N2.** The plan's item 8 says imagecodecs "cannot write P 12". `imagecodecs.jpegls_encode(a, bitspersample=12)` in 2026.10.10 writes SOF55 P 12, measured, so that sentence of the plan's reasoning is wrong. No tracked prose repeats it.
- **N3.** The plan's AC15 still says "naming the four files", while the lead's ruling, the provenance and the pins gate record five.
- **N4.** The per-line exhaustion check in `decode_scan` and the one in `verify_scan_end` each go unobserved alone (M3a, M3b). The per-line one is an early exit, so the redundancy is harmless.

## Verified clean

- The A.16 run loop and the A.15 encoder mirror against my own derivation, every index and boundary, and on 907 CharLS streams, with the B line corrected, all exact.
- The C.2.4.1.1.1 CLAMP on both arms, and the AC6 hand values, against my derivation and 168 CharLS streams across every CLAMP arm.
- The A.7.1 miss past the line refused, M2 red.
- The five vendored files' digests, `RITK_PATCHED_HASHES`, the provenance digest, a planted byte in each file refused by name, a deleted file refused, no `unsafe` added, no F-239 file touched, and the vendored suite 320 of 320 on base and tip with no published test edited.
- `validate_presets`: P from SOF55, MAXVAL default `65535 >> (16 - P)`, every LSE ID 1 segment checked up to SOS, and an eleven-byte payload required. Each refusal leaves `out` untouched, `pristine=true` on every refusal my probe saw.
- Wasm unaffected: the two graphs reach neither codec crate, and both module sizes equal their recorded figures.
- D-20's new sentences: five files, every other source file unchanged, complete-byte hashes for each patched file. D-31's reworded clause: F-243's patch touches no JPEG 2000 file, true.
- `docs/lld/benchmarks.md`: the 3.9 per cent and the "about eleven times" follow from its own table, and my runner medians match.
- `docs/lld/corpus.md`: the oracle census does not name the banded rows, true. `README.md`'s added sentence, true on the tip as far as it goes.
- `more.html`'s added sentence, true. The parts of `use.html:631` other than its first sentence and the padding clause, true.
- No CharLS, dwv, Horos or Grok source was opened. CharLS was only run through imagecodecs and pyjpegls.

## Limitations and handoff

- **AC14 is not met.** The ITU download returned HTTP 500 for this pass as it did for the plan. ISO's text is paid. Every derivation above is from my own knowledge of T.87, corroborated by CharLS's behaviour, and D1's clause citation is my recollection, not a reading.
- No oracle run, no GPU, no browser, so AC18 is open. `bin/ocelli.sh bench --compare` was not run, AC13's comparison stays owed.
- The real-CT check used five slices of one corpus series, encoded in scratch and never written to the repository.
- D1 and D2 need the lead. D1 is a pre-existing vendored defect outside the plan's hunks that the story's acceptance and `use.html` cannot be true without. D2 contradicts the premise of the padding ruling. D3, D4 and D5 are for the implementer.
- A fresh independent pass reviews the remediation. The scratch copies, targets and streams were deleted with Python `shutil.rmtree` at the end of this pass.
