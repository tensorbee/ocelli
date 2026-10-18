# F-040 review, pass 1

**Reviewed**: working tree on `work/f-040-grok` (the agent suffix, not the AGPL JPEG 2000 project). Unstaged `git diff` of `crates/ocelli-render/src/graph.rs`, `lib.rs`, `probe.rs`, `docs/lld/README.md` and `docs/lld/render-graph.md`, plus untracked `crates/ocelli-render/tests/upload.rs`. Nothing staged. `shaders/voi.wgsl` is not in the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Independent pass. The reviewer did not write the code. Machine is Darwin arm64, Apple M4 Max, Metal. The fixture opened an adapter. It did not take the tier C return.

### Stored samples, not windowed values

`write_stored_samples` copies the caller `&[f32]` with `bytemuck::cast_slice` into an `R32Float` texture (`STORED_FORMAT` at `crates/ocelli-render/src/graph.rs:217`). There is no clamp and no call into `LutChain` on that path. The fragment entry already in the graph samples with `textureLoad` and passes `.r` to `lut_chain`, which is `shaders/voi.wgsl` unchanged by this diff. That function treats its argument as a stored value and applies slope, then the window, then invert.

Uploading a windowed value would window it again. The ignored test uploads the four stored inputs `[-160, 40, 240, -60]` at slope 1 and intercept 0 and compares the readback with both the section 18.3 table and `LutChain::map_into`. A second application would miss `127.819` and `127.5` by far more than `0.001`. The test passed.

Negatives survive. Input `-160` read back as `0` under LINEAR. An `Rgba8Unorm` stand-in would have clamped the stored value to `0` before the window, and the window of stored `0` at centre 40 is not `0`.

### A window change does not re-upload

`set_voi` writes thirty-two uniform bytes with `queue.write_buffer` and sets `dirty.voi` (`graph.rs:912-918`). It does not call `write_texture`, `upload_stored` or `uploads += 1`. `upload_stored` is the only increment (`graph.rs:973`), and it runs only after `write_slot` returns.

`section_18_3_stored_samples_match_the_chain_and_voi_does_not_upload` asserts `uploads == 1` after `set_voi` to LINEAR_EXACT and again after `set_voi` to centre 240, while the readback changes to the new chain. Replacing the samples then asserts `uploads == 2`. A `set_voi` that called `upload_stored` would make the first of those `assert_eq`s fail. The test executed. `bin/ocelli.sh test ocelli-render --test upload -- --test-threads=1 --include-ignored --nocapture section_18_3` printed no tier C skip and finished `ok` in 0.08s. No mutation was required. The assertion is on the counter the only texture write increments, and that assertion ran.

`tick` binds the existing source view and submits once (`graph.rs:1147-1163`). It does not create or fill the stored texture. wgpu-core 30.0.1 records `write_texture` into pending writes and flushes them on the next `queue.submit`, so the copy is not a second application submit beside the frame.

### LINEAR, LINEAR_EXACT, D-13, tolerance

The new code does not contain a window formula. `git diff` does not touch `voi.wgsl` or `ocelli-pixel`. The shader still has separate functions. LINEAR uses `c - 0.5` and `w - 1`. LINEAR_EXACT uses neither. Both use `<=` at the low end and `>` at the high end (`shaders/voi.wgsl:56-75`). The CPU chain in `apply_window` matches that split.

Hand check at centre 40, width 400, output 0 to 255, against PS3.3 C.11.2.1.2 and C.11.2.1.3.2 rather than against the test comment:

- LINEAR. `c' = 39.5`, `w' = 399`, lower `-160`, upper `239`. `-160 <= -160` is `0`. Centre is `((40 - 39.5) / 399 + 0.5) * 255 = 127.81955`, table `127.819`, gap `0.00055`. `240 > 239` is `255`. `-60` is `63.90977`, table `63.910`.
- LINEAR_EXACT. Lower and upper are `-160` and `240`. `-160 <= -160` is `ymin`, so `0.000`, not the HLD table's `1.594`. That is D-13. Centre is `127.5`. `240 > 240` is false and the body is `255`. `-60` is `63.75`.

`FIXTURE_TOLERANCE` is `0.001` (`tests/upload.rs:21`). The comparison is strict `<`, the same operator as `tests/voi_shader.rs`. The constant is not widened. The LINEAR_EXACT row in the test is `[0.0, 127.5, 255.0, 63.75]`, and the test also asserts the first readback sample is `0.0` after the LINEAR_EXACT `set_voi`.

### Format refusal

`Upload::stored` returns `FormatUnavailable` unless `allowed_usages` contains both `TEXTURE_BINDING` and `COPY_DST` (`graph.rs:286-289`). `register_stored`, the extent-change path and `rebind_stored` all ask `get_texture_format_features(STORED_FORMAT)` and all create with that same constant. There is no other format in the new code. `Rgba8Unorm` is not a fallback.

The no-GPU test builds `TextureFormatFeatures` with the binding missing and with `COPY_DST` missing. Both are `FormatUnavailable`. It also asserts `STORED_FORMAT` is `R32Float` and not `Rgba8Unorm`. A wrong length is refused by `Upload::accepts` with no device, and by `upload_stored` on the GPU test, which leaves `uploads` at 1.

### Same extent, unsafe, bindgen, one LUT

`upload_stored` calls `replace_stored` only when the remembered width or height differs (`graph.rs:963-970`). The same-extent path only `write_texture`s. The fixture's second upload is 4 by 1 into the 4 by 1 texture and the readback follows the new samples, so the write reached the bound texture. No `Vec` is built in `upload_stored`. Section 20's size-class staging pool is the mechanism the approved plan explicitly does not build. `write_texture` takes the caller slice. wgpu-core aligns its own staging copy. This story does not add a second pool and does not claim one.

`rg` over `graph.rs` and `tests/upload.rs` finds no `unsafe`, no `wasm_bindgen`, no `unwrap(`, no `expect(` and no `as` cast. New conversions are `TryFrom` and `checked_mul`. `unreachable!` in the test matches the existing graph tests and is not on a library path.

### Tiers

Tier C cannot construct a graph. `RenderGraph::new` still returns `TierCUnavailable` when `ctx.caps().tier` is `Cpu` (`graph.rs:492-495`), so there is no texture to upload. The CPU path stays `LutChain::map_into`. Tier A and tier B share the `R32Float` path. A device whose format features lack sampling or `COPY_DST` gets `FormatUnavailable` and no substitute image. That matches the new tier table in `docs/lld/render-graph.md`.

### Prose

New lines in `docs/lld/render-graph.md` and `docs/lld/README.md` have no em-dash and no prose semicolon. Rust comments in the diff were scanned the same way. The LLD statements checked against the code are the format refusal, the unclamped stored copy, the same-extent reuse, the `uploads` counter, `rebind_stored` not incrementing it, and tier C remaining on `LutChain::map_into`.

### Tests that ran

`bin/ocelli.sh test ocelli-render --test upload -- --test-threads=1 --include-ignored` reported `4 passed, 0 failed, 0 ignored`. That is the length and format unit test plus the three GPU tests: the section 18.3 rows with the counter, the extent replacement, and recovery recreating the texture without counting an upload. `bin/ocelli.sh test ocelli-render --lib graph -- --test-threads=1` reported `3 passed`. The whole ignored `ocelli-render` suite was not re-run.
