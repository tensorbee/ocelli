# F-242, MPEG-2 video decoded in Rust

**Status**: draft
**Epic ref**: E4.12
**Sprint**: S48
**Estimate**: 7w

Story row, `docs/sprints/BACKLOG.md`: "MPEG-2 video decoded in Rust: the four MPEG2 MP@ML and MP@HL transfer syntaxes decoded by a decoder written here from ITU-T H.262 with no dependency, in the program stream and bare PES as well as F-241's containers, so a local MPEG-2 file plays in the folder viewer and @ocelli/core where cornerstone3D 5.8.2 needs a transcoding server, with hand-computed fixtures for inverse quantisation and prediction, IDCT accuracy measured, size and speed recorded, and every MPEG-2 stream this decoder cannot decode refused by name". Layer All, depends on F-241, milestone M20.

`allocation.json` notes: "Decided by the operator on 2026-11-07, F-241 question 4: a Rust MPEG-2 decoder now, so local MPEG-2 files play, beating cornerstone, its own story by question 9. No dependency unless docs/SOURCE-POLICY.md is satisfied. The patent position is what the policy's patent section relies on and is marked for verification. Open questions are the operator's."

**Why E4.12.** E4 is the codec epic and F-241 took E4.11. E4.9 stays free for the reason F-241's plan gives, F-145's recorded `E4.9 (UNRESOLVED)` defect.

**Why S48.** It depends on F-241 in S47, for the demuxer, the `VideoStream` interface, the `YBR_PARTIAL_420` conversion and the frame-identity clips, and `docs/sprints/SPRINT_PLAN.md` never places a story at or before the sprint holding what it depends on. S48 holds F-121 and F-122 at 16w, and this story takes it to 23w. If F-241 carries out of S47, this story moves with it.

This plan is a draft. Every repository path below was read on `sprint/s46` at 97b1e93d. ITU-T H.262 was not opened for this draft, and every clause number below is the lead's recollection of its structure, marked for AC14. dwv, Horos and Grok were not opened. No MPEG-2 decoder's source was opened, question 4.

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
5. **MONOCHROME2.** PS3.5 8.2.5 allows a single-component MPEG-2 file, and an H.262 4:2:0 stream always carries chroma. Question 2.
6. **IDCT.** Question 1.
7. **Size limits.** A coded size above the level's limit or F-239's `MAX_PIXELS` is refused by name before any plane is allocated. Every table index and every motion vector is range-checked against the reference picture, and a vector pointing outside it is a named refusal, never a clamped read.
8. **Errors.** A corrupt slice refuses the frame by name and keeps the last good frame. No error concealment is applied, because concealment draws pixels the stream does not contain.
9. **Patents.** `docs/SOURCE-POLICY.md`'s patent section, written in F-241's approving change, records the operator's decision that MPEG-2 is decoded in Rust and what it relies on. The assessment is below and question 3 asks the operator to confirm it.

## Source policy and patent assessment

Copyright. The decoder is written here from H.262 and ISO/IEC 13818-1, with no dependency added, so the copyright half of `docs/SOURCE-POLICY.md` is met by a `cargo tree` listing showing no new crate, AC12. No decoder's source is opened. libmpeg2 is GPL-2.0 and ffmpeg's MPEG-2 decoder is LGPL-2.1-or-later, and the policy's reason for blocking dwv, that translating source into Rust is a translation, applies to both. Neither is in the policy's table today, question 4. ffmpeg stays a host tool that is run and never read, for the synthetic clips and AC9's comparison.

Patents, as read by the lead on 2026-11-07 and **marked for verification by the operator**, AC15:

| What is relied on | Source | Status |
|-------------------|--------|--------|
| The last United States patent in the MPEG-2 pool, US 7,334,248, expired on 13 February 2018 | The pool administrator's MPEG-2 page at via-la.com, and press reports of February 2018 | Read, not verified against the patent record |
| The pool's list dated 1 January 2026 says patents remain active only in Malaysia | The same page, which links "Final-January-1-2026-MPEG-2-Attachment-1.pdf" | The attachment was not opened. The page was read through a summary |
| The Malaysian patents are MY 163465-A and MY 177139-A, "Synchronization arrangement for a compressed video signal", possible expiry 15 September 2032 and 8 September 2035 | A third-party tracker of the pool list, bryanquigley.com | Not verified against the list or the Malaysian register. The title suggests a systems-layer claim and the lead draws no conclusion from it |
| A pool's list covers only the patents its members contributed | General knowledge | Not checked for MPEG-2 |

So the operator's premise that MPEG-2's patents have expired holds for the United States by the pool's own statement and does not hold for Malaysia by the same list. The operator may take counsel, and nothing here is legal advice.

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
- Refusals by name: a profile other than Main or a level above High, 4:2:2 or 4:4:4 chroma, scalable extensions, a coded size above the level or `MAX_PIXELS`, a motion vector outside its reference, a corrupt slice, and question 2's answer for MONOCHROME2.

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
| AC5 | IDCT accuracy, H.262 Annex A. The IDCT passes the accuracy test Annex A specifies, over the input ranges and block count it specifies, each statistic recorded beside its limit | `tests/mpeg2_idct.rs` | A fast IDCT whose error exceeds the limit at the range edges | implementation |
| AC6 | Demux. The same synthetic clip in MPEG-PS, a bare PES, an elementary stream, MPEG-TS and MP4 gives the same sample table | `tests/video_demux.rs` | A pack header read as a PES, a PTS wrap | implementation |
| AC7 | Frame identity, natively. For each synthetic clip, frame `n` requested in a shuffled order carries the embedded code `n`, all `N` of `N`, for progressive, interlaced field-picture and interlaced frame-picture clips | `examples/folder-viewer/tests/video.rs` | Decode order shown as display order, a field pair shown as two frames | implementation |
| AC8 | Frame identity through `@ocelli/core` on Chrome stable on the F-216 host | `packages/core` browser test | The native session correct and the wasm build not | integrated, Chrome |
| AC9 | Measured divergence from ffmpeg. Every plane of every frame of each clip against ffmpeg's decoded planes, largest and mean absolute difference per plane and per frame recorded with ffmpeg's version and command, and the frame code read back exactly from every frame. Recorded under D14, never claimed equal, because Annex A does not define one IDCT | Recorded in `docs/lld/codecs.md` | A stride or plane-order defect, which shows as a large difference where an IDCT difference is small | implementation, reviewer |
| AC10 | Refusals by name, each keeping the last frame: 4:2:2, a scalable extension, a level above High, a coded size above `MAX_PIXELS`, a vector outside its reference and a corrupt slice | `tests/mpeg2_refuse.rs`, `examples/folder-viewer/tests/video.rs` | A clamped read, a grey frame, a panic | implementation |
| AC11 | Mutation. Removing mismatch control, rounding half-sample averages down and swapping field parity turn AC1, AC3 and AC4 red | Recorded in the review | Tests that pass on wrong code | implementation, reviewer |
| AC12 | No dependency. `cargo tree` for `ocelli-codec`, `ocelli-wasm` on `wasm32-unknown-unknown` and `ocelli-native` names no crate absent at base | Recorded in `docs/lld/build-targets.md` | A decoder crate added under another name | implementation |
| AC13 | Size. The module carrying video, by F-241's AC21, measured on base and tip with a listing by symbol. The delta is held to that module's recorded budget and its tolerance, and the render and decode modules are unchanged. A delta beyond the tolerance stops the story for an operator decision | Recorded in `docs/lld/build-targets.md` | A budget raised to make room | implementation |
| AC14 | The reviewer checks every transcription against PS3.5, H.262 and H.222.0, names the editions read, and confirms the clause numbers this draft gives from recollection | Review note | Fixtures and code that share one wrong transcription | reviewer |
| AC15 | The patent table above is verified against the pool's list attachment and the Malaysian register, or left marked unverified with the operator's decision recorded | Recorded in `docs/SOURCE-POLICY.md` | A belief recorded as a fact | operator |
| AC16 | Speed, natively. A new `tools/bench` subject `codec.mpeg2_decode`, release build on the F-216 host, decoded frames per second over a 720 by 576 clip at 25 per second and a 1920 by 1080 clip at 30 per second of 300 frames each, with the GOP length, median and 95th percentile of 50 random `decode_to` calls, and peak resident memory. Threshold by question 5 | Recorded in `docs/lld/benchmarks.md` | A decoder measured against nothing | external, sprint verification |
| AC17 | Speed in the browser. The same clips through `@ocelli/core` on Chrome stable on the F-216 host, sustained frames per second and late frames as F-241's `viewer.video_playback` counts them | same | A native-only claim | external, sprint verification |
| AC18 | Better than cornerstone3D. F-223's video mode, F-241's AC25, on the same MPEG-2 clips: cornerstone's bare page given a local file records its failure, and Ocelli's frame identity and frames per second are recorded beside it | same | A claim of better with no cornerstone observation | external, sprint verification |

## Approach

1. Transcribe H.262 clauses 6 to 8, Annex A and Annex B, and the H.222.0 program stream and PES fields, into `docs/lld/codecs.md` before code.
2. `ps.rs` and `pes.rs`.
3. Headers and slices, then the macroblock layer and VLCs.
4. Inverse quantisation and the IDCT, by question 1.
5. Prediction, frame first, then field, 16 by 8 and dual prime.
6. The session, display reordering and the random access rule.
7. Registry capability, both targets' refusal replaced by playback.
8. Synthetic clips, AC9's comparison, size and speed.

Rejected: binding a C decoder, which needs `unsafe` outside the two permitted files and a dependency. A WebCodecs route, which has no MPEG-2 entry. Error concealment, item 8.

## Dependencies, write ownership and resources

Depends on F-241 for `VideoStream`, `SampleTable`, `PlaneBuffer`, the conversion, the refusal this story replaces, the video wasm module decision and the frame-identity clips. Writes `crates/ocelli-codec/src/video/mpeg2/`, `ps.rs`, `pes.rs`, `registry.rs` and their tests, the folder viewer's video sentences, `scripts/corpus_synth.py`, `corpus/manifest.tsv` and `tools/bench/`. Chrome on the F-216 host is exclusive for AC8, AC17 and AC18.

The estimate, 7w, is the lead's: transcription and headers 1w, the macroblock layer and VLC tables 1.5w, inverse quantisation and IDCT 0.5w, prediction 1.5w, the program stream and PES 0.5w, the session and both targets 1w, clips and measurements 1w.

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
| Speed | The browser's hardware decoder on a transcoded stream | CPU decode in Rust | Recorded, AC16 to AC18, threshold by question 5 |

## Parity surface covered

Appendix B's VIDEO viewport type for MPEG-2 content, beyond cornerstone's reach for local files. No scored oracle row.

## Deviations

None. The decoder sits behind F-241's interface, adds no dependency and needs no `unsafe`. D-32 does not apply, because no WebCodecs object is involved.

## LLD impact

- `docs/lld/codecs.md`: the MPEG-2 session, the transcribed clauses, AC5's statistics and AC9's divergence.
- `docs/lld/build-targets.md`: AC12 and AC13.
- `docs/lld/benchmarks.md`: AC16 to AC18.
- `docs/lld/corpus.md`: the synthetic MPEG-2 rows.

## User-facing documentation impact

- `README.md`: MPEG-2 video plays locally, decoded in Rust, and the video refusal list loses MPEG-2.
- `examples/folder-viewer/src/use.html`: the MPEG-2 route and the functions it calls.
- `examples/folder-viewer/src/more.html`: the video card F-241 adds, if it exists at implementation, else no change.

## Open questions

Each has the lead's recommendation. None is answered, and the plan stays draft.

1. **IDCT.** H.262 Annex A permits any IDCT meeting its accuracy test. Options: a separable IDCT in `f64` from Annex A's definition, rounded once, or a fixed-point fast IDCT that must pass the same test. Affects AC5, AC9 and AC16. **Recommendation: `f64` first**, because it is Annex A's definition with one rounding and is hand-checkable, replaced by a fixed-point IDCT only if AC16 or AC17 miss question 5's threshold, and then only after it passes AC5.
2. **MONOCHROME2.** PS3.5 8.2.5 allows a single-component MPEG-2 file, and the stream still carries chroma. Options: the Y plane as an 8-bit MONOCHROME2 frame through the monochrome LUT chain with its stored values unchanged, the stream's colour drawn by PS3.5 8.2.8's precedence sentence, or refused by name. Affects AC7 and AC10. **Recommendation: the Y plane through the monochrome chain**, since the Photometric Interpretation names it and the chroma of a monochrome source carries nothing, with the page naming that chroma was present and not drawn.
3. **The patent position.** The pool's list dated 1 January 2026 states patents remain active in Malaysia, and a tracker names two expiring in 2032 and 2035, so "expired" holds for the United States and not everywhere. Options: proceed on the operator's decision as recorded in `docs/SOURCE-POLICY.md`, proceed after counsel, or hold the story. Affects every AC and AC15. **Recommendation: the operator confirms after reading the pool's list attachment, and implementation starts only after that**, because the npm package is published worldwide. Not legal advice.
4. **Copyleft decoders' source.** libmpeg2, GPL-2.0, and ffmpeg's MPEG-2 decoder, LGPL-2.1-or-later, are not in the policy's table. Options: rows in "Extensions to the table" with Read NO and Depend no, ffmpeg kept as a host tool that is run, or no row. Affects AC14. **Recommendation: the two rows**, written in this plan's approving change, so a reviewer comparing against ffmpeg's output cannot drift into its source.
5. **Speed threshold.** Options: real time natively for both clips and in the browser for 720 by 576, real time everywhere, or measured with no threshold. Affects AC16 and AC17. **Recommendation: real time natively for both clips and in Chrome for 720 by 576, 1920 by 1080 in Chrome measured and recorded**, because MP@ML is the common case for older ultrasound and endoscopy video.
6. **Conformance streams.** ISO/IEC 13818-4's conformance bitstreams would test the decoder against streams the lead did not make. Their terms are not known. Options: assess them under "The rule for anything not yet listed" and use them if they pass, or synthetic clips only. Affects AC7 and AC9. **Recommendation: synthetic clips for this story, and the assessment recorded**, so the decision is visible.
