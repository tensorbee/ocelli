# F-242, MPEG-2 video decoded in Rust

**Status**: approved
**Epic ref**: E4.12
**Sprint**: S48
**Estimate**: 8w

Story row, `docs/sprints/BACKLOG.md`: "MPEG-2 video decoded in Rust: the four MPEG2 MP@ML and MP@HL transfer syntaxes decoded by a decoder written here from ITU-T H.262 with no dependency, in the program stream and bare PES as well as F-241's containers, so a local MPEG-2 file plays in the folder viewer and @ocelli/core where cornerstone3D 5.8.2 needs a transcoding server, with hand-computed fixtures for inverse quantisation and prediction, IDCT accuracy measured, size and speed recorded, and every MPEG-2 stream this decoder cannot decode refused by name". Layer All, depends on F-241, milestone M20.

`allocation.json` notes: "Decided by the operator on 2026-11-07, F-241 question 4: a Rust MPEG-2 decoder now, so local MPEG-2 files play, beating cornerstone, its own story by question 9. No dependency unless docs/SOURCE-POLICY.md is satisfied. The patent position is what the policy's patent section relies on and is marked for verification. Questions 1 to 6 answered by the operator on 2026-11-07: proceed on patents with the Malaysian risk recorded, f64 IDCT, MONOCHROME2 as the Y plane, libmpeg2 and ffmpeg's decoder source read-blocked, real-time thresholds, synthetic clips with a recorded assessment of the conformance streams."

**Why E4.12.** E4 is the codec epic and F-241 took E4.11. E4.9 stays free for the reason F-241's plan gives, F-145's recorded `E4.9 (UNRESOLVED)` defect.

**Why S48.** It depends on F-241 in S47, for the demuxer, the `VideoStream` interface, the `YBR_PARTIAL_420` conversion and the frame-identity clips, and `docs/sprints/SPRINT_PLAN.md` never places a story at or before the sprint holding what it depends on. S48 holds F-121 and F-122 at 16w, and this story takes it to 24w. If F-241 carries out of S47, this story moves with it.

The operator answered questions 1 to 6 on 2026-11-07. Every answer is applied below and recorded under "Decided by the operator on 2026-11-07". The plan is approved with no deviation row. The approving change writes the two read-blocked rows of question 4 into `docs/SOURCE-POLICY.md`'s "Extensions to the table" and the operator's patent decision into its patent section. The estimate rose from 7w to 8w for the checker enforcement of question 4 and the work of questions 3 and 6. Every repository path below was read on `sprint/s46` at 97b1e93d, and `scripts/source_provenance_check.py`, `scripts/guards/catalogue.py` and `scripts/tests/test_source_provenance_check.py` at 4fede25. ITU-T H.262 was not opened for this draft, and every clause number below is the lead's recollection of its structure, marked for AC14. dwv, Horos and Grok were not opened. No MPEG-2 decoder's source was opened, question 4.

## Normative source, transcribed

### HLD section 21, `docs/hld/18-codec-registry.md`

F-241's plan transcribes the `Decoder` trait and the registry rule. This story adds no `Decoder` implementer. MPEG-2 frames depend on earlier frames, so the decoder is a session behind F-241's `VideoStream` interface, item 1 of F-241's "What the specification does not cover".

### DICOM PS3.5 2026d, sections 8.2.5 and 8.2.6, read at dicom.nema.org on 2026-11-07

```text
8.2.5   MPEG2 MP@ML. Photometric Interpretation (0028,0004) shall be
        YBR_PARTIAL_420 in the case of multi-component data, and MONOCHROME2
        in the case of single component data. Bits Allocated 8, Bits Stored 8.
        Rows, Columns, Cine Rate (0018,0040) and Frame Time (0018,1063) consistent
        with Main Profile / Main Level: at most 720 by 480 at 30 per second or
        720 by 576 at 25 per second.
        "For the Non-Fragmentable Encapsulated Transfer Syntax, one Fragment
        shall contain the whole MPEG2 stream. For the Fragmentable
        Encapsulated Transfer Syntax, the stream may be segmented into
        multiple Fragments."
        "The container format for the video bit stream is not constrained.
        For example, it may MPEG-2 Transport Stream (MPEG-TS), MPEG-2 Program
        Stream (MPEG-PS), MPEG-2 Elementary Stream (MPEG-ES), MPEG-2
        Packetized Elementary Stream (MPEG-PES)" or MP4.
        "When decompressing, should the characteristics explicitly specified
        in the compressed data stream be inconsistent with those specified in
        the DICOM Data Elements, those explicitly specified in the compressed
        data stream should be used to control the decompression."
8.2.6   MPEG2 MP@HL. As 8.2.5, and "Rows (0028,0010) shall be either 720 or
        1080", "Columns (0028,0011) shall be 1280 if Rows is 720, or shall be
        1920 if Rows is 1080", at 25, 30, 50 or 60 per second.
```

Read through the site's own summary, as F-241's transcription was, and checked against the text by AC14.

### DICOM PS3.6 2026d, Table A-1

`1.2.840.10008.1.2.4.100`, `.100.1`, `.101` and `.101.1`, MPEG2 Main Profile at Main Level and at High Level, each with its fragmentable form. Transcribed in F-241's plan.

### DICOM PS3.3 2026d, C.7.6.3.1.2, YBR_PARTIAL_420

The forward equations, transcribed in F-241's plan, whose conversion this story reuses unchanged.

### ITU-T H.262 | ISO/IEC 13818-2, not yet transcribed

H.262 is the same text as ISO/IEC 13818-2 and is published by ITU-T without charge, so it is the edition this story reads. The implementer transcribes, before writing code, from the edition AC14 names:

- clause 6, the syntax and semantics of the sequence, sequence extension, GOP, picture, picture coding extension, quantiser matrix extension, slice and macroblock layers,
- clause 7.2 and Annex B, the variable length codes, Tables B-1 to B-15,
- clause 7.3, the zigzag and alternate scans,
- clause 7.4, inverse quantisation: the intra DC rule by `intra_dc_precision`, the formula for the other coefficients with `quantiser_scale` and the weighting matrices, saturation to `-2048 ..= 2047`, and mismatch control on coefficient 63,
- clause 7.5 and Annex A, the IDCT definition and its accuracy requirement,
- clause 7.6, motion compensation: frame and field prediction, 16 by 8 prediction, dual prime, half-sample interpolation with its rounding, and the forming and combining of predictions,
- clause 8, the Main profile and the Main and High levels.

The lead's recollection, not a transcription: inverse quantisation and prediction are exact integer arithmetic, and the IDCT is not. Annex A defines the ideal IDCT and requires an implementation to meet an accuracy test, so two conforming decoders may differ by a level in a block and the difference may accumulate along a prediction chain until the next intra picture. That is why AC9 measures and does not compare for equality.

### ISO/IEC 13818-1, systems

The program stream pack and system headers, the PES packet header and its timestamps, read by the demuxer. F-241 reads the transport stream from the same standard. The implementer transcribes the fields from the edition AC14 names, ITU-T H.222.0 being the same text and free.

### What the HLD does not give

No HLD section mentions video or MPEG-2. Everything below about decoding is the lead's, built on H.262.

## What the specification does not cover

1. **Where the decoder sits.** A session behind F-241's `VideoStream`, in `ocelli-codec/src/video/mpeg2/`, fed the sample table F-241's demuxer builds, decoding from a random access point to the requested sample. A random access point is an I picture that begins a GOP or follows a sequence header. Both targets call the same session: the folder viewer's local process natively, and `@ocelli/core` in the wasm module F-241's AC21 places video in. No WebCodecs object is involved, so D-32 does not apply and nothing crosses the boundary but encoded bytes.
2. **Output.** Three 8-bit planes, `I420`, the shape F-241's conversion already accepts, so stage 4 is unchanged. Display order is the sample table's presentation order, and B pictures are reordered as H.262's display process gives, never by time multiplied by rate.
3. **Interlace.** A field picture pair is woven into one frame, as are frame pictures with field DCT. The page names the stream interlaced and no deinterlacing is applied, as F-241 item 11 does for H.264.
4. **Colour.** The sequence display extension's `matrix_coefficients`, when present, selects the matrix by F-241 item 6's rule, and its absence selects PS3.3's. MPEG-2 has no full-range flag, so the range is studio.
5. **MONOCHROME2.** PS3.5 8.2.5 allows a single-component MPEG-2 file, and an H.262 4:2:0 stream always carries chroma. **Decided by the operator, question 2:** when Photometric Interpretation is MONOCHROME2, the Y plane is the frame, an 8-bit MONOCHROME2 frame through the monochrome LUT chain with its stored values unchanged, so studio-range black stays stored value 16 and the window decides how it is shown. The chroma planes are decoded, because prediction needs them, and not drawn, and the page says the file carried chroma that was not drawn. PS3.5 8.2.8's precedence sentence is not read as overriding this, because the stream does not signal a photometric interpretation.
6. **IDCT.** **Decided by the operator, question 1:** a separable IDCT in `f64` written directly from H.262 Annex A's definition, its result rounded once to an integer and saturated as clause 7.5 says. A fixed-point IDCT replaces it only if AC16 or AC17 miss their threshold, and only after it passes AC5's Annex A accuracy test, the same test the `f64` form is held to.
7. **Size limits.** A coded size above the level's limit or F-239's `MAX_PIXELS` is refused by name before any plane is allocated. Every table index and every motion vector is range-checked against the reference picture, and a vector pointing outside it is a named refusal, never a clamped read.
8. **Errors.** A corrupt slice refuses the frame by name and keeps the last good frame. No error concealment is applied, because concealment draws pixels the stream does not contain.
9. **Patents.** `docs/SOURCE-POLICY.md`'s patent section, written in F-241's approving change, records the operator's decision that MPEG-2 is decoded in Rust and what it relies on. **Decided by the operator, question 3:** proceed with the risk recorded. The approving change of this plan writes into that section the two Malaysian patents, MY 163465-A and MY 177139-A, their possible expiry in 2032 and 2035, that the operator decided to proceed knowing them, and that this is not legal advice.

## Source policy and patent assessment

Copyright. The decoder is written here from H.262 and ISO/IEC 13818-1, with no dependency added, so the copyright half of `docs/SOURCE-POLICY.md` is met by a `cargo tree` listing showing no new crate, AC12. No decoder's source is opened. libmpeg2 is GPL-2.0 and ffmpeg's MPEG-2 decoder is LGPL-2.1-or-later, and the policy's reason for blocking dwv, that translating source into Rust is a translation, applies to both. **Decided by the operator, question 4:** both are rows in "Extensions to the table" with Read **NO** and Depend no, written in the approving change. ffmpeg is allowed only as a host tool that is run, for the synthetic clips and AC9's comparison.

**Enforcement, checked for the approving change.** `scripts/source_provenance_check.py` enforces dwv, Horos and Grok three ways: a dependency name in a manifest, matched by prefix after one `lib`, by `blocked_in_name`, a URL in `BLOCKED_URLS`, and any mention outside `POLICY_FILES` with no negation within 200 characters. The new rows are only prose until the checker reads them, which is the gap the checker exists to close, so enforcement is in this story's scope, Approach step 10. libmpeg2 gets all three. ffmpeg cannot take the mention rule, because ffmpeg is named as a host tool in four tracked plans today and that use is allowed, so ffmpeg's decoder source gets the dependency and URL rules only: a manifest naming `ffmpeg` or `avcodec` after one `lib` prefix, which catches `ffmpeg-next`, `ffmpeg-sys-next` and `libavcodec`, and a URL to its source repository or its generated source listings. `scripts/guards/catalogue.py`'s `_blocked_project` reads the first `**NO**` Read row of the policy, which stays dwv, so the existing probes are unchanged. Measured at 4fede25: no tracked file outside this plan names libmpeg2, and no tracked file links ffmpeg's source.

Patents, as read by the lead on 2026-11-07. The operator decided to proceed knowing them, question 3, and the third-party rows stay marked unverified:

| What is relied on | Source | Status |
|-------------------|--------|--------|
| The last United States patent in the MPEG-2 pool, US 7,334,248, expired on 13 February 2018 | The pool administrator's MPEG-2 page at via-la.com, and press reports of February 2018 | Read, not verified against the patent record |
| The pool's list dated 1 January 2026 says patents remain active only in Malaysia | The same page, which links "Final-January-1-2026-MPEG-2-Attachment-1.pdf" | The attachment was not opened. The page was read through a summary |
| The Malaysian patents are MY 163465-A and MY 177139-A, "Synchronization arrangement for a compressed video signal", possible expiry 15 September 2032 and 8 September 2035 | A third-party tracker of the pool list, bryanquigley.com | Not verified against the list or the Malaysian register. The title suggests a systems-layer claim and the lead draws no conclusion from it |
| A pool's list covers only the patents its members contributed | General knowledge | Not checked for MPEG-2 |

So the premise that MPEG-2's patents have expired holds for the United States by the pool's own statement and does not hold for Malaysia by the same list. The operator decided on 2026-11-07 to proceed knowing that, and the decision is recorded in `docs/SOURCE-POLICY.md`. The operator may take counsel, and nothing here is legal advice.

## Problem and reproducer

After F-241, a DICOM file with transfer syntax `1.2.840.10008.1.2.4.100` and Photometric Interpretation `YBR_PARTIAL_420` is demuxed and then refused by name with a sentence naming this story, F-241 AC5, on both targets. No browser's WebCodecs registry has an MPEG-2 entry, so F-241's route cannot reach it. An MPEG-PS file is refused before demux. A new decoder is absent, so this is a story.

## What cornerstone3D 5.8.2 does

Read for F-241's plan from the shipped ESM under `tools/oracle/node_modules/@cornerstonejs/`. cornerstone3D has no MPEG-2 decoder and plays video only through a server's `/rendered` resource in an HTML `<video>` element, which no browser plays as MPEG-2. Inference, as F-241 states it: an MPEG-2 file plays only through a server that transcodes it, and a local MPEG-2 file never plays.

## Scope, API and compatibility

In scope:

- `ocelli-codec/src/video/mpeg2/`: `headers.rs`, `vlc.rs` with Annex B's tables as `const` arrays, `quant.rs`, `idct.rs`, `motion.rs`, `picture.rs` and `session.rs`. `Mpeg2Session::open(&SampleTable, &[u8]) -> Result<Mpeg2Session, VideoError>` and `decode_to(&mut self, sample: usize, out: &mut PlaneBuffer) -> Result<(), VideoError>`, behind F-241's `VideoStream`, with reference pictures pre-sized from the sequence header and reused.
- `ocelli-codec/src/video/ps.rs` and `pes.rs`: the program stream and a bare PES, into F-241's `SampleTable`.
- `Registry::capability` reports the four MPEG-2 syntaxes available through `Capability::Stream`, replacing F-241's refusal.
- Both targets: the F-241 MPEG-2 refusal sentence is replaced by playback, through F-241's video source and the folder viewer's held frame, with no new shell API.
- Refusals by name: a profile other than Main or a level above High, 4:2:2 or 4:4:4 chroma, scalable extensions, a coded size above the level or `MAX_PIXELS`, a motion vector outside its reference and a corrupt slice. A MONOCHROME2 file is drawn by item 5, not refused.
- `scripts/source_provenance_check.py`, Approach step 10: `libmpeg2` in `BLOCKED` with its reason, a URL pattern for its hosting, and a dependency-and-URL-only set for ffmpeg's decoder source, `ffmpeg` and `avcodec` by `blocked_in_name`'s rule and URL patterns for its source repository and generated listings, with no mention rule. Cases in `scripts/tests/test_source_provenance_check.py`, and one guard probe per new refusal in `scripts/guards/catalogue.py`, so `python3 scripts/guard_census.py`'s unwatched bucket stays empty.
- `scripts/corpus_synth.py`: the synthetic clips, made by running ffmpeg as a host tool, never reading it, question 6.
- `docs/SOURCE-POLICY.md`: the licence assessment of the ISO/IEC 13818-4 conformance bitstreams against "The rule for anything not yet listed", recorded as a row with its date and a short subsection, question 6.

Non-goals: error concealment, deinterlacing, 4:2:2 Profile, scalable profiles, MPEG-1 streams, audio, encoding, and a GPU decode.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Folder viewer local process | `Mpeg2Session` through `VideoStream` | Native, one session per open stream | Held converted frame, as F-241 |
| `@ocelli/core` worker | The same session in the video wasm module | One per stream per worker, D5 | Frames through F-231's session cache |
| F-241's conversion | `ColorTransform` with `YbrPartial420` | Unchanged | Display bytes |
| Registry callers | `Registry::capability` | The four MPEG-2 syntaxes become available as streams | `Capability::Stream`, and `decode_frame` still refuses |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Opened | `decode_to(n)` | Decode from the random access point at or before `n`, sample `n` in `out` | Out of range refused, `out` untouched |
| Decoded up to `m` | `decode_to(n)`, `m < n` in the same GOP | Decoding continues from `m` | As above |
| Any | A corrupt slice | The frame refused by name | The last good frame kept, the session reset to the next random access point |

Invariant: the frame returned for sample `n` is the same whatever order the samples were requested in, AC7.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Reference pictures | `Mpeg2Session::open`, sized from the sequence header | The session | None, internal | A new sequence header with a different size refuses by name |
| Output planes | The caller, F-241's `PlaneBuffer` | Generation, as F-241 | `decode_to` returns `Ok` | On error `out` is not written, and a repeated `decode_to(n)` returns the same bytes |

## Numbered acceptance inventory

Synthetic clips are written by `scripts/corpus_synth.py` with ffmpeg's MPEG-2 encoder, a host tool, with F-241's embedded frame code, never a real patient video.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Inverse quantisation, H.262 7.4. Hand-computed blocks for an intra DC at each `intra_dc_precision`, a non-intra coefficient under the default matrix and a loaded matrix, linear and non-linear `quantiser_scale`, saturation at both bounds, and mismatch control on an even sum. The implementer computes each value from the transcribed formula and records the working | `crates/ocelli-codec/tests/mpeg2_quant.rs`, H.262 7.4 | The intra rule applied to a non-intra block, a missing sign term, mismatch control on the wrong coefficient | implementation |
| AC2 | VLC and scan. A hand-assembled bitstream for one macroblock decodes to the coefficients its codes say, under both scans and both intra VLC tables, B-14 and B-15 | `tests/mpeg2_vlc.rs`, Annex B | Table B-15 read for B-14, the alternate scan ignored | implementation |
| AC3 | Half-sample prediction, H.262 7.6. A 4 by 4 reference with distinct values, a vector of half a sample in x, in y and in both, gives the hand-computed averages with H.262's rounding | `tests/mpeg2_motion.rs` | Truncating division, or rounding the two-tap and four-tap cases alike | implementation |
| AC4 | Field and dual-prime prediction. Hand-computed predictions for field prediction in a frame picture, 16 by 8 prediction in a field picture and dual prime, from H.262 7.6's transcribed rules | same | The wrong parity field, or the dual-prime differential vector unscaled | implementation |
| AC5 | IDCT accuracy, H.262 Annex A, question 1. The `f64` IDCT passes the accuracy test Annex A specifies, over the input ranges and block count it specifies, each statistic recorded beside its limit. A fixed-point IDCT, if AC16 or AC17 force one, passes the same test before it replaces the `f64` form, and both results are recorded | `tests/mpeg2_idct.rs` | A fast IDCT adopted without the test, or one whose error exceeds the limit at the range edges | implementation |
| AC6 | Demux. The same synthetic clip in MPEG-PS, a bare PES, an elementary stream, MPEG-TS and MP4 gives the same sample table | `tests/video_demux.rs` | A pack header read as a PES, a PTS wrap | implementation |
| AC7 | Frame identity, natively. For each synthetic clip, frame `n` requested in a shuffled order carries the embedded code `n`, all `N` of `N`, for progressive, interlaced field-picture and interlaced frame-picture clips. A MONOCHROME2 file of the progressive clip gives each frame's Y plane as its stored values, byte for byte against the YBR_PARTIAL_420 file's Y plane, through the monochrome chain, and the page shows the chroma-not-drawn sentence, question 2 | `examples/folder-viewer/tests/video.rs` | Decode order shown as display order, a field pair shown as two frames, a MONOCHROME2 file range-expanded or drawn in colour | implementation |
| AC8 | Frame identity through `@ocelli/core` on Chrome stable on the F-216 host | `packages/core` browser test | The native session correct and the wasm build not | integrated, Chrome |
| AC9 | Measured divergence from ffmpeg. Every plane of every frame of each clip against ffmpeg's decoded planes, largest and mean absolute difference per plane and per frame recorded with ffmpeg's version and command, and the frame code read back exactly from every frame. Recorded under D14, never claimed equal, because Annex A does not define one IDCT | Recorded in `docs/lld/codecs.md` | A stride or plane-order defect, which shows as a large difference where an IDCT difference is small | implementation, reviewer |
| AC10 | Refusals by name, each keeping the last frame: 4:2:2, a scalable extension, a level above High, a coded size above `MAX_PIXELS`, a vector outside its reference and a corrupt slice | `tests/mpeg2_refuse.rs`, `examples/folder-viewer/tests/video.rs` | A clamped read, a grey frame, a panic | implementation |
| AC11 | Mutation. Removing mismatch control, rounding half-sample averages down and swapping field parity turn AC1, AC3 and AC4 red | Recorded in the review | Tests that pass on wrong code | implementation, reviewer |
| AC12 | No dependency. `cargo tree` for `ocelli-codec`, `ocelli-wasm` on `wasm32-unknown-unknown` and `ocelli-native` names no crate absent at base | Recorded in `docs/lld/build-targets.md` | A decoder crate added under another name | implementation |
| AC13 | Size. The module carrying video, by F-241's AC21, measured on base and tip with a listing by symbol. The delta is held to that module's recorded budget and its tolerance, and the render and decode modules are unchanged. A delta beyond the tolerance stops the story for an operator decision | Recorded in `docs/lld/build-targets.md` | A budget raised to make room | implementation |
| AC14 | The reviewer checks every transcription against PS3.5, H.262 and H.222.0, names the editions read, and confirms the clause numbers this draft gives from recollection | Review note | Fixtures and code that share one wrong transcription | reviewer |
| AC15 | Patents, question 3. `docs/SOURCE-POLICY.md`'s patent section names MY 163465-A and MY 177139-A with their possible expiry, states the operator decided to proceed knowing them, and states this is not legal advice. The tracker's rows stay marked unverified until someone checks the pool's attachment | Review note | A belief recorded as a fact | reviewer |
| AC16 | Speed, natively, question 5. A new `tools/bench` subject `codec.mpeg2_decode`, release build on the F-216 host, over a 720 by 576 clip at 25 per second and a 1920 by 1080 clip at 30 per second of 300 frames each. Recorded: decoded frames per second over the clip, the GOP length, median and 95th percentile of 50 random `decode_to` calls, and peak resident memory. Acceptance: real time at both sizes, decoded frames per second over the whole clip at or above the stream's rate | Recorded in `docs/lld/benchmarks.md` | A decoder measured against nothing, or a real-time claim from one fast GOP | external, sprint verification |
| AC17 | Speed in the browser, question 5. The same clips through `@ocelli/core` on Chrome stable on the F-216 host, sustained frames per second and late frames as F-241's `viewer.video_playback` counts them. Acceptance: real time at 720 by 576, sustained frames per second at or above the stream's rate. 1920 by 1080 is measured and recorded with no threshold | same | A native-only claim | external, sprint verification |
| AC18 | Better than cornerstone3D. F-223's video mode, F-241's AC25, on the same MPEG-2 clips: cornerstone's bare page given a local file records its failure, and Ocelli's frame identity and frames per second are recorded beside it | same | A claim of better with no cornerstone observation | external, sprint verification |
| AC19 | Provenance enforcement, question 4. `scripts/source_provenance_check.py` refuses a `Cargo.toml` or `package.json` naming `libmpeg2`, `ffmpeg-next`, `ffmpeg-sys-next` or `libavcodec`, a tracked URL to libmpeg2's hosting or ffmpeg's source repository, and a mention of libmpeg2 with no negation, and stays clean on a plan naming ffmpeg as a host tool and on the tracked tree. Each refusal has a guard probe | `scripts/tests/test_source_provenance_check.py`, `scripts/guards/catalogue.py` | Rows in the policy that nothing reads, or an ffmpeg mention rule that refuses legitimate host-tool prose | implementation |
| AC20 | Conformance bitstreams, question 6. The ISO/IEC 13818-4 streams are assessed against the policy's three questions and the answer recorded in `docs/SOURCE-POLICY.md` with its date. They enter `corpus/manifest.tsv` only if all three answers permit it, and otherwise the record says why not, as the pydicom files' record does. This story's evidence uses the synthetic clips either way | Recorded in `docs/SOURCE-POLICY.md` | Conformance streams used on a belief about their terms | implementation, reviewer |

## Approach

1. Transcribe H.262 clauses 6 to 8, Annex A and Annex B, and the H.222.0 program stream and PES fields, into `docs/lld/codecs.md` before code.
2. `ps.rs` and `pes.rs`.
3. Headers and slices, then the macroblock layer and VLCs.
4. Inverse quantisation and the `f64` IDCT, item 6.
5. Prediction, frame first, then field, 16 by 8 and dual prime.
6. The session, display reordering and the random access rule.
7. Registry capability, both targets' refusal replaced by playback.
8. Synthetic clips through `scripts/corpus_synth.py`, AC9's comparison, size and speed.
9. MONOCHROME2 by item 5.
10. The provenance checker's new rules, tests and guard probes, AC19, and the conformance-stream assessment, AC20.

Rejected: binding a C decoder, which needs `unsafe` outside the two permitted files and a dependency. A WebCodecs route, which has no MPEG-2 entry. Error concealment, item 8.

## Dependencies, write ownership and resources

Depends on F-241 for `VideoStream`, `SampleTable`, `PlaneBuffer`, the conversion, the refusal this story replaces, the video wasm module decision and the frame-identity clips. Writes `crates/ocelli-codec/src/video/mpeg2/`, `ps.rs`, `pes.rs`, `registry.rs` and their tests, the folder viewer's video sentences, `scripts/corpus_synth.py`, `corpus/manifest.tsv`, `tools/bench/`, `scripts/source_provenance_check.py`, its tests, `scripts/guards/catalogue.py`, `ci/guard-probe-budget.json` if the probe count needs it, and `docs/SOURCE-POLICY.md`. The guard catalogue is a shared file, so item 10 integrates on its own commit. Chrome on the F-216 host is exclusive for AC8, AC17 and AC18.

The estimate, 8w, is the lead's: transcription and headers 1w, the macroblock layer and VLC tables 1.5w, inverse quantisation and IDCT 0.5w, prediction 1.5w, the program stream and PES 0.5w, the session and both targets 1w, clips and measurements 1w, the provenance checker with its probes 0.5w, and the MONOCHROME2 path with the conformance-stream assessment 0.5w.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-codec` and `check ocelli-folder-viewer` while iterating, then the feature profile once at the integration tree. AC13 runs F-241's wasm measurement on base and tip. AC16 to AC18 run on the F-216 host with the release build. MPEG-2 is proven natively first, F-241's question 8, and AC8 and AC17 add the browser. No oracle row is claimed, because the reference decodes no MPEG-2.

## Boundary and tier

- wasm-bindgen: not touched. The decoder is plain Rust in `ocelli-codec`.
- Pixels across the boundary: no. Encoded bytes go down, and decode happens in the core on both targets.
- Render-loop allocation: none. Reference and output planes are pre-sized from the sequence header.
- unsafe: none
- Tier A (WebGPU): full. Decode and conversion are CPU code, and the converted frame is drawn as any colour frame.
- Tier B (WebGL2): full, for the same reason.
- Tier C (CPU): full, the same code on every tier.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Inverse quantisation, H.262 7.4 | `tests/mpeg2_quant.rs`, AC1 |
| fixture | Half-sample, field and dual-prime prediction, H.262 7.6 | `tests/mpeg2_motion.rs`, AC3, AC4 |
| fixture | VLC decode and scan, Annex B and 7.3 | `tests/mpeg2_vlc.rs`, AC2 |
| fixture | IDCT accuracy, Annex A | `tests/mpeg2_idct.rs`, AC5 |
| unit | Program stream and PES demux, ISO/IEC 13818-1 | `tests/video_demux.rs`, AC6 |
| integration | Frame identity and refusals natively | `examples/folder-viewer/tests/video.rs`, AC7, AC10 |
| browser | Frame identity in `@ocelli/core` | `packages/core` browser test, AC8 |
| conformance | Measured divergence from ffmpeg | AC9 |

## cornerstone3D 5.8.2 is the floor

| Behaviour | cornerstone3D 5.8.2 | Ocelli after F-242 | Axis and measurement |
|-----------|---------------------|--------------------|----------------------|
| A local MPEG-2 file | Never plays | Plays on both targets | Better, AC7, AC8, AC18 |
| MPEG-2 from a DICOMweb server | Plays only if the server transcodes to a `<video>` format through `/rendered` | Plays the bulk stream | Better where the server does not transcode, AC8 |
| Which frame is frame N | `1 + round(currentTime * fps)` | Sample N of the stream's table | Better, AC7, AC18 |
| Interlaced MPEG-2 | The transcoder's and the browser's choice, unstated | Woven, named interlaced, not deinterlaced | Visible rather than unstated |
| Window, invert, readout, statistics | As F-241's table | As F-241's table | As F-241 |
| Speed | The browser's hardware decoder on a transcoded stream | CPU decode in Rust | Real time natively at both sizes and in Chrome at 720 by 576, AC16 to AC18 |

## Parity surface covered

Appendix B's VIDEO viewport type for MPEG-2 content, beyond cornerstone's reach for local files. No scored oracle row.

## Deviations

None. The two read-blocked rows and the patent record are policy changes in `docs/SOURCE-POLICY.md`, not deviations. The decoder sits behind F-241's interface, adds no dependency and needs no `unsafe`. D-32 does not apply, because no WebCodecs object is involved.

## LLD impact

- `docs/lld/codecs.md`: the MPEG-2 session, the transcribed clauses, AC5's statistics and AC9's divergence.
- `docs/lld/build-targets.md`: AC12 and AC13.
- `docs/lld/benchmarks.md`: AC16 to AC18.
- `docs/lld/corpus.md`: the synthetic MPEG-2 rows and AC20's outcome.
- `docs/lld/guards.md`, if it exists at implementation, else the guard catalogue's own notes: the new provenance refusals.

## User-facing documentation impact

- `README.md`: MPEG-2 video plays locally, decoded in Rust, and the video refusal list loses MPEG-2.
- `examples/folder-viewer/src/use.html`: the MPEG-2 route and the functions it calls.
- `examples/folder-viewer/src/more.html`: the video card F-241 adds, if it exists at implementation, else no change.

## Decided by the operator on 2026-11-07

Each answer is the operator's unless it says lead. The original options and recommendations are kept in the history of this file at 4fede25.

1. **IDCT.** `f64` straight from H.262 Annex A, rounded once. A fixed-point version only if the speed rows miss, and only after it passes Annex A's accuracy test. Applied in item 6, AC5 and Approach step 4.
2. **MONOCHROME2 MPEG-2.** The Y plane through the monochrome chain, and the page says chroma was present and not drawn. Applied in item 5, Scope and AC7.
3. **Patents.** Proceed, recording the risk. `docs/SOURCE-POLICY.md`'s patent section names MY 163465-A and MY 177139-A with their possible expiry, says the operator decided to proceed knowing them, and says this is not legal advice. Written in the approving change. Applied in item 9, the assessment and AC15.
4. **Copyleft decoder source.** Rows in `docs/SOURCE-POLICY.md` marking libmpeg2, GPL-2.0, and ffmpeg's MPEG-2 decoder source, LGPL-2.1-or-later, as not to be read or depended on, ffmpeg allowed only as a host tool that is run. Written in the approving change. Lead, applied: the checker should enforce them, and does so in this story, libmpeg2 by all three rules and ffmpeg's source by the dependency and URL rules only. Applied in the assessment, Scope, AC19 and Approach step 10.
5. **Speed.** Real time natively at 720 by 576 and 1920 by 1080, real time in Chrome at 720 by 576, 1080p in Chrome measured and recorded. Applied in AC16, AC17 and the floor table.
6. **Conformance streams.** Synthetic clips now, made with ffmpeg as a host tool and wrapped by `scripts/corpus_synth.py`, plus a recorded licence assessment of the ISO/IEC 13818-4 conformance streams. Applied in Scope, AC20 and Approach steps 8 and 10.

## Open questions

None. Questions 1 to 6 are answered above. AC14's reviewer check of the cited standards and of the clause numbers this plan gives from recollection, and AC15's unverified tracker rows, are evidence rows with an owner and a stage, not open decisions.
