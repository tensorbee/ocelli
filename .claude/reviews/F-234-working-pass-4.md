# F-234 review, pass 4

**Reviewed**: `git diff 9af6c146 bf662c87` on `work/f-234-xai`, HEAD `bf662c87fcfec53db9710497e9de493745e3246f`, tree `506fc7b0103c41be61b18c3b731b2de1a6965d96`. The range touches `crates/ocelli-dicom/src/parse.rs`, `crates/ocelli-dicom/tests/part10.rs` and `docs/lld/dicom-ingest.md`. `git status` was empty before the pass. Afterwards the only new path is this report, and HEAD and `git write-tree` are unchanged.
**Author / reviewer**: GLM in opencode wrote fix round 3, per the caller. The reviewer is a Claude subagent spawned by the lead. It did not write the plan, the lead rulings, the fix, or the pass 1 to 3 reports.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **The lead ruling after pass 3, the fixture route.** I derived the depth from the PS3.16 2026d tables read now, not from the fixture or its comment. TID 1500 row 6 is the "Imaging Measurements" CONTAINER and row 9 includes TID 1501. TID 1501 row 10 includes TID 300. TID 300 row 1b includes TID 301. TID 301 row 11 is INFERRED FROM INCLUDE TID 315. TID 315 row 1 is the CODE and row 2 is `> HAS PROPERTIES NUM` with no further rows. Counting open Sequences from the root: root Content Sequence (1), Imaging Measurements Content Sequence (2), Measurement Group Content Sequence (3), TID 300 NUM Content Sequence holding the CODE (4), CODE Content Sequence holding the row 2 NUM (5), Measured Value Sequence (0040,A300), Type 2 in PS3.3 Table C.18.1-1 (6), Measurement Units Code Sequence (0040,08EA), Type 1 in that table (7), Equivalent Code Sequence (0008,0121), Type 3 in PS3.3 Table 8.8-1 (8). The (0400,0561) > (0400,0550) wrapper of C.12.1.1.9 gives 10. That matches pass 3 and the tree.
- **The fixture against that derivation.** The bytes were written to a file and walked by a hand-written Explicit VR LE reader that reads tags in file order, so it does not depend on pydicom's sorted iteration. Depth 10, every Item ascending, every value even length. pydicom 3.0.1 gives depth 10 and the expected structure.
- **The new measuring test.** It goes red on removing a level, adding a level, removing the Equivalent Code Sequence, breaking the walker three ways, and on the constant at 9. The constant at 17 fails to compile.
- **The walker against the preflight.** With the compile-time assert removed in a scratch copy, `MAX_SEQUENCE_DEPTH = 9` refuses the fixture with `SequenceTooDeep` and 10 accepts it. So the walker's "matching the preflight" counts what the preflight counts on this input.
- **The rest of the suite** at the shipped constant, and the user documents for staleness.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree at bf662c87 | now | 0 |
| `cargo clippy -q -p ocelli-dicom --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target`, `pipefail` | now | 0, no output |
| `cargo test -q -p ocelli-dicom -p ocelli-folder-viewer` | same | now | 0. part10 53 passed. Folder viewer 72, 60, 1, 1, 1, 24 |
| `grep -n "pub const MAX_SEQUENCE_DEPTH"` | `parse.rs:57` | now | `64`. The assert is `64 >= 4 * 10 = 40` and compiles |
| PS3.16, PS3.6, PS3.3 HTML from `dicom.nema.org/.../current` | pages headed "PS3.16 2026d", "PS3.6 2026d", "PS3.3 2026d" | now | TID rows and Types as in Acceptance challenged. TID 1500, 1501, 300, 301 and 315 all read "Type: Extensible" |
| Fixture dumped by a scratch test, walked by a raw Explicit VR LE reader | scratch copy from `git archive HEAD` under `target/f234-p4`, own target dir | now | 556 bytes, max depth 10, no out-of-order tag in any Item, no odd length |
| pydicom 3.0.1 over the same file | scratch venv | now | depth 10. (0400,0561) > (0400,0550) > 5 x (0040,A730) > (0040,A300) > (0040,08EA) > (0008,0121), Numeric Value after the units sequence |
| A, fixture level removed (`num_content` wraps `measured_value`) | scratch | now | red, `the_fixture_measures_the_recorded_deepest_shape_depth` |
| B, fixture level added (extra root Content Sequence) | scratch | now | red, same test |
| E, Equivalent Code Sequence dropped | scratch | now | red, same test |
| C, walker stops recursing | scratch | now | red, same test |
| D, walker counts Items as levels | scratch | now | red, same test |
| H, walker skips the first element of each Item | scratch | now | red, same test |
| G, `DEEPEST_SHAPE_DEPTH = 9` | scratch | now | red, same test (pass 3 N1 replay) |
| F, `DEEPEST_SHAPE_DEPTH = 17` | scratch | now | compile error on the margin assert |
| Assert removed, `MAX_SEQUENCE_DEPTH` 9 then 10 | scratch | now | 9: both fixture tests refused with `SequenceTooDeep`. 10: both pass. `REPOSITORY_TID_1500` measures 5 by the same walker |
| pydicom depth scan of `corpus/data` | main checkout, every file with a DICM prefix | now | 100 files, deepest 2, `synthetic/ct_multiframe_perframe.dcm` |
| `git log -- packages/core/src` | worktree | now | last change 2026-11-05, before the 2026-11-06 writer receipts of pass 2 |
| `prose_check.py`, `source_provenance_check.py`, `guard_census.py` | worktree | now | 0, 0, "1134 refusal(s) ... all claimed", 0 watched by nothing |
| Added prose lines and the commit message scanned for em-dash, en-dash and semicolon | the range | now | none outside code |

The scratch copy, its target dir, the venv and the downloaded NEMA pages were deleted with Python `shutil.rmtree`. Every mutation script asserted its anchor occurred exactly once and confirmed the restored file matched the original. The worktree was never checked out, restored, reset, stashed or cleaned.

## Defects

None.

## Smells

None.

## Nitpicks

- N1. `parse.rs:44` and `dicom-ingest.md:75` say "The deepest known shape is the PS3.16 2026d measurement chain", and the fixture doc says "The deepest known shape the rule measures". A second route is known at the same depth. TID 1500 row 7 includes TID 1410, whose measurements include TID 1419, and TID 1419 row 15 is `> INFERRED FROM INCLUDE TID 315` under the row 5 NUM. That gives root, Imaging Measurements, group, NUM, CODE, then (0040,A300), (0040,08EA) and (0008,0121), also 8. Pass 3 named it. The lead ruling says the prose says known "because ... the deepest is not unique", and "the deepest known shape" still reads as unique. The figures 8, 10 and 64 are unaffected. "A deepest known shape" or "one of the two deepest known shapes" would match the ruling's reason. This does not block, because the ruling chose this wording and every number it carries is right.
- N2. `parse.rs` and `dicom-ingest.md` cite the Measured Value Sequence as "of PS3.3 C.18". It sits in Table C.18.1-1, Numeric Measurement Macro, section C.18.1, which the pre-fix comment cited. The citation is true at the containing level and imprecise.
- N3. `dicom-ingest.md` says "Re-measured on 2026-11-07: ... the TID 1500 document `writeTid1500` writes is 5 deep". What was measured on 2026-11-07 is the captured `REPOSITORY_TID_1500` bytes (pass 3, and this pass at 5). The writer itself was last run on 2026-11-06 (pass 2). The next sentence makes exactly that distinction for the other three TypeScript writers. The claim is true because the captured bytes are that document and `packages/core/src` has not changed since 2026-11-05. Saying "the captured TID 1500 document" would apply the ruling's date rule the same way to all four writers.

## Verified clean

- **Pass 3 D1** is closed. The fixture builds the TID 1500 > 1501 > 300 > 301 row 11 > 315 row 2 route and parses to 10. Each of the three prose locations and the fixture doc names that route and says 8 and 10. `DEEPEST_SHAPE_DEPTH = 10`. I found no remaining "seven", "nine", "7 open" or "9 wrapped" under `crates/ocelli-dicom` or `docs/lld`.
- **Pass 3 S1** is closed. Measurement Units Code Sequence (0040,08EA) comes before Numeric Value (0040,A30A) in the bytes. In the units Item the order is (0008,0100), (0008,0102), (0008,0104), (0008,0121). Every Content Sequence Item and both wrapper Items hold one element. The raw reader found no out-of-order tag anywhere.
- **Pass 3 N1** is closed. The measuring test parses through `parse_part10` and walks the parsed object, and mutations A to H show it tracks the fixture and the constant in both directions.
- **Pass 3 N2** is closed in substance. The other three writers are now dated 2026-11-06 and said not to be re-measured, and N3 above is what is left.
- **Tags and VRs against PS3.6 2026d**: (0008,0100) SH, (0008,0102) SH, (0008,0104) LO, (0008,0121) SQ, (0040,08EA) SQ, (0040,A300) SQ, (0040,A30A) DS, (0040,A730) SQ, (0400,0550) SQ, (0400,0561) SQ. Values "1 ", "2 ", "99SYN ", "99EQ", "synthetic unit", "synthetic equivalent" and "12.5" are all even length.
- **`code_item`'s doc**. Table 8.8-1a heads those three attributes "BASIC CODED ENTRY ATTRIBUTES", and the helper writes them ascending.
- **The walker's doc**. It counts a Sequence and not its Items, and `items()` is `Some` only for a Sequence value. The limit 9 versus 10 experiment shows it agrees with the preflight on this fixture.
- **Every changed prose sentence** in `parse.rs`, `part10.rs` and `dicom-ingest.md` was checked: the TID row citations, the Types of (0040,A300), (0040,08EA) and (0008,0121), "Extensible" for all five templates, "eight open Sequences, ten wrapped", "next power of two at or above four times ten" (40, giving 64), "2 deep over 100 DICOM files", "5 deep", "10 wrapped", and the 2026-11-06 date of the KOS, SC and synthetic slice figures, which pass 2's receipt carries as 2, 0 and 0.
- **The 2 MiB debug claim** still holds. `the_deepest_measured_shape_parses`, `parses_at_the_sequence_depth_limit` and the proptest pass on HEAD.
- **User documents.** `use.html:531` names the constant and the variant but no value, and is still true. `README.md` and `more.html` say nothing about the limit.

## Limitations and handoff

- Not exercised: Explicit VR Big Endian nesting, release builds, the corpus `--ignored` suite (pass 3 ran it on an unchanged parser), `bin/ocelli.sh gate native` and `--sprint`. No GPU or oracle job was run. The TypeScript writers were not rebuilt.
- The template sweep for deeper routes was not repeated beyond TID 1410, 1419 and 315. PS3.16 marks the templates Extensible, so no figure is a ceiling, as the prose now says.
- The LLD's "recorded in the F-234 handoff" points at `.claude/handoffs/F-234-ready.md`, which does not exist yet. Whoever writes it must carry 8 and 10, not the 7 and 9 of the pass 2 ruling.
- This pass reports 0 defects and 0 smells for the reviewed range. That is a bounded clean review of fix round 3, not completed acceptance or sprint readiness. The nitpicks are the implementer's or lead's choice.
