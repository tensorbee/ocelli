# F-X021 review, pass 5

**Reviewed**: uncommitted working tree on branch `sprint/s20`. `git rev-parse HEAD` is `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. Nothing was staged. `git diff --stat` is 23 files, 1492 insertions and 138 deletions. Untracked product sources are `crates/ocelli-codec/src/jpegls/sample_interleaved.rs` and `tools/oracle/src/produce_current.rs`. Untracked prior reviews `F-X021-produce-pass-1.md` through `pass-4.md` were not part of the product diff. This pass is an independent review. The reviewer did not author the diff.
**Author / reviewer**: working-tree author is whoever produced the uncommitted F-X021 tree. Reviewer is an independent microscope pass and did not write that tree. Three constants were mutated during the pass and restored. After restore, `git status --short` and `git diff --stat` matched the tree recorded at the start.
**Result**: 5 defects, 0 smells, 1 nitpick

## Acceptance challenged

Challenged the design rule that candidate bytes are a function of Ocelli decode, `LutChain::map_into`, and F-052 `Volume::reslice`, with no new pixel formula. The discriminating case is synthetic SIGMOID centre 40 and file width 0.5. PS3.3 C.11.2.1.3.1 at modality 39.5 is `255 / (1 + exp(4))`, about 4.586, whose nearest byte is 5. The recovered window is centre 40 and width 1.5, whose value at the same modality is about 53.2 and whose nearest byte is 53.

The reference sidecar still records `windowWidth` 0.5 and `voiLutFunction` SIGMOID. The reference frame digest is `03da6fdededcadf69d33416c37ec80bd5a32c3ded31cf685bc122eda16155aea`. Sampling the 12 by 20 content rectangle, stored column 7 is modality 39.5 and its grey code is 53 on both the reference raw and the candidate raw. The product fixture `sigmoid_width_below_one_keeps_the_file_width` still expects 4.586. Forcing `presented_sigmoid_window` to keep the file width made `sigmoid_width_below_one_follows_the_recovered_window` fail. The low sample was 0 where the recovered expectation is 1. That mutation was reverted and the same test then passed.

This rejects a candidate that runs the product LUT on the file window. It also rejects a byte copy of the reference. The sigmoid render hashes differ, and 21504 grey samples differ by exactly one code. Those one-code pairs are reference 127 against candidate 128, and reference 239 against candidate 238. `row/sha256` matches across the two sidecars. The candidate `frame.sha256` equals the SHA-256 of `/tmp/ocelli-fx021-fix/synthetic__ct_sigmoid_width_half.raw`.

A second presentation check is stored 149 at LINEAR centre 40 and width 400. The continuous display is about 197.48. Nearest half-up is 197. `map_preset_into` emits 198. Changing the ramp width from 1024 to 256 made `grey_linear_matches_the_reference_1024_ramp` fail with bytes 49, 58, 60 and 61 against 198, 232, 239 and 246. That mutation was reverted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat` | `sprint/s20` worktree, nothing staged | Executed at start and again after the three reverts | HEAD `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. Stat unchanged at 23 files, 1492 insertions, 138 deletions |
| Read `/tmp/ocelli-fx021-fix-gate/compare.json` | Candidate `/tmp/ocelli-fx021-fix`, reference `tools/oracle/out` | Prior receipt, not regenerated | `gateVerdict` pass, 102 views, 71 pass, 31 unmeasured, 0 fail, 0 absent, 1 declared volume refusal, 2 unsupported source rows. Run hashes differ |
| Sigmoid column codes and hash trace | `synthetic__ct_sigmoid_width_half`, file window 0.5 | Executed now, aggregates only | Modality 39.5 is code 53 on both sides. 21504 grey samples differ by 1. No sample differs by more than 1. Candidate raw digest equals its sidecar `frame.sha256` |
| `bin/ocelli.sh test ocelli-pixel sigmoid_width_below_one_keeps_the_file_width` | Product `VoiTransform`, width 0.5 | Executed now | Exit 0. Display at modality 39.5 stays about 4.586 |
| `bin/ocelli.sh test ocelli-oracle sigmoid_width_below_one_follows_the_recovered_window` | Producer draw of stored 151, 158, 160, 170 | Executed now, then under a file-width mutation, then after revert | Exit 0, then 101 with low byte 0 against expected 1, then exit 0 after revert |
| `bin/ocelli.sh test ocelli-render grey_linear_matches_the_reference_1024_ramp` | Grey LINEAR 40/400, stored 149, 202, 213, 224 | Executed now, then with ramp width 256, then reverted | Exit 0 with bytes 198, 232, 239, 246 and nearest 197. Mutation exit 101. Constant restored |
| `bin/ocelli.sh test ocelli-codec a_near_lossless_rgb_codestream_is_refused` | `jpegls_corpus_rgb8.jls` with NEAR byte at offset 32 set to 1 | Executed now, then with the lossless-only guard opened, then reverted | Exit 0. Opened guard exit 101 because `near_lossless().decode` returned `Ok(())`. Guard restored |
| JPEG-LS marker walk | Same lossless RGB fixture | Executed now | SOF55 precision 8, 64 by 96, three components. SOS NEAR is 0 and ILV is 2 at file offsets 32 and 33 |
| `.github/workflows/ci.yml` evidence step | Unchanged workflow | Read now | `python3 scripts/verify_ledger.py check-commit HEAD`. No `compare produce` and no GPU step |
| `.githooks/pre-push` | Working tree | Read now | `check-commit` is called with `--require-comparison` |

## Defects

### D1, SIGMOID file width below one is replaced before the product formula

**Where**: `tools/oracle/src/produce.rs:1314` and `tools/oracle/src/produce.rs:1359`
**What**: For `VoiFunction::Sigmoid` and a file width in `(0, 1)`, `presented_sigmoid_window` rebuilds the cornerstone recovered window and passes that pair to `VoiTransform`. Width 0.5 at centre 40 becomes width 1.5 at centre 40. The sidecar width stays 0.5. The candidate code at modality 39.5 is 53. The product LUT on the file width is about 4.586.
**Why it is wrong**: PS3.3 C.11.2.1.3.1 is `range / (1 + exp(-4 * (x - center) / width)) + ymin` on the file window. LINEAR's `c - 0.5` and `w - 1` do not apply. The approved design says VOI comes from the file, units stay in the existing LUT, and this entry adds no second formula. Decision D14 asks for a measured divergence, not a candidate rewritten so the view meets section 25.1. `crates/ocelli-pixel/src/lut.rs:656` and `crates/ocelli-render/shaders/voi.wgsl:83` do not perform this recovery. The producer does, in front of them.
**Evidence**: The product test passed on the file width. The producer test failed when recovery was forced off, low byte 0 against expected 1, and passed again after revert. The gate record `synthetic__ct_sigmoid_width_half` is `pass` with informative `maxAbsDiff` 1 and signed mean 0. `referenceDivergenceEntry` is null. Docs at `docs/lld/oracle.md:1243` and `docs/lld/comparator.md:1035` correctly say the product LUT keeps the file width and that the producer passes the recovered window. That prose matches the code. It does not make the code the file-window formula.
**Scope and regression**: Every SIGMOID stack view whose file width is in `(0, 1)`. Present on this working tree. Smallest regression is `sigmoid_width_below_one_follows_the_recovered_window` against `sigmoid_width_below_one_keeps_the_file_width`. The product shader was not re-run on a GPU.

### D2, Volume candidates are not F-052 ortho reslice plus the product LUT

**Where**: `tools/oracle/src/produce_current.rs:769`, `tools/oracle/src/produce_current.rs:928`, `crates/ocelli-volume/src/ortho.rs:174`
**What**: `Volume::construct` still returns `NonUniformSpacing` for an irregular series. The producer then moves each slice onto `|d_last - d_first| / (n - 1)` and constructs that series. `render_reformat` cuts with `Volume::reslice_nearest`, replaces samples from a float32 ray hit, draws, then overwrites LINEAR pixels with a 1024-wide OpenGL ramp times `1 - exp2(-127 * raySteps)`, then blends partial quad coverage by `(value * hits + 2) / 4`. `Volume::reslice` is not called. `reslice_oblique` still uses analytic `sample_at`.
**Why it is wrong**: The design says volume views use `Volume::construct` and `Volume::reslice`, null slab is one plane, and a new sampling function is out of scope. `reslice_nearest`, the endpoint move, `linear_s`, `gl_linear_unit`, and `reference_volume_byte` are new formulas. They are not `LutChain::map_into`.
**Evidence**: Search of `tools/oracle` finds `reslice_nearest` and no `Volume::reslice` call. Current volume sidecars are LINEAR. Gate `volume__real__mr_eay131__AXIAL` is `pass` with `maxAbsDiff` 0. The six synthetic uniform and nonuniform reformats have `maxAbsDiff` 0 and are `unmeasured` only because the informative fraction is below the floor. `docs/lld/comparator.md:1274` describes this path and matches the code, including the byte match it claims for `volume__synthetic__ct_series_uniform`. The description is not a false sentence. The path is still outside the approved producer.
**Scope and regression**: All nine built volume reformats. Present on this tree. A regression that calls `Volume::reslice` and the existing stack LUT would move the axial MR render hash that is now equal to the reference. That mutation was not run. `docs/lld/volumes.md` still says `sample_at` is the sample kernel. `reslice_nearest` rounds and then calls `sample_at`, so that sentence remains true.

### D3, Extended JPEG samples are shifted after product decode

**Where**: `tools/oracle/src/produce.rs:581` and `tools/oracle/src/produce.rs:1277`
**What**: After `registry.decode`, unsigned 12-bit samples of transfer syntax `1.2.840.10008.1.2.4.51` are increased by `jpeg_extended_12_level_residual`. That value is `round((2056 * 4095 / 255 - ((2048 << 4) + 8)) / 16)`, which is 15. `decode_extended` in `crates/ocelli-codec/src/jpeg.rs:226` returns the decoder plane and does not add 15.
**Why it is wrong**: The design says candidate bytes are Ocelli decode plus the existing LUT. The codec fixture is still the DCMTK level. Adding the jpeg.js bias in the producer makes the candidate a third reconstruction.
**Evidence**: The sidecar for `syntax__jpeg_extended_12` is unsigned, 16 allocated, 12 stored, high bit 11, and that transfer syntax. Its gate render hashes differ. Informative `maxAbsDiff` is 1 and signed mean is about 0.0898. The view is `unmeasured` with qualifier `weak` because the informative fraction is about 0.054. Closeness to the reference does not make the added 15 a product decode.
**Scope and regression**: That one syntax and representation. Present on this tree. Removing the addend is the smallest regression. The DCMTK codec test was not re-run.

### D4, A left-aligned container is unpacked with a high bit the product refuses

**Where**: `tools/oracle/src/produce.rs:795` and `tools/oracle/src/produce.rs:856`
**What**: `pixel_facts` keeps the file high bit, then calls `StoredBits::new` with `bits_stored - 1` and discards that value. `unpack_frame` builds the description with the same substituted high bit. `synthetic__ct_signed_12in16_left` is 16 allocated, 12 stored, high bit 15, signed. The conforming high bit is 11. The right-aligned twin already has high bit 11. Both candidate frames match the reference exactly.
**Why it is wrong**: PS3.3 C.7.6.3 shifts by `HighBit + 1 - BitsStored`. `StoredBits::new` at `crates/ocelli-pixel/src/stored_pixel.rs:185` returns `InvalidHighBit` unless `high_bit == bits_stored - 1`. The design says a candidate is product decode plus the LUT. Substituting a high bit the product refuses is a second unpack. `docs/lld/comparator.md:1314` states this and is accurate. The behaviour is still not product decode.
**Evidence**: Sidecar tags as above. Gate render hashes for the left view are equal, and full-frame `maxAbsDiff` is 0. The same is true of the right view. `a_left_aligned_high_bit_is_unshifted_like_the_reference` expects `pixel_facts` to accept high bit 15.
**Scope and regression**: Native frames whose high bit is not `BitsStored - 1`, including the left-aligned synthetic. Present on this tree. Building `StoredBits` from the file high bit would refuse the left view and is the smallest regression. That refusal was not executed as a separate process. It is the `StoredBits::new` condition.

### D5, Grey LINEAR stack bytes use a 1024 ramp instead of the product display byte

**Where**: `crates/ocelli-render/src/colour.rs:350` and `crates/ocelli-render/src/cpu.rs:527`
**What**: `CpuRaster::write_mono` calls `map_preset_into`. For grey `LINEAR` and `LINEAR_EXACT` with output range 0 to 255, `preset_index` ignores `display_index` and stores the byte from a 1024-wide ramp. Stored 149 at LINEAR 40/400 has nearest display byte 197 and ramp byte 198. The GPU preset path is `colour.mode == 1` and `table_index(display, colour.last)` in `crates/ocelli-render/src/graph.rs:607`, which is the nearest byte. `SIGMOID` and the stops preset stay on `display_index`.
**Why it is wrong**: HLD section 18 requires the LUT arithmetic once. The design says stack presentation reuses that LUT and adds no new formula. A CPU grey byte that is not the nearest product display, while the GPU shader still emits that nearest byte, is a silent tier split. Section 25.1 is not a reason to change the quantiser.
**Evidence**: The ramp test passed and asserts both 198 and nearest 197. Setting the width constant to 256 failed with 49, 58, 60 and 61. The constant was restored. Gate `syntax__explicit_vr_le` now has `maxAbsDiff` 0. It is still `unmeasured` because its informative fraction is about 0.0037. The stack producer reaches this path through `raster_settled`.
**Scope and regression**: CPU grey LINEAR and LINEAR_EXACT draws, including scored stack candidates. GPU mode 1 is unchanged. Present on this tree. The smallest regression is the ramp test above. No adapter was opened, so the GPU disagreement is from the shader text, not a new readback.

## Smells

None.

## Nitpicks

### N1, Colour pass assertion cannot fail

**Where**: `tools/oracle/src/produce_current.rs:2864`
**What**: `colour_is_emitted_and_not_recorded_as_pass` checks `summary.reason(id)` against `colour-not-scored`, `palette-not-scored`, or `ybr-not-scored`, then `assert_ne` against `Some("pass")`. The second assertion cannot fail after the first. The test never loads a comparator record.
**Why it does not block**: The live `assert_eq` does fail if the producer reason becomes `pass`. The existing colour tolerance still records those eight views as `unmeasured` with `unstated-threshold`, not `pass`, in the gate receipt. The dead line does not by itself score a colour frame.

## Verified clean

Trigger 1 is closed for the product formula. `apply_window` and `voi_sigmoid` use the supplied width. LINEAR subtracts 0.5 and 1. LINEAR_EXACT subtracts neither. SIGMOID has no boundary clamp. The product test pins width 0.5 to about 4.586. `lut.rs` and `voi.wgsl` are not in this diff. Docs that say the product LUT keeps the file width match those functions.

Trigger 3 is closed. `JpegLsDecoder::decode` returns `UnsupportedPixelFormat` for an RGB descriptor when the decoder is not the lossless mode, and it does that before `sample_interleaved::decode`. `validate_header` requires NEAR 0 for `.80` and NEAR above 0 for `.81`. SOS NEAR is read after `Ns` component specifiers, at fixture offset 32, with ILV 2 in the next byte. Opening the mode guard made the near-lossless decoder return `Ok(())` on that mutated stream, so the refusal test is not vacuous. The guard was restored. Line interleave remains outside the accepted shapes. No `unsafe` and no `wasm-bindgen` were added in the scanned producer, codec, colour, or ortho files.

Trigger 2's prose in `docs/lld/comparator.md` matches the code. `reslice_oblique` still samples the analytic index. `reslice_nearest` rounds a non-negative half away from zero. The uniform and nonuniform synthetic reformats are byte-identical to the reference and are unmeasured only for low information. That is not a false match claim. It is D2 because the design asked for `Volume::reslice`.

CI evidence remains `check-commit HEAD` with no render. Pre-push passes `--require-comparison`. Feature text in `docs/lld/comparator.md` and `.claude/commands/verify.md` says CI does not pass that flag. The run-level candidate hash `2bf5c039809c7ae533f227ef497f60cbd89860ed554c89c8cf16467b0299b1d9` differs from the reference hash, so this candidate directory is not a copy of `tools/oracle/out`. Shared input digests `manifestSha256`, `renderParamsSha256`, `unsupportedSha256`, `volumeParamsSha256`, and `volumeTruthSha256` match. Colour, palette, and YBR views in the receipt are `unmeasured`, not `pass`. `volume__real__ct_cmb_mml` stays `refused-as-declared` with no frames.

Checked casts in the new volume and producer arithmetic are integer conversions after finite range checks, or `f32` and `f64` construction from small integers. No new trait and no `Box<dyn>` showed up in the new sampling and producer files. The codec registry still has separate lossless and near-lossless decoders.

## Limitations and handoff

Pass 4 does not cover this tree. This pass did not run `compare produce` or `compare gate` and did not write under `/tmp/ocelli-fx021-fix` or `/tmp/ocelli-fx021-fix-gate`. A green `gateVerdict` was not treated as proof of PS3.3. No GPU adapter was opened. The corpus-backed test `window_change_moves_the_candidate` was not run. The volume path was not mutated. Cornerstone source was not opened. Reference behaviour was taken from the existing raw aggregates and sidecars.

The five defects are current. Trigger 1 remains open at the producer even though the product LUT and shader are clean. Trigger 2 remains open as behaviour, not as a false paragraph. Trigger 4 remains open as the JPEG residual and the substituted high bit. Trigger 5 remains open because stack CPU grey bytes disagree with `display_index` and with the GPU `table_index`. Trigger 3 is closed. Trigger 6 is the nitpick above. The next owner should remove the extra formulas rather than retarget the tests at them. This pass is not sprint acceptance.
