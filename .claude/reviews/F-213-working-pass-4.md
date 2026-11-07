# F-213 review, working pass 4

**Reviewed**: `work/f-213-xai`, range `bc4a96ad..f68b8ed9`, HEAD `f68b8ed91860e5790f9ea688dd889d31da216305`. No unstaged tracked paths when the pass began or ended. Untracked `.venv` and `tools/oracle/out` exist and are outside the diff.
**Author / reviewer**: a Claude implementer wrote the range, remediating `.claude/reviews/F-213-working-pass-3.md` (D1 linear post, D2 every-frame decision, D3 a wait that can fail, D4 interim tile label, N4). This pass is an independent Claude reviewer launched by the sprint lead. It did not write or edit any reviewed code.
**Result**: 1 defect, 2 smells, 5 nitpicks

## Acceptance challenged

- Pass 3 D1, replayed with `f213p3-bigpost.mjs` on the release binary of HEAD, twice. Per-frame VOI against the shared-VOI control: 500 frames 31 ms against 6 ms, 1000 frames 30 ms against 10 ms, 2000 frames 57 ms against 19 ms. Pass 3 measured 656, 2469 and 9662 ms for the per-frame files. Extended past pass 3: 8000 frames 249 ms against 75 ms, and 40000 frames 1138 ms against 377 ms, where the replies pass the 16 MiB cap and `kept` stayed 0. The curve is linear. Closed.
- Pass 3 D2, replayed with `f213p3-lateprobe.mjs` and `f213p3-latepage.mjs` on `f213p3-files/late.dcm` and `late-rle.dcm`. The scan now says `windowsPerFrame` true and `kept` is non-zero. Through the page, key 4 then ArrowDown shows `WW 1500  WL 300  F1ONLY` at grey 83 and 0, and key 5 then ArrowDown shows the same with no fault. Closed.
- The step the lead named, a file where only the last frame carries a per-frame VOI. Five frames built for this pass by `f213p4-last.py`, frames 0 to 3 with an empty per-frame item falling back to top-level `40\-600`, `400\1500`, `TOPA\TOPB`, frame 4 alone carrying `300`, `1500`, `F4ONLY`, explicit LE and RLE. The scan says per frame and keeps replies. Through the page, key 5 then four ArrowDowns gave `TOPB 236/94` on frames 1 to 3 and `WW 1500  WL 300  F4ONLY` at 83 and 0 on frame 4, no fault. `GET /held` then read `files` 0 and `frames` 5, so frame 4's list came from the kept replies. ArrowUp returned to `TOPB`. Correct on HEAD, but see S1.
- Pass 3 D3. Mutation P1 below sets the multiframe's frame count to 3, and window.mjs now fails with `the process never held all 3 frames without the posted bytes` and exit 1. Pass 3's PM6 left it green. Closed.
- Pass 3 D4, replayed with `DELAY=1500 f213p3-tiles.mjs single`. During the delay the tiles read `Image 2  WW 400  WL 40 128/0`, `Image 3  WW 1500  WL 300 83/0`, `Image 4  WW 400  WL 40 128/0`, then `B1 236/94`, `ONLY2 83/0`, `B3 236/94`. This is the case the lead named, where the frame's own reply (400/40) differs from the chosen window (1500/-600), and the label gives the numbers it is drawn at. The greys agree with PS3.3 C.11.2.1.2.1 LINEAR: HU 40 at 40/400 is `((40 - 39.5) / 399 + 0.5) * 255 = 127.82`, 128, and at 300/1500 is `((40 - 299.5) / 1499 + 0.5) * 255 = 83.36`, 83. Closed for the inspect wait, see N1 and S2.
- Linked F-209 path, `f213p3-tiles.mjs link` with and without the 1.5 s delay. Key 5 on the one-frame source moved the target tiles from `A0 128/0, A1 128/0, ONLY2 83/0, A3 128/0` to `B0 236/94, B1 236/94, ONLY2 83/0, B3 236/94` and its corner to `WW 1500  WL -600  B0`, and key 4 moved them back, in both runs.
- Kept replies against body replies, `f213p4-bytes.mjs`, every frame plus one past the count. Explicit LE, RLE, the late file in both syntaxes, the last-frame file in both syntaxes, the 500-frame per-frame and shared files and `le16.dcm`: every frame equal byte for byte, 200 and 200, and past the count 400 and 400. JPEG-LS was left out by instruction.
- The rewrite of the reply against the old one. A binary built from `bc4a96ad` in a scratch copy answered the same `/inspect` by body requests. Every reply of every file above is byte-identical to HEAD apart from `windowsPerFrame`, which HEAD now gives as one value for the whole file. On the late and last-frame files the base gave frame 0 `false` and the per-frame frames `true`.
- Per-frame spacing and edges, which the rewrite moved into `frame_fields`. A two-frame copy of `le.dcm` with per-frame Pixel Spacing 1 and 2 and orientation `1\0\0\0\1\0` then `0\1\0\0\0\-1` answered frame 0 `1 1 R L A P` and frame 1 `2 2 A P H F`, by id and by body, which is what PS3.3 C.7.6.2.1.1 gives for those direction cosines. See N3.
- The test-only counter. `INSPECT_WORK` and both increments sit behind `#[cfg(test)]`. `nm` and `strings` find no `INSPECT_WORK` in `target/release/ocelli-folder-viewer` and find it in the test lib. Compiled out.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | worktree at f68b8ed9 | now | 0. lib 88, preview 106, measure 16, value 24, every other suite ok |
| `bin/ocelli.sh gate viewer-browser` | worktree, release build | now | 0, ALL GREEN. multiframe and late lines both `F1ONLY` 83 and 0, 0 faults, 0 posts |
| `scripts/tests/test_guard_catalogue.py` by unittest, `guard_census.py`, `--check-runbook`, `prose_check.py` | worktree | now | 57 ok, census 0, runbook 0, prose clean over 953 files |
| `f213p3-bigpost.mjs` on `f213p3-big` and `f213p4-big` | synthetic 8 by 8 enhanced CT, pydicom 3.0.2 | now, HEAD release | as above, linear to 40000 frames |
| `f213p3-lateprobe.mjs`, `f213p3-latepage.mjs`, `f213p4-lastpage.mjs` | `f213p3-files/late*.dcm`, `f213p4-files/last-*.dcm` | now | as above |
| `f213p3-tiles.mjs single` and `link`, each also with `DELAY=1500` | synthetic five-frame LE, a one-frame source | now | as above |
| `f213p4-bytes.mjs` with `DUMP`, HEAD and a `bc4a96ad` binary | nine synthetic files | now | id equals body everywhere, HEAD equals base apart from `windowsPerFrame` |
| `f213p4-facts.mjs` | `f213p4-files/spacing.dcm` | now | per-frame spacing and edges by id and body |
| `f213p4-nowinpage.mjs`, HEAD and base binary | two windowless CT slices | now | see N2 |
| Rust mutations R0 to R12, `f213p4-mut.py`, scratch copy of f68b8ed9 with its own `CARGO_TARGET_DIR`, deleted with `shutil.rmtree` | scratch copy | now | listed below |
| Page mutations P0 to P4, `f213p4-pmut.py`, window.mjs from the same copy | scratch copy | now | listed below |

Rust mutations, each reverted after its run, the copy's three changed files hashed equal to HEAD before deletion:

- R2, a plane list built per frame in `frame_fields`, and R11, a fresh `InspectParts` per frame, which is pass 3's quadratic shape: red in `per_frame_replies_build_the_shared_parts_once`.
- R3, the decision read from frame 0 alone: red in `later_frames_with_their_own_windows_are_per_frame`.
- R5, the kept reply drops the window fields: red in three preview tests.
- R6, the one-frame check removed, R7, the cap inside the builder ignored, R8, the charge omits the middle part: each red in `kept_replies_are_charged_at_their_heap_bytes`. Pass 3's N4 M9 and M14 are now caught.
- R10, the window rows read from frame 0: red in two preview tests.
- Green: R4, the decision read from the last frame alone, R12, the decision read from frame 1 alone, see S1. R9, the spacing and edge fields read from frame 0, which is also green on the base commit, see N3.

Page mutations: P0 control green. P1, the multiframe wait for 3 frames: red, exit 1. P2, the wait returns at once: green, which is expected for a precondition. P3, the tile label without the WW and WL fallback: green, see S2. P4, R3 under the page: red, the late file showed `TOPB` at 236 and 94 on frame 1 and the fault.

## Defects

### D1, the LLD says one post projects the metadata and builds the plane list once, and a post does both twice

**Where**: `docs/lld/cache.md:197`, "`InspectParts` projects the metadata and builds the frame-independent fields and the plane list once per post". The same claim is the comment of `per_frame_replies_build_the_shared_parts_once` in `examples/folder-viewer/src/preview.rs`, "One post builds the frame-independent part and the plane list once".
**What**: `file_route` in `examples/folder-viewer/src/serve.rs:1660` and `:1666` calls `inspect_json(&body, frame)` and then `per_frame_replies(&body, KEPT_INSPECT_CAP)`. Each parses the Part 10 bytes and builds its own `InspectParts`, so one `POST /file` parses twice, projects the metadata twice and builds the plane list twice. The unit test counts `per_frame_replies` alone, so it cannot see the second. Shared-VOI files pay it too, because `per_frame_replies` now builds the plane list before it decides the file keeps nothing.
**Why it is wrong**: a factual sentence in the LLD that is false is a defect under the microscope's severity table. The "linear in the frame count" half of the same sentence is true, as measured above. The cost is a factor of two on a linear post, not a quadratic one.
**Evidence**: code trace of `file_route`. The measured 2000-frame per-frame post, 57 ms, includes both builds.
**Scope and regression**: introduced by this range. The smallest fix is the prose, for example "once for each reply set it builds", in the LLD and the test comment. A code fix would build one `InspectParts` per post and take both the reply and the kept set from it, and a counter around `file_route` would then pin it.

## Smells

### S1, the every-frame decision's test passes when only the last frame, or only frame 1, is read

**Where**: `examples/folder-viewer/tests/preview.rs` `later_frames_with_their_own_windows_are_per_frame` and window.mjs's `LATE` file. Both use a file whose frames 1 and 2 carry their own VOI, so frame 1 and the last frame are both per frame.
**What**: R4, `windows_per_frame` reading only the last frame, and R12, reading only frame 1, leave the whole folder-viewer suite green. HEAD reads every frame and is correct on the last-frame file above. A refactor to either narrower rule, which is the same locally reasonable shortcut pass 3's D2 caught, would pass every test and bring back pass 3's wrong label on a file whose own-VOI frame is in the middle.
**Why it is a smell**: README step 6 says "when any frame's window comes from its own per-frame group, not only frame 0's", and the microscope's second check asks that a test fail when the code is wrong.
**Scope and regression**: this range. The smallest regression is a file whose only own-VOI frame is neither frame 1 nor the last, for example frame 2 of five, asserting `windowsPerFrame` true and that frame's own list. R4 and R12 are the checks that it goes red.

### S2, the interim tile label has no test

**Where**: `examples/folder-viewer/src/page.html:2562` and README step 6, "While its frame's list is on its way its label gives the WW and WL it is drawn at".
**What**: P3, which puts back the old label with no numbers, leaves window.mjs green. `tileProblems` judges only settled tiles. The fix is correct today, as the delayed replay shows, but nothing would notice it being lost.
**Why it is a smell**: the sentence is the remediation of pass 3's D4 and it is executed by nothing in the gate, which is the second check's failure shape.
**Scope and regression**: this range. Pass 3 named the needed piece, a delayed `/inspect` route. A window.mjs step could route `**/inspect` through a delay, read the tiles once before it ends, and require each tile without a window word to carry `WW` and `WL` equal to the numbers its greys imply. P3 is the check that it goes red.

## Nitpicks

- N1. In the delayed linked run one sample read `Image 2 128/0`, a tile showing its previous picture with no window in its label. `paintTiles` resets each label to `Image N` before awaiting that tile's preview, and the tiles are fetched one after another, so each tile is unlabelled for one preview round trip. Not introduced by this range. Measured under 40 ms here, unmeasured when a preview waits for grey room.
- N2. A tile drawn at full range now reads `Image 1  WW 2000  WL 2000`, where the base read `Image 1` and the single view's corner reads `WW 2000  WL 2000  full range`. The numbers are the ones drawn, but the tile does not say full range. The new page comment describes only the interim case.
- N3. No test reads per-frame spacing or edges from `/inspect` for a frame other than 0. R9 is green on HEAD and on `bc4a96ad`. Behaviour is right, as `f213p4-facts.mjs` shows.
- N4. Carried from pass 3: N1 (each tile list arrival repaints every tile), N2 (an id past the count after the bytes are dropped), N3 (replies of evicted ids go only when asked or under cap pressure). Not touched by this range.
- N5. Carried from pass 3 N4 M12: `keep_inspect`'s replace branch in `examples/folder-viewer/src/held.rs:1501` is still unreachable, because every post gets a fresh id. M11's branch was removed with the plane tail split.

## Verified clean

- Pass 3 D1, D2, D3 and D4 closed as replayed above. Pass 3 N4 M9 and M14 closed, R6 and R7 red. Pass 3 N5 closed, the LLD now names the strings and the frame list.
- The kept replies equal the body replies for every frame of every file built, explicit LE and RLE, and the rewritten reply equals the base reply apart from the one intended field.
- `INSPECT_WORK` is test-only and absent from the release binary.
- The counter test goes red for both a per-frame plane list and pass 3's per-frame `InspectParts`.
- README's pool paragraph and step 6, `docs/lld/cache.md`'s paragraph apart from D1, use.html's changed paragraph and the catalogue's new text were each executed against the tree. The catalogue's "a multiframe step whose process never holds every frame without the posted bytes" is what P1 shows.
- No new `as` cast and no pixel arithmetic changed. Greys still come from `ocelli-pixel`.

## Limitations and handoff

- Synthetic files only. No JPEG-LS, by instruction, and no JPEG 2000 or HTJ2K copy, which reach `per_frame_replies` through the same parse.
- `bin/ocelli.sh gate --sprint` was not run, per the verify-once-at-sprint-end rule.
- The scratch copies `f213-p4` and `f213-p4-base` and the target `f213-p4-target` were deleted with `shutil.rmtree`. Scripts and files of this pass are under the scratchpad with the `f213p4-` prefix.
- Next owner: the implementer fixes D1's prose or the double build, adds S1's middle-frame file and S2's delayed tile step. A fresh independent pass is then needed on the changed material, re-running R4, R12 and P3.
