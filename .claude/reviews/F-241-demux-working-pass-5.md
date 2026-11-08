# F-241 review, pass 5, Rust demux slice

**Reviewed**: work/f-241-demux, range 5bf50302..e16ee42e at tip e16ee42e, one commit. Before and after the pass, `git status` shows only the untracked `.venv` and `tools/oracle/out` symlinks, plus this file.
**Author / reviewer**: the author is a Claude implementer agent, and the commit carries the operator's git identity. The reviewer is an independent Claude reviewer agent that wrote none of the range.
**Result**: 0 defects, 1 smell, 1 nitpick

## Acceptance challenged

- **Pass 4 S1 and N1**, replayed with pass 4's own two surviving mutants and with a dump into a directory that does not exist.
- **The new builder paths against 7.3.3, 7.3.3.1 and 7.3.3.2**, field by field from Rec. ITU-T H.264 (V15) (08/2024), then through ffmpeg `trace_headers` as an independent reader on every new row.
- **The 7.4.3 bound after operation 5**, from the clause text: ( 1 - MaxPicOrderCntLsb ) to 2^31 - 1. log2_max_pic_order_cnt_lsb_minus4 0 gives MaxPicOrderCntLsb 16, so -15 is the bound and -16 one past it.
- **The POC expectations of `reference_b_slice_reads_through_to_its_marking`**, derived by hand from 8.2.1 and 8.2.1.1 before running anything.
- **The implementer's claim about the committed clip**, by tabulating every slice header of `h264_bpyramid.h264` under `trace_headers`, and the question of whether any host encoder reaches the two paths, by encoding with every H.264 encoder ffmpeg offers here.
- **Four mutants of my own**, each a plausible wrong edit to the B path. One survives (S1).
- **No valid stream refused.** Pass 3's 117 fresh clips, the three committed b-pyramid clips and a fresh VideoToolbox B-frame clip on three routes.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -q -p ocelli-codec`, scratch `CARGO_TARGET_DIR` | `git archive` export of e16ee42e (`f241-p5-mut`) | now | exit 0. lib 42 passed, video_demux 45 passed, every suite green |
| `cargo clippy -q -p ocelli-codec --all-targets -- -D warnings` | same export | now | exit 0 |
| `OCELLI_F241_ROW_DUMP=f241-p5-rows cargo test -p ocelli-codec --lib every_` | same export | now | 160 files, the 153 of pass 4 plus the 7 new rows |
| `OCELLI_F241_ROW_DUMP=<missing dir>/x cargo test --lib every_` | same export | now | FAILED, 2 tests, each `OCELLI_F241_ROW_DUMP: cannot write <path>/h264_base.h264: Some(Os { code: 2, kind: NotFound ... })`. No directory was created |
| ffmpeg `trace_headers` on the 7 new rows | `f241-p5-rows` | now | field order as 7.3.3. Only complaints: `out=[-128,127]` on "l1 weight 128" and "l1 offset -129", plus slice-data concealment lines from the decoder |
| `trace_headers` on `h264_bpyramid.h264`, every slice header tabulated | committed fixture, SHA256SUMS `shasum -c` OK | now | 48 slices. PPS weighted_bipred_idc 2. 7 reference B slices, all ref_pic_list_modification_flag_l1 0, no B pred_weight_table |
| VideoToolbox `-bf 3 -profile:v high`, then `trace_headers` | `f241-p5-vt.h264`, 48 frames | now | weighted_bipred_idc 0, 8 reference B slices, all ref_pic_list_modification_flag_l1 0 |
| the generator's x264 arguments for `h264_bpyramid.mp4`, re-run | host ffmpeg and libx264 | now | sha256 c3a13d91...85fd, byte for byte the committed file |
| `f241-p5-mutate.py`, 12 mutants, lib tests and video_demux | export of e16ee42e | now | 11 killed, 1 survived (`P5-l1-count-from-l0`). Log `f241-p5-mutate.log` |
| pass 4's two mutants, every row printed | same export, restored and diffed equal to e16ee42e after | now | listed under Verified clean |
| S1 fix in a second export (`f241-p5-fix`), two rows | e16ee42e plus the rows | now | green as written. Red under `P5-l1-count-from-l0`: both rows `memory_management_control_operation over 6` |
| harness `dump` built against the worktree, then `f241-p2-ffcmp.py` | pass 3's 117 clips in `f241-p3-clips` | now | 117 of 117 OK, `117 clips compared, 0 disagree`, dump byte for byte `f241-p3-dump-fresh.tsv` |
| same, on `f241-p5-clips` | the three committed b-pyramid clips and the VideoToolbox clip as .h264, .mp4 and .m2t | now | 6 of 6 OK, `6 clips compared, 0 disagree` |

## Defects

None.

## Smells

### S1, the list 1 weight loop's count is never different from list 0's, so a parser that uses the wrong count stays green

**Where**: crates/ocelli-codec/src/video/h264.rs:602, `(slice_type == B).then_some(l1)`, which takes its count from :581. Every new weight row (h264.rs:2285 to :2307) and the POC test (h264.rs:2665) builds on `reference_b` (h264.rs:1802), which sets no override, so num_ref_idx_l0_active_minus1 and num_ref_idx_l1_active_minus1 are both the PPS default 0.
**What**: mutant `P5-l1-count-from-l0` replaces `then_some(l1)` with `then_some(l0)`, so 7.3.3.2's list 1 loop runs num_ref_idx_l0_active_minus1 + 1 times. It survives the lib tests and video_demux. A mutant that drops the l1 override at :581 leaves l1 at the PPS default and would pass for the same reason. That one was not run.
**Why it matters**: 7.3.3.2 bounds the list 1 loop by num_ref_idx_l1_active_minus1, which 7.3.3 sets from the override or from num_ref_idx_l1_default_active_minus1. A parser with the wrong count misaligns every field after the table in any explicit-weighted B slice where the two counts differ, which includes dec_ref_pic_marking. So it is the mechanism of pass 4 S1 one step further down. A missed or invented operation 5 moves the picture under 8.2.1 with no error. The lead ruling for pass 4 asks for rows that exercise "the l1 loop of 7.3.3.2", and the loop's bound is the part of it that is not exercised.
**Evidence**: `f241-p5-mutate.log`, `P5-l1-count-from-l0 SURVIVED`. In `f241-p5-fix`, these two rows after "reference B bottom -15 with mmco 5" are green as written and red under the mutant:
```rust
("reference B l1 count 2", Craft { override_l0: Some(0), override_l1: Some(1),
    weights_l1: vec![(-4, 5), (6, -7)], ..reference_b() }, None),
("reference B l1 count 2, weight 128", Craft { override_l0: Some(0), override_l1: Some(1),
    weights_l1: vec![(-4, 5), (128, 0)], ..reference_b() }, Some("prediction weight")),
```
`trace_headers` reads the first as num_ref_idx_l0_active_minus1 0, num_ref_idx_l1_active_minus1 1, two l1 entries (-4, 5) and (6, -7), then operations 5 and 0. It reads the second's second l1 weight as 128 `out=[-128,127]`. Under the mutant both rows give `memory_management_control_operation over 6`.
**Fix**: add the two rows above, or equivalent rows where the l1 count is above the l0 count and a second l1 entry carries the refusal.
**Scope and regression**: present at e16ee42e. No verdict on a stream changes today. The consumer is `h264::access_units` and through it every container route. No host encoder writes weighted_bipred_idc 1 (see Verified clean), so synthetic rows are the only coverage for this loop.

## Nitpicks

- N1, crates/ocelli-codec/src/video/h264.rs:1802. `reference_b` sets `bottom: true` and `delta_bottom: -15`, and `slice(true)` writes that delta into the IDR as well. The IDR is then TopFieldOrderCnt 0 and BottomFieldOrderCnt -15, and 8.2.1 says a bitstream shall not give Min( TopFieldOrderCnt, BottomFieldOrderCnt ) other than 0 for a coded IDR frame. So the accepted rows built on `reference_b` are not conforming at the IDR, for a reason unrelated to the B slice. The parser does not check that constraint, so no row changes verdict. The pre-range rows "bottom -15 with mmco 5" and its neighbours have the same shape. If that constraint is ever checked, these rows go red for the wrong reason. Writing delta_pic_order_cnt_bottom 0 in the IDR, or setting `bottom` only for the rows that need it, avoids that.

## Verified clean

- **Pass 4 S1, closed for both named mutants.** `P4-no-l1-modification` turns all 7 new rows red, each with `memory_management_control_operation over 6`, and also fails `reference_b_slice_reads_through_to_its_marking`. `P4-no-l1-weights` turns 6 of 7 red and fails the POC test. The one it misses is "l1 abs_diff 16", which is refused inside the modification and so before the weights. That matches the commit message's "where its read precedes the refusal". The remaining gap in the same loop is the new S1.
- **Pass 4 N1, closed.** bits.rs:365 to :371 asserts on the write's result and names the path, and a dump to a missing directory fails the run with the path in the message, as in the evidence table.
- **The builder against 7.3.3, 7.3.3.1 and 7.3.3.2.** `trace_headers` reads the "l1 abs_diff 15" B slice as nal_ref_idc 1, nal_unit_type 1, slice_type 6, frame_num 1, pic_order_cnt_lsb 2, delta_pic_order_cnt_bottom -15, direct_spatial_mv_pred_flag 1, num_ref_idx_active_override_flag 0, ref_pic_list_modification_flag_l0 0, ref_pic_list_modification_flag_l1 1, modification_of_pic_nums_idc 0, abs_diff_pic_num_minus1 15, modification_of_pic_nums_idc 3, luma_log2_weight_denom 2, chroma_log2_weight_denom 1, luma_weight_l0_flag 1, weight 3, offset -2, chroma_weight_l0_flag 0, luma_weight_l1_flag 1, weight -4, offset 5, chroma_weight_l1_flag 0, adaptive_ref_pic_marking_mode_flag 1, operations 5 and 0, slice_qp_delta 0. That is the syntax table's order exactly, with chroma_log2_weight_denom and the chroma flags present because ChromaArrayType is 1. The PPS writes weighted_bipred_idc as u(2) from `bipred`, and the NAL header 0x21 is nal_ref_idc 1 with type 1. The other six rows read the intended value at the intended field: abs_diff 16, l1 weight 127 with offset -128, weight 128, offset -129 and delta_pic_order_cnt_bottom -16.
- **The bounds the rows test.** abs_diff_pic_num_minus1 is 0 to MaxPicNum - 1 by 7.4.3.1, and MaxPicNum is MaxFrameNum, 16 here, so 15 and 16. The l1 weights and offsets have the l0 semantics by 7.4.3.2, -128 to 127. The bound after operation 5 is 1 - 16 = -15, and the parser checks it at h264.rs:635 to :642 only after the marking is read, so the -16 row is refused only when the read reaches the operation.
- **The POC test by hand, 8.2.1.1.** The IDR gives prevPicOrderCntMsb 0 and prevPicOrderCntLsb 0. The B picture has pic_order_cnt_lsb 2, so PicOrderCntMsb 0 and TopFieldOrderCnt 2. With operation 5, 8.2.1 sets tempPicOrderCnt to 2 and TopFieldOrderCnt to 2 - 2 = 0, and the operation empties the DPB, so the picture starts segment 1: (1, 0). Without it the picture stays in segment 0 at POC 2: (0, 2). Both are what the test asserts.
- **The implementer's claim about the committed clip is true.** All 48 slices were tabulated. The PPS has weighted_bipred_idc 2, which is implicit prediction with no pred_weight_table in a B slice, and weighted_pred_flag 1, so 21 P slices carry one. There are 7 reference B slices, nal_ref_idc 2, every one with ref_pic_list_modification_flag_l1 0. Six of them carry adaptive marking with one operation 1, which is what the generator comment at generate_video.py:220 to :224 says. 18 B slices are non-reference. So the clip exercises reference B marking but reaches neither pass 4 mutant.
- **No host encoder reaches either path.** ffmpeg here offers libx264, libx264rgb and h264_videotoolbox. x264 writes weighted_bipred_idc 2 and never sets the l1 flag. A VideoToolbox clip with `-bf 3` writes weighted_bipred_idc 0 and 8 reference B slices, all with the l1 flag 0. Explicit bi-predictive weights and list 1 reordering exist in the syntax but no encoder available to this pass writes them, so the synthetic rows are the coverage, and that is acceptable.
- **Own mutants.** `P5-ref-B-no-marking` skips dec_ref_pic_marking for B slices and is killed by "reference B bottom -16 with mmco 5" (accepted) and by the POC test. `P5-bipred-idc-2` reads pred_weight_table for B at weighted_bipred_idc 2 rather than 1 and is killed by both l1 weight refusals, the -16 row and the POC test. `P5-l1-mod-no-diff` lifts the abs_diff bound in list 1's modification only and is killed by "l1 abs_diff 16". `P5-l1-count-from-l0` survives, S1. Pass 4's six other mutants are still killed.
- **The b-pyramid test.** The clip is regenerated byte for byte by the generator's arguments, its three SHA256SUMS lines check, and the harness gives 48 samples on each route with ffprobe agreeing on packet position, size, key flags and a single pts offset for MP4 and TS, and on display order for Annex B. ffprobe reports has_b_frames 2.
- **No valid stream refused.** 117 of 117 of pass 3's clips, with the dump identical to pass 3's, and 6 of 6 new clips. An empty .m2t produced by a failed remux of mine was refused as `UnknownContainer`, which is correct, and was replaced by a remux from the MP4.
- **Scope and prose.** The range touches `crates/ocelli-codec/src/video/` only inside `#[cfg(test)]`, plus fixtures and one generator entry. It adds no `unsafe`, no `as` cast, no `#[allow]` and no dependency. No added line or the commit message has an em-dash, an en-dash or a prose semicolon, and `scripts/prose_check.py` is clean. The three user documents mention none of this module.

## Limitations and handoff

- The second mutant named under S1, the l1 override dropped at h264.rs:581, was reasoned about and not run.
- The new rows' list modifications name pictures that do not exist in a two-picture stream, which 8.2.4.3.1 restricts. The parser checks only the syntax bound, as for list 0, so this is recorded here and not as a finding.
- Not exercised: field-coded streams outside MP4, multi-program TS and MV-HEVC. No oracle, GPU or browser job was run, as instructed, and `gate --floor` was not rerun, since the range is test code and fixtures.
- Next: implementation adds the S1 rows and optionally takes N1, then a fresh independent pass 6. Scratch inputs are in the session scratchpad under `f241-p5-`: the rows, the clips, the traces, the mutation script and its log and both dumps. The two source exports were deleted after the pass.
