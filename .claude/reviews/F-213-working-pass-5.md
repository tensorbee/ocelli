# F-213 review, working pass 5

**Reviewed**: `work/f-213-xai`, range `0f74fe95..966d319c`, HEAD `966d319cd936611c684ee1496d677352740448ab`. No unstaged tracked paths when the pass began or ended. Untracked `.venv` and `tools/oracle/out` exist and are outside the diff.
**Author / reviewer**: a Claude implementer wrote the range, remediating `.claude/reviews/F-213-working-pass-4.md` (D1 two parses per post, S1 the every-frame decision read narrower, S2 the interim tile label untested, N2 the full-range tile label). This pass is an independent Claude reviewer launched by the sprint lead. It did not write or edit any reviewed code.
**Result**: 1 defect, 1 smell, 4 nitpicks

## Acceptance challenged

- Pass 4 D1, counted per post on both paths. The shipping binary builds `Viewer::server`, so every `POST /file` with a body takes the pooled branch of `file_route`. A counting build of HEAD, with one stderr line per `project_metadata`, `plane_tokens` and branch taken, answered one post of each of `le.dcm`, `rle.dcm`, `mid-le.dcm`, `mid-rle.dcm`, `big500_pf.dcm`, `big500_shared.dcm` and a windowless slice with `pooled 1 direct 0 project 1 planes 1`, on worker threads 2 or 3. The same counting build of `0f74fe95` gave `project 2 planes 2` for every one of those files. The direct branch is what `one_file_post_parses_and_builds_its_planes_once` drives, through `Viewer::new` with no slot, and it reads `(1, 1)`. Code trace agrees: `inspect_post` calls `parse_part10` once, `InspectParts::new` once, and `kept_replies` consumes that same `InspectParts`. Closed in code, but see D1 for the paragraph around the fixed sentence and S1 for the pooled branch.
- Pass 4 S1. `a_middle_frame_alone_with_its_own_window_is_per_frame` builds five frames with only frame 2 carrying its own window. R3 (frame 0 alone), R4 (last frame alone), R12 (frame 1 alone) and a new R13 (frames 0, 1 and last) each turn it red. Closed.
- The middle-frame case through the page. Five frames built for this pass by `f213p5-mid.py` from pass 3's `late.dcm`, only frame 2 carrying `300`, `1500`, `F2ONLY`, explicit LE and RLE. Key 5 then four ArrowDowns gave `TOPB 236/94` on frames 0, 1, 3 and 4 and `WW 1500  WL 300  F2ONLY` at 83 and 0 on frame 2, no fault, both syntaxes. `GET /held` then read `files` 0 and `frames` 5, so frames 3 and 4 were answered from the kept replies. The greys are PS3.3 C.11.2.1.2.1 LINEAR: HU 40 at 300/1500 is `((40 - 299.5) / 1499 + 0.5) * 255 = 83.35`, 83, HU -800 is below `299.5 - 749.5 = -450`, 0, and at -600/1500 HU 40 is `((40 + 600.5) / 1499 + 0.5) * 255 = 236.46`, 236, HU -800 is `((-800 + 600.5) / 1499 + 0.5) * 255 = 93.56`, 94.
- Pass 4 S2. window.mjs now holds every `/inspect` back 2500 ms and requires tiles 2 to 4 to read `WW 400  WL 40` at 128 and 0 before their lists arrive. P3, the old label with no numbers, and P5, the two numbers swapped, each turn window.mjs red with exit 1. P6, the delay cut to 1 ms, also goes red, so the step depends on the hold. The timing margin was measured with `f213p5-interim.mjs`, three runs: no `/inspect` is sent before Shift+G, the three are sent 3 to 9 ms after it, and all four tiles are drawn 13 to 27 ms after it, against the step's 2200 ms wait and the 2500 ms hold. Closed.
- The scan reply and the kept replies against the previous head. Release binaries of `0f74fe95` and HEAD answered, for each of 13 synthetic files, `POST /file` at frame 0, the middle frame, the last frame and one past the count, then `/inspect` by id and by body for every frame and one past the count. Files: `le`, `le16`, `rle`, `late`, `late-rle` (pass 3), `last-le`, `last-rle`, `spacing` (pass 4), `mid-le`, `mid-rle` (this pass), the 500-frame per-frame and shared files, and one windowless slice. The two dump trees, 2130 lines, are identical byte for byte, and by id equals by body for every frame of every file. A truncated Part 10 file and a non-DICOM body give the same 400 reply and `kept` 0 on both. JPEG-LS was left out by instruction.
- Pass 4 N2, the full-range tile label, on two windowless CT slices. HEAD reads `Image 1  WW 2000  WL 2000  full range` beside a corner of `WW 2000  WL 2000  full range`, and key 1 gives `Image 1  Soft tissue` and a corner of `WW 400  WL 40  Soft tissue`. The `0f74fe95` binary read `Image 1  WW 2000  WL 2000`. The README sentence holds. See N1 for the test.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | worktree at 966d319c | now | 0. lib 89, preview 107, measure 16, value 24, every other suite ok, both new tests ok |
| `bin/ocelli.sh gate viewer-browser` | worktree, release build | now | 0, ALL GREEN. `tiles, lists held back` line reads `WW 400  WL 40` at 128/0 on tiles 2 to 4 |
| `test_guard_catalogue.py` by unittest, `guard_census.py`, `--check-runbook`, `prose_check.py` | worktree | now | 57 ok, 0 watched by nothing, runbook 0, voice rules clean over 954 files |
| `f213p5-count.sh` on counting builds of HEAD and `0f74fe95` | seven synthetic files, port 18913 | now | HEAD 1 projection and 1 plane build per post on the pooled branch, base 2 and 2 |
| `f213p5-bytes.mjs` with `DUMP`, HEAD and `0f74fe95` release binaries | 13 synthetic files | now | dumps identical, id equals body everywhere |
| `f213p4-lastpage.mjs` on `f213p5-files/mid-*.dcm` | synthetic, HEAD release | now | as above |
| `f213p5-interim.mjs`, three runs, `DELAY=2500` | window.mjs's four-frame tile file | now | as above |
| `f213p4-nowinpage.mjs`, HEAD and base | `f213p4-nowin` | now | as above |
| Rust mutations, `f213p5-mut.py` | scratch copy of 966d319c, own `CARGO_TARGET_DIR` | now | listed below |
| Page mutations, `f213p5-pmut.py`, window.mjs from the same copy | scratch copy | now | listed below |

Rust mutations, each reverted after its run with the file hash checked equal:

- R0 control green.
- M1, a second `inspect_json` put back on the direct branch: red in `one_file_post_parses_and_builds_its_planes_once`.
- M2, the same on the pooled branch alone: green, see S1.
- M3, `inspect_post` building the kept set from a second parse: red in `per_frame_replies_build_the_shared_parts_once` and `one_file_post_parses_and_builds_its_planes_once`.
- R3: red in the middle-frame test and `later_frames_with_their_own_windows_are_per_frame`. R4, R12, R13: each red in the middle-frame test.

Page mutations: P0 control green. P3 and P5 red, exit 1, with `while its list was on its way tile N showed ...` for tiles 2 to 4. P6 red. P7, the full-range suffix dropped: green, see N1. P8, the decision read from frame 1 alone under the page: green, which is expected because window.mjs's per-frame files all have frame 1 per frame, and R12 covers it in Rust.

## Defects

### D1, the LLD names `per_frame_replies` as what computes the kept replies, and the range made it test-only

**Where**: `docs/lld/cache.md:195` to `:196`, "The replies are computed from the parsed data set by `per_frame_replies`, so every transfer syntax keeps them the same way."
**What**: the range turned `per_frame_replies` in `examples/folder-viewer/src/preview.rs:2748` into a `#[cfg(test)]` wrapper whose doc comment says it is "for the pool's own tests". The shipping binary has no function of that name. The kept replies are computed by `kept_replies`, called from `inspect_post`, which the very next sentence of the same paragraph names. The sentence sits on the line the range edited and was left as it was.
**Why it is wrong**: a factual sentence in the LLD that is false of the tree is a defect under the microscope's severity table. A reader following it to the code finds a test helper, not the production path.
**Evidence**: `git grep -n per_frame_replies` finds it only at the LLD line, the cfg(test) definition and three test call sites. `serve.rs:24` imports `inspect_post` and no longer imports `per_frame_replies`.
**Scope and regression**: introduced by this range. The smallest fix is the prose, for example naming `inspect_post` in that sentence or folding it into the next one. No test change.

## Smells

### S1, the one-parse test drives only the branch the shipping binary never takes

**Where**: `examples/folder-viewer/src/serve.rs:2806` `one_file_post_parses_and_builds_its_planes_once`, against `file_route` at `serve.rs:1652`.
**What**: the test builds `Viewer::new` and a `BodyCharge` with `slot: None`, so it runs the `else` branch of `file_route`. `serve()` builds `Viewer::server`, and every bodied post there takes the pooled branch, which the counting build confirmed on every file above. The pooled closure holds its own copy of the `inspect_post` call. M2, a second `inspect_json` added to that closure alone, which is exactly pass 4's D1 shape, leaves the whole folder-viewer suite green. `INSPECT_WORK` is thread-local, so even a test through the pooled branch could not see work done on a worker thread with the current counter.
**Why it is a smell**: the test comment says "a whole `POST /file` of a per-frame multiframe projects the metadata once and builds the plane list once", and the microscope's composition check says tests for one entry point do not prove another. HEAD is correct today, as the counting build shows.
**Scope and regression**: this range. Two small ways out. One closure, built once and either passed to `slot.call` or called in place, so the direct test runs the same code the pool runs. Or a pooled case through `Viewer::server` with a counter the test thread can read, such as an atomic under `#[cfg(test)]`. M2 is the check that either goes red.

## Nitpicks

- N1. The full-range tile label has no test. P7, the `full range` suffix dropped, leaves window.mjs green. Ruling, asked for by the lead: not a smell under the plan. No AC in `.claude/plans/F-213-design.md` covers tile labels, the plan's padding paragraph says "The full-range window of a file with no window is not changed by this story", and the change remediates pass 4's N2, a nitpick. Losing the suffix returns the tile to `WW 2000  WL 2000`, the numbers it is drawn at, which pass 4 accepted. It would make one README sentence false, so a windowless file in window.mjs's tile step would be cheap insurance, but its absence does not block.
- N2. `README.md:277` was not rewrapped, a 125-character line in a paragraph wrapped near 76.
- N3. Carried from pass 4 N1, still present: a tile shows its previous picture under a bare `Image N` for one preview round trip while it repaints. Seen again in `f213p5-interim.mjs` at 2512 to 2520 ms, as each held list arrived.
- N4. Carried from pass 4: N3 (no test reads per-frame spacing or edges for a frame other than 0), N4 (pass 3's N1 to N3) and N5 (`keep_inspect`'s unreachable replace branch). Not touched by this range.

## Verified clean

- Pass 4 D1 closed in code on both branches, measured on the pooled one and by the unit test on the direct one. Pass 4 S1 and S2 closed, each by mutations that now go red. Pass 4 N2 closed in behaviour.
- `inspect_post` keeps `inspect_json`'s error mapping. A refused parse returns `part10_error` and `None`, a refused `InspectParts::new` returns its error and `None`, and a reply error skips the kept set, which `file_route` discarded on that branch anyway. Observed equal to the base on a truncated and a non-DICOM body.
- `reset_inspect_work`, `inspect_work`, `per_frame_replies` and the `pub(crate)` on `preview::tests` are all under `#[cfg(test)]`.
- The scan reply and the kept replies are byte-identical to the previous head's for every frame of 13 synthetic files, plain and RLE.
- README's changed sentence, the changed LLD sentence apart from D1's neighbour, the page comment and the catalogue's new clause were each executed. The catalogue's "a tile whose frame list is held back and whose label does not give the WW and WL it is drawn at" is what P3 and P5 show.
- `use.html` and `more.html` say nothing about tile labels or the full range, so nothing there went stale.
- No new `as` cast and no pixel arithmetic changed. Greys still come from `ocelli-pixel`.

## Limitations and handoff

- Synthetic files only, no patient data. No JPEG-LS, by instruction, and no JPEG 2000 or HTJ2K copy.
- `bin/ocelli.sh gate --sprint` was not run, per the verify-once-at-sprint-end rule.
- Method note for later passes: two `git archive` copies sharing one `CARGO_TARGET_DIR` reuse each other's artefacts, because cargo hashes workspace members by relative path and the archived files carry commit times older than the built artefact. The first HEAD build here silently produced the base binary. It was caught by its hash and the missing `const drawnAt` text, and the changed files were touched before the real build.
- The scratch copies `f213-p5` and `f213-p5-base` and the target `f213-p5-target` were checked equal to their commits file by file, then deleted with `shutil.rmtree`. The counting binaries were deleted. Scripts and files of this pass are under the scratchpad with the `f213p5-` prefix.
- Next owner: the implementer fixes D1's sentence and S1's pooled branch. A fresh independent pass is then needed on the changed material, re-running M2.
