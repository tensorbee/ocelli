# Current sprint, S46

**Milestone**: M14, out-of-core volume streaming, and M20, local folder viewer.
**Branch**: `sprint/s46`
**Opened**: 2026-11-05
**Goal**: Brick decomposition and multiscale pyramid generation, and the folder viewer at the level of the viewers radiologists use.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-119 | E30.1 | Brick decomposition and multiscale pyramid generation | Rust | 10w | done |
| F-208 | E38.18 | Folder viewer viewports: layouts from 1 by 1 to 3 by 3, per-viewport series and view state, an active viewport, a series dragged from its thumbnail onto a cell, and the four-image view kept as a tile mode | TS | 6w | in-progress |
| F-209 | E38.19 | Folder viewer linked viewports: scroll by position in a shared frame of reference, synced window and zoom, and reference lines from the library | TS | 4w | in-progress |
| F-210 | E38.20 | Folder viewer value at the pointer: the modality value and its unit from the modality stage of the LUT chain, HU on CT, returned by the local process | Rust | 3w | done |
| F-211 | E38.21 | Folder viewer measurements: angle, Cobb angle, bidirectional, arrow and probe, region area with mean, SD, minimum and maximum in modality units, editable handles and a measurement list | All | 6w | done |
| F-212 | E38.22 | Folder viewer four corners at parity: birth date, sex, age, study time, institution, slice location and thickness, kVp and mAs, TR and TE, a scale bar, and edge letters from Patient Orientation on projection images | All | 4w | in-progress |
| F-213 | E38.23 | Folder viewer window presets by modality on number keys, every VOI window in the file listed by its explanation, and a region window from the library | All | 3w | in-progress |
| F-214 | E38.24 | Folder viewer mouse profiles and preferences: the current binding, a scroll-on-left profile and a window-on-left profile, saved in the browser, and cine at Frame Time (0018,1063) | TS | 2w | pending |
| F-215 | E38.25 | Folder viewer marks written as a TID 1500 document and key images as a Key Object Selection through @ocelli/core, saved to a download and loaded back from a folder | TS | 4w | pending |
| F-216 | E38.26 | Folder viewer performance: each file posted once and held in a bounded cache, decode reused across window changes, measured budgets for open, first image, scroll rate and resident memory, and a release build | All | 4w | done |
| F-217 | E38.27 | Licence hygiene: no copyleft dependency in any tracked manifest, a permissive JPEG reference decoder for the corpus, licence allowlist gates for Rust, npm and Python, and a third-party notices file | All | 2w | done |
| F-218 | E38.28 | Dependency advisories in shipped packages: adm-zip below 0.6.1 reaches the published @ocelli/core runtime through dcmjs, resolved by a patched override or a proof that no shipped path reaches it, with an audit gate over shipped dependencies | TS | 1w | done |
| F-219 | E38.29 | Sprint plan checker refusals that no guard probe triggers: the four refusals in gen_sprint_plan.py the S46 guard review found unwatched, each given a fixture that turns it red | Test | 1w | done |
| F-220 | E38.30 | Shipped binary notices: the full licence text of every crate compiled into the wasm module and the native binaries, generated from the lockfile and checked by a gate | Build | 2w | done |
| F-221 | E38.31 | Published package contents: the @ocelli/core tarball stops shipping compiled test files, with a packages gate refusal for any test file in a published tarball | TS | 1w | done |
| F-222 | E30.7 | Pyramid means skip non-finite F32 samples: a coarser level averages only the finite samples of its box by their overlap weight, a box with no finite sample stays NaN, and negative zero is written as positive zero | Rust | 1w | done |
| F-223 | E38.32 | Comparative benchmark against cornerstone3D 5.8.2: the folder viewer and a bare cornerstone3D page on the same studies, browser and machine, with every process's memory counted, a pixel check before any timing, and a generated summary | All | 3w | pending |
| F-224 | E5.7 | Frame identity: the decoded tier answers a lookup only when the frame's content id, from SOP Instance UID, transfer syntax and encoded bytes, and its description and plane match, DicomwebClient takes a documented cache mode defaulting to no-cache, and the folder viewer keys page caches by the picked File, lets the same folder be chosen again, and refuses to show a file that changed on disk | All | 3w | done |
| F-225 | E3.8 | DICOM conformance report: ocelli-dicom checks each value against the VR rules of PS3.5 section 6.2 and Type 1, 1C and 2 presence for seven PS3.3 modules on CR, CT, MR and PET, a finding never carries a value or stops display, @ocelli/core reads the report, and the folder viewer shows a Checks badge and list | All | 4w | done |
| F-226 | E5.8 | Release by series and study: ImageCache::release_series and BrickCache::release_volume over one Lru range removal, a cache.released ring event decoded by @ocelli/core, and the folder viewer forgetting the previous folder's held files and frames on the local process when another folder or the synthetic study is opened or the page is left | All | 3w | done |
| F-227 | E38.33 | Automatic window per series when the file has none: one window from the 1st to 99th nearest-rank percentile of modality values pooled over up to 16 frames spread through the series, each frame through its own rescale, leaving out stored zero and the Pixel Padding Value range, computed once in ocelli-pixel, labelled Auto window in the folder viewer, and every window in the file still used as it is | All | 3w | in-progress |
| F-228 | E38.34 | Content Security Policy: the minimal policy an embedding page needs for the Ocelli worker and wasm path, documented and proven by a browser probe of the built example viewer with minimality probes, and the folder viewer served under a strict hash policy with compressed frames decoded and drawn | All | 2w | done |
| F-229 | E4.10 | JPEG XL decoding: the three JPEG XL transfer syntaxes through jxl-oxide, lossless decoded exactly, JPEG recompression rebuilt and decoded by the existing JPEG path, unsupported features refused as unavailable, with synthetic corpus rows and declared oracle rows | Rust | 3w | done |
| F-230 | E16.6 | Public options refuse unknown keys: every options and request object in @ocelli/core refuses an unknown key at run time with a coded OcelliError naming it, DicomwebError and the bulk, command and instance errors share that class, and loadCore rejects with the initialiser's cause instead of resolving false | TS | 2w | done |
| F-231 | E5.9 | Session frame cache: the wasm session keeps committed stack frames in a caller-budgeted ImageCache checked by ContentId, reports eviction, pressure and release through the ring, and exports a residency query, a show-from-cache call, and releaseSeries and releaseStudy in @ocelli/core | Rust+TS | 4w | done |
| F-232 | E38.35 | Unsafe guard soundness: unsafe_code forbidden by rustc through the workspace lint table on every target of every workspace member, and scripts/unsafe_allowlist_check.py rebuilt on a left-to-right Rust tokenizer with a planted unsafe test for each of the five triggers the F-229 review found | Build | 2w | done |
| F-233 | E38.36 | SIMD wasm builds compile SIMD code: the SIMD codec verification variants pass +simd128 to every crate through a target-scoped RUSTFLAGS, a native arm step refuses a SIMD variant whose dependency rlibs match the plain build, and D-22's F-X013 claim is rechecked on that build | Build | 1w | done |
| F-234 | E3.9 | Bounded sequence depth in parse_part10: the structural preflight refuses sequence nesting deeper than a named constant with a typed ParseError before dicom-rs recurses, with tests at the limit and one past on a 2 MiB thread and a fuzz-shaped regression | Rust | 1w | done |
| F-236 | E16.7 | acceptTransferredFrame honours stackIndex and stackEpoch: a transferred frame carrying both keys commits through the selected-frame export and is refused for a stale index or epoch, with tests at the unit and built wasm levels | TS | 1w | done |
| F-237 | E19.6 | The wasm module ships outside the @ocelli/core tarball: docs/RELEASE.md preflight item 6 and the published list name where the module and its licence files ship and how a release proves the module is the one its HEAD builds, with the packages gate checking that place | Build | 1w | done |
| F-238 | E19.7 | Native build without wgpu's Metal backend: the shipped native build compiles no objc2 crate, proven by the resolved dependency graph for every Apple target, while development builds keep Metal for the tier A gate | Build | 2w | pending |
| F-239 | E38.37 | Folder viewer colour frames: RGB, YBR_FULL, YBR_FULL_422, YBR_ICT, YBR_RCT and PALETTE COLOR with Planar Configuration drawn through ocelli-pixel's ColorTransform on the decoder's reported output space, multi-component JPEG 2000 and HTJ2K decoded with their component transforms, window and invert per channel as cornerstone3D 5.8.2 applies them, region statistics per channel, colour above 8 bits scaled by one library rule, an ICC profile named as not applied, and every colour shape this build cannot draw refused by name | All | 12w | done |
| F-240 | E38.38 | Matrix and tone-curve ICC profiles applied in ocelli-pixel stage 4 for colour frames, lookup-table profiles refused by name until F-145 | All | 5w | done |
| F-243 | E4.13 | JPEG-LS decoded per ITU-T T.87: the vendored ritk-codecs scan decoder raises the run index only for a full run segment, a scan that runs out of data or leaves unread bytes before EOI is refused, an LSE preset this decoder does not apply is refused by name, default thresholds clamp as C.2.4.1.1.1 gives them, with CharLS-encoded trigger fixtures and banded corpus rows under .80 and .81 | Rust | 3w | done |
| F-244 | E3.10 | Encapsulated Pixel Data with an empty Basic Offset Table opens per PS3.5 A.4: dicom-object 0.10.0 vendored with its collector patched so the zero-length first Item is the table and every later Item a Fragment, at the top level and inside an Icon Image Sequence, for every encapsulated syntax the registry decodes, single-frame and multi-frame, with frames that span Fragments delimited by their codestream start and end markers where A.4 lets a frame span Fragments, a present Extended Offset Table applied through EncapsulatedFrameIndex::from_extended, and an empty-table corpus row per decodable syntax | Rust | 2w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority.

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S46 / {print $2, $9}'
```

## What this sprint is

S45 is closed at tag `s45`. That close delivered F-206 and F-207 and published nothing.
This sprint starts M14. F-119 decomposes a volume into bricks and generates
the multiscale pyramid. F-058 is done, so the volume model already has the
level axis this story fills. F-208 to F-216 bring the local folder viewer to the
baseline of the viewers radiologists use. They were found by a browser test
on 2026-11-05 and the operator placed them here. The operator assigned the
implementation of every story in this sprint to an external coding agent,
with design, review and finalisation in Claude. Closing this sprint does not publish.

## Review findings carried into a story

- **F-207 polish review S1, carried into F-208.** The display aspect has no
  behavioural test. Swapping `aspectX` and `aspectY` where they are assigned in
  `examples/folder-viewer/src/page.html` still passes every test, because the
  only checks are pinned source strings. The fix needs a headless browser
  harness, and F-208 is the story that builds one for its viewport checks. The
  lead carried it under the operator's instruction of 2026-11-05, "we cannot
  compromise the accuracy, quality and robustness so you make a call". It is
  not waived. F-208 carries it as a numbered
  acceptance row: the single view and the first tile of the synthetic study
  measure 64 to 67.2 within 1e-3, and the assignment swap must turn that row
  red. F-208 does not complete while that row is open.
- **F-217 fresh-clone build, fixed as a follow-up on 2026-11-07.** F-217's
  Vite 8 to Vite 7 move made `npm run build` fail with ENOENT on a clone with
  no `crates/ocelli-wasm/pkg`, so CI's frontend job would have failed on push.
  The follow-up integrated from `work/f-217-fresh-build` marks the wasm URL
  external for the worker build when the pkg is absent, decodes the URL path,
  and adds `scripts/viewer_build_check.py` to `gate packages`. F-217 stays
  done, and its follow-up AS_BUILT entry records the three open items for a
  separate story: the built `dist` hard-codes the build machine's `/@fs` path,
  the viewer stays `core not built` in Chromium even with the pkg, and dev mode
  answers 403 because `server.fs.allow` is percent-encoded. F-266 is the
  draft follow-up filed for S47.

## Pulled forward from S47

The operator decided on 2026-11-07 to pull bounded S47 stories forward while implementer slots are idle. F-237 and F-238 are approved and implemented in S46 worktrees, F-237 on F-228's branch and F-238 on F-233's. F-236 is approved on 2026-11-07 after F-231 integrated at `eb0f81b3`, re-read against that head in its plan's lead amendments, implemented from the sprint head, and integrated on 2026-11-08. It forwards `stackIndex`, `stackEpoch` and `content` from `acceptTransferredFrame` into the existing commit routes, and writes `packages/core/src/bulk.ts` and its two test files. This close proposal moves the allocation of F-237 and F-238 from S47 to S46 beside F-236. It does not mark either story done or supply CI or host acceptance. F-241's full video story remains S47.

## Questions for the operator

- **F-238 recorded host gaps and owed Linux proof.**
  AC4's Windows execution and tier report and AC1's Intel Mac build and
  runtime observation remain unverified for want of those hosts, as the
  approved plan records. The resolved Intel Mac dependency graph is observed.
  Linux still requires an actual `ubuntu-latest` native step 6 build and run.
  The lead's acceptance-stage clarification permits that observation on an
  authorized qualification ref before completion. Run `42576164087` at
  `8fdee914` failed before that step, which was skipped. It supplies no Linux
  acceptance.

- **F-237 qualification CI observation.** The TypeScript job, including
  `gate packages`, passed in that same exact-head run. The overall workflow
  failed. The plan's acceptance-stage clarification preserves this bounded
  job observation without claiming the entire run or a later tree passed.
  Formal integration and completion remain open. AC3 and AC5 remain release
  obligations under the approved plan.

- **Non-finite F32 samples in the pyramid, before F-120.** F-119's plan does
  not say what a NaN, an infinity or a negative zero does to a coarser level.
  Today the mean passes NaN and infinity through unchanged, so one NaN voxel
  spreads to eight times its volume at each coarser level, and an all `-0.0`
  box is written as `+0.0`. No render reaches this yet, because a volume
  refuses 32-bit samples and nothing calls `generate_pyramid` until F-120.
  The choices are to let NaN through and have every tier draw it as no
  value, to leave non-finite samples out of the mean and re-weight, or to
  refuse to generate. The same answer covers non-finite input to
  `level_voxel_to_level0`. F-119 pins the current behaviour in
  `non_finite_floats_keep_the_current_bits` so a change is visible.
  **Answered by the operator on 2026-11-05:** leave non-finite samples out of
  each mean and divide by the finite weight only, keep a box with no finite
  sample as NaN, and write negative zero as positive zero. F-222 implements it.

- **Licence file waivers the lead decided, for the operator to confirm.**
  F-217's plan lets the lead waive a licence file only when its text is an
  allowlisted licence that grants nothing copyleft, or when it is an empty
  file that states no licence. Four rows in
  `ci/licences.toml` use that rule: the empty `LICENSE-heif`,
  `LICENSE-jetraw` and `LICENSE-libjxs` placeholders in imagecodecs
  2026.10.10, for codecs not compiled into the wheel, and et-xmlfile 2.0.0's
  `LICENCE.python`, which is the CPython licence stack and names the GPL only
  in a governing-law clause. Changing any of them to `operator`, or refusing
  it, is a one-row edit.
  **Confirmed by the operator on 2026-11-05.** All four stay as recorded.

- **The `@ocelli/core` tarball carries no wasm module, so `/release` would
  refuse.** `docs/RELEASE.md` preflight item 6 requires the module built from
  the release HEAD inside `@ocelli/core`. `npm pack --dry-run -w
  packages/core` lists 281 entries on c775931 and none ends in `.wasm`. No
  S46 story builds the module into the package. Found while designing F-220,
  outside its scope. Not blocking S46, because S46 publishes nothing. The
  choice is a story that packs the module, or a release rule that says where
  the module ships instead.
  **Answered by the operator on 2026-11-06:** change the release rule. The
  module ships outside the `@ocelli/core` tarball and `docs/RELEASE.md` item 6
  says where. A story in S47 decides where and rewrites the rule. F-237
  implements it.

- **The objc2 crates in the native binaries may carry an Apple SDK
  redistribution question.** F-220's first review read the upstream objc2
  `LICENSE.md`, which flags that redistributing these crates, generated from
  Apple SDK headers, under the Xcode licence is unclear. They reach the
  native binaries through wgpu's Metal backend. Not blocking S46, because
  S46 publishes nothing. The choice is legal advice before a native release,
  or a native build that leaves them out.
  **Answered by the operator on 2026-11-06:** a native build without wgpu's
  Metal backend, so the objc2 crates are not shipped. A story in S47.
  F-238 implements it.

- **`acceptTransferredFrame` ignores a correctly spelt `stackIndex` and
  `stackEpoch`.** Found by F-230's first review and older than F-230. Now
  that both keys are on F-230's accepted list, a caller who sets them gets
  neither a refusal nor the effect. The choice is a story that honours them,
  or removing them from that entry point's accepted keys.
  **Answered by the operator on 2026-11-06:** a story in S47 honours both keys
  with tests. F-236 implements it.

- **A deeply nested sequence overflows the stack in `parse_part10`.** Found
  by F-225's first review while fuzzing, before the conformance checker runs,
  so it is older than F-225. With `panic = "abort"` a stack overflow ends the
  process, so one crafted file can stop the folder viewer's local process.
  The choice is a story that bounds sequence depth in the parser and refuses
  deeper input with an error, or accepting the risk for local files.
  **Answered by the operator on 2026-11-06:** fixed in S46. A new story bounds
  sequence depth in the parser and refuses deeper input with a typed error.
  F-234 implements it.

- **A Rust advisory nobody gates: RUSTSEC-2021-0153 on the `encoding` crate.**
  Found by F-229's first review running `cargo deny check`. The crate comes
  in through dicom-rs, not through any S46 story. F-218's advisory gate audits
  the npm packages Ocelli ships, and the Rust gates run `cargo deny check
  licenses` only, so no gate reports a Rust advisory today. The choice is a
  story that adds `cargo deny check advisories` to the floor with a reviewed
  exception list, or recording this one as an accepted risk.
  **Accepted by the operator on 2026-11-05 as a known risk for now.** No
  gate is added in S46.

- **The VOI shader rescales with one fused rounding on Apple GPUs.** Found by
  F-227's fourth review on an Apple M4 Max under Metal: the shader's
  `stored * slope + intercept` in `crates/ocelli-render/shaders/voi.wgsl`
  compiles to a fused multiply-add, so tier A can differ from `LutChain` by up
  to eight f32 units at the top of the range, not the two F-041 recorded. In a
  window one unit wide this drew the 99th percentile black on tier A and white
  on tier C. F-227 now accepts a window only with a 16 unit margin, which covers a
  shader rescale within 16 units of the CPU value. Any other narrow window on tier A is still exposed. The choice
  is a story that makes the shader's rescale match `LutChain` on every tier, or
  a recorded divergence with a minimum window width.
  F-227's fifth review found a second case: where `stored * slope` nearly
  cancels the intercept, the fused rounding error reaches hundreds of millions
  of units and flips greys at any window width, in 15,879 of 800,000 adversarial
  draws. A minimum window width cannot cover that case, and F-227's own
  constant window is exposed to it too.
  **Answered by the operator on 2026-11-06:** a story in S47 makes the shader's
  rescale match `LutChain` on every tier, with a tier A fixture that fails today.
  F-235 implements it.
- **The unsafe guard has blind spots.** F-229's eighth review found that
  `scripts/unsafe_allowlist_check.py`'s string pattern opens a false string
  after a `'"'` character literal, after a raw string holding a quote or
  ending in a backslash, and after a line comment with an unpaired quote, and blanks the code that follows. An
  `unsafe` block planted at `crates/ocelli-dicom/src/dicomweb.rs:418` passed the
  guard. No audited count changes. The choice is a story that gives the guard a
  real Rust lexer, or a recorded limit on the guard.
  **Answered by the operator on 2026-11-06:** fixed in S46. A new story makes
  the guard sound against every trigger above. F-232 implements it.
- **No build compiles the jxl-oxide SIMD code.** The same review found that
  `bin/ocelli.sh native` passes `+simd128` after `cargo rustc --`, which reaches
  only the final example crate, so the SIMD code of its dependencies is never
  built. D-22's F-X013 SIMD check is built the same way and may not test what it
  says.
  **Answered by the operator on 2026-11-06:** fixed in S46. A new story builds
  every crate with `+simd128` for the SIMD variants and rechecks F-X013's claim. F-233 implements it.
- **A colour series in the folder viewer shows a fault, not an image.** Every
  frame that is not `MONOCHROME1` or `MONOCHROME2` stops at
  `PreviewError::NotMonochrome` in `examples/folder-viewer/src/preview.rs`,
  and the page shows "this preview shows monochrome frames only". The library
  already has the colour stage, F-030's `ColorTransform` in `ocelli-pixel`.
  **Decided by the operator on 2026-11-07:** the folder viewer previews RGB,
  YBR_FULL, YBR_FULL_422, YBR_ICT, YBR_RCT and PALETTE COLOR frames with
  Planar Configuration through that stage and the decoder's reported output
  space, so the colour arithmetic exists once in the library. F-239
  implements it, scheduled first in the folder viewer chain, ahead of F-212.
  F-212 now depends on it.
  **F-239's design questions answered by the operator on 2026-11-07:** each
  decoder reports its own output length, F-239 also decodes multi-component
  JPEG 2000 and HTJ2K with their component transforms, window and invert act
  on each colour channel as cornerstone3D 5.8.2 does, a region on a colour
  frame reports R, G and B statistics, colour above 8 bits and palette entries
  above 255 are scaled by one `ocelli-pixel` rule, and a file with an ICC
  profile is drawn with a sentence that the profile was not applied. F-240
  applies matrix and tone-curve ICC profiles after F-239, filed the same day.
  **Answered by the operator later on 2026-11-07:** the vendored JPEG 2000
  crates are patched with recorded provenance, HTJ2K first, the ICC subset is
  built with no dependency, a profile outside it is drawn and named, neither
  story may grow the wasm build or slow the viewer, and cornerstone3D 5.8.2 is
  the floor that both stories must meet or beat on a measured axis. Both plans
  are approved, with deviations D-29, D-30 and D-31. F-239 is 12w in a codec
  part and a viewer part, and F-240 is 5w.
- **No decoder handles DICOM video.** After F-239 a `YBR_PARTIAL_420` frame is
  refused by name, and the sixteen MPEG-2, H.264 and HEVC transfer syntaxes of
  PS3.6 are not in the codec registry's known list. cornerstone3D 5.8.2 plays
  video only through a server's `/rendered` resource in an HTML `<video>`
  element, and fails on a local file.
  **Decided by the operator on 2026-11-07:** file a video story now, at least
  what cornerstone3D does for video in the folder viewer and the library and
  better on a measured axis. F-241 is filed for S47 at 12w, E4.11, with a draft
  plan. Its nine questions, among them the decode route, the patent position
  and MPEG-2, are open for the operator, each with the lead's recommendation.
  **Answered by the operator later on 2026-11-07:** H.264 and HEVC decode
  through WebCodecs in a worker on both targets, with the stream demuxed in
  Rust and the colour converted once in `ocelli-pixel`, and no shipped artefact
  may contain a software H.264 or HEVC decoder, now a section of
  `docs/SOURCE-POLICY.md`. The stream's own colour description wins over
  PS3.3's matrix when present. The `VideoFrame` held in the worker for one
  copy is deviation D-32. F-241 is approved at 12w in a library part and a
  viewer part. MPEG-2 is decoded in Rust by F-242, filed for S48 at 7w, E4.12,
  with a draft plan whose six questions are open for the operator, the
  patent position among them, because the pool's list still names patents
  active in Malaysia.
  **Answered by the operator later on 2026-11-07:** proceed with the two
  Malaysian patents recorded in `docs/SOURCE-POLICY.md`, an `f64` IDCT from
  H.262 Annex A, a MONOCHROME2 file drawn from its Y plane, libmpeg2 and
  ffmpeg's MPEG-2 decoder source read-blocked and enforced by the provenance
  checker, real time natively at both sizes and in Chrome at 720 by 576, and
  synthetic clips with a recorded assessment of the conformance streams.
  F-242 is approved at 8w with no deviation.
- **A monochrome JPEG-LS frame can draw wrong pixels with no error.** A
  reviewer saw an Enhanced CT frame under `.80` drawn as flat grey. The vendored
  `ritk-codecs` run-mode loop raises RUNindex when a run reaches the end of a
  line on a partial segment, so every later run reads the wrong length, and
  the scan decoder returns `Ok` from a desynchronised or truncated scan. 916 of
  1,024 samples were wrong in each frame of the reported file. No corpus row
  reaches the trigger, which is why every test passed.
  **Decided by the operator on 2026-11-07:** a new S46 story, done next,
  patches the vendored scan decoder to T.87 A.7.1, refuses a scan that runs out
  of data or leaves unread bytes before EOI, refuses by name an LSE preset the
  decoder does not apply, records the patch and amends D-20, and adds
  CharLS-encoded trigger fixtures and banded corpus rows. F-243 implements it,
  E4.13 at 3w, approved with no new deviation. It lands before F-239's codec
  part, whose `ritk-codecs` patch merges onto it, so F-239 now depends on it.
- **Encapsulated Pixel Data with an empty Basic Offset Table does not open.**
  PS3.5 A.4 lets the table Item have length zero under every encapsulated
  syntax. dicom-object 0.10.0 `build_encapsulated_data` clears its first-item
  flag only when an Item value is read, so after an empty table the first
  Fragment is read as the table. All 70 empty-table probe files across the 14
  decodable encapsulated syntaxes were refused, by the frame index or by
  `parse_part10`, and no decoder was reached. A compressed icon with an empty
  table can refuse the whole file. 0.10.1 and upstream master carry the same
  code. No corpus row has an empty table, which is why nothing caught it.
  **Added by the operator on 2026-11-08:** fix it in S46 now. F-244 implements
  it, E3.10 at 2w. The lead confirmed the route: dicom-object 0.10.0 is
  vendored with a one-line patch and recorded provenance, as `ritk-codecs` is,
  and D-18 is amended in the implementing change. An upgrade replaces the
  vendored copy once a published release carries the fix. No new deviation.

## What is carried in

- **D-22 still blocks publication.** The version stays 0.1.0.
- **One cache.** A brick is a key in `ocelli-cache`. There is no second cache.
- **Level 0 is full resolution.** A series that fits in memory still has the level axis.
- **F-202 to F-205 stay unscheduled.** This sprint does not build them.

## The defect class this sprint is exposed to

A child level that is not a downsample of level 0. Spacing on that level copied from slice thickness instead of the dimension ratio. A missing level that returns level 0. A second brick edge beside `BrickGrid::new`.

## What done means

F-119 generates the coarser levels by downsampling level 0. `append_level` stores caller-supplied bytes and does not downsample, and this story is that downsample. Level 0 stays the full grid. Child spacing is level 0 spacing times the level 0 dimension divided by the child dimension, on each axis. The brick edge stays the one `BrickGrid::new` already chooses. Opacity correction stays F-121.

## Acceptance observations still pending

The current status of F-208, F-209, F-212 and F-213 is reopened after reconciling their approved plans with their historical delivery notes. Their implementation is integrated and reviewed. The remaining numbered observations are F-208 AC8 to AC12, F-209 AC16 to AC20 and AC22, F-212 AC12 and F-213 AC13. F-209 AC21 is a mutation control with recorded evidence, not an operator click-through. The original tracker and AS_BUILT entries are preserved and qualified by the dated acceptance correction. No observation, carry or scope waiver is inferred from keeping the checklist pending.
