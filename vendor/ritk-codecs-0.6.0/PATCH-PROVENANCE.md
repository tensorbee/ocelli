# ritk-codecs 0.6.0 provenance

This directory contains the published `ritk-codecs` 0.6.0 package from
crates.io. The downloaded `.crate` archive has SHA-256 digest
`5fb65755a819c6ba38bf8aaf146f4ccc23d2fe217c8161a61bf00517c9d1cce5`.
Its `.cargo_vcs_info.json` identifies upstream Git commit
`33497ccd55b44e004c0b8314a1bcc2e0fc9cb3ed` and path
`crates/ritk-codecs`.

`PACKAGE-INVENTORY.sha256` records every file in the unmodified published
package. No published file was removed. Ocelli changes the `jpeg-decoder`
dependency in `Cargo.toml` and `Cargo.toml.orig` to disable its default Rayon
feature, and patches four source files of the `jpeg_2000` module under F-239,
five source files and one published test of the `jpeg_ls` module under F-243,
and three `jpeg_ls` files' comments under F-269, recorded below. Every other
file is unchanged.

## The manifest patch

Ocelli changes only the `jpeg-decoder` dependency in `Cargo.toml` and
`Cargo.toml.orig` to disable its default Rayon feature. Their digests are
recorded in `scripts/pin_and_size_check.py` beside the inventory.

## The F-239 patch, deviation D-31

Four source files carry a recorded patch so that a three-component JPEG 2000
Part 1 codestream decodes, with the codestream's own inverse multiple
component transform applied before the DC level shift is restored. The
published decoder refuses `Csiz != 1` and `MCT != 0` outright, and PS3.5
8.2.4 permits a JPEG 2000 codestream to carry three components whose
photometric names that transform. The patch's own text adds no `unsafe`.

- `src/jpeg_2000/packet/reader.rs`: the packet walk generalises to
  `num_components` components of one shared geometry, and
  `decode_tile_planes` returns every component's level-shifted samples. The
  inverse transform runs inside the decode, choosing the reversible RCT or
  the irreversible ICT by the wavelet, before the irreversible path's single
  rounding. `decode_tile_part` delegates with one component, so its own
  decode is unchanged and an error's text alone gained the component index.
  The file also carries a `#[cfg(test)]` unit test
  pinning `inverse_ict_9_7` to the F-239 plan's AC25 hand values at a 1e-4
  tolerance, which the corpus rows cannot carry because an encoder's
  quantization moves them.
- `src/jpeg_2000/image.rs`: `decode_j2k_fragment_colour` validates a
  three-component codestream against PS3.5 8.2.4 Table 8.2.4-1, decodes all
  three components per tile-part, and restores the DC level shift with the
  range limit the reference decoders apply.
- `src/jpeg_2000/mod.rs` and `src/lib.rs`: the new public entry point
  `decode_jpeg2000_fragment_colour` is exported beside the existing one.

Their SHA-256 digests are:

- `src/jpeg_2000/image.rs`: `8c7a1c8b1147369c60b11fdd3132339cf798b2f4c40327ac00ffd34ca3f30e44`
- `src/jpeg_2000/mod.rs`: `a3e14bca44ec66090df7a1d3a69da08e74f01f2b8cbcd769c9073c300bf7d52a`
- `src/jpeg_2000/packet/reader.rs`: `2ed1bf22d0607915048fcb807f6ccaa599c05eb29057a2b19c95ef73e611efc1`
- `src/lib.rs`: `accfac2f2cfe750a0b8df2f3409e00a46c98ed2eeeef338469a84ad997ed7def`

Every other published file is byte-identical to the archive, and the pins
gate refuses any unrecorded byte. F-239's patch adds one test of its own to
`reader.rs`, and the suite keeps the published refusal of a three-component
codestream through `decode_jpeg2000_fragment`, because the monochrome entry
point is unchanged.

The exact upstream `LICENSE-MIT` and `LICENSE-APACHE` files from the identified
commit are retained here. Ocelli selects the MIT alternative for redistribution.
Their SHA-256 digests are:

- `LICENSE-MIT`: `8ff8d4621b3a1aa9df7006e0eaeacd5dfb686dcaa328e86c692374fbf2c4874f`
- `LICENSE-APACHE`: `b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1`

## F-243, JPEG-LS run mode, context reset, end of scan and default thresholds

The published single-component JPEG-LS decoder returned `Ok` with wrong
samples on flat or banded monochrome frames and on frames encoded from real CT
slices, and never refused a scan that ran out of data. Every undamaged stream
of F-243's generated corpus that CharLS opens decodes with the patch to the
samples CharLS 2.4.4 gives. A damaged scan can read as a valid
scan of another image, which no decoder can detect, so no claim is made that
every damaged scan is refused. Each change below names
what it follows. The clauses of ITU-T T.87, which is ISO/IEC 14495-1, cited
here were checked against T.87 (06/1998) from itu.int, recorded in
`.claude/reviews/F-243-ac14-standard-check.md`, and the upstream clause
citations in the patched files were corrected against that text.
`scripts/pin_and_size_check.py` records each digest in `RITK_PATCHED_HASHES`
and refuses any other byte.

- `src/jpeg_ls/scan.rs`. The run-mode loop, T.87 F.1 step 18: a run hit
  adds `min(2^J[RUNindex], samples left on the line)`, and RUNindex rises only
  when that count is the full segment. The published loop raised it on the hit
  that only fills the end of a line, which desynchronised every later run. A
  miss whose remainder carries the run past the line is refused rather than
  clamped. A decoded prediction error outside the range an encoder reduces it
  to is refused, in regular mode and at a run interruption, where the published
  code wrapped or clamped it to some sample. A scan that runs out of
  entropy-coded data is refused at the end of the line where it ran out. The
  new `verify_scan_end` requires the bits after the last sample to be zero and
  at most 7, the rest of one byte, and EOI to follow, after any 0xFF fill
  bytes (T.87 D.2). A whole byte more is refused, because review pass 2
  measured damaged scans that read as a valid scan one byte shorter opening to
  wrong samples under a 15-bit allowance. CharLS 2.4.4 opens most streams with
  one 0x00 inserted before EOI, so that is a named gap. Of these rules, CharLS
  was measured to open some streams with an error outside the coded range and
  limited-length codes with extra prefix zeros, which Ocelli refuses because
  no encoder writes them.
  SHA-256 `5fa6512032ced5ec593394d65111ae296ecb3702dbdc1dfe178a27dacdd2c559`.
- `src/jpeg_ls/bitstream.rs`, the bit reader. It records that a read asked for
  more bits than the segment holds and where the terminating marker is, and
  reports how many bits are left before that marker, whether all are zero,
  and whether the marker after any fill bytes is EOI. The limited-length
  Golomb code's unary prefix must end with a 1 after `LIMIT - qbpp - 1` zeros.
  SHA-256 `49367ccb7467dd91b267a2dfe5a1f70f70d872bf28243fdbbf817e9a34a68cbb`.
- `src/jpeg_ls/decoder.rs`, one call to `verify_scan_end` after the scan. It is
  called here rather than inside `decode_scan` because a published test of
  `decode_scan` passes entropy-coded bytes with no marker after them.
  SHA-256 `d898917107a8d2575a04cc47ba53748785f04c1b661235f283ede6744f8a4b11`.
- `src/jpeg_ls/context/mod.rs`. The default thresholds follow C.2.4.1.1.1's
  `CLAMP(i, j, MAXVAL)`, which gives `j` when `i` is above MAXVAL or below
  `j`. (The F-269 audit below restyles the comment's written form to
  Figure C.3's two-argument `CLAMP(i, j)` with MAXVAL ambient, the same
  rule. This bullet records the F-243 patch's own wording and is kept as
  history.) The published function saturated at MAXVAL, which at P 8 and NEAR 34
  gives T3 255 where the standard gives 177. And at a context RESET B is
  halved with an arithmetic shift, which floors, as CharLS does. The published
  `/ 2` rounds an odd negative B toward zero, which moved the bias correction
  and desynchronised most realistic frames. `update_context` is shared by the
  decoder and the encoder, so both change.
  SHA-256 `7a39478fed96333a1654ce3f8ad4029866abe2e28db26262fc8618e54d586a95`.
- `src/jpeg_ls/encoder.rs`, the mirror of the run-mode change, A.7.1 code
  segment A.16: the hit that only fills the end of a line no longer raises
  RUNindex. Ocelli's shipped paths do not call this encoder. It is fixed so
  that the published round-trip suite pairs it with the fixed decoder rather
  than with the defect.
  SHA-256 `be209fabf20a8b54b53c6156eb4de9922464c5192e600f3a6b88da6d5aa4b988`.
- `src/jpeg_ls/tests/conformance.rs`, **an edit to a published test**, by the
  lead's ruling. `jpeg_ls_fragment_2x2_all_zero_decodes_correctly` builds a 2
  by 2 frame whose scan byte was `0xF8`: four hit bits fill the frame and the
  padding after them carries a 1. CharLS 2.4.4 refuses that stream, and so does
  the patched decoder, so the byte is now `0xF0`, the same four hit bits with
  zero padding. The test's assertion is unchanged.
  SHA-256 `aee89d4a0a49b21d8f6ebc4229e4458cc4c037300dad36babcb17f08f4acc512`.

The patch adds no `unsafe` and no dependency. F-243's patch changes no test
count: `cargo test --lib` over this package passes the same 320 published
tests before and after it, with the one published test edited as above, and
F-239's patch adds one test of its own, for 321 in total on the merged tree.

## F-269, the factual comments audited against ITU-T T.87

The S47 comment-audit story. Every factual comment under `src/jpeg_ls/` was
checked against the code below it and the clause it names, using ITU-T
T.87 (06/1998) only, from the official PDF with SHA-256
`4ca6f9816fb024b954fc811c8c6821814b45beb1690dff32ed1e8aef609a8778`. No
CharLS or bundled implementation source was opened. The audit changes
comments only: no Rust logic, constant, signature, fixture or test body was
touched, and no new vendor file was added. Four files carry comment edits,
listed after the inventory with their final digests. The audit's first
review round corrected the clause citations in three comment blocks and
re-measured every line locator. Its second round required the inventory to
be enumerated mechanically from every `src/jpeg_ls/` file, which found one
false comment standing unlisted and one rating resting on a false reason,
both corrected below, and re-measured every locator from the final patched
bytes.

### The four F-243 pass 8 comments

- `src/jpeg_ls/context/mod.rs:1` to `:2`, the module header sentence. "Lossless
  decoding" was too narrow for a module whose `CodingParams::new` takes NEAR
  and whose encoder shares it. Now names lossless and near-lossless coding,
  the decoder and the encoder. T.87 A.2.1, and the module's own consumers.
- `src/jpeg_ls/context/mod.rs:28` to `:29` and `:16` to `:17`, the `n` field.
  The "to avoid division by zero" reason did not match the code it described:
  A.2.1 step 2 prescribes `N = 1`, and A.5.1's k selection is a shift
  comparison with no division. Now cites A.2.1 step 2. The field's doc and
  the header bullet were changed to the same claim.
- `src/jpeg_ls/context/mod.rs:321` to `:339`, the `default_thresholds` doc.
  The claim "DICOM data is >= 8-bit so the high branch is the production
  path" was false: `decoder.rs` accepts precision from 2, and PS3.5 8.2.3
  permits Bits Stored 2 to 16 in an 8 or 16-bit container, so `MAXVAL < 128`
  takes the low branch. The doc now transcribes both arms of C.2.4.1.1.1,
  Figures C.4 and C.5, with the floored FACTORs and the two-argument CLAMP
  of Figure C.3, plus the inspection control that MAXVAL 127, NEAR 0 gives
  (2, 3, 10) on the low arm and the wrong (2, 3, 4) if the high arm is
  generalised down. The earlier design paragraph had abbreviated T2 and T3,
  while the published vendor doc carried their full formulas and already
  wrote the clamp against MAXVAL, `[NEAR + 1 / T1 / T2, MAXVAL]`, which the
  F-243 patch comment restyled as the three-argument
  `CLAMP(i, NEAR + 1 / T1 / T2, MAXVAL)` and the audit corrected to
  Figure C.3's two-argument form with MAXVAL ambient.
- `src/jpeg_ls/scan.rs:163` to `:167`, the local gradients and the three
  gradient lines they annotate. A.3.1 names D1, D2 and D3 only the local
  gradients, so the orientation adjectives "vertical", "horizontal" and
  "diagonal" are dropped without claiming the standard proves them false.
  The arithmetic `D1 = d - b`, `D2 = b - cc`, `D3 = cc - a` is unchanged.

### The k cap record, AC3

- `src/jpeg_ls/context/mod.rs:229` to `:247`, the free `compute_k` doc,
  `src/jpeg_ls/context/mod.rs:117` to `:132`, the
  `RunInterruptionContext::compute_k` doc, and
  `src/jpeg_ls/scan.rs:298` to `:311`, the `in_coded_range` doc, now state
  the caller-specific premises F-243 pass 8 recorded as unwritten: the
  decoder's `k < qbpp` cap rests on `in_coded_range` refusing before both
  state updates, including before the run-interruption update, and the
  encoder instead applies `modulo_reduce` before its updates. The
  run-interruption helper's comparison target is recorded as its actual
  `TEMP = A + (N >> 1) * run_interruption_type`, with the type 0 or 1
  supplied by the constructor. `in_coded_range` is decoder-only and not
  shared. Shared premises are the initial `A` and `N` from A.2.1 step 2 via
  `CodingParams::new`, `RANGE`, `qbpp = ceil(log2 RANGE)`, and `RESET = 64`.
  Pass 8's damaged-stream sweep was a decoder measurement, roughly two
  million cap bindings with the refusal disabled, and is not re-proven
  here, and no encoder measurement is claimed.

### Full factual-comment inventory, AC2

Every factual comment block under `src/jpeg_ls/`, grouped by contiguous
same-claim block. Classification: `correct` (the comment matches the code
and the clause it names) or `edited` (changed, listed above). Non-factual
and decorative comments, section banners and error-variant field labels are
recorded as excluded. The tests' arithmetic annotations were each re-derived
by hand.

- `bitstream.rs:1` to `:8`, A.1 bit stuffing. Correct: an encoded `0xFF`
  data byte is followed by one stuffed zero bit and marker prefixes
  terminate the scan, matching A.1's note.
- `bitstream.rs:10` to `:24`, reader fields. Correct, implementation
  description, no clause.
- `bitstream.rs:28` to `:34`, `SegmentEnd` fields. Correct, F-243 record.
- `bitstream.rs:39`, `:59`, `:83`, `:94`, `:100`. Correct,
  implementation-only.
- `bitstream.rs:106` to `:116`, `read_golomb`. Correct: LIMIT
  `2*(bpp + max(8, bpp))` is A.2.1 step 1's formula and glimit
  `LIMIT - J[RUNindex] - 1` is A.7.2 item 8, both verified in the T.87 text.
- `bitstream.rs:118`, `:127` to `:143`. Correct, F-243 measured records.
- `bitstream.rs:149`, `:154` to `:158`, `:176`. Correct: D.2 names optional
  `0xFF` fill bytes before markers.
- `bitstream.rs:188`, `:197` to `:206`, `:225`. Correct, implementation.
- `bitstream.rs:241` to `:245`, writer mirror. Correct: normal and escape
  forms match A.5.3's LG(k, LIMIT).
- `bitstream.rs:266` to `:273`, flush. Correct: A.1's note is the stuffed
  follow-byte rule cited.
- `bitstream.rs:292` to `:410`, the test block. Correct, test-local
  arithmetic re-derived (0b10110000 cases, 0xFF 0x00 stuffing, MErrval
  encodings, and the k=1 code at `:409` to `:410`).
- `marker.rs:1` to `:15`. Correct: the eight constants match the marker
  codes. SOF55 0xFFF7, LSE 0xFFF8 and the rest are implementation facts.
- `parser.rs:1`, `:6`. Correct, implementation-only.
- `reconstruction.rs:1` to `:5`. Correct: two rows suffice for causal
  prediction.
- `sample_limits.rs:1`. Correct, section banner.
- `mod.rs:1` to `:9`, the module architecture. Correct, implementation.
- `mod.rs:32` to `:36`, `decode_jpeg_ls_fragment`. Correct:
  implementation-only description.
- `decoder.rs:1` to `:13`, the header and `InterleaveMode`. Correct: the
  decoder refuses multi-component and non-None interleave, and C.2.3 names
  the interleave modes.
- `decoder.rs:35` to `:50`, state fields. Correct, implementation.
- `decoder.rs:57`, `:73`. Correct.
- `decoder.rs:115` to `:117` and `:165` to `:167`, the allocation guards.
  Correct: the guards precede the buffers and 65535 by 65535 is the stated
  hostile bound.
- `encoder.rs:1` to `:17`, the module header and evidence tier. Correct:
  the TS UIDs, the lossless and near-lossless profile, the shared context
  model, and the A.4.4 reconstruction bound are all as cited. The
  cross-implementation sentence is a historical record and stays qualified
  as one-time.
- `encoder.rs:32` to `:56`, the public function doc and doctest. Correct:
  parameters and refusal list match `validation.rs`, and the doctest
  assertions match the emitted SOI/EOI bytes. No doctest body changed.
- `encoder.rs:74` to `:105`, header emission annotations. Correct, each
  names the byte written.
- `encoder.rs:110` to `:111`, `:121` to `:122`. Correct, implementation.
- `encoder.rs:134` to `:136`, `:152`, `:162`. Correct.
- `encoder.rs:168`. Correct: the hit appends one full `2^J[idx]` segment,
  the loop body of A.8 step 17 b.
- `encoder.rs:172` to `:174`. Edited: the comment said the decoder clamps
  there, and it does not. On that branch `acc + seg <= run_len <= remaining`
  and then `acc >= remaining`, so the full segment exactly fills the line,
  the decoder's `full.min(remaining - run_len)` returns `full` unchanged
  (`scan.rs:190`), and its RUNindex rises because `count == full`
  (`scan.rs:192`). Both sides raise RUNindex on a full segment that ends
  the line, A.8 step 17 b on the encoder and F.1 step 18 b ii on the
  decoder, and the comment now says that. Nothing clamps on this branch.
- `encoder.rs:184` to `:187`, `:191` to `:192`, `:200`. Correct: the
  A.7.1/A.16 RUNindex rule is F-243's, verified against A.8 step 17 and
  code segments A.15 and A.16.
- `encoder.rs:205`, `:217`, `:222` to `:223`, `:246`, `:257`, `:259`.
  Correct: the run interruption section is A.7.2, the parity-pick is the
  inverse of `compute_error_value`, and the canonical-error notes match
  A.5.2's k=0 XOR step.
- `encoder/validation.rs:9` to `:65`, the error enum. Correct:
  implementation-only field labels. One nit kept: `:56` says "precision and
  SOS representation" for the NEAR limit, where the limit is
  `min(u8::MAX, 2^(P-1) / 2)` from `sample_limits.rs` and the SOS NEAR
  field is a u8. True at both terms, left unchanged.
- `scan.rs:1` to `:6`, the module header. Correct: A.3 to A.7 and F.1.
- `scan.rs:17` to `:20`, the J table. Correct: A.2.1 step 3 initialises
  `J[0..31]` and a hit extends the run by `2^J[i]`, matching F.1 step 18 b.
- `scan.rs:26` to `:30`, `:34` to `:49`, the predictor enum. Correct:
  ISO defines the edge-detecting predictor of A.4.1 and the SOS header
  selects none. The per-arm notes match `predict`'s arms.
- `scan.rs:53` to `:58`, the A.4.1 proof. Correct against code segment A.5.
- `scan.rs:72` to `:77`, boundary conditions. Correct against A.2.1.
- `scan.rs:91`, `:93`. Correct.
- `scan.rs:107`, `:114`. Correct, implementation.
- `scan.rs:120` to `:128`, `decode_scan`. Correct: A.3 to A.6 regular per
  F.1 steps 2 to 17 and A.7 run per F.1 step 18.
- `scan.rs:150` to `:152`, `:160`. Correct.
- `scan.rs:163` to `:167`. Edited, listed above.
- `scan.rs:169`, `:175` to `:186`, `:199`, `:206` to `:207`, `:215`,
  `:220`, `:242`, `:253`, `:259`, `:262`, `:267`, `:278`, `:282`. Correct:
  the run-mode comments cite F.1 step 18 and the F-243 refusals, and
  `:206` to `:207`'s "not a stream any encoder writes" matches A.16's
  `AppendToBitStream(RUNcnt, J[RUNindex])` with a remainder that cannot
  exceed `2^J - 1` on a conformant stream.
- `scan.rs:288` to `:289`. Correct, F-243 record.
- `scan.rs:298` to `:311`. Edited, listed above under the k cap.
- `scan.rs:316` to `:324`, `verify_scan_end`. Correct: A.1 pads an
  incomplete final byte with zero-valued bits before a marker, at most 7
  of them, and D.2's optional `0xFF` fill bytes may precede it. The EOI
  requirement and the refusal reasons are the F-243 record.
- `scan.rs:348` to `:403`, the test module. Correct: the `predict_adaptive`
  cases at `:348`, `:354` and `:360` re-derive against code segment A.5,
  the `predict` boundary arms at `:366`, `:372`, `:378` and `:384` re-derive
  against the `predict` arms, and `:403` names the default thresholds.
  Edited: `:390` to `:392`, the 2x2 run test said the test byte provides
  five hit bits plus padding, while the decode reads exactly four hit
  bits, two per row, and the fifth 1 is unread padding, the same byte the
  F-243 section above describes for the conformance test.
- `context/mod.rs:4` to `:15`, the header's two sections. Correct: the
  A.3 selection paragraph matches A.3.1's gradients, A.3.3's nine levels
  and A.3.4's sign flip and index range, and the state bullets match
  A.2.1 step 2's `a_init` and section 3.3's C clamp.
- `context/mod.rs:19` to `:34`, `:45` to `:55`, `:58` to `:66`, `:71` to
  `:76`, `:93`. Correct: 365 contexts from A.3.4, RESET 64 from Table C.2,
  MIN_C/MAX_C from section 3.3, the initial state from A.2.1 step 2, and
  `Default` kept qualified as test-only.
- `context/mod.rs:145` to `:147`. Correct: the sign-mapping condition is
  A.7.2's and matches both callers.
- `context/mod.rs:179` to `:182`, `update_context`. Correct: A.6.1 code
  segment A.12. The published comment named A.6.1 only. The F-243 patch
  text already cites A.12's content, and the comment body names the shift
  floor explicitly. Classified correct after the F-243 edit, no F-269
  change. The F-243 arithmetic-shift record inside the body is at
  `:189` to `:192`.
- `context/mod.rs:189` to `:192`, the F-243 arithmetic-shift record.
  Correct: `>>=` on `i32` floors, A.6.1 Code segment A.12 halves at RESET,
  and a truncating `/ 2` would leave an odd negative B one too high.
- `context/mod.rs:198`, the bias-correction note. Correct, the branch
  below it is A.6.2 Code segment A.13's B and C update.
- `context/mod.rs:229` to `:247`. Edited, listed above under the k cap.
- `context/mod.rs:251`, the shift-comparison note. Correct, it restates
  A.5.1 Code segment A.10's `N << k >= A` test.
- `context/mod.rs:258` to `:262`, `:286` to `:289`, `:299` to `:306`.
  Correct: `quant` is A.3.3 code segment A.4
  with the NEAR dead-zone, `sign_normalize` and the index mapping are
  A.3.4's one-to-one requirement, and the partition arithmetic re-derived
  (5 + 36 + 324 = 365).
- `context/mod.rs:310`, `:313` and `:316`, the partition notes. Correct:
  5 + 36 + 324 = 365 contexts over `[0, 365)`, A.3.4's one-to-one map.
- `context/mod.rs:321` to `:339` and `:358` to `:361`. Edited, listed
  above.
- `context/mod.rs:348` to `:349`, the low-precision branch. Correct:
  Figure C.5's `FACTOR = 256 / (MAXVAL + 1)` divides the basic thresholds.
- `context/mod.rs:370` to `:385`, `CodingParams`. Correct: RANGE
  `MAXVAL + 1` for lossless is A.2.1 step 1, `qbpp` and `limit` match, and
  `a_init` is A.2.1 step 2's formula.
- `context/mod.rs:406` to `:407`, `modulo_reduce`. Correct: A.4.5 code
  segment A.9, and the interval `[-RANGE/2, (RANGE+1)/2)` matches A.4.5's
  `(−RANGE/2 .. ceil(RANGE/2) − 1)`.
- `context/mod.rs:419` to `:420`, `quantize_error`. Correct: A.4.4's
  quantization, identity at NEAR 0.
- `context/mod.rs:430` to `:433`, `reconstruct`. Correct: the wrap and
  clamp match A.4.4's reconstructed-value computation with F.1's modulo
  step, and NEAR 0 reduces to the modulo-RANGE form.
- `context/mod.rs:445` to `:449`, `inverse_map`. Correct: A.5.2 code
  segment A.11's regular mapping, even forward and odd backward.
- `context/tests.rs:5` to `:21`, the quant boundaries. Correct, each
  annotation re-derived against A.3.3's T1/T2/T3 arms.
- `context/tests.rs:54` to `:84`, `context_index` cases. Correct, the
  index arithmetic re-derived (5, 40, 41, 364).
- `context/tests.rs:95` to `:97`, the 8-bit defaults. Correct: FACTOR 1
  gives (3, 7, 21).
- `context/tests.rs:106` to `:110`, the 16-bit defaults. Correct: the
  4095 cap gives 18, 67, 276, and the parenthetical records what the
  published assertions were replaced with.
- `context/tests.rs:119`, `:126`. Correct, re-derived.
- `context/tests.rs:138` to `:167`, `inverse_map` cases. Correct against
  A.5.2.
- `context/tests.rs:174` to `:186`, `compute_k` cases. Correct, the shift
  comparisons re-derived.
- `context/tests.rs:193` to `:195`, the
  `update_context_accumulates_errval` annotations. Correct, re-derived:
  errval 3 lifts A to 3 and N from 1 to 2.
- `context/tests.rs:206` to `:213`, the
  `update_context_bias_positive_decrements_b` derivation. Edited at
  `:206` to `:209` and `:212`: the block contained five lines of
  self-questioning scratch ("in the code?", "Wait let me re-check", "Hmm")
  that resolved to the value the assertions already pin. Rewritten as the
  resolved derivation, which cites A.6.1 Code segment A.12 for the A, B
  and N update and A.6.2 Code segment A.13 for the `b > 0` bias branch:
  errval 0 adds 0, N rises from 4 to 5, `b > 0` subtracts the new N giving
  0. Assertion bodies unchanged.
- `tests/conformance.rs:15`, `:31` to `:33`, `:42` to `:43`, `:52`.
  Correct: the F-243 scan-byte record and the derivation notes re-derived.
- `tests/decoder.rs:25` to `:26`. Correct: A.4.4's bound is exact.
- `tests/round_trip.rs:1` to `:5`, `:58`, `:67` to `:68`, `:103` to
  `:104`, `:115`, `:122`, `:139` to `:140`, `:172` to `:177`. Correct:
  implementation and regression records, the A.4.4 NEAR bound and the
  trailing-0xFF stuffing fix.
- `tests/parser.rs:18` to `:43` and `:57` on, the byte-stream annotations.
  Correct, each names the field it annotates.

Excluded as non-factual or decorative: the `bitstream.rs:195` writer
banner, error-variant field labels that restate their field names, and
`#[allow(dead_code)]` attributes. No factual claim in any of them. The
clause-citing `// ──` banners above are listed in the inventory as
factual instead.

Files edited by this audit, comments only, at their final digests:

- `src/jpeg_ls/context/mod.rs`: SHA-256 `24af96f8abb4e5aae0b94428426c949f41ad85438daae0ff1cfa7a3289986421`
- `src/jpeg_ls/context/tests.rs`: SHA-256 `b62259de352e6353fc4decc2e9a2e05700246a7d8b87dcfa46935bde66f57f95`
- `src/jpeg_ls/encoder.rs`: SHA-256 `8ad43406e62dc7c01ced6744f838b68121ba8f74c99fb96e24c842155abaad75`
- `src/jpeg_ls/scan.rs`: SHA-256 `29b9eac0579b3279d883304a2ac739d8034e7e68bd0a6c60289d6e627dbb659f`

`context/tests.rs` enters the patched-file map for the first time: it was
unpatched before this audit and now carries a comment edit. The other
three were already patched under F-243 and take new digests. The other
thirteen `jpeg_ls` files are unchanged and remain at their recorded or
published bytes.

Source concerns for Root, no code changed: none new. The audit's second
pass enumerated every comment block in every `src/jpeg_ls/` file
mechanically and corrected the two false comments the first pass had
left standing, the encoder's line-filling-hit note and the 2x2 test's
hit-bit count. No factual comment is left doubtful under this audit.
