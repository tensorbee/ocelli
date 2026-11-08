# F-243 AC14, the transcribed normative source against the published T.87

Reviewer: independent AC14 reviewer, 2026-11-07. Worktree `ocelli-wt-f-243`, tip 15d4817e. Plan `.claude/plans/F-243-design.md`, section "Normative source, transcribed" (lines 41 to 99) and rows AC6 and AC7.

## Source

- **ITU-T T.87 (06/1998)**, English, from itu.int only. Edition page `https://www.itu.int/rec/recommendation.asp?lang=en&parent=T-REC-T.87-199806-I`, file `dologin_pub.asp?lang=e&id=T-REC-T.87-199806-I!!ZPF-E&type=items`, a zip holding `software/T87/t87.pdf` (75 pages, PDF title "ITU-T Rec. T.87 (06/98) Information Technology - Lossless and near-lossless compression of continuous-tone still images - Baseline", running header "ISO/IEC 14495-1 : 1999 (E)"). The plain `PDF-E` link returned HTTP 500, the zip link worked. Saved to the session scratch directory, never to the repository.
- The same zip carries `software/T87/Software.zip`. It was **not opened**, because it is implementation source and AC14 is against the standard alone.
- Page numbers below are the printed page numbers of the Recommendation. Printed page N is PDF page N + 6.
- Floor and ceiling brackets are lost in text extraction, so printed pages 17 and 34 were rendered as images and the brackets read from the image.
- **ITU-T T.81 was not obtainable from itu.int.** Its edition page (`T-REC-T.81-199209-I`) says "this document is only available through payment" and gives access to TIES users only. Every download link returns a TIES login page. No other source was substituted, so T.81 B.1.1.2 is **not checked**. T.87 itself carries the fill-byte rule in D.2, page 38, and that sentence is checked instead (row F1).

## Verdict

- In scope (A.7.1, J table, C.2.4.1.1, C.2.4.1.1.1, C.2.1, and the transcription flag's arithmetic): **30 claims checked, 27 agree, 3 differ.** One further claim on plan line 57 is a statement about Ocelli's code and is not checkable against the standard.
- **All three differences are clause attributions, not content.** Every formula, constant, table value and rule in the section agrees with the published text.
- AC6's hand values: **all agree** (five parameter sets recomputed).
- AC7's hand values: **all agree**. Its citation "Table C.1" repeats difference D2, and its refusal is a decoder choice the standard permits rather than one it requires (see AC7 below).
- **No difference reaches arithmetic or control flow.** The differences reach code only as comment citations, listed under "Where the differences reach code".

## Differences

| ID | Plan line | Plan says | Standard | Clause and page |
|----|-----------|-----------|----------|-----------------|
| D1 | 41 | "code segments A.15 (encoder) and A.16 (decoder)" | A.16 is an encoder segment, titled "Encoding of run segments of length less than rg". The decoder's run procedure is F.1 step 18, in Annex F, which is marked "(This annex does not form an integral part of this Recommendation" | A.7.1.2, Code segment A.16, p. 24. Annex F, F.1 step 18, p. 47 |
| D2 | 59 | "`J`, T.87 Table C.1 (A.7.1)" | Table C.1 is "LSE marker segment parameter sizes and values for JPEG-LS preset coding parameters". J is defined in A.2.1 step 3: "Initialise the variables for the run mode: RUNindex=0 and J[0..31]" | Table C.1, p. 33. A.2.1 step 3, p. 17. Repeated in F.1 step 1.e, p. 46 |
| D3 | 93 | "T.87 C.2.1, byte stuffing and the end of the scan" | C.2.1 is "High level syntax" and reads in full that "Figure B.2 of CCITT Rec. T.81 ... applies". The stuffing and padding rules are in A.1 and repeated as a note in C.1.2 | C.2.1, p. 31. A.1, p. 15. C.1.2 NOTE, p. 31 |

## Line by line

### A.7.1, run mode (plan lines 41 to 63)

| Row | Plan line | Claim | Clause and page | Result |
|-----|-----------|-------|-----------------|--------|
| A1 | 41 | Heading, A.15 encoder, A.16 decoder | A.7.1.2 Code segments A.15 and A.16, p. 24 | **Differs**, D1. A.15 encoder agrees, A.16 is not a decoder segment |
| A2 | 43 | A hit adds `min(2^J[RUNindex], remaining)`, RUNindex rises only for the full `2^J[RUNindex]` | F.1 step 18.b.i and 18.b.ii, p. 47: "Fill the image with 2^J[RUNindex] samples of value Ra, or until a line is completed" and "If exactly 2^J[RUNindex] samples were filled ... and RUNindex<31, then increase RUNindex by one". Encoder side A.15, p. 24 | Agrees |
| A3 | 46 to 50 | Decoder hit loop, cap at 31, stop at line end | F.1 step 18.a, 18.b, p. 47 | Agrees |
| A4 | 52 to 53 | On a `0` bit, read `J[RUNindex]` bits and add them, the run interruption follows | F.1 step 18.c.i, p. 47 | Agrees. Note: the block stops before F.1 step 18.c.ii, "If RUNindex>0, decrement RUNindex by one", and A.7.2 item 8, p. 25, uses J[RUNindex] "before the decrement specified in Code segment A.16". The plan does not contradict either, it ends before them |
| A5 | 57, first sentence | Encoder writes one `1` bit without raising RUNindex for a run that reaches the line end short of `2^J[RUNindex]` | Code segment A.16 `else if (RUNcnt > 0) AppendToBitStream(1,1)` with no RUNindex change, p. 24. A.8 step 17.c.i, p. 27 | Agrees |
| A6 | 57, remainder | Ocelli's colour decoder implements the rule at `sample_interleaved.rs:144` to `:172` and refuses a miss whose remainder passes the line | Not a statement of the standard | Not checkable against the standard. The quoted comment's rule agrees with A2. The refusal is a decoder choice: clause 7, p. 11, NOTE: "The decoding process is not specified for non-compliant compressed image data" |
| A7 | 59 | J is "T.87 Table C.1 (A.7.1)" | Table C.1 p. 33, A.2.1 step 3 p. 17 | **Differs**, D2 |
| A8 | 62 | `J = 0 0 0 0 1 1 1 1 2 2 2 2 3 3 3 3 4 4 5 5 6 6 7 7 8 9 10 11 12 13 14 15` | A.2.1 step 3, p. 17, and F.1 step 1.e, p. 46: `{0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 9,10,11,12,13,14,15}` | Agrees, all 32 entries. The plan's `J[0..32]` is half-open, the standard's `J[0..31]` is inclusive (A.2.1 step 2 defines `[0..i]` as i+1 instances). Same 32 entries |

### C.2.4.1.1, LSE preset coding parameters (plan lines 65 to 67)

| Row | Plan line | Claim | Clause and page | Result |
|-----|-----------|-------|-----------------|--------|
| P1 | 67 | Segment `LSE Ll ID MAXVAL T1 T2 T3 RESET` | Figure C.2, p. 33 | Agrees (the figure labels the length "LI", the text "Ll") |
| P2 | 67 | ID one byte, the five parameters 16 bits each, `Ll = 13` | Table C.1, p. 33: Ll 16 bits value 13, ID 8 bits value 1, MAXVAL T1 T2 T3 RESET 16 bits each | Agrees |
| P3 | 67 | A parameter value of zero selects its default | C.2.4.1.1, p. 33: "a value of 0 indicates reverting to default values as given in Table C.2" | Agrees |
| P4 | 67 | `NEAR + 1 <= T1 <= MAXVAL`, `T1 <= T2 <= MAXVAL`, `T2 <= T3 <= MAXVAL` | Table C.1, p. 33 | Agrees |
| P5 | 67 | RESET in `3 ..= max(255, MAXVAL)` | Table C.1, p. 33: "0, or 3 ≤ RESET ≤ max(255, MAXVAL)" | Agrees. Note: the plan omits Table C.1's MAXVAL row, "0, or 1 ≤ MAXVAL < 2P". An omission, not a contradiction |
| P6 | 67 | Default MAXVAL `2^P - 1`, default RESET 64 | Table C.2, p. 33 | Agrees |

Context the plan does not transcribe, recorded because it bears on P3: C.2.4.1.1, p. 33, says the default T1, T2 and T3 "are calculated from the values of MAXVAL and NEAR that are in force at the time that T1, T2, and T3 are used". Ocelli's `check_preset` (`crates/ocelli-codec/src/jpegls.rs:436` to `:466`) refuses any non-default MAXVAL and any partial default thresholds, so the vendored `scan.rs:139` to `:143` (all three from the segment when T1 is non-zero, all three defaults otherwise) never meets a stream where the two readings part.

### C.2.4.1.1.1, default thresholds (plan lines 69 to 87)

All on p. 34. Brackets confirmed from the rendered page image.

| Row | Plan line | Claim | Standard | Result |
|-----|-----------|-------|----------|--------|
| T1 | 72 | `BASIC_T1 = 3, BASIC_T2 = 7, BASIC_T3 = 21` | Table C.3 | Agrees |
| T2 | 73 | Branch on `MAXVAL >= 128` | "In the case where MAXVAL ≥ 128" | Agrees |
| T3 | 74 | `FACTOR = floor((MIN(MAXVAL, 4095) + 128) / 256)` | same, floor brackets | Agrees |
| T4 | 75 | `T1 = CLAMP(FACTOR * (BASIC_T1 - 2) + 2 + 3 * NEAR, NEAR + 1, MAXVAL)` | Figure C.4 | Agrees. The standard writes `CLAMP(i, j)` with MAXVAL implicit, the plan passes it explicitly |
| T5 | 76 | `T2 = CLAMP(... (BASIC_T2 - 3) + 3 + 5 * NEAR, T1, MAXVAL)` | Figure C.4 | Agrees |
| T6 | 77 | `T3 = CLAMP(... (BASIC_T3 - 4) + 4 + 7 * NEAR, T2, MAXVAL)` | Figure C.4 | Agrees |
| T7 | 78 to 79 | Otherwise `FACTOR = floor(256 / (MAXVAL + 1))` | "Otherwise, if MAXVAL < 128", floor brackets | Agrees |
| T8 | 80 | `T1 = CLAMP(MAX(2, floor(BASIC_T1 / FACTOR) + 3 * NEAR), NEAR + 1, MAXVAL)` | Figure C.5 | Agrees |
| T9 | 81 | `T2 = CLAMP(MAX(3, floor(BASIC_T2 / FACTOR) + 5 * NEAR), T1, MAXVAL)` | Figure C.5 | Agrees |
| T10 | 82 | `T3 = CLAMP(MAX(4, floor(BASIC_T3 / FACTOR) + 7 * NEAR), T2, MAXVAL)` | Figure C.5 | Agrees |
| T11 | 84 | `CLAMP(i, j, MAXVAL) = j if i > MAXVAL or i < j, otherwise i` | Figure C.3: "CLAMP(i, j) = j if i > MAXVAL or i < j, i otherwise." | Agrees |
| T12 | 87 | CLAMP is not saturating, a value above MAXVAL becomes `j` | Follows from Figure C.3 | Agrees |

### C.2.1, byte stuffing and the end of the scan (plan lines 93 to 95)

| Row | Plan line | Claim | Clause and page | Result |
|-----|-----------|-------|-----------------|--------|
| S1 | 93 | The rules below are C.2.1 | C.2.1, p. 31 | **Differs**, D3 |
| S2 | 95 | After `0xFF` in entropy-coded data the next byte's MSB is a stuffed zero and it carries seven data bits | A.1, p. 15: "a single byte with the value X'FF' in a coded image data segment shall be followed with the insertion of a single bit '0'. This inserted bit shall occupy the most significant bit of the next byte." C.1.2 NOTE, p. 31 | Agrees |
| S3 | 95 | `0xFF` followed by a byte with the high bit set is a marker and ends the segment | A.1, p. 15: "If the X'FF' byte is followed by a single bit '1', then the decoder shall treat the byte which follows as the second byte of a marker" | Agrees |
| S4 | 95 | The encoder pads the final byte to a byte boundary | A.1, p. 15: "An incomplete byte, just before a marker, is padded with zero-valued bits". C.1.2 NOTE, p. 31 | Agrees. The plan does not say the pad bits are zero. A.1 NOTE 1 adds that this "differs from the padding method specified in CCITT Rec. T.81" |
| F1 | (plan line 327, not in the transcribed section) | 0xFF fill bytes before EOI are accepted | D.2, p. 38: "In all cases where markers are appended to the compressed image data, optional X'FF' fill bytes may precede the marker." | Agrees with T.87 D.2. T.81 B.1.1.2 not checked, see Source |

### Transcription flag (plan line 99)

| Row | Claim | Recomputed from Figure C.4 | Result |
|-----|-------|----------------------------|--------|
| X1 | P 16, NEAR 0 gives `16 + 2`, `64 + 3`, `272 + 4`, that is `18, 67, 276`, and NEAR 3 gives `16 + 2 + 9`, `64 + 3 + 15`, `272 + 4 + 21`, that is `27, 82, 297` | FACTOR = floor((4095 + 128) / 256) = floor(4223 / 256) = 16. `16*1 + 2 = 18`, `16*4 + 3 = 67`, `16*17 + 4 = 276`, then `+ 3*3`, `+ 5*3`, `+ 7*3` | Agrees |

### Outside AC14's list, checked in passing and not counted

Plan line 91, A.2.1 step 1 (p. 17): RANGE, qbpp, bpp and LIMIT agree. For NEAR 0 the plan's single RANGE formula gives `MAXVAL + 1`, which is A.2.1's lossless case. The near-lossless bound agrees with definition 3.1.50 (p. 3), "differs from the corresponding one ... by not more than a pre-specified value".

## AC6, recomputed

From C.2.4.1.1.1, Figures C.3 and C.4, p. 34.

| Input | FACTOR | Raw T1, T2, T3 | After CLAMP | Plan | Result |
|-------|--------|----------------|-------------|------|--------|
| P 8, NEAR 34, MAXVAL 255 | floor(383 / 256) = 1 | 1 + 2 + 102 = 105, 4 + 3 + 170 = 177, 17 + 4 + 238 = 259 | 105 (35 ≤ 105 ≤ 255), 177 (105 ≤ 177 ≤ 255), 259 > 255 so `j` = T2 = 177 | 105, 177, 177 | Agrees |
| P 8, NEAR 0 | 1 | 3, 7, 21 | 3, 7, 21 | 3, 7, 21 | Agrees |
| P 16, NEAR 0, MAXVAL 65535 | floor(4223 / 256) = 16 | 18, 67, 276 | unchanged | 18, 67, 276 | Agrees |
| P 16, NEAR 3 | 16 | 27, 82, 297 | unchanged | 27, 82, 297 | Agrees |

Also confirmed: NEAR 34 is legal at P 8, since C.2.3 (p. 32) allows NEAR up to `min(255, floor(MAXVAL / 2))` = 127. NEAR 33 gives raw T3 252, inside MAXVAL, so 34 is the first 8-bit NEAR where the CLAMP arm changes T3, as plan line 135 says. The vendored `default_thresholds` at `vendor/ritk-codecs-0.6.0/src/jpeg_ls/context/mod.rs:295` to `:326` computes Figures C.3 to C.5 as written (located only, not used as evidence).

## AC7, recomputed

From A.1 (p. 15), A.2.1 step 3 (p. 17), A.7.1.2 Code segments A.15 and A.16 (p. 24) and F.1 step 18 (p. 47). P 8, one line of 13 samples, first line, so Ra = Rb = Rc = Rd = 0, every gradient is 0 and run mode is selected (Code segment A.2). RUNindex starts at 0.

- **`FF 30`.** A.1 discards the stuffed MSB of `0x30`, leaving the bit string `11111111 0110000`. Eight `1` bits at RUNindex 0 to 7, with J = 0, 0, 0, 0, 1, 1, 1, 1, add 1, 1, 1, 1, 2, 2, 2, 2 = 12 samples, each a full segment, so RUNindex ends at 8. The next bit is `0`, a miss. J[8] = 2 bits `11` = 3, so the run would end at 12 + 3 = 15, past the 13-sample line. **Agrees with the plan.** A compliant encoder cannot write this: at 12 samples with one left, A.16 writes `0` and RUNcnt in 2 bits only when a different sample interrupts the run inside the line, which forces RUNcnt 0. Clause 7's NOTE (p. 11) leaves such data unspecified, so refusing it is a permitted choice, not a requirement of A.7.1.
- **`FF 40`.** Bits `11111111 1000000`. Eight full hits reach 12 at RUNindex 8, the ninth `1` fills min(4, 1) = 1 sample to reach 13, the line end, and RUNindex stays 8 because the count is not 4. **Agrees: 13 zeros.** It is also exactly what A.15 and A.16 emit for 13 equal samples: eight `1` bits from A.15, then one `1` from A.16's `else if (RUNcnt > 0)`, a stuffed `0` after the `FF` byte, then zero padding, giving `FF 40`.
- AC7's citation "T.87 A.7.1 and Table C.1" carries D2. The rule it relies on is A.7.1 with J from A.2.1.

A small script written from the standard's text reproduced both results and all of AC6 (`FF 30`: miss at 12, RUNindex 8, remainder 3, end 15. `FF 40`: line complete at 13, RUNindex 8, nine `1` bits).

## Where the differences reach code

Comments only. No arithmetic, constant or branch relies on a wrong clause, and the behaviour at each site matches the standard's content.

| Difference | Site | Added by | Note |
|------------|------|----------|------|
| D1 | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs:178` "ITU-T T.87 A.7.1 (code segment A.16)" on the decoder's run loop | F-243 | The decoder rule is F.1 step 18, p. 47 |
| D1, related | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/encoder.rs:182` to `:185` cites "code segment A.15" for the one-bit end-of-line hit that leaves RUNindex unchanged | F-243 | That branch is A.16's `else if (RUNcnt > 0)`, p. 24. A.15 is the full-segment loop. Not a plan line, found while locating A5 |
| D1, borderline | `crates/ocelli-codec/tests/jpegls.rs:1414` names the encoder defect "code segment A.15" | F-243 | The increment lives in A.15, the branch that must not increment is A.16. Defensible either way |
| D2 | `crates/ocelli-codec/tests/jpegls.rs:1438`, `crates/ocelli-codec/tests/fixtures/generate_jpegls.py:151` | F-243 | J is A.2.1 step 3 |
| D2 | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs:16`, `crates/ocelli-codec/src/jpegls/sample_interleaved.rs:20` | base tree | Pre-existing. The 32 values at both sites agree with A.2.1 |
| D3 | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/bitstream.rs:28`, `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs:305`, `crates/ocelli-codec/tests/fixtures/generate_jpegls.py:154` | F-243 | The rules are A.1 and C.1.2 |
| D3 | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/bitstream.rs:3`, `:197`, `:271` | base tree | Pre-existing upstream citation |

Correcting the vendored comments would change patched bytes that `scripts/pin_and_size_check.py` hashes, so that is a decision for the lead and not something this record does.

## Limitations

- T.81 B.1.1.2 is unchecked, because itu.int sells T.81 and no other source was allowed. T.87 D.2 (p. 38) states the same fill-byte permission for JPEG-LS and was checked.
- The text came from `pdftotext`, which drops bracket and superscript glyphs. Pages 17 and 34 were rendered and read as images to confirm floor and ceiling brackets, but other pages were read as text only. In A.2.1 the rendered image itself shows only part of the ceiling glyph on qbpp.
- The 1998 edition is the only T.87 checked. ITU lists no later edition on the T.87 page fetched, but amendments and the ISO/IEC 14495-1 corrigenda were not looked for.
- The vendored decoder was read only to place each line, as the brief allows. Whether it is correct is the other review rows' job, not AC14's.
- `Software.zip` in the ITU archive was not opened.
