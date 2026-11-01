# F-X021 review, pass 6

**Reviewed**: uncommitted working tree on branch `sprint/s20`. `git rev-parse HEAD` is `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. Nothing was staged. `git diff --stat` is 26 files, 1612 insertions and 161 deletions. Untracked product sources are `crates/ocelli-codec/src/jpegls/sample_interleaved.rs` and `tools/oracle/src/produce_current.rs`. Untracked reviews `F-X021-produce-pass-1.md` through `pass-5.md` are outside the product diff. The ramp-width mutation below was restored. `crates/ocelli-render/src/colour.rs` SHA-256 `10eaa975bc0cb804073f41cbb1f654c8088831e1c34e5e78ba5d234b46c46c9f` matched before and after.
**Author / reviewer**: the working-tree author is whoever produced the uncommitted F-X021 tree. This reviewer did not write that tree. Independence is for this pass only.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

AC6. Grey `LINEAR` uses one 1024-wide ramp on the CPU and the GPU. The discriminating stored value is 149 at centre 40 and width 400, slope 1 and intercept 0.

PS3.3 C.11.2.1.2 with `c - 0.5` and `w - 1` gives `((149 - 39.5) / 399 + 0.5) * 255`, about 197.481. The nearest display byte is 197. A float32 transcription of `reference_ramp_index` and of WGSL `reference_ramp_byte` both return 198. The same pair at stored 202, 213 and 224 is nearest 231, 238 and 245, and ramp 232, 239 and 246. Stored 40 at the same window is about 127.820. Both quantisers return 128. `MONOCHROME1` for stored 149, after the chain has already inverted the display, returns 57 on both transcriptions.

The transcription sweep covered display 0.00 through 255.00 in steps of 0.01, with inversion off and on. That is 51002 pairs and 0 disagreements. Setting the CPU width constant from `1024.0` to `256.0` made `grey_linear_matches_the_reference_1024_ramp` fail. The bytes were 49, 58, 60 and 61 against 198, 232, 239 and 246. Exit code 101. The constant was restored and the file hash matched.

That rejects a CPU grey `LINEAR` byte that is still the nearest display code. The GPU fragment uses the same width, the same edge clamp, and the same half-even store. `select` takes the ramp when `colour.pad == 1`, `fn_kind` is 0 or 1, and the output range is 0 to 255. `fn_kind` 2 stays on `table_index`. This pass did not read a GPU framebuffer. The shader string test checks that `reference_ramp_byte` occurs once. It does not pin 1024. The centre value 128 does not separate 197 from 198.

A second derived case is decision 5's SIGMOID width. Width 0.5 at centre 40 recovers to width 1.5 at centre 40. Stored 151, 158, 160 and 170 with slope 0.25 are modality 37.75, 39.5, 40 and 42.5. The recovered curve is about 0.631, 53.195, 127.5 and 254.676. Nearest bytes are 1, 53, 128 and 255. The file-width value at modality 39.5 is about 4.586 and its nearest byte is 5. `crates/ocelli-pixel/tests/voi.rs` still asserts 4.586_483_5. That product test was read and was not the cargo filter for this pass.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat` | `sprint/s20`, nothing staged | Executed now | HEAD `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. 26 files, 1612 insertions, 161 deletions. Two untracked product paths as named above |
| Float32 recovery, residual, and ramp transcription | Decision 5 constants and `colour.rs` / `graph.rs` text | Executed now | Recovered window is 40 and 1.5. Residual rounds from 15.0588 to 15. Stored 149 is nearest 197 and ramp 198 on both transcriptions. Sweep 0 of 51002 |
| `linear_s` against the product `LINEAR` unit | Stored `-200` through `4199` at windows 40/400, 40/1.5, 1024/4096 and -1000/2000 | Executed now | 883 unit bit differences. 0 composite-byte differences on that grid. Width 1 is a separate class and the volume path refuses width at or below 1 |
| Sidecar scan of `tools/oracle/out` | 93 sidecars with `highBit` and `bitsStored` | Executed now | One mismatch, stack `synthetic__ct_signed_12in16_left`, high bit 15, bits stored 12, allocated 16, signed. Nine volume reformats are `LINEAR`. None has width at or below 1 |
| `bin/ocelli.sh test ocelli-render grey_linear_matches_the_reference_1024_ramp` | CPU width mutated from 1024 to 256, then restored | Executed now | Exit 101. Bytes 49, 58, 60, 61 against 198, 232, 239, 246. 106 tests filtered out. Hash restored |
| Read `/tmp/ocelli-fx021-fix-gate/compare.json` | Prior candidate directory. Not regenerated | Prior receipt | `gateVerdict` pass, 102 views, 71 pass, 31 unmeasured, 0 fail, 0 absent, `green` true |
| `.github/workflows/ci.yml` and `.githooks/pre-push` | Working tree | Read now | CI runs `check-commit HEAD`. Pre-push calls `check-commit` with `--require-comparison` |

Prior receipt aggregates, not re-rendered here. `synthetic__ct_sigmoid_width_half` is `pass`, hashes differ, informative `maxAbsDiff` 1, 21504 samples at one code, 0 at two or more, signed mean 0. `syntax__jpeg_extended_12` is `unmeasured` and `weak`, hashes differ, informative `maxAbsDiff` 1, 960 samples at one code, signed mean about 0.0898. `synthetic__ct_signed_12in16_left` is `pass` with equal hashes and signed mean 0. `volume__synthetic__ct_series_uniform__AXIAL` has equal hashes and is `unmeasured` with informative fraction 0. Colour, palette, YBR and the ultrasound row cited in the producer test are `unmeasured` in that receipt.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, Centre-byte comment names the nearest table

**Where**: `tools/oracle/src/produce.rs:2837`
**What**: `a_fractional_linear_display_keeps_the_nearest_code` says the grey table uses the same rule as shader `table_index` and therefore stores 128. Stored 40 at `LINEAR` 40/400 is about 127.820. The nearest byte and the 1024 ramp both store 128. Grey `LINEAR` on this tree uses `reference_ramp_index` and `reference_ramp_byte`.
**Why it does not block**: The asserted byte is the ramp byte. Changing the ramp width leaves this assertion green and turns `grey_linear_matches_the_reference_1024_ramp` red, which this pass observed.

## Verified clean

Pass 5's five defects are closed against the current design text, decision 5 included. The code matches those sentences. The product LUT and `voi.wgsl` still evaluate SIGMOID as `range / (1 + exp(-4 * (x - center) / width)) + ymin` on the width they are given. `LINEAR` uses `c - 0.5` and `w - 1`, with `<=` at the low end and `>` at the high end. `LINEAR_EXACT` uses neither adjustment.

Trigger 1. `presented_sigmoid_window` runs before `VoiTransform` when the function is SIGMOID and the file width is in `(0, 1)`. Width 0.5 at centre 40 becomes width 1.5 at centre 40. A file width of at least 1 is unchanged. `lut.rs` and `voi.wgsl` do not recover that width. Decision 5 names both halves. The derived nearest bytes are 1, 53, 128 and 255.

Trigger 2. `Volume::construct` still returns `NonUniformSpacing` for an irregular series. `endpoint_grid` then places slice k on `|d_last - d_first| / (n - 1)` and construction is retried. `DuplicatePosition` and `ShearedVolume` are other errors and stay refused. `render_reformat` shifts the plane by the float32 ray offset and cuts with `Volume::reslice_nearest`. That method rounds a positive half away from zero and then calls `sample_at`. The oblique test source expects index `(0.5, 0.5, 0.5)` to read voxel 170, while the trilinear mix of the same point stays 95. That test was read and was not the cargo filter. For `LINEAR` with width above 1, `paint_reference_tone` replaces the stack byte with the OpenGL `LINEAR` sample of the 1024 table, times `1 - exp2(-127 * raySteps)`, then `cover_reference_quad` blends partial coverage by `(value * hits + 2) / 4`. `linear_s` uses the product `LINEAR` operators. At the upper bound both return the top of the range. The interior unit can differ by a float bit. On the scanned grid that bit did not move the composite byte. A ray miss marks the sample absent. A face hit that disagrees with the shifted nearest voxel is read by a 1 by 1 `reslice_nearest`. This pass did not count how many current pixels take that replacement.

Trigger 3. `jpeg_extended_12_level_residual` is `round((2056 * 4095 / 255 - ((2048 << 4) + 8)) / 16)`, which is 15. `apply_reference_jpeg_level` adds it only for transfer syntax `1.2.840.10008.1.2.4.51`, bits stored 12, and pixel representation 0, after unpack and before the LUT. `decode_extended` returns the decoder plane. Decision 5 names that split.

Trigger 4. Stack `unpack_frame` builds `StoredBits` with `bits_stored - 1`. `pixel_facts` keeps the file high bit and still asks `StoredBits::new` to accept the conforming bit, so a file high bit of 15 on 12 stored bits passes the stack path. `StoredBits::new` with the file high bit remains a refusal. The only mismatched sidecar is the stack named above. `decoded_slice` passes the file high bit into `StoredBits::new`. Every current volume member in the scanned sidecars is conforming, so that call sees the same bit the stack substitution would use.

Trigger 5. CPU `preset_index` and GPU `fs_main` both take the 1024 ramp for grey `LINEAR` and `LINEAR_EXACT` with output 0 to 255. `SIGMOID` and the stops preset stay on the nearest display byte. `fusion.wgsl` still calls `table_index`. The design lists fusion as out of scope. The CPU ramp test is independent of the width constant, as the mutation showed.

Pass 5 nitpick N1 is closed. `colour_is_emitted_and_not_recorded_as_pass` asserts `colour-not-scored`, `palette-not-scored`, `ybr-not-scored`, and ultrasound reason `mono`. It no longer asserts inequality with `pass` after an exact reason.

The JPEG-LS RGB path still returns `UnsupportedPixelFormat` when `mode` is anything other than lossless, and it does that before `sample_interleaved::decode`. This pass did not open that guard. No new `as` cast, `unsafe`, or `wasm-bindgen` appeared in `produce_current.rs`, `sample_interleaved.rs`, or the ramp additions. `reslice_nearest` is a method on `Volume`. The interleaved JPEG-LS decoder is a module. CI evidence remains `python3 scripts/verify_ledger.py check-commit HEAD` with no `compare produce` and no GPU step.

## Limitations and handoff

This pass did not run `compare produce` or `compare gate` and did not write under `/tmp/ocelli-fx021-fix` or `/tmp/ocelli-fx021-fix-gate`. The gate file was read as a prior receipt. A green verdict is not proof of PS3.3. No GPU adapter was opened and the fragment shader was not parsed again with naga. The only cargo filter was the ramp test. Corpus-backed producer tests, including the left-aligned frame compare and the window mutation, were not run. The product file-width test and the JPEG-LS refusal test were not re-run. The ray-replacement branch and the volume `LINEAR_EXACT` / `SIGMOID` early return were not executed against a fixture. Nine current volume reformats are `LINEAR`.

The review passes on this tree. It is not sprint acceptance. The next owner can treat pass 5's five defects and its nitpick as closed, and can correct the centre-byte comment if a later edit touches that test.
