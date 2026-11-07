# F-224 review, pass 4

**Reviewed**: `git diff 71261c31 86708497`, the folder viewer half of F-224 (commits 1e2a0bfc, 1251f60f, 86708497) in worktree `ocelli-wt-f-224`, branch `work/f-224-xai`, HEAD `867084977ecd719e6b7538e0f6547ddf315c574b`, tree `92b8cf9afb9248c9fe946c765b68d05b2d76a22c`. No unstaged tracked paths. Untracked `.venv` and `tools/oracle/out` were present and are not part of the review. Normative source: `.claude/plans/F-224-design.md`, decisions 8 to 11, AC13 to AC17 and AC19, and "Lead amendments for the folder viewer half, against the sprint head after F-226", which win where they differ.
**Author / reviewer**: rounds by a Claude implementer, per the lead's brief. The commit author field is Atul Sharma, so authorship rests on the brief, not on a trailer. Reviewer Claude Opus 5.5 as an independent `/microscope` subagent. It wrote none of this material and none of the earlier F-224 material.
**Result**: 3 defects, 2 smells, 2 nitpicks

## Acceptance challenged

- **The late-reply guard in `requestGrey`, the race the implementer could not drive (its P9).** Driven with a reviewer script that holds every `/preview` reply in Playwright's `page.route` after the process answered, so the reply carries the old bytes and arrives after the probe. Main view: image 1 drawn, held on, `i` (invert, a new cache key), file 0 rewritten with its modification time 5 s later, window `focus`, the status counts the file, then the replies are released. HEAD: the viewport shows the decision 10 sentence and the grey cache holds 0 keys for file 0's serial. With the guard line removed (P9) and the binary rebuilt: fault `""`, the canvas painted 74 at (5, 2), which is 255 minus the old grey 181, and 1 cache key for file 0. Prefetch: on image 2, held on, `i`, so file 0's inverted grey is a prefetch, mark, release, then `ArrowUp`. HEAD: sentence, 0 keys before the step. P9: 1 key before the step and the old inverted grey painted. So the guard is correct and load-bearing, and `changed.mjs` does not watch it (S1).
- **During the hold**, the viewport kept showing the earlier non-inverted image with no sentence until the held reply settled. The final state is right, and decision 10 does not bound that latency, so it is not counted.
- **Tile mode.** Five files, Shift+G. File 1 (tile 2) changed and focus: tiles `8x8, 1x1, 8x8, 8x8`, no fault text, status counts 1. Then file 0 (tile 1) changed and focus: the whole tile grid is hidden, the viewport shows the sentence and status counts 2, so tiles 3 and 4, whose files did not change, are hidden too. D1.
- **Another viewport.** Layout 2 by 1, the same series loaded into both cells, file 0 changed: both cells show the sentence, and `GET /held` counts 4 for six files with two changed. Two series in two cells: a change in the inactive cell's series is not found at focus, which README and `use.html` state ("of the active series"). Moving the active cell to the other series leaves the status saying "1 file of this series changed" for a series with none changed. D2.
- **The linked window path from F-209.** Two cells on one series, cell 1 on the changed file 0, cell 0 active on image 2, `links.window` on, preset `lung` applied in cell 0. `copyWindow` gave cell 1 preset `lung`, and cell 1 kept the sentence with no stage shown. The new key misses and `ensureFile` refuses.
- **Thumbnail.** Three files, the thumbnail is drawn from the middle file. File 1 changed and focus: the thumbnail's red-channel sum went from 2,391,296 to 0 and `thumbFiles` emptied. With the thumbnail loop in `markChanged` disabled (`if (true) continue;`), `changed.mjs` exits 0. S2.
- **Can an unchanged file be marked changed?** A `touch` that moves only the modification time, bytes identical, is marked changed and forgotten. That is decision 10's rule (a refused read marks the file), and the sentence says "changed or was removed". A deleted file is marked too, and the status counted 2 for one touched and one deleted file. Not counted, see N2.
- **Reopen shows new bytes.** After a detected change and a new `setInputFiles`, the canvas at the 16 points equals the process grey for the new bytes, no fault, status `1 series.` with the note gone. `changed.mjs` covers the same for the restored modification time case, where the size and time are equal across the rewrite.
- **Forget against the server's held state.** `markChanged` deletes the id from `openIds` before `forgetIds([id])`, so the next `beginOpen` does not name it twice. `changed.mjs` shows held 3 then 2 and 410 for the old id, and the two-cell case above shows 4 of 6. A re-post through `postFile` that is in flight when the probe marks the file would put a fresh id into `openIds` and `filePosts` after the forget. `ensureFile` never uses it because the changed check comes first, and the next open or `pagehide` forgets it. This needs a 410 eviction and a post landing inside the probe's window, and I did not drive it. Unconfirmed, not counted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate viewer-browser` | worktree HEAD 86708497, Chromium 151.0.7922.34 | now | 0. aspect, readout, link, forget all green. changed: sentence, old grey on 16 of 16 points before the fault hid it, held 3 then 2, old id 410, restored unmarked, probe 1693 reads of 1693 bytes, peak 4, 145.6 ms |
| `cargo test -p ocelli-folder-viewer` | worktree HEAD | now | 0. 202 tests passed across the targets, including `the_page_keys_pixels_by_file_and_probes_for_changes` |
| `python3 scripts/guard_probe.py --only` the three `viewer-browser.changed-*` probes | worktree HEAD | now | 0. All three red for their declared reason, controls green |
| `python3 scripts/guard_census.py`, `--check-runbook`, `test_guard_catalogue.py` | worktree HEAD | now | 0, 0, OK |
| `npx eslint examples/folder-viewer/tests/browser/changed.mjs`, `python3 scripts/prose_check.py` | worktree HEAD | now | 0, clean over 923 files |
| `bin/ocelli.sh gate --list` | worktree HEAD | now | the `viewer-browser` row names changed.mjs and F-224 |
| Reviewer race script, cases lateMain, latePrefetch, tiles, twoCells, touched, reopen, thumb, linked | scratch copy of HEAD from `git archive`, own `CARGO_TARGET_DIR`, release build, synthetic 8 by 8 CT only, `OCELLI_PORT=0`, one browser at a time | now | as in Acceptance challenged |
| P9, guard line removed, lateMain and latePrefetch | scratch copy | now | old grey painted and cached in both |
| T1, thumbnail loop disabled, `changed.mjs` | scratch copy | now | 0, green |
| T2, `PROBE_WIDTH = 3`, `changed.mjs` | scratch copy | now | 1, "the probe had at most 3 reads in flight, not 4" |
| T4, `dropFileGrey(file)` removed from `markChanged`, `changed.mjs` | scratch copy | now | 1, "the viewport showed \"\", not the changed-file sentence" |
| `use.html` section 12 Rust sample, verbatim, as a scratch integration test of `ocelli-cache` with the context it names | scratch copy | now | compiles, the match takes `Hit`. Three unused-binding warnings from the sample's empty arms |
| Implementer mutations P1 to P12 | progress note `.claude/scratch/F-224-progress.md` | prior receipt, not re-run except P9 | as recorded there |

## Defects

### D1, a changed file in tile mode is not shown as the sentence

**Where**: `examples/folder-viewer/src/page.html`, `paintTiles` catch branch (tile set to 1 by 1), reached from `markChanged`'s comment "every shown viewport on it is drawn again, which shows FILE_CHANGED". Prose: `README.md:191` "a file the browser refuses to read is forgotten by the process and shown as "This file changed..."", `use.html` section 12 "its viewport reads "This file changed...""
**What**: in tile mode a changed file on tiles 2 to 4 becomes a blank tile with no sentence anywhere in the viewport. A changed file on tile 1 hides the whole grid behind the sentence, including tiles whose files did not change.
**Why it is wrong**: decision 10, "if it is on screen the viewport shows the sentence ... instead of the old image". The two user documents state the same without a tile exception. The implementer listed this as open.
**Evidence**: reviewer case `tiles` on HEAD: `afterTile2` tiles `8x8, 1x1, 8x8, 8x8`, fault `""`. `afterTile1` tiles hidden, sentence shown.
**Scope and regression**: tile mode only. No stale grey is shown, so this is a statement and presentation defect, not a stale pixel. Smallest fix is either the sentence (or a short form) in the tile's label for a `FileChanged` error, or a documented tile exception in README, `use.html` and the `markChanged` comment. A tile case in `changed.mjs` would pin whichever is chosen.

### D2, the status count names the wrong series after the active cell moves

**Where**: `examples/folder-viewer/src/page.html`, `setActive`, which calls neither `noteChanged` nor `probeActive`. `noteChanged` counts the active series.
**What**: with two cells on two series and one changed file in cell 0's series, a click on cell 1 leaves the status reading "1 file of this series changed on disk after the folder was opened." while cell 1's series has none changed.
**Why it is wrong**: decision 10, "The status line counts the changed files of the series", and README "The status line counts such files". The line states a count for "this series" that is false.
**Evidence**: reviewer case `twoCells`, `s2`: `active 1`, status "2 series. 1 file of this series changed on disk after the folder was opened."
**Scope and regression**: any layout with more than one cell. Smallest fix is `noteChanged()` in `setActive`. Whether activating a cell should also probe is a lead call, since the documented triggers are focus, visibility and series selection. A two-cell case that moves the active cell and reads the status would pin it.

### D3, README says every image the page keeps is keyed by the file object

**Where**: `README.md:191`, "every image the page keeps is keyed by the file object that folder choice gave it, never by the file's name, size or time"
**What**: thumbnails are kept in `thumbBitmaps` keyed by `series.key`, which is `"uid:" + Series Instance UID` or `"file:" + order`. Only the grey cache and the in-flight map are keyed by `fileSerial`.
**Why it is wrong**: the lead amendment says "Thumbnails are keyed by series, not by file". The README sentence contradicts it. `use.html` section 12 says "every grey it keeps", which is true.
**Evidence**: `grep -n "thumbBitmaps.set" page.html` gives `thumbBitmaps.set(series.key, ...)`. `groupSeries` builds the key.
**Scope and regression**: README only. Thumbnails are cleared on every open, so no stale thumbnail follows from it. Fix: "every grey the page keeps", as `use.html` has it.

## Smells

### S1, the late-reply guard in `requestGrey` is watched by nothing

**Where**: `page.html`, `requestGrey`, `if (changedFiles.has(image.file)) throw changedError();`, and `examples/folder-viewer/tests/browser/changed.mjs`.
**Failure mechanism**: removing the guard leaves `changed.mjs`, the AC16 pin and the gate green (the implementer's P9, and confirmed here), while the reviewer's held-reply cases then paint and cache the old grey of a changed file in the main view and through prefetch. That is the stale pixel this story exists to remove, and a later refactor of `requestGrey` or `fetchGrey` can lose it without a red check.
**Smallest regression**: a held-reply case in `changed.mjs`. `page.route("**/preview")` with `route.fetch()`, hold, a new key (`i`), rewrite and focus, release, then assert the sentence and that no cache key starts with file 0's serial. The reviewer script did exactly this in about 3 s.

### S2, the thumbnail branch of `markChanged` is watched by nothing

**Where**: `page.html`, `markChanged`, the `thumbFiles` loop.
**Failure mechanism**: `changed.mjs` changes file 0 of three, and the thumbnail is drawn from the middle file, so the loop never matches. Disabling it leaves `changed.mjs` green (T1). The lead amendment makes this branch the whole of the thumbnail rule, and a regression would keep the old grey in the series list.
**Smallest regression**: rewrite the middle file in one case and read the thumbnail canvas, which went to all zero on HEAD.

## Nitpicks

- **N1.** `more.html` `#worker`: "A file rewritten on disk after the folder was opened is forgotten too, once the browser refuses to read it at the next focus, return to the page or series selection". Only files of the active series are read then. README and `use.html` say so, and `more.html` could say "a file of the active series" for the same precision.
- **N2.** A `touch` with identical bytes is reported as changed. That follows from decision 10 and the sentence covers it with "changed", but the user documents could say a moved modification time alone counts.

## Verified clean

- AC13: the sentence after focus, the status ending with the count, held one fewer, 410 for the old id, image 2 drawing and equal to the process grey. T4 and the implementer's P1, P8, P11, P12 show the check fails for each wrong page.
- AC14: the chosen-again canvas equals the process grey of the new bytes at 16 points and differs from the old at (5, 2). The comment's arithmetic is right under PS3.3 C.11.2.1.2.1 LINEAR: stored 1147 is 123 HU, ((123 - 39.5) / 399 + 0.5) * 255 = 180.86, and stored 1000 is -24 HU, ((-24 - 39.5) / 399 + 0.5) * 255 = 86.92, matching the observed 181 and 87. The restored case is the equal size and equal time rewrite, which is where a path, size and time key would hit, and P3 shows it red.
- AC15: the restored rewrite is not marked, the status has no "changed", the held image stays, a new open shows the new bytes.
- AC16: the pin checks copy before reset before `readFolder(files)`, the selection probe, the focus and visibility listeners, the one-byte slice, the serial key and the refusal order in `ensureFile`. The browser check counts reads on a series row click.
- AC17: 1693 reads of 1693 bytes, peak 4, 145.6 ms against 1000. T2 red at width 3.
- AC19 and the documents: README paragraph apart from D1 and D3. `use.html` 5.6 has eight `postWadoInstance` options, matching `WadoInstanceRequest` and `WADO_INSTANCE_REQUEST_KEYS`. `refuseUnknownOptions` throws `OcelliError` `UnknownOption`. `post` is called once with the body and boundary. `cache` is spread only when defined, so the client default `"no-cache"` applies. Section 12's paragraph matches `ImageCache::get` and `HeldFrame::matches`, and its Rust sample compiles verbatim and hits. `more.html` `#worker`: `ImageCache` appears in no wasm, worker or `packages/core` source, and the `cache` sentence matches `instance-bytes.ts`. Decision 11's three limits appear in README and `use.html`.
- Lead amendments: the forget is `forgetIds`, F-226's helper, with no new route. `cacheKey` changed only its file prefix. The check is wired into the `viewer-browser` arm and its `--list` row. The `postWadoInstance` option table is present.
- The catalogue's three probes go red for their stated reasons, the census and runbook check pass, and the renumbered runbook rows 584 to 589 agree with `--check-runbook`.
- Boundary and tier: no wasm-bindgen, no new route in `serve.rs`, no pixel arithmetic in the page. The page's grey still comes from the local process.
- No regression in F-208, F-209, F-210, F-211, F-216 or F-226 on the evidence of the `viewer-browser` gate and `cargo test -p ocelli-folder-viewer`, both green on HEAD.

## The implementer's six open points

The list was not on disk in the worktree or the scratchpad, so these are the six open items in `.claude/scratch/F-224-progress.md`. If the lead holds a different list, the rulings below may not cover it.

| Point | Ruling |
|-------|--------|
| AC18, bench at sprint verification | Not a defect. The plan stages it as external at sprint verification |
| CHANGELOG at integration | Not a defect. The lead amendment says so |
| Tile mode shows a blank tile, not the sentence | Defect, D1 |
| P9, the late-reply guard race not driven | Guard correct on HEAD, but unwatched, S1 |
| P3b, serial key with an open that keeps the cache, green | Not a defect. That is decision 8 working, the serial alone separates two opens |
| P7, `PROBE_WIDTH = 8`, the source pin green | Not a defect. The pin does not claim the width, and the browser check goes red for it (and for 3, T2) |

## Limitations and handoff

- `visibilitychange` was not driven in a browser. It is pinned in source by the AC16 test only.
- The in-flight re-post race against the forget is reasoned, not driven, and is not counted.
- Only Chromium 151.0.7922.34 headless was used. AC18 was not run.
- The scratch copy at `/private/tmp/claude-501/.../scratchpad/f224-p4` and its target directory were removed with Python `shutil.rmtree` after the pass. The reviewer race script lived only there.
- Next owner: the implementer for D1 to D3, S1 and S2, then a fresh independent pass 5 on the changed material. This pass is a bounded review of the folder viewer half and does not establish sprint readiness.
