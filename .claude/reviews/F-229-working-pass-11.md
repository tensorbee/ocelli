# F-229 review, pass 11

**Reviewed**: `git diff a087f61 HEAD`, HEAD `bb7b7c6` on `work/f-229-xai`, AC24 documentation only. One path changed: `docs/lld/oracle.md` (2 lines). No unstaged tracked changes before or after the pass.
**Author / reviewer**: the sprint lead (Claude) wrote `bb7b7c6`. This pass is an independent Claude reviewer that wrote none of the reviewed material.
**Result**: 0 defects, 0 smells, 0 nitpicks (pass 10's N1 to N3 carried, not re-counted)

## Acceptance challenged

AC24, the documentation half. Pass 10's D1 was that `oracle.md:576-577` encoded 2 unsupported rows of 92 while the section below lists seven. I checked three things. First, that the replacement sentence states no count. Second, that it is true of the strict two-way match `run.mjs` performs. Third, that no sentence in "What cornerstone3D 5.8.2 could not render, measured", or in the paragraphs either side of it, now gives a count or claim that contradicts the seven rows the gate measured (`unsupported 7` in pass 10's gate receipt). The wrong behaviours these checks rule out are: a new count that goes stale again, a sentence that overstates what the match guarantees, and a leftover number in the section that disagrees with 1 + 1 + 5.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git diff a087f61 HEAD` | bb7b7c6 | now | one hunk. "covers 90 rows and knows it, and one that covers 90 and reports 92" became "knows which rows it covers and one that reports rows it never rendered". No digits |
| `tools/oracle/run.mjs:1040-1085` read | HEAD | now | a failed row is counted only under `unsupported`, and only if `entryFor` matches. Otherwise a problem is pushed. Each `claimedRows` path that did not land in `accountedFor` pushes a stale-claim problem. `decoded/presented/readBack` count only stages that were reached |
| `tools/oracle/src/unsupported.mjs` read | HEAD | now | `entryFor` / `validateRecord` match the four conjuncts the LLD paragraph above the sentence names |
| `tools/oracle/unsupported.json` read | HEAD | now | five entries. YBR 422 (1 row), deflated (1 row), and .110 (2), .111 (1), .112 (2). Seven rows, three JPEG XL entries |
| `grep jpegxl corpus/manifest.tsv` | HEAD | now | exactly five `syntax/jpegxl_*.dcm` rows, under .110 x2, .111 x1, .112 x2 |
| `grep` for counts in `oracle.md` | HEAD | now | no count left in lines 563-631. The other counts (49, 281, 385, 761, 1105, 1143) are the ones pass 10 already listed under N1 |
| `python3 scripts/prose_check.py` | worktree | now | exit 0, 864 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | exit 0, 1710 files |
| `bin/ocelli.sh gate oracle` | a087f61 | prior receipt (pass 10) | `applicable 100, read back 93, unsupported 7`. Not re-run, as the brief required. The diff touches only prose |

## Defects

None. **Pass 10 D1 is closed.** Lines 576-577 now give no count, so nothing in them can disagree with the seven-row list.

## Smells

None.

## Nitpicks

None new. Pass 10's N1 (older stale counts: lines 49, 281, 385, 761, 1105, 1143, 1152, plus "both JPEG-LS rows" / "those five rows" at 611-616 and in `unsupported.json` `$notes`, where the manifest has three JPEG-LS rows), N2 and N3 are still open for the lead. I re-read the JPEG-LS sentence at 611-616 because it is the paragraph directly after the edited section. It is about the HTJ2K and JPEG-LS rows the reference does render. It does not contradict the seven unsupported rows or the new sentence, so it stays under N1 and is not a finding in this pass. `unsupported.mjs:5-6` still quotes the plan's "covers 30 rows ... reports 91". That is a quotation, attributed to the plan, in a file this diff did not touch.

## Verified clean

- The new sentence is true against `run.mjs`. A row that fails at any boundary is never counted as read back. It either matches a named entry, which counts it as unsupported, or it fails the run. So the oracle cannot report a row it never rendered as covered. The stale-claim loop means a row that renders cannot hide behind an entry either, so "knows which rows it covers" holds in both directions. This is what the two bullets just above the sentence describe.
- The section intro "The rows below" and its three bullets list 1 + 1 + 5 = 7 rows. That matches `unsupported.json` and the gate's `unsupported 7`.
- "The three entries, one per syntax" is true: .110, .111 and .112 each have one entry.
- The YBR bullet ("fails at read back", 480 against 720 bytes) and the deflated bullet (default path, boundary at decode) agree with their entries' `boundary` and `why`.
- The paragraph after the section ("four `@cornerstonejs/codec-*` binaries listed above") still matches line 134's list of four packages.
- The edited lines contain no em-dash and no prose semicolon. The prose check is clean.

## Limitations and handoff

- No oracle, GPU or browser job was run, as the brief required. The measured seven comes from pass 10's gate receipt on `a087f61`. The only change since then is prose.
- The review covered AC24's documentation in this section and the paragraphs around it. It is not a whole-file audit of `oracle.md`.
- Tree unchanged: HEAD `bb7b7c6`, no tracked modification. The only new file is this report.
- Next: the lead decides on N1 to N3 from pass 10. This pass is a clean bounded pass for AC24's documentation. It is not acceptance of F-229 as a whole.
