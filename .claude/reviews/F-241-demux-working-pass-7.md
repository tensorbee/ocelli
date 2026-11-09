# F-241 review, pass 7, Rust demux slice

**Reviewed**: work/f-241-demux, range 00401cca..6c2a78c4 at tip 6c2a78c4, one commit (6c2a78c4, pass 6 remediation). It touches `crates/ocelli-codec/src/video/h264.rs` and `h265.rs` only. Before and after the pass, `git status` shows only the untracked `.venv` and `tools/oracle/out`, plus this file.
**Author / reviewer**: the author is a Claude implementer agent, and the commit carries the operator's git identity. The reviewer is an independent Claude reviewer agent that wrote none of the range.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 6 S1, the reverse sweep ruling, the starter rule and PPS row, N1 and N2**, each replayed with the mutant or trace that raised it.
- **Both sweeps**, rerun from the implementer's unchanged scripts against a `git archive` of the tip: the pass 5 forward sweep (89 mutants) and the pass 6 reverse sweep (42 mutants).
- **The two remaining equivalence arguments**, `E-no-output-from-15` (new) and `E-no-output-to-cra` (carried), judged from the call graph and Table 7-1.
- **Every new row against its clause**, by ffmpeg 9.0.2 `-bsf:v trace_headers` as a host tool on the dumped row streams, HEVC with pass 3's VPS in front.
- **16 reviewer mutants** aimed at the new rows and the new test, in both directions where a condition has two.
- **No valid stream refused**: pass 3's 117 clips and the six b-pyramid and VideoToolbox clips, dumped and compared with ffprobe.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -q -p ocelli-codec`, scratch `CARGO_TARGET_DIR` | `git archive 6c2a78c4` (`f241-p7-sweep`), diffed equal to the worktree | now | exit 0. lib 46 passed (45 at pass 6, plus the new starter test), video_demux 45 passed, every other target green |
| `cargo clippy -q -p ocelli-codec --all-targets -- -D warnings`, `cargo fmt -p ocelli-codec --check` | same export | now | both exit 0 |
| `f241-impl-sweep.py <export> --check`, then the full run | implementer's pass 5 script, unchanged | now | 89 of 89 anchors unique. 87 killed, `H-marking-idr-width` and `E-no-output-to-cra` survive. `f241-p7-forward.log` is line for line `f241-record/sweep5-p6-after.log`, kill lists included, and verdict for verdict pass 6's `f241-p6-sweep.log` |
| `f241-impl-reverse.py <export> --check`, then the full run | implementer's pass 6 script, unchanged | now | 41 of 42 anchors. `E-reserved-after-vcl-dropped` is `ANCHOR x0`, since N1 removed the clause it targets. 40 killed, `E-no-output-from-15` survives. `f241-p7-reverse.log` is byte for byte `f241-record/reverse-p6-after.log` |
| the reverse log's first 33 rows against pass 6's `f241-p6-extra.log` | same order, same mutants | now against prior receipt | the 8 pass 6 survivors: 6 now killed, 1 retired by anchor, 1 (`E-no-output-from-15`) still surviving and argued equivalent. The other 25 keep KILLED |
| `f241-p7-extra.py`, 16 reviewer mutants with the same harness | second tip export (`f241-p7-own`) | now | 12 killed, 4 survive. Log `f241-p7-extra.log`, discussed below |
| `OCELLI_F241_ROW_DUMP=f241-p7-rows cargo test --lib every_` | `f241-p7-own` | now | 187 row streams. Against pass 6's 183 in `f241-p6-rows`: 4 new, 5 HEVC segments rows changed, every other stream byte for byte equal, including the two type 1 rows pass 6 proposed |
| ffmpeg `trace_headers` on every new or changed row, `f241-p7-trace.py` (the implementer's trace script with only its scratch path changed) | `f241-p7-rows` | now | field order and values as below. No trace complaint on any accepted row. `pps sps id 16` is refused by ffmpeg too |
| harness `dump` built against the worktree at 6c2a78c4, then `f241-p2-ffcmp.py` | `f241-p3-clips`, 117 clips | now | `117 clips compared, 0 disagree`. Dump byte for byte `f241-p3-dump-fresh.tsv` |
| same | `f241-p5-clips`, `h264_bpyramid` and `h264_p5_vt_b3` as .h264, .mp4 and .m2t | now | `6 clips compared, 0 disagree`. Dump byte for byte `f241-p5-dump-new.tsv` |
| `python3 scripts/prose_check.py`, plus a grep of the added lines and the commit message for U+2013, U+2014, semicolons, `as` casts, `unsafe` and `#[allow]` | worktree at 6c2a78c4 | now | exit 0, no hits |

## Pass 6 findings replayed

- **S1, closed.** `Craft.always_zero` is written at h264.rs:1625 and tested at :1723 as `self.poc_type == 1 && !self.always_zero`, which is 7.3.3's condition. The two rows at :2848 and :2857 are byte for byte the streams pass 6 proposed. `R-type1-delta-when-always-zero` is killed by "delta_pic_order_always_zero_flag 1" and `R-type1-delta1-always` by "delta_pic_order_cnt 3, no bottom". `trace_headers` reads the first as delta_pic_order_always_zero_flag 1 with no delta in either slice, and the second as bottom_field_pic_order_in_frame_present_flag 0 with one delta_pic_order_cnt, value 3, in the P slice. My `P7-type1-delta0-when-always-zero`, which reads only delta[ 0 ] when the flag is 1, is killed by the same row.
- **The reverse sweep, closed.** It holds pass 6's 33 in pass 6's order plus 9 of the implementer's, and reruns to 40 of 42 as reported: 40 killed, 1 equivalent, 1 retired.
- **The starter rule, closed.** `reserved_non_vcl_41_to_44_start_an_access_unit_at_layer_0` (h265.rs:1281) kills both pass 6 survivors, `E-nvcl41-not-starter` and `E-nvcl-44-not-starter`. It also kills my three adjacent mutants, which together pin both ends of the range and the layer test: `P7-starter-to-45` (45 made a starter), `P7-starter-from-42` (41 dropped) and `P7-starter-any-layer` (`layer == 0` replaced by `true`).
- **The PPS row, closed.** "sps id 15" kills `E-pps-sps-id-bound` (bound 14) and my `P7-sps-id-bound-14`. "pps sps id 16" kills `P7-pps-sps-id-bound-16`. `trace_headers` reads sps_seq_parameter_set_id 15 and pps_seq_parameter_set_id 15 in the first, and pps_seq_parameter_set_id 16 in the second, which ffmpeg refuses with "must be in [0,15]". The row's citation, 7.4.3.3.1, is where H.265 says pps_seq_parameter_set_id is 0 to 15.
- **N1, closed.** h265.rs:800 is now `kind <= nal::VCL_LAST && picture.is_some()`. Restoring the clause (`P7-after-vcl-not-reserved`) survives, as pass 6's argument predicts, so removing it changed nothing.
- **N2, closed.** The second TRAIL_R segment is written with lsb 3 (h265.rs:1261). `trace_headers` reads slice_pic_order_cnt_lsb 3 and 3 for the two segments in every non-dependent segments row (base, 7 extra bits, colour_plane_id 2, pic_output_flag 0, sps id 15), and dependent_slice_segment_flag 1 with no lsb in the dependent row, which is 7.4.7.1's "when present ... the same in all slice segment headers of a coded picture". These five are the only streams that changed against pass 6's dump.

## Defects

None.

## Smells

None.

## Nitpicks

- N1, crates/ocelli-codec/src/video/h264.rs:2918. "Operation 5 has no field of its own, Table 7-9." The statement is true, but Table 7-9 only names what each operation does. That operation 5 carries no field comes from the dec_ref_pic_marking syntax of 7.3.3.3, so that is the clause to cite.

## Verified clean

- **"nal_ref_idc 3, not IDR"** (h264.rs:2932). The builder writes `(3 << 5) | 0x01`, 0x61. `trace_headers` reads nal_ref_idc 3 and nal_unit_type 1 in the P slice, no idr_pic_id, adaptive_ref_pic_marking_mode_flag 1, then operations 5 and 0. By 7-1 in 7.4.1, IdrPicFlag is `nal_unit_type == 5` and nothing else, as the comment says. The row kills pass 6's `R-idr-from-type` and my `P7-idr-from-ref-idc-3`. `P7-ref-idc-low-bit` and `P7-ref-idc-high-bit` (one bit of nal_ref_idc dropped) are killed too.
- **"mmco 5 then slice_qp_delta 4"** (h264.rs:2923). `trace_headers` reads operation 5 as `00110`, operation 0 as `1`, then slice_qp_delta `0001000 = 4`. By Table 9-3 a positive se(v) value k has codeNum 2k - 1, so 4 is codeNum 7, which is the code shown. 7.3.3.3 reads no field after operation 5. A reader that took a ue(v) field there would take operation 0's `1` as the field and then read codeNum 7 as an operation, which `ue_max(6, ..)` refuses, so the comment's mechanism is right. It kills `R-mmco5-reads-field` (a ue(v) field) and my `P7-mmco5-reads-flag` (a one-bit field), and `P7-mmco-bound-7` is killed by the existing "mmco 7" row.
- **The starter test against 7.4.2.4.4.** The clause lists RSV_NVCL41 to RSV_NVCL44 with nuh_layer_id 0 among the units that open an access unit after the last VCL unit. Types 45 to 47 are in the list that "shall not precede the first VCL NAL unit", and NOTE 3 says units above layer 0 are not used for boundary detection. The test's five cases, (41, 0) and (44, 0) starting and (45, 0), (41, 1) and (44, 1) not, are exactly that. The header bytes follow 7.3.1.2: type in bits 1 to 6, the layer's top bit in bit 0, its low five bits above nuh_temporal_id_plus1 1. The unit is inserted at index 5 of `Segments::units`, which is between the second TRAIL_R segment and the CRA. The expected table moves a starter's bytes into the CRA's access unit and any other unit's bytes into the TRAIL_R's, and keeps every order and random access flag, which is what the clause and 7.4.2.2 give for a unit the demuxer only bounds.
- **`E-no-output-from-15` is equivalent.** It widens no_output_of_prior_pics_flag's range at h265.rs:570 from 16..=23 to 15..=23, so it differs only for type 15. `first_slice_poc_lsb` has one caller, h265.rs:784, in the `0..=nal::VCL_LAST` arm, and the arm before it at :781 takes every `reserved_vcl` type, 15 among them (Table 7-1 reserves RSV_VCL_R15). So the function never sees type 15. That arm is itself guarded: `E-reserved-arm-dropped` is killed by `a_reserved_vcl_unit_is_ignored`. I accept the argument. `E-no-output-to-cra` is equivalent for the same reason with 22 and 23, and `H-marking-idr-width` keeps pass 6's accepted argument, since the range does not touch `parse_slice`.
- **Earlier rows keep their verdicts.** The forward log matches pass 6's verdict for verdict. Of the 183 row streams pass 6 dumped, 178 are byte for byte unchanged, and the 5 that changed differ only in N2's lsb and still pass. The builder's new fields default to the old constants (`qp_delta` 0, `ref_idc` 2 giving 0x41, `always_zero` false, `Sps.id` 0 giving the old `ue(0)`, `pps_sps_id` falling back to `sps.id`), so no existing row's bytes moved through them.
- **No valid stream refused.** 117 of 117 of pass 3's clips and 6 of 6 b-pyramid and VideoToolbox clips agree with ffprobe, and both dumps are byte for byte the earlier passes'.
- **Scope and prose.** The only non-test change is N1's clause removal at h265.rs:800. Everything else is under `#[cfg(test)]`. No `unsafe`, no `as` cast, no `#[allow]`, no dependency. The shifts in the new builder code fit `u8` (at most `45 << 1` and `3 << 5`). Prose check is clean. The three user documents mention none of this module. The commit message's claims (33 plus 9 mutants, every survivor but one killed, one argued equivalent, the starter test, the 15 and 16 row, the clause gone, both segments carrying the lsb) all check against the logs and traces above.

## Limitations and handoff

- **Outside this range, not counted.** Three of my mutants survive on lines the range did not write:
  - `P7-starter-unspec-dropped`: the UNSPEC48 to UNSPEC55 starters at h265.rs:752 have no test. 7.4.2.4.4 lists them beside 41 to 44, so a case for 48 and 55 in the new test would close it.
  - `P7-sps-id-bound-16`: the SPS parser's own bound at h265.rs:328 has a row at 15 but none at 16. Under the mutant an SPS with id 16 is accepted and dropped by `get_mut`, where 7.4.3.2.1 makes it a refusal.
  - `P7-after-vcl-layer0-only` is equivalent. At h265.rs:800 `after_vcl` already equals `picture.is_some()` after any VCL unit, the same invariant pass 6 used for N1.
- **A correction to pass 6's record.** Pass 6 reported its 33 extra mutants as "26 killed, 7 survive". Its own `f241-p6-extra.log` shows 25 killed and 8 surviving, the eighth being `E-no-output-from-15`, which pass 6 discusses in its text. The verdicts were right and only the count was off.
- Not exercised: field-coded streams outside MP4, multi-program TS, MV-HEVC. No oracle, GPU or browser job was run, as instructed, and `gate --floor` was not rerun.
- **This is the converging pass.** Both sweeps hold at the reported verdicts, every pass 6 finding is closed, and nothing new blocks. Scratch inputs are in the session scratchpad under `f241-p7-`: both sweep logs, the extra mutant script and log, the row dumps, the trace script and the clip dumps. The source exports and target directories were deleted after the pass.
