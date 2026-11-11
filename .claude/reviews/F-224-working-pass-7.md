# F-224 review, pass 7

**Reviewed**: the merge commit `acf87d8c` "F-224, merge sprint/s46" only, in worktree `ocelli-wt-f-224`, branch `work/f-224-xai`, tree `1b148e26be9aff81414f89b33a2ad6bd03a3365b`. Parents are `730e9f50` (F-224 pass 6 clean) and `a2b3098b` (sprint/s46 tip, F-212 integrated). Merge base `69306d05`. The worktree matched `acf87d8c` with no tracked change. Untracked `.venv` and `tools/oracle/out` are not part of the review.
**Author / reviewer**: merge by the lead. Reviewer Claude Opus 5.5 as an independent `/microscope` subagent, which wrote none of the merged material.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Method

Each side's change set was taken from the merge base. F-224 touched 32 paths and the sprint touched 133. Nine paths are on both sides.

- **Paths on one side only.** Every sprint-only path is byte-identical in `acf87d8c` and `a2b3098b`, and every F-224-only path is byte-identical in `acf87d8c` and `730e9f50`. No path outside the union of the two sides changed.
- **Paths on both sides.** For each of the nine, `git merge-file` was run on the base, F-224 and sprint blobs and the result compared with the merged blob. Seven merge cleanly and are byte-identical to the merge: `bin/ocelli.sh`, `crates/ocelli-cache/src/lib.rs`, `examples/folder-viewer/src/more.html`, `examples/folder-viewer/src/page.html`, `examples/folder-viewer/src/use.html`, `README.md` and `scripts/guards/catalogue.py`. The two conflicts are taken below.

## `examples/folder-viewer/tests/preview.rs`

Both sides appended at the same end of the file, so the conflict is one region. The merged file differs from the three-way output only in the three marker lines and in the `    }`, `    Ok(())`, `}` lines that close F-224's test before the F-212 block, which the shared suffix had supplied to only one side. F-224's `block_from` helper and its test sit at lines 4405 and 4421, and the F-212 section begins at line 4497.

| Version | `#[test]` | Distinct test names | Top-level `fn` | Distinct `fn` |
|---------|-----------|---------------------|----------------|---------------|
| base | 79 | 79 | 142 | 142 |
| F-224 pass 6 | 80 | 80 | 144 | 144 |
| sprint tip | 94 | 94 | 164 | 164 |
| merge | 95 | 95 | 166 | 166 |

F-224 adds one test, `the_page_keys_pixels_by_file_and_probes_for_changes`, and the sprint adds fifteen. Neither side removes one. The merged set of test names equals the union of the two sides exactly, and so does the set of top-level function names, so no test was lost or duplicated and no helper name collides.

## `docs/runbooks/guard-verification.md`

Regenerated. `python3 scripts/guard_census.py --check-runbook` exits 0 on the merged tree and reports `OK: 1159 refusal(s) in 80 file(s), all claimed`, with 0 watched by nothing. The generated table has 627 numbered rows against the sprint's 624, and the three added rows are F-224's `viewer-browser.changed-old-image`, `viewer-browser.changed-claimed-detection` and `viewer-browser.changed-whole-file`. F-224's LIMIT paragraph for `changed.mjs` is present once. Every other line differing from the sprint version is a renumbered row.

## `examples/folder-viewer/src/page.html`

Auto-merged, identical to the three-way output. The single inline script (lines 877 to 5242) was extracted and parsed with acorn. It parses, and a scan of every program and block scope for repeated `function`, `class`, `let`, `const` and `var` names found 0 duplicates across 590 scopes and 372 top-level declarations. `fileSerial`, `markChanged`, `changedFiles`, `ensureFile`, `cacheKey`, `paintOverlays` and `paintScale` are each declared once. F-212 removed `personName` and `dicomDate`, and nothing in the merged page refers to either.

How the two sides meet:

- `cacheKey` and `dropFileGrey` carry F-224's `fileSerial` prefix. F-212 adds `calibrationSource` and `calibrationColumn` to the facts `requestGrey` returns, which travel inside the cached grey and are untouched by the key. F-224's two `FileChanged` throws in `requestGrey` stay in place around F-212's new header reads.
- A changed file in single view reaches `showFault`, which now also hides F-212's scale bar. In Four images F-224 calls `hideFault` and `syncStage`, and `paintScale` hides the bar whenever `vp.tileMode` is set. So the bar never stands over the changed-file sentence.
- F-224's tile label code in `paintTiles` and its `.tiles .tile-label.changed` style are untouched by F-212, whose CSS, DOM and `ensureViewport` additions are for `.scale`, `.scale-label` and `.scale-line` only.
- `examples/folder-viewer/tests/browser/changed.mjs` refers to no corner, overlay, scale or removed function. It was not run, per the brief.

## `README.md`, `use.html`, `more.html`

All three auto-merged identically to the three-way output, and the two sides edit different paragraphs. F-224's paragraph on choosing the same folder again follows the forget paragraph in the local series viewer section, and F-212's corner, edge letter and scale bar text is in item 8 of the walkthrough. Both use the control name "Four images". The F-224 `DicomwebClient` `cache` paragraph and the sprint's decoder family sentence sit in separate sections. The text reads as one document with no sentence from either side contradicting the other.

## Other paths on both sides

`bin/ocelli.sh` keeps F-224's `changed.mjs` in the `viewer-browser` arm and its gate list entry alongside F-233's renumbered native steps, and `bash -n` accepts it. `crates/ocelli-cache/src/lib.rs` keeps both sides and the crate builds under the test run. `scripts/guards/catalogue.py` imports under the census and its 92 `id=` probe names are distinct.

## Checks run

| Check | When | Result |
|-------|------|--------|
| one-sided paths equal their side's tip | now | all equal |
| `git merge-file` on the nine shared paths | now | 7 identical, 2 conflicts resolved as above |
| test and fn name counts in `tests/preview.rs` | now | 95 tests, union of both sides, no duplicate |
| acorn duplicate-declaration scan of `page.html` | now | parses, 0 duplicates |
| `python3 scripts/guard_census.py --check-runbook` | now | 0 |
| `cargo test -p ocelli-folder-viewer` | now | 0. lib 81, preview 95, measure 14, value 24, the three single-test files 1 each |
| browser gate | not run | a verification run was using it |
