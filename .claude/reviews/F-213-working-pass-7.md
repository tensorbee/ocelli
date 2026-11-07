# F-213 working review, pass 7

Scope: the merge of `sprint/s46` into `work/f-213-xai` only, commit `b48b97b3`, parents `926024f7` (F-213, clean at pass 6) and `aba1cb09` (sprint/s46), merge base `a2b3098b`.

Result: **0 defects, 0 smells, 0 nitpicks.**

## Method

- `git merge-tree --write-tree 926024f7 aba1cb09` against `b48b97b3`. The merge commit differs from the automatic merge in exactly five paths: the four conflicted files (`bin/ocelli.sh`, `docs/runbooks/guard-verification.md`, `examples/folder-viewer/src/page.html`, `scripts/guards/catalogue.py`) and `examples/folder-viewer/src/serve.rs`, whose one added line is the `"frame-cache"` entry. No other path was edited by hand.
- For each of the eight files both sides changed, `git merge-file -p ours base theirs` against the merged blob. `docs/lld/cache.md`, `examples/folder-viewer/src/use.html`, `examples/folder-viewer/tests/preview.rs` and `README.md` merged without conflict and equal the merged tree byte for byte. The other four were read hunk by hunk below.
- For `scripts/guards/catalogue.py`, every line either side added relative to the base was counted in the merged file. Ours added 73 lines, theirs 114, none missing, and the merged length is exactly `ours + theirs - base` at 14,591 lines. The file parses.

## Findings by file

**`examples/folder-viewer/src/page.html`.** The one conflict is `cacheKey`. The resolution takes F-213's window part, `view.activePreset || "file" + fileKeyIndex(image, view)`, and F-224's prefix, `fileSerial(image.file)`. The key is `serial \0 frame \0 window \0 invert`.

- One file window cannot be served for another. The window part is `"file" + index` or `"file" + "default"`, an override is `center/width/fn`, and a preset is one of `soft_tissue`, `lung` or `bone` (`preset_token` in `preview.rs`). None of those can equal another, and `\0` separates every field.
- A changed file's old grey cannot be served. `dropFileGrey` drops every cache and inflight key starting with `fileSerial(file) + "\0"`, and every key `cacheKey` builds starts with exactly that, whatever window part F-213 adds after it. The delimiter keeps serial 1 from matching serial 12. A reply already in flight when the file is marked throws `changedError` in `requestGrey` before `fetchGrey`, the prefetch or the tile path can remember it. A new folder choice gives new `File` objects and so new serials.
- Every other `cacheKey` caller (`showIndex`, `fetchGrey`, `schedulePrefetch`, `prefetchCovered`, `paintTiles`) goes through the one function, and no key is built by hand anywhere else. `fileIdentity` remains only for `noteKey`, as its comment says.
- `tests/preview.rs` pins both halves on the merged body, line 418 for `"file" + fileKeyIndex(image, view)` and line 4581 for `return fileSerial(image.file) + ` with no `fileIdentity(`, and both pass.

**`bin/ocelli.sh`.** The gate description names all six browser checks and both F-IDs. The `viewer-browser` arm runs `window.mjs &&` then `changed.mjs ;;`, so both run and either failing fails the gate. `bash -n` accepts the file and `python3 scripts/ci_floor_check.py` exits 0.

**`scripts/guards/catalogue.py`.** The first conflict holds both sandbox mutators, `_window_check_accepts_a_stale_grey` and the three `_changed_check_*` functions, separated correctly. The second holds the `viewer-browser.window` Guard and the `viewer-browser.changed` Guard as two complete entries. Each mutator's search string occurs exactly once in its target, one in `window.mjs` and three in `changed.mjs`. `python3 scripts/guard_census.py` reports 0 watched by nothing.

**`docs/runbooks/guard-verification.md`.** `python3 scripts/guard_census.py --check-runbook` exits 0, so the resolved runbook is the rendered one. Rows 597 to 600 and both limit bullets are present.

**`examples/folder-viewer/src/serve.rs`.** The import closure of `packages/core/src/index.ts`, walked independently from the sources, has 48 modules and equals `LIBRARY_FILES` exactly. `frame-cache` is reached by a value export at `index.ts` line 250 to 257, so it is a runtime edge and not a type-only one. Nothing outside `packages/core/src` was added, so the allowlist still covers only `@ocelli/core`'s own modules. `library_files_match_the_index_closure` passes.

**Prose.** No sentence in `README.md`, `use.html`, `more.html` or `docs/lld/` describes the grey key in a way the merged key contradicts. F-224's sentence that greys are keyed by the file object and F-213's window list text are both true of the merged function.

## Verification run

`cargo test -p ocelli-folder-viewer`: every suite passed (90, 1, 16, 108, 1, 1 and 24 tests, plus two empty binaries). The browser gate and the floor were not run, by instruction.
