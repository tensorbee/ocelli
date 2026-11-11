# F-207 polish review, pass 2

**Reviewed**: working tree on `sprint/s46`, HEAD `d48c46853660672230d4540e07f8865742e27856`, unstaged paths only. The blobs below were pinned at the start of the pass and copied to the scratchpad.

| Path | Blob |
|------|------|
| `examples/folder-viewer/src/page.html` | `c7ac1927aac997bd72e2e035b41f722d429e3f62` |
| `examples/folder-viewer/src/more.html` | `bbaebab93a7e4dcc1cd3e59ffef14825d31ccae5` |
| `examples/folder-viewer/src/use.html` | `fa56c9db78d38b1718bb261604c980e090d9e17d` |
| `examples/folder-viewer/src/serve.rs` | `a8cf6884c24b0dfd03114ebb538b865b650c5d8e` |
| `examples/folder-viewer/tests/preview.rs` | `2519d1c17661318605e1553dfb0fc5cf31ab1fe0` |
| `docs/sprints/BACKLOG.md` | `45569fa67beecaab304660bc85f52580a61cf5d4` |
| `docs/sprints/allocation.json` | `2d13f4ecc36fad742877a1c03740c744c1310b22` |
| `docs/sprints/SPRINT_PLAN.md` | `e40eba1e9ec70f73548feee483fa7587c03b5f74` |
| `docs/sprints/CURRENT_SPRINT.md` | `ba12d79add1ac0fb95e7216f1f95e77d57507a11` |

The three pages served at `http://127.0.0.1:8742/`, `/more` and `/use` hashed to the same blobs as the files.

**Changed during the pass and not reviewed**: the four sprint documents moved again while this pass ran. They now add F-217 (licence hygiene) to S46. The current blobs are `e03b8b3` (BACKLOG), `23fad77` (allocation) and `f810bc0` (SPRINT_PLAN), and CURRENT_SPRINT also changed. The F-217 rows and anything else that landed after the pins are outside this pass.

**Author / reviewer**: the reviewed material was written by other participants. The reviewer is a Claude subagent that wrote none of it. Three read-only Explore subagents fact-checked API names in `more.html` and `use.html` against the source. The reviewer re-checked each of their findings that is reported here (file and line given). The only file edited is this report.

**Result**: 13 defects, 1 smell, 7 nitpicks

## Acceptance challenged

- **D1 of pass 1, a changed file reported as the process being gone.** The pass-1 script was replayed: open a 3-file folder, append 16 bytes to `s0.dcm`, press `i`. The status stayed `1 series.` with no fault. The view read the new UNREADABLE sentence. ArrowDown showed the next image and cleared the view fault. Adjacent case: `/preview` and `/synthetic` were aborted through `page.route`, which simulates a dead process. The status became GONE_STATUS. After the routes came back, the first uncached image restored `Synthetic study, not a patient. 1 series.` A probe that answers 404 (a different process on the port) still counts as gone. **Closed.**
- **D2 of pass 1, tool help sentences.** Hovering `tool-rect`, `tool-length`, `tool-scroll` and `tool-wl` filled both the tooltip and the sentence bar. With Length selected and nothing hovered, the bar kept the Length sentence. In 2x2, hovering Rectangle gave `Drawing works in the single view.`. Every SENTENCE key has an element. The only buttons with no SENTENCE are `remove`, `edit-text` and `close-headers`, which are text-labelled. **Closed.**
- **D3 of pass 1, clipped toolbar.** Sweep at 1920, 1440, 1439, 1280, 1024 and 800 px. No toolbar button is outside the bar at any width. The bar wraps to 2 rows at 1280 and 1024, and to 3 rows at 800. `#wrap` shrinks to match, and the view still fits. Closing and reopening the panels at 1280 and 1024 re-wraps and refits the view. The page has no horizontal or vertical scroll at any of those widths. **Closed.**
- **D4 of pass 1, empty alphabetic PN group.** `=山田^太郎=やまだ^たろう` gives `山田, 太郎`, and `==やまだ^たろう` gives `やまだ, たろう`. The Adams example is unchanged. **Closed.** N3 below is one adjacent case.
- **D5 of pass 1, smoothing.** The tiles are `pixelated` at 620x500 (fit below 1) at DPR 1 and 2, and at 1400x900. The single view switches to `auto` exactly when `scale*min(aspect)` falls below 1 (zoom 74%, effective 0.954), and it is `pixelated` above that. This matches the operator decision stated in the brief. **Closed.**
- **S1 of pass 1, aspect test.** Still open, see S1.
- **`OCELLI_PORT`.** The built binary was run with `""`, `"   "`, `65536`, `-1` and `abc`. Each one exits 1 with `the local page could not start: OCELLI_PORT is not a port number`. With `8742`, ` 8742 ` and `+8742` the error is `Address already in use`, so the value is trimmed, parsed, and the bind fails while the running viewer holds the port. That supports the doc comment's "a second copy fails to bind". `0` was not run, because doing so would start a second viewer. It reaches `bind(("127.0.0.1", 0))` by reading the code. Unset reaches `Ok(0)` and the unchanged `println!("http://{addr}/")`, so the default protocol still holds.
- **Nothing is published.** `git tag -l 'v*'` is empty. The CHANGELOG's only section is `## Unreleased`. `npm view @ocelli/core` and `npm view @ocelli/react` both return E404. crates.io says `ocelli-core` and `ocelli-dicom` do not exist.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo test -q -p ocelli-folder-viewer` | working tree = pinned blobs | now | 0, 31 passed |
| `cargo build -p ocelli-folder-viewer`, then the binary with eight `OCELLI_PORT` values under `perl -e 'alarm 3'` | pinned `serve.rs` | now | as described under Acceptance challenged |
| `python3 scripts/backlog_check.py` | live docs, after F-217 landed | now | 0, `OK: 240 F-IDs` |
| `python3 scripts/gen_sprint_plan.py --check` | live docs | now | 0, `OK: 203 planned F-IDs`. It passes over D11 |
| `python3 scripts/prose_check.py`, `python3 scripts/source_provenance_check.py` | live tree | now | 0 and 0. prose_check does not scan HTML |
| `prose.py` (HTML parser, visible text outside `pre`, `code`, `style` and `script`, plus title, aria-label, alt and placeholder) and a grep of JS string literals | pinned three pages | now | no em-dash and no semicolon. One en-dash in a page.html string literal, D1 |
| `d1.js`, `d1b.js`, `d1c.js`, `d2.js`, `d3.js`, `d3b.js`, `d5.js`, `d5b.js`, `lay.js` under `scratchpad/review2/`, Chromium headless | served page = pinned blobs, the synthetic study, a 3-file copy of synthetic slices | now | quoted per finding |
| `node` eval of the pinned `personName` | pinned page.html | now | see D4 replay and N3 |
| Three Explore fact checks of `more.html` and `use.html` against `crates/`, `packages/`, `examples/viewer-react`, `bin/ocelli.sh` | pinned pages, live source | now | each finding reported here was re-checked by the reviewer at the cited line |

## Defects

### D1, an en-dash in visible page text
**Where**: `examples/folder-viewer/src/page.html:1868`, `"Images " + (viewIndex + 1) + "–" + last + " / " + count`
**What**: the 2x2 corner reads `Images 1–4 / 12` with U+2013.
**Why it is wrong**: the brief's prose rule forbids en-dashes in visible text. `scripts/prose_check.py:54` treats `[—–]` as one class. The string is new in this change, since HEAD has no en-dash.
**Evidence**: `grep -n "[–—]" page.html more.html use.html` returns only this line. `git show HEAD:…page.html | grep "[–—]"` returns nothing.
**Scope and regression**: the 2x2 lower-left corner. Pass 1 quoted this string without flagging it. Smallest fix: `" to "`, giving `Images 1 to 4 / 12`.

### D2, the /more MPR example cannot produce a sample
**Where**: `more.html:242` (`let volume = Volume::assemble(candidate)?;`) and the comment at `:257`
**What**: `Volume::assemble` builds geometry with an empty `VoxelStore` and `SlicePresence::new(count)`, and marks no slice present (`crates/ocelli-volume/src/lib.rs:563-581`). `sample_at` returns `Absent` for any corner on a slice that is not present (`ortho.rs:270-271`). So the example as written yields only `PlaneSample::Absent`. The comment `// outside the volume, never stored zero` gives the wrong main meaning. The variant's own doc is "A corner with non-zero weight sits on a slice that has not landed" (`ortho.rs:64`).
**Why it is wrong**: the page says "The code below is real API from this repository". A reader copying this gets an all-absent plane and a comment telling them it is outside the volume.
**Evidence**: read at the cited lines. `Volume::construct` (`lib.rs:584-595`) is documented as "The returned volume is complete".
**Smallest fix**: use `Volume::construct(candidate)?` at `:242`. Comment `Absent` as "not landed or outside the volume, never a stored zero".

### D3, the /use build commands state what they do wrongly
**Where**: `use.html:260` and `:262`
**What**: `bin/ocelli.sh wasm  # debug build plus the size gate`. The `wasm)` arm (`bin/ocelli.sh:504-513`) runs only `wasm-pack build … --target web --out-dir pkg`. Its comment says "Release by default". It runs no size check. The size check is the `wasm` gate (`bin/ocelli.sh:195-200`). `npm run build  # tsc --build for each workspace package`. The root script is `npm run build --workspaces --if-present`, and `examples/viewer-react` builds with `vite build`.
**Why it is wrong**: false claims about commands. `:259`, "then measured against the recorded size budget", is true only through `gate wasm`.
**Smallest fix**: `# release build into crates/ocelli-wasm/pkg` and add `bin/ocelli.sh gate wasm  # build plus the size budget`. Make the npm comment `# each workspace's build script`.

### D4, `compare produce` does not call `LutChain::map_into`
**Where**: `use.html:816`
**What**: the page says produce writes from "`LutChain::map_into`, `CpuRaster` and `Volume::reslice_nearest`". `grep -rn map_into tools/oracle/src` finds nothing. Grey goes through `CpuRaster`, which maps per sample with `chain.apply` (`crates/ocelli-render/src/cpu.rs:762`).
**Smallest fix**: say "a `LutChain` through `CpuRaster`".

### D5, `drawing` is not the tier's availability
**Where**: `use.html:496`
**What**: "the tier's `drawing` availability". `useOcelliViewport` returns `drawing: TIER_C_DRAWING` (`packages/react/src/useOcelliViewport.ts:103`), which is the constant `"unavailable"` (`packages/core/src/viewport.ts:54`). It does not read the tier.
**Smallest fix**: "and `drawing`, which is `"unavailable"` today because the shell does not draw".

### D6, `attachSideBySide` does not place anything
**Where**: `more.html:231`
**What**: "places an Ocelli element beside an element another library owns". `packages/core/src/alongside.ts:23-49` enables the seam on the Ocelli element, records the pair in a WeakMap, and returns `{ocelli, neighbour, release}` or `null`. It does no DOM placement or layout.
**Smallest fix**: "pairs an Ocelli element with an element another library owns and enables the seam on it".

### D7, `overlayGrid` is not in `crosshair.ts`
**Where**: `more.html:192`
**What**: "Lives in … `packages/core/src/crosshair.ts`" covers the four imports on `:193`. `overlayGrid` is `packages/core/src/intersection.ts:138`.
**Smallest fix**: name both files.

### D8, the DICOMweb sink `kind` is an object, not a string
**Where**: `use.html:397`
**What**: "with a `kind` of `qido-json`, `wado-uri-part10`, …". `DicomwebResponseKind` (`packages/core/src/bulk.ts:153-162`) is `{type: "qido-json"}`, `{type: "wado-uri-part10"}`, `{type: "wado-rs-instances", boundary}` or `{type: "wado-rs-frames", boundary, mediaType}`.
**Smallest fix**: "a `kind` whose `type` is …".

### D9, the wasm exports are not all in `session.rs` and `canvas.rs`
**Where**: `use.html:271`
**What**: `install_panic_hook` (used at `:283`) and `panic_record_ptr` and `panic_record_len` (used at `:792-793`) are defined in `crates/ocelli-wasm/src/lib.rs:79-99`.
**Smallest fix**: add `lib.rs` to the sentence.

### D10, the CHANGELOG does not open with "Nothing is published"
**Where**: `use.html:229`
**What**: "The changelog's only section is *Unreleased* and opens with the words "Nothing is published"". The section opens `Repository bootstrap. Nothing is published.` (`CHANGELOG.md:9`).
**Smallest fix**: "and says "Nothing is published"".

### D11, SPRINT_PLAN's milestone summaries count M20 stories as M14
**Where**: `docs/sprints/SPRINT_PLAN.md:653` (`_S45 to S45, 2 stories, 13 engineer-weeks._` under M20) and `:668` (`_S46 to S49, 15 stories, 85 engineer-weeks._` under M14). The cause is `scripts/gen_sprint_plan.py:133-147`, and its check at `:225-234` repeats the same walk.
**What**: the generator assigns a whole sprint to the milestone of its first story. In the pinned allocation, F-208 to F-216 carry `"milestone": "M20"` and `"sprint": "S46"`. They are rendered under `### M14` and counted in M14's total. Counted by each story's own field, the pinned allocation gives M20 S45 to S46, 11 stories, 49w, and M14 S46 to S49, 6 stories, 49w. The BACKLOG agrees: its M20 section lists F-206 to F-216 and its M14 section lists F-119 to F-124.
**Why it is wrong**: both lines are false generated prose. `gen_sprint_plan.py --check` cannot catch them, because `rendered_milestones` is "Built by the same walk `render` uses". On the live tree with F-217 the M14 line is now `16 stories, 87`, still wrong.
**Evidence**: `Counter((milestone, sprint))` over the pinned allocation gives `('M20','S46'): 9, ('M20','S45'): 2, ('M14','S46'): 1, …`.
**Smallest fix**: render and check milestone spans and counts from `story["milestone"]`. Emit S46 under each milestone that has stories in it, or say in one line that S46 is shared. Regression: a check fixture where one sprint holds two milestones must reproduce the per-milestone counts.

### D12, the F-216 note misstates what the page posts today
**Where**: `docs/sprints/allocation.json:2902` (F-216 `notes`)
**What**: "Every slice change and every window drag posts the whole file again today". Window drags do post on every move. A slice change does not always post: `fetchGrey` returns a page-cache hit without a request (`page.html:1621-1624`). `d1c.js` confirmed this. After a recovery, toggling invert back to a cached image sent no request. Prefetch also posts neighbours before they are shown.
**Smallest fix**: "Every window drag step posts the whole file again, and every slice not in the page cache posts its file, so a 17 MB radiograph is uploaded on each drag step."

### D13, /more is not one column
**Where**: `more.html:44-47` (`.index` with `grid-template-columns: repeat(auto-fill, minmax(200px, 1fr))`) and `:123-138`
**What**: at 1280 px the feature index is a two-column grid of link and status pairs (`lay.js`: `indexColumns: 2`, screenshot `review2/more-1280.png`). It is one column at 375 and 320.
**Why it is wrong**: the brief requires `/more` to be "one column of plain sentences (plus code fragments)". The index is neither one column nor sentences. The operator may waive this. Until then it contradicts the stated requirement.
**Smallest fix**: `grid-template-columns: minmax(0, 1fr)` on `.index`, or drop the index and leave the section headings.

## Smells

### S1, carried from pass 1 and partly closed: the aspect pin guards one line, not the geometry
**Where**: `examples/folder-viewer/tests/preview.rs:217-222` against `page.html:1435`, `:1706-1707` and `:1774`
**Mechanism**: the new test checks only that the page text contains `return { x: facts.column / least, y: facts.row / least };`. That rejects a swap inside `greyAspect`. It does not reject a swap at the consumers: `aspectX = aspect.y; aspectY = aspect.x;` at `:1706-1707`, or `node.dataset.aspect = String(aspect.x / aspect.y)` for tiles and thumbnails at `:1774`. Both would display the wrong pixel aspect with the suite green. It also turns red on an equivalent rewrite of the line. The brief's restriction to one file meant this pass could not apply the mutation. The conclusion follows from `contains` being the only assertion. The author's mutation was of the pinned line itself, which is the one form the pin catches.
**Smallest fix**: what pass 1 asked for, a behavioural check. A Playwright or node test opens the synthetic study and asserts that the `#view` and `#tiles canvas` width-to-height ratio is 64/67.2 to 1e-3. Observed now: `0.952368` for the view and `0.95237` for the tiles at 1400x900.

## Nitpicks

- N1. `serve.rs:90-99`. An empty or whitespace `OCELLI_PORT` is refused rather than treated as unset, although the doc comment says the port is ephemeral "unless `OCELLI_PORT` names one". A non-UTF-8 value falls to `Err(_) => Ok(0)` without a message. `+8742` is accepted. `chosen_port` has no unit test.
- N2. `page.html:1093-1099`. After the process comes back, GONE_STATUS stays until the next uncached preview. Showing only cached images leaves the status claiming the process stopped. GONE_STATUS also says "Images already shown stay" when the failing request was for the current image, which the fault has replaced (`d1b.js`: `w: 0`). The images do come back from the cache.
- N3. `page.html:1882-1883`. `find(Boolean)` picks a non-empty but all-caret group, so `^=山田^太郎` gives `""`. The fix is to find the first group with any non-empty component. It is rare in practice.
- N4. Imprecisions in /use and /more that stay true as far as they go. Line `use.html:233` gives dcmjs as "for the TID 1500 bridge", but `tid2010.ts` and `secondary-capture.ts` use it too. Line `:234` leaves out the `react-dom >=18` peer. Line `:376` says writeFrame "Releases the ticket on failure", which holds only for commit refused. Line `more.html:320` gives FrameMismatch as "the two UIDs differ", but an empty UID gives it too (`ocelli-render/src/fusion.rs:59`). Line `more.html:280` gives the slab tier-C reason as "no CPU reducer", where the source says "no device image".
- N5. `use.html:229`. The status chapter names only `ocelli-wasm`'s `publish = false`. Eleven crates have no publish field. Both npm packages carry `publishConfig.access: public` and no `"private"`. "Nothing is published" is still true, and the registries confirm it. A reader may still take the chapter as describing a publish guard.
- N6. `use.html:798-800`. `isUsable` returns `boolean` and is not a type predicate (`packages/core/src/fatal.ts:58`). `status.message` therefore runs as JavaScript and would not type-check as TypeScript.
- N7. `CURRENT_SPRINT.md` says the operator assigned implementation to the "Grok Build CLI". CLAUDE.md bans the AGPL Grok JPEG 2000 codec, which must not be used, by that name, and `source_provenance_check.py:51` keys on `grok`. Naming the vendor would stop a later reader conflating the two. The check passes today. The claim itself is an operator decision this pass cannot verify.

## Verified clean

- Every pass-1 trigger was replayed with an adjacent case, as listed under Acceptance challenged. No page errors appeared in any run.
- `serve.rs`. The doc comment matches the behaviour except for N1. The `InvalidInput` kind and message reach `main.rs`'s `eprintln!` and exit 1. Unset still binds 127.0.0.1 ephemerally and prints `http://127.0.0.1:<port>/`.
- Hard requirements for `/use`, from `.claude/plans/F-207-design.md:98` and the brief. It covers plain JavaScript with `@ocelli/core` (chapter 5), React with `OcelliViewport` from `@ocelli/react` (chapter 6), and Rust with `parse_part10` and `LutChain` (chapters 7 and 9). Chapter 9's hand-built chain matches README:253-278. The page says "This folder viewer is not the wasm path" and "Nothing is published" (`:181`).
- Neither page contains a `<canvas` substring, and neither has an `img`, `svg`, `canvas`, `picture` or `video` element. `/more` names overlay, shutter, reference line, MPR, volume, fusion, histogram and metadata, and it shows no result number.
- Neither page has visible em-dashes or prose semicolons. There is no horizontal page scroll at 320, 375 or 1280 px. `pre` and `.table-wrap` scroll inside themselves. The image page `/` is 464 px wide at 375, which no requirement covers.
- API surface, confirmed by the fact checks and spot-checked by the reviewer. Every imported name in both pages is exported from its crate root or from `packages/*/src/index.ts`. The argument orders checked match the source: `LutChain::new`, `VoiTransform::new`, `ModalityTransform::new`, `present_cpu_plane`, `show_slab`, `show_fusion`, `FusionRequest` fields, `ShutterShape` fields, `SyncScene::register` and `set_point`, `DicomwebClient` options and methods, `writeFrame` meta, `OcelliViewport` props, and `STACK_COMMANDS` (11 kinds in 10 rows, with the flips sharing a row). Every "example page reads X unavailable" label exists in `examples/viewer-react/src/{layout,manipulation,measurement}-panel.tsx`. The registry's 16 UIDs and families, 15 available, and the D-18 and D-19 references are correct. All docs/lld files in chapter 17 exist.
- Docs, pinned blobs. F-208 to F-216 have E38.18 to E38.26. Every `depends_eids` maps to the `depends_fids` given, and Frame Time is (0018,1063). `S45 is closed at tag s45. That close delivered F-206 and F-207`: `git log s44..s45` shows exactly those two stories. All three checks pass. "Found by a browser test on 2026-11-05" and "the operator placed them" are provenance this pass cannot verify, and they are not counted.

## Limitations and handoff

- The sprint documents changed during the pass to add F-217. Those changes need their own pass.
- The S1 consumer-swap mutation was not executed, because this pass may edit only this report. Chromium headless only. No Firefox, Safari, touch or real fullscreen. The OCELLI_PORT=0 path was not run, to avoid starting a second viewer. The dead process was simulated with `page.route` aborts, because the running viewer may not be stopped.
- The API fact checks read the source. No snippet on either page was compiled or run.
- Next: the authors remediate D1 to D13 and S1. D11 needs a generator change, D13 needs either the CSS fix or an operator waiver, and S1 needs a behavioural test. Then a fresh independent pass 3 reviews the changed material.
