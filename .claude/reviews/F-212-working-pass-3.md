# F-212 working pass 3, the sprint/s46 merge

Scope: the merge commit 72978bff only, whose parents are 012e0e7d (F-212 clean
at pass 2) and c008b425 (sprint/s46). The merge base is 5451cb7d.

Result: F-212 pass 3, 0 defects, 0 smells, 0 nitpicks.

## F-212 changes survive

For each of the eleven files in `git diff 5451cb7d 012e0e7d`, the added and
removed lines of that diff were compared with the added and removed lines of
`git diff sprint/s46 72978bff` for the same file. All eleven are identical,
including the two conflicted files `examples/folder-viewer/src/lib.rs` and
`examples/folder-viewer/tests/preview.rs`, and the auto-merged `page.html`,
`preview.rs`, `serve.rs` and `README.md`. Both diffs report 1661 insertions
and 54 deletions over the same eleven files.

## Sprint changes survive

For each of the 160 files in `git diff 5451cb7d sprint/s46`, the change lines
were compared with `git diff 012e0e7d 72978bff`. Both name the same 160 files.
159 are identical. The one difference is in `tests/preview.rs`, where a single
added blank line sits at a different hunk position because the two sides'
hunks align differently around the appended blocks. The file content is not
affected, and `cargo fmt -p ocelli-folder-viewer --check` is clean.

## tests/preview.rs, no test lost or duplicated

`#[test]` counts: base 76, F-212 87 (11 new), sprint 83 (7 new), merge 94. The
sorted test function names of the merge equal the union of the base, the 11
F-212 names and the 7 sprint names exactly. No name repeats in any revision.
Neither side removed a base test. The F-212 block starts after the sprint's
last test, and F-212's AC11 edit near line 336 is present.

## Regions both sides touched

- `page.html`. The sprint added forget state (`FORGET_LINES`, `PAGE_TOKEN`,
  `openIds`, `beginOpen`, `keepId`, the `pagehide` beacon) and the
  `X-Ocelli-Page` header on both `/file` posts. F-212 changed `requestGrey`'s
  facts, `paintOverlays`, added `paintScale` and `SCALE_SIZES`, removed
  `personName` and `dicomDate`, and added the scale nodes to `ensureViewport`.
  The hunks do not overlap. No function or upper-case constant is declared
  twice, nothing still calls `personName` or `dicomDate`, and the one inline
  script parses under `new Function`.
- `preview.rs`. The sprint added the JPEG XL registration and the `Xyb` arm of
  `photometric_name`. F-212 changed `inspect_json`, `frame_facts`, the
  orientation letters and `Preview`. They are disjoint and the crate builds.
- `serve.rs`. The sprint added `/forget`, `PageToken` on `Request` and
  `HeaderParts`, and flights in the preview paths. F-212 added three fields to
  `Grey`, set in `from_preview`, and the edge source and calibration reply
  headers in `write_reply_by`. The sprint's new replies are text replies that
  carry no `Grey`, so no grey reply misses the calibration headers.
- `README.md`. The sprint's forget paragraph and transfer syntax count and
  F-212's corners and scale bar item 8 read coherently in the merged text.

## lib.rs attribute

`#![doc(test(attr(forbid(unsafe_code))))]` is the first item after the crate
doc comment and before `mod corners`. `python3 scripts/lint_policy_check.py`
exits 0 and reports 16 library roots forbidding unsafe code in their
doctests.

## Run

`cargo test -p ocelli-folder-viewer` passed every target, with 94 of 94 in
`tests/preview.rs`. The browser gate was not run, by instruction.
