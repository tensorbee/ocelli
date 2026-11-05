# Current sprint, S46

**Milestone**: M14, out-of-core volume streaming, and M20, local folder viewer.
**Branch**: `sprint/s46`
**Opened**: 2026-11-05
**Goal**: Brick decomposition and multiscale pyramid generation, and the folder viewer at the level of the viewers radiologists use.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-119 | E30.1 | Brick decomposition and multiscale pyramid generation | Rust | 10w | done |
| F-208 | E38.18 | Folder viewer viewports: layouts from 1 by 1 to 3 by 3, per-viewport series and view state, an active viewport, a series dragged from its thumbnail onto a cell, and the four-image view kept as a tile mode | TS | 6w | pending |
| F-209 | E38.19 | Folder viewer linked viewports: scroll by position in a shared frame of reference, synced window and zoom, and reference lines from the library | TS | 4w | pending |
| F-210 | E38.20 | Folder viewer value at the pointer: the modality value and its unit from the modality stage of the LUT chain, HU on CT, returned by the local process | Rust | 3w | pending |
| F-211 | E38.21 | Folder viewer measurements: angle, Cobb angle, bidirectional, arrow and probe, region area with mean, SD, minimum and maximum in modality units, editable handles and a measurement list | All | 6w | pending |
| F-212 | E38.22 | Folder viewer four corners at parity: birth date, sex, age, study time, institution, slice location and thickness, kVp and mAs, TR and TE, a scale bar, and edge letters from Patient Orientation on projection images | All | 4w | pending |
| F-213 | E38.23 | Folder viewer window presets by modality on number keys, every VOI window in the file listed by its explanation, and a region window from the library | All | 3w | pending |
| F-214 | E38.24 | Folder viewer mouse profiles and preferences: the current binding, a scroll-on-left profile and a window-on-left profile, saved in the browser, and cine at Frame Time (0018,1063) | TS | 2w | pending |
| F-215 | E38.25 | Folder viewer marks written as a TID 1500 document and key images as a Key Object Selection through @ocelli/core, saved to a download and loaded back from a folder | TS | 4w | pending |
| F-216 | E38.26 | Folder viewer performance: each file posted once and held in a bounded cache, decode reused across window changes, measured budgets for open, first image, scroll rate and resident memory, and a release build | All | 4w | in-progress |
| F-217 | E38.27 | Licence hygiene: no copyleft dependency in any tracked manifest, a permissive JPEG reference decoder for the corpus, licence allowlist gates for Rust, npm and Python, and a third-party notices file | All | 2w | done |
| F-218 | E38.28 | Dependency advisories in shipped packages: adm-zip below 0.6.1 reaches the published @ocelli/core runtime through dcmjs, resolved by a patched override or a proof that no shipped path reaches it, with an audit gate over shipped dependencies | TS | 1w | done |
| F-219 | E38.29 | Sprint plan checker refusals that no guard probe triggers: the four refusals in gen_sprint_plan.py the S46 guard review found unwatched, each given a fixture that turns it red | Test | 1w | done |
| F-220 | E38.30 | Shipped binary notices: the full licence text of every crate compiled into the wasm module and the native binaries, generated from the lockfile and checked by a gate | Build | 2w | in-progress |
| F-221 | E38.31 | Published package contents: the @ocelli/core tarball stops shipping compiled test files, with a packages gate refusal for any test file in a published tarball | TS | 1w | done |
| F-222 | E30.7 | Pyramid means skip non-finite F32 samples: a coarser level averages only the finite samples of its box by their overlap weight, a box with no finite sample stays NaN, and negative zero is written as positive zero | Rust | 1w | done |
| F-223 | E38.32 | Comparative benchmark against cornerstone3D 5.8.2: the folder viewer and a bare cornerstone3D page on the same studies, browser and machine, with every process's memory counted, a pixel check before any timing, and a generated summary | All | 3w | pending |
| F-224 | E5.7 | Frame identity: the decoded tier answers a lookup only when the frame's content id, from SOP Instance UID, transfer syntax and encoded bytes, and its description and plane match, DicomwebClient takes a documented cache mode defaulting to no-cache, and the folder viewer keys page caches by the picked File, lets the same folder be chosen again, and refuses to show a file that changed on disk | All | 3w | pending |
| F-225 | E3.8 | DICOM conformance report: ocelli-dicom checks each value against the VR rules of PS3.5 section 6.2 and Type 1, 1C and 2 presence for seven PS3.3 modules on CR, CT, MR and PET, a finding never carries a value or stops display, @ocelli/core reads the report, and the folder viewer shows a Checks badge and list | All | 4w | pending |
| F-226 | E5.8 | Release by series and study: ImageCache::release_series and BrickCache::release_volume over one Lru range removal, a cache.released ring event decoded by @ocelli/core, and the folder viewer forgetting the previous folder's held files and frames on the local process when another folder or the synthetic study is opened or the page is left | All | 3w | pending |
| F-227 | E38.33 | Automatic window per series when the file has none: one window from the 1st to 99th nearest-rank percentile of modality values pooled over up to 16 frames spread through the series, each frame through its own rescale, leaving out stored zero and the Pixel Padding Value range, computed once in ocelli-pixel, labelled Auto window in the folder viewer, and every window in the file still used as it is | All | 3w | pending |
| F-228 | E38.34 | Content Security Policy: the minimal policy an embedding page needs for the Ocelli worker and wasm path, documented and proven by a browser probe of the built example viewer with minimality probes, and the folder viewer served under a strict hash policy with compressed frames decoded and drawn | All | 2w | pending |
| F-229 | E4.10 | JPEG XL decoding: the three JPEG XL transfer syntaxes through jxl-oxide, lossless decoded exactly, JPEG recompression rebuilt and decoded by the existing JPEG path, unsupported features refused as unavailable, with synthetic corpus rows and declared oracle rows | Rust | 3w | pending |
| F-230 | E16.6 | Public options refuse unknown keys: every options and request object in @ocelli/core refuses an unknown key at run time with a coded OcelliError naming it, DicomwebError and the bulk, command and instance errors share that class, and loadCore rejects with the initialiser's cause instead of resolving false | TS | 2w | done |
| F-231 | E5.9 | Session frame cache: the wasm session keeps committed stack frames in a caller-budgeted ImageCache checked by ContentId, reports eviction, pressure and release through the ring, and exports a residency query, a show-from-cache call, and releaseSeries and releaseStudy in @ocelli/core | Rust+TS | 4w | pending |

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

## Questions for the operator

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

- **The objc2 crates in the native binaries may carry an Apple SDK
  redistribution question.** F-220's first review read the upstream objc2
  `LICENSE.md`, which flags that redistributing these crates, generated from
  Apple SDK headers, under the Xcode licence is unclear. They reach the
  native binaries through wgpu's Metal backend. Not blocking S46, because
  S46 publishes nothing. The choice is legal advice before a native release,
  or a native build that leaves them out.

- **`acceptTransferredFrame` ignores a correctly spelt `stackIndex` and
  `stackEpoch`.** Found by F-230's first review and older than F-230. Now
  that both keys are on F-230's accepted list, a caller who sets them gets
  neither a refusal nor the effect. The choice is a story that honours them,
  or removing them from that entry point's accepted keys.

## What is carried in

- **D-22 still blocks publication.** The version stays 0.1.0.
- **One cache.** A brick is a key in `ocelli-cache`. There is no second cache.
- **Level 0 is full resolution.** A series that fits in memory still has the level axis.
- **F-202 to F-205 stay unscheduled.** This sprint does not build them.

## The defect class this sprint is exposed to

A child level that is not a downsample of level 0. Spacing on that level copied from slice thickness instead of the dimension ratio. A missing level that returns level 0. A second brick edge beside `BrickGrid::new`.

## What done means

F-119 generates the coarser levels by downsampling level 0. `append_level` stores caller-supplied bytes and does not downsample, and this story is that downsample. Level 0 stays the full grid. Child spacing is level 0 spacing times the level 0 dimension divided by the child dimension, on each axis. The brick edge stays the one `BrickGrid::new` already chooses. Opacity correction stays F-121.
