# F-241 review, pass 1, Rust demux slice

**Reviewed**: work/f-241-demux, range a2b3098b..95450d31 at tip 95450d31, four commits, no unstaged or staged tracked changes (`git status` shows only the untracked `.venv` and `tools/oracle/out` symlinks).
**Author / reviewer**: author is the Claude implementer agent that took over an unfinished GLM run (commits carry the operator's git identity). Reviewer is an independent Claude reviewer agent that wrote none of the range. The reviewer did not see the implementer's final report, so the implementer's mutation catalogue and its survivor list were not available.
**Result**: 3 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **AC3** (MP4 table, 60 samples, ctts order, sync at 0, 12, 24, 36, 48). Challenged against ffprobe 9.0.2 as an independent ISO reader, not against the test's own box walker. Every MP4 clip, 22 files: byte offset, size and key flag equal ffprobe's packet by packet in decode order, and pts differs from ffprobe's by one constant per file (1024, 512 or 0 ticks, the edit list shift ffmpeg applies and the demuxer does not). Rejects decode order as display order and a zero-based `stss`.
- **AC4** (TS and Annex B give the AC3 table). TS: 9 files, key flags equal ffprobe's and pts equal ffprobe's modulo 2^33, the wrap clip included. Annex B: 7 files, the demuxer's presentation order equals the order in which ffmpeg's own decoder outputs frames, matched by packet position. That checks POC types 0 and 2 and HEVC 8.3.1 against a decoder rather than against the muxer's ctts. Rejects a lost PicOrderCntMsb and a missing PTS unwrap.
- **AC5** (MPEG-2 in ES, MP4 and TS, PS and PES refused naming F-242). ES display order equals ffmpeg's decoder output order within one byte of position (stuffing zero), TS and MP4 as above. Both refusals name F-242.
- Composition: the colour precedence of plan item 6 across MP4 and Annex B of one stream. This found D3.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-codec`, scratch target | worktree at 95450d31 | now | exit 0, video_demux 32 passed, every suite green |
| `python3 generate_video.py` check mode, TMPDIR in scratch | worktree fixtures | now | `OK: 39 clips rebuild byte for byte`, exit 0 |
| same, scratch copy with one byte of `hevc_poc.h265` changed | scratch copy | now | `MISMATCH hevc_poc.h265`, exit 1, so the check really rebuilds |
| `shasum -c` over SHA256SUMS | tracked clips | now | 39 of 39 OK |
| harness `dump` then ffprobe packet and frame comparison | all 39 clips | now | every table agrees, see Acceptance challenged |
| ffmpeg as reader of mutated record bytes | copies of `h264_gop12.mp4`, `hevc_main.mp4`, `h264_colr.mp4`, `mpeg2_gop4.mp4` | now | see ISO layouts below |
| harness `fuzz 400`, debug build, overflow checks on | 39 clips, 400 seeded corruptions each, 400 truncations each, three codecs | now | 94,821 runs, 0 panics |
| harness `one` on a crafted SPS | `f241-p1-scaling.h264` | now | panic at h264.rs:283, `attempt to add with overflow`, D1 |
| harness `one` on a crafted `stss` | `f241-p1-stss.mp4` | now | decode index 55 reported random access, ffprobe keys 0, 12, 24, 36, 48, D2 |
| harness `one` on one HEVC stream in MP4 and Annex B | `f241-p1-range.mp4`, `.h265` | now | full_range true from colr in MP4, false from VUI bare, D3 |
| 30 mutants in a `git archive` export | export of 95450d31 | now | 12 red, 18 survived, see Smells |
| `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --all-features` | export of 95450d31 | now | no `ocelli-codec` node |
| `gate wasm` at tip | implementer's `f241-record/gate-wasm-tip.txt` | prior receipt | render 2,179,402, decode 16,455, OK |
| harness `timing`, release, 201 runs per clip | all clips | now | median 0.6 to 36 us per clip |
| `bin/ocelli.sh gate --floor` after `npm ci` | scratch clone of 95450d31 | now | exit 1: 27 passed, 1 failed, 0 skipped. lint, types and packages ran. `wasm` reports render 2,179,402 and decode 16,455, the recorded tip figures. The failure is `test`: `ocelli-oracle` `produce::tests::jpeg_extended_12_level_matches_the_reference` and `dx_window_snap_matches_the_reference_bound` panic at tools/oracle/src/produce.rs:3213 because `tools/oracle/out/run.json` is absent in a clone. The range does not touch `tools/oracle`. sprint/s46 fixed it in b61cfe28, which is not an ancestor of 95450d31. External to this slice |

## Defects

### D1, a hostile SPS panics the demuxer in any build with overflow checks

**Where**: crates/ocelli-codec/src/video/h264.rs:283, `next = (last + delta).rem_euclid(256)` in `skip_scaling_list`
**What**: `delta` is an unbounded `se(v)` up to 2^31 - 1, and `last + delta` overflows `i32`. In the dev and test profiles, overflow checks are on and the demuxer panics. In release it wraps, which happens to give the right residue mod 256, so the shipped behaviour is right only by accident of two's complement.
**Why it is wrong**: Rec. ITU-T H.264 (08/2024) 7.4.2.1.1.1, "The value of delta_scale shall be in the range of -128 to +127, inclusive". An out of range value is a malformed SPS and should be refused by name. The task and the test `damaged_input_never_panics` both claim no panic on damaged input.
**Evidence**: a 20-byte Annex B SPS, profile_idc 100, seq_scaling_matrix_present_flag 1, list 0 present, delta_scale 2^31 - 1 (`f241-p1-craft.py`). Debug harness: `panicked at .../h264.rs:283:20: attempt to add with overflow`, exit 101. Release harness: `Truncated`. The random sweep of 94,821 runs did not reach it, so the existing never-panics test cannot either.
**Scope and regression**: every container, because avcC, TS and Annex B all reach `parse_sps`. Fix: `bits.se()` then refuse outside -128..=127 with `Malformed { reason: "delta_scale outside -128 to 127" }`, and add the crafted SPS as a test that expects that error. A unit test in h264.rs can build the bytes with a small bit writer.

### D2, MP4 tables are read past their own box

**Where**: crates/ocelli-codec/src/video/mp4.rs:529 to :536 (`stss`), :561 to :576 (`stsc`), also :108 (`hdlr`), :194 to :199 (`mdhd`) and :415 to :425 (`colr`)
**What**: these read entries at `box.body + offset` from the whole file buffer, checked against the file length only. An entry count larger than the box reads the next box's bytes as entries. `runs()` and `SampleSizes::read` check `box.end`, and these do not.
**Why it is wrong**: a sample table that does not parse must be refused by name (plan, Scope, refusals, and AC15 "a bad sample table"). Here it is silently accepted with a wrong random access point, so a seek can start the decoder on a non-sync sample.
**Evidence**: `h264_gop12.mp4` with the `stss` entry count raised from 5 to 6 and a 56-byte `free` box placed after `stss`, moov rebuilt with correct sizes (`f241-p1-stss.py`). ffprobe keeps keys at decode 0, 12, 24, 36, 48. The demuxer returns OK and also marks decode index 55, read from the `free` box's size field.
**Scope and regression**: present on 95450d31. Fix: read every table from `data.get(box.range())` and let the existing `be_*` helpers return `Truncated` at the box end. Regression test: the crafted file above, expecting `Truncated { structure: "MP4 sync sample box" }`, and the same for an `stsc` count one too large.

### D3, the container's range replaces a range the stream signals

**Where**: crates/ocelli-codec/src/video/mp4.rs:150 to :159
**What**: when the VUI has video_signal_type_present_flag 1 and colour_description_present_flag 0, `colour_described` is false and the whole `colr` description is taken, its full_range flag included, discarding the video_full_range_flag the stream states explicitly. One stream then gives two ranges by container.
**Why it is wrong**: PS3.5 8.2.8, quoted in the plan, says the characteristics explicitly specified in the compressed stream should control. Plan item 6 reads "the VUI matrix_coefficients and video_full_range_flag, or the MP4 colr box of type nclx when the VUI has none". The VUI here has a range. A range read wrong is a studio against full range conversion downstream, a quietly wrong pixel in every frame.
**Evidence**: `hevc_main10.mp4` (x265 VUI range flag 0, no code points) copied with `-color_range pc ... -movflags +write_colr` to `f241-p1-range.mp4`, colr bytes `6e636c78 0001 0001 0001 80`. Demux of the MP4: `ColrBox, 1, 1, 1, full_range: true`. Demux of the same stream as Annex B: `Vui, 2, 2, 2, full_range: false`. ffprobe on the bare stream: `color_range=tv`.
**Scope and regression**: MP4 only. If the lead reads "has none" as "has no code points", the defect becomes a plan question rather than a code fix, but the current rule still overrides an explicit stream flag. Smallest fix consistent with item 6: code points from `colr` and range from the VUI when the VUI signals a range only, and name both sources. Regression test: the clip above added to `colour_description_from_the_stream_then_colr`.

## Smells

### S1, frame ordering branches that no test executes

**Where**: h264.rs PocType::One (8.2.1.2), the non-reference arm of 8-12, memory_management_control_operation parsing and the mmco5 segment, the 8-3 `>=` boundary, the pps_id clause of 7.4.1.2.4, SEI as an access unit opener, Extended_SAR and the 8-list scaling count. h265.rs nuh_layer_id above 0 ending a VCL run (added in commit 2f04797d), EOS before a CRA, sub-layer PTL skips, SLNR in prevTid0Pic. mpeg2.rs interlaced mb_height and the field picture refusal. ts.rs discontinuity_indicator. mp4.rs mdhd version 1.
**What**: each of these survives a mutation that changes its behaviour. Mutants that survived: R2, R3, R4, R5, R6, R7, R8, R9, R10, R11, R12, R13, R14, R16, R18, R19. R15 (avcC SPS length one short) and R20 (PMT reparsed) also survived and are judged equivalent, because the parser never needs the SPS trailing bits and a repeated PMT is the same table.
**Failure mechanism**: these branches decide which picture is frame N. A later edit to any of them goes unnoticed, and `ac4_poc_type_2_follows_frame_num` cites 8.2.1.3 while its clip has no non-reference picture, so the 8-12 arm it implies is covered is not. The nuh_layer_id change is a remediation no test has seen.
**Fix**: unit tests in h264.rs and h265.rs that feed `PocState::next` and the HEVC POC step with hand-built Sps and Slice values, expected values worked from 8-6 to 8-12, 8-1 and 8-2 in comments, and crafted short byte streams for the layer, EOS and discontinuity cases. x264 cannot write POC type 1 or adaptive marking, so these need constructed inputs rather than clips.

## Nitpicks

- N1, h265.rs:24 to :27. The comment derives "below MaxDpbSize" with MaxDpbSize at most 16, which gives at most 15, while `MAX_DELTA_POCS` is 16. Lenient and harmless, but the comment and constant disagree.
- N2, h264.rs:532 to :537. modification_of_pic_nums_idc 4 and 5 belong to the MVC syntax, the 7.3.3.1 table has no field for them, and the `_` arm is unreachable after `ue_max(5)`.
- N3, mp4.rs. pts are composition times without the edit list, a constant 512 or 1024 ticks from ffprobe on these clips. Order and spacing are right. The plan lists no `elst`, so this is noted, not required.
- N4, ts.rs:170. current_next_indicator is not checked, so a "next" PAT or PMT would be used.

## Verified clean

- Clause check against the published texts in this session's scratchpad: Rec. ITU-T H.264 (V15) (08/2024), H.265 (V10) (07/2024), H.222.0 v9 (08/2023), H.262 (02/2000), text extracted by the reviewer with pdftotext. H.264 7.3.2.1.1 with the 2024 profile_idc list, 7.3.2.1.1.1, E.1.1 to the colour fields, 7.3.2.2, 7.3.3 to dec_ref_pic_marking, 7.3.3.1 to 7.3.3.3, 7.4.1.2.3, 7.4.1.2.4, 8.2.1, 8.2.1.1 to 8.2.1.3 including the mmco5 rules: all field widths and order match. Crop units match Table 6-1. H.265 7.3.1.2, 7.3.2.2.1, 7.3.3 including the 88-bit sub-layer block and reserved_zero_2bits, 7.3.4, 7.3.7 with 7-61 and 7-62, 7.3.2.3.1, 7.3.6.1 to slice_pic_order_cnt_lsb, E.2.1 to field_seq_flag, 7.4.2.4.4 including NOTE 1, 8.3.1 with prevTid0Pic: all match. H.222.0 Table 2-2, Table 2-5, Table 2-6, 2.4.3.3 continuity and duplicates, Table 2-21, Table 2-22 headerless ids, Table 2-30, Table 2-33, Table 2-34 values used, 2.4.3.7 commencement rules for H.262, AVC and HEVC, Annex A polynomial and taps: all match. H.262 6.2.2.1, 6.2.2.3, 6.2.2.4, 6.2.3, 6.2.3.1, Table 6-1, 6.3.3 sizes and the zero-value rule for both sizes, 6.3.9, 6.1.1.6: all match.
- ISO layouts, confirmed with ffmpeg as an independent reader. avcC: lengthSizeMinusOne is the low two bits of byte 4 (changing them makes ffmpeg report 430 decode errors, changing only the reserved bits gives 0), SPS length at byte 6 and SPS at byte 8, PPS after it, both byte-equal to the units ffmpeg's h264_mp4toannexb wrote. hvcC: lengthSizeMinusOne is the low two bits of byte 21 (51 errors, high bits and bytes 19 and 20 give 0), numOfArrays at 22, the array layout consumes the 2,448-byte record exactly, and VPS, SPS and PPS equal ffmpeg's hevc_mp4toannexb output. colr nclx: primaries, transfer and matrix at body offsets 4, 6 and 8 and full_range in the top bit of byte 10, each moving ffprobe's reported value when changed. nclc matches the QuickTime layout. esds: objectTypeIndication found at body offset 13 by the descriptor walk, and changing it to 0x20 makes ffprobe report mpeg4. Visual sample entry 78 bytes and mdhd version 0 confirmed by ffprobe agreeing on every sample offset and the 1/12800 time base. avc1 codec strings equal ffmpeg's DASH writer for three clips, case aside.
- Test oracle: no expected table comes from `demux`. The box and TS readers are separate code, and ffprobe now independently agrees with what they expect. The two relative tests (duplicate packet, empty mvex) compare demux with itself by design.
- Generator: reproducible, check mode proven to rebuild, all clips are lavfi testsrc2 or sine or hand-built from them, no DICOM. `.m2t`, `.h264`, `.h265`, `.m2v`, `.mpg`, `.pes` and `.bin` appear in no suffix list of `scripts/` or `bin/ocelli.sh` except `.ts` in `source_provenance_check.py`, which the naming avoids.
- Incident: the working tree equals 95450d31, nothing staged or modified, reflog shows four commits on the branch base and nothing else, the clip digests verify and the generator rebuilds all 39. Nothing was lost.
- Scope: `git diff --name-only a2b3098b..95450d31` touches only `crates/ocelli-codec/`. No unsafe, no new dependency, no `as` cast in the module, every index through `get`.
- Size: `ocelli-codec` is in neither wasm graph, so the delta of 0 holds by construction, and the recorded tip sizes are within budget.
- Mutants killed: R1 (emulation prevention), R17 (PTS_DTS_flags), R21 (stss zero-based), R22 (PTS bit field), R23 (8-3 wrap), R24 (8.3.1 wrap), R25 (MPEG-2 group), R26 (random access), R27 (nclx range bit), R28 (presentation sort), R29 (4:2:0 crop unit), R30 (compatibility bit reverse).

## Limitations and handoff

- The ITU texts could not be fetched now (the download was refused) and were read from copies already in this session's scratchpad, genuine ITU PDFs with their title pages checked. H.262 is the 02/2000 edition, as the implementer cited.
- ISO/IEC 14496-12, 14496-15 and 14496-1 were not read. The HEVC dotted codec string (14496-15 Annex E) is not confirmed: no free source with an example was available locally and ffmpeg writes none. Its fields are right per H.265 7.3.3, and only the textual form is from memory.
- The implementer's six survivors and their reasons were not in my inputs, so I could not judge them individually. The self-review says 5 of 58, and the brief says six. My own catalogue stands in their place.
- The implementer's timing figures were not in my inputs, so mine are reported without a comparison. They come from an opt-level 3 harness build, not the workspace's opt-level z release.
- Not exercised: multi-program TS, a PES with an empty payload, field-coded H.264 outside MP4, any real-world clip.
- The floor's one red gate is external, see Evidence. It clears when the branch takes b61cfe28 from sprint/s46.
- Next: implementation remediates D1, D2, D3 and S1, then a fresh independent pass 2 reviews the changed material.
