# F-243 review, pass 4

**Reviewed**: `work/f-243-xai`, range `d8d659f7..ca8956ab`, tip `ca8956ab`, worktree `ocelli-wt-f-243`, clean against the tip when the pass began. Untracked `.venv` and `tools/oracle/out` only. The range is the implementer's remediation `0e36a1a9` and the lead's plan wording edit `ca8956ab`. Scratch work ran in a `git archive` extraction of `ca8956ab`.
**Author / reviewer**: written by a Claude implementer (`0e36a1a9`) and the sprint lead (`ca8956ab`). Reviewed by an independent Claude reviewer that wrote none of the diff, the plan or the rulings. The rulings were taken as given.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- **No decoding change.** `git diff d8d659f7..ca8956ab` over `*.rs` and `*.py`, comment lines removed, leaves only the two hash lines in `scripts/pin_and_size_check.py`. The one vendored source file in the range, `bitstream.rs`, changes two comment lines. No arithmetic, no test assertion and no decoder branch moved.
- **The 149 of 150, independently.** I decoded all 150 streams of `jpegls_charls_streams.tsv` with CharLS 2.4.4 through imagecodecs 2026.10.10 myself, reading P, rows, columns and NEAR from each stream's own SOF55 and SOS rather than from the verdicts TSV. CharLS opens 149 and refuses one, `jpegls_final_ff_one_padded_u8`, which `generate_jpegls.py:169` builds by hand. Ocelli, driven from a scratch test with my own frame descriptions, gives byte-identical samples on all 149 and refuses the one. So use.html's "149 of 150, the other a hand-built stream both refuse" is true, and so are the new comments at `tests/jpegls.rs:1234` and `:1293`.
- **The 136 are one byte each (N2).** CharLS on every `ins:00` and `ins:0000` case, decoded now: 136 `ins:00` open to the stream's own samples, 14 refuse, and all 150 `ins:0000` refuse, each verdict matching the TSV digest. Ocelli refuses all 136, the `0xa5` buffer untouched. A naive count from the TSV gives 137 and 1, which is the hand-built stream whose own digest is `refuse`, not a two-byte case.
- **Fill bytes (N3).** CharLS opens `jpegls_two_band_u16_32x32.jls` with 1, 2, 3 and 10 `0xff` bytes before EOI, each to its undamaged decode, which equals the fixture's raw. The comment's "open both to the frame's own samples" holds.
- **Over-long prefix codes (N4).** Hand-built P 8 1 by 1 streams, a run interruption whose escape code carries 22 zeros plus 0 to 30 extra, remainders 0, 100, 200 and 254, byte stuffing applied. CharLS opens every extended code to the unextended code's sample, 1, 51, 101 and 128, all 120 extended cases. Ocelli opens the unextended codes at remainders 0, 100 and 200 and refuses every extended one with the buffer untouched, as the comment says.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff d8d659f7..ca8956ab` filtered to non-comment code lines | the range | now | only `RITK_PROVENANCE_SHA256` and the `bitstream.rs` entry of `RITK_PATCHED_HASHES` |
| SHA-256 of the six patched files and `PATCH-PROVENANCE.md` | worktree at `ca8956ab` | now | all six match `RITK_PATCHED_HASHES` and appear in PATCH-PROVENANCE, `bitstream.rs` `77ca61a7...`, PATCH-PROVENANCE `d9b57fdd...` |
| `bin/ocelli.sh gate pins` | worktree | now | 0, ALL GREEN |
| `gate pins`, one byte of `bitstream.rs` changed | scratch extraction | now | 1, "src/jpeg_ls/bitstream.rs has undeclared changes beyond its recorded patch". Restored from `git show`, 0 |
| `gate pins`, one byte of `PATCH-PROVENANCE.md` changed | scratch extraction | now | 1, "PATCH-PROVENANCE.md has undeclared changes from the exact reviewed patch record" |
| `cargo test -p ocelli-codec` | worktree, `CARGO_TARGET_DIR=$PWD/target` | now | 0, `tests/jpegls.rs` 33 passed, every other binary green |
| `prose_check.py`, `deviation_check.py`, `source_provenance_check.py`, `unsafe_allowlist_check.py` | worktree | now | all 0 |
| CharLS over the 150 corpus streams, Ocelli over the same | my decode, scratch test | now | CharLS 149 open, 1 refuse. Ocelli 149 byte-identical, 1 refuse |
| CharLS and Ocelli over `ins:00` and `ins:0000` | 300 cases from the TSV's streams | now | 136 one-byte cases CharLS opens to own samples, Ocelli refuses all 136 |
| CharLS over the fill test's frame | 1, 2, 3, 10 fill bytes | now | all open, equal to the raw |
| CharLS and Ocelli over 124 hand-built limited-length codes | P 8 1 by 1 | now | as above under N4 |
| imagecodecs `jpegls_encode(bitspersample=12)` | 8 by 8 ramp | now | SOF55 P 12, lossless round trip exact |
| whole-tree `git grep` for unqualified CharLS agreement | tip, `.claude/reviews` excluded | now | D1 below, everything else qualified or historical |

## Defects

### D1, the plan's documentation item still says JPEG-LS frames decode as CharLS decodes them

**Where**: `.claude/plans/F-243-design.md:352`, "`examples/folder-viewer/src/use.html:630`, after the codec table: JPEG-LS frames decode as CharLS decodes them, a truncated or over-long scan is refused, ...".
**What**: an unqualified claim, wider than pass 3's D1 class, that every JPEG-LS frame decodes as CharLS decodes it. It is false on the tree: the 136 one-byte `ins:00` cases CharLS opens to their own samples are refused by Ocelli, measured now. It also no longer describes use.html, which is at line 631 and says the qualified "every undamaged stream of a generated corpus that CharLS opens, 149 of 150".
**Why it is wrong**: a claim in prose that is false, `.claude/commands/microscope.md` section 3. The lead's ruling after pass 3, D1, requires that a grep for the old claim across the tree comes back empty, and pass 3's D1 named the plan's AC21 and parity row but not this bullet.
**Evidence**: `git grep -n -i -E "as CharLS (does|decodes)"` outside `.claude/reviews` returns this line. The Ocelli refusal of the 136 is under Acceptance challenged.
**Scope and regression**: prose only, no pixel, present on `ca8956ab`, pre-dating this range and missed by pass 3. The plan is not hash-pinned. Smallest fix: "after the codec table: JPEG-LS frames decode to CharLS 2.4.4's samples on every undamaged stream of the generated corpus that CharLS opens, a truncated or over-long scan is refused, ...", or point at use.html's actual sentence.

## Smells

None.

## Nitpicks

- **N1.** `.claude/plans/F-243-design.md:387`, edited in `ca8956ab`, still says "The documents say what is measured: every undamaged stream in both corpora that CharLS opens". After N5 the documents name only the generated corpus, and pass 2's corpus is no longer cited by any of them. It is a dated ruling record and the lead's to word, so it does not block, but "both corpora" no longer matches what the documents say.
- **N2.** `docs/lld/codecs.md:114` to `:115` now wraps as "decodes to its\n  samples. The\n  published", a one-word line left by the edit. Rendered Markdown reflows it.

## Verified clean

- **Pass 3 D1, closed apart from D1 above.** `PATCH-PROVENANCE.md:27` to `:29`, the plan's AC21 at `:247` and parity row at `:327`, `docs/lld/codecs.md:114` and `:139` to `:141`, and `tests/jpegls.rs:1234` and `:1293` all carry "that CharLS opens" or say the one refusal by name, and each was executed against my own decode. `RITK_PROVENANCE_SHA256` is re-pinned and the gate is green. The remaining grep hits are qualified, or are `plan:147` (the reported frame's expected result, which holds), `plan:377` (the pass 1 ruling, superseded) and `plan:383` (the pass 2 ruling quoting pass 2's review, which does say 6,724 of 6,724).
- **N2 closed.** `codecs.md:152` says "one 0x00 byte", and all 136 are `ins:00`, measured. The plan's parity row still says "0x00 bytes" across 136 cases, which is not a count of bytes per case and was outside N2.
- **N3 closed.** The comment now states the measurement, re-measured.
- **N4 closed.** The comment now says CharLS opens such a scan, re-measured. Ocelli's refusal re-measured on the same streams.
- **N5 closed.** 6,724 is gone from PATCH-PROVENANCE, the LLD and use.html and survives only in the dated rulings and the review records.
- **N8 closed.** Plan item 8 no longer says imagecodecs cannot write P 12. The remaining "with one exception" still holds, as the 12-bit fixture is pyjpegls's.
- **Pins.** Every recorded digest matches its file, and the gate goes red on a one-byte edit to the patched `bitstream.rs` and to PATCH-PROVENANCE.
- **use.html, README, more.html, D-20.** use.html:631 is true as executed above. README:187 to :191, more.html:394 and the D-20 row make no unqualified agreement claim and are unchanged in this range.
- **Policy.** No dwv, Horos, Grok or CharLS source was opened. CharLS ran only through imagecodecs. Nothing tracked was edited except this file. No DICOM was written outside scratch.

## Limitations and handoff

- The scratch tests first shared `CARGO_TARGET_DIR` with the worktree and cargo reused the scratch test binary for one worktree run, which listed 36 tests. I ran `cargo clean -p ocelli-codec -p ritk-codecs` and re-ran from the worktree source, 33 tests, and only that run is counted above.
- No damaged-stream sweep was repeated. The range changes no decoding code, so pass 3's 481,440 cases stand as a prior receipt for the unchanged decoder.
- AC14, AC18 and AC13's comparison stay owed as in pass 3. No oracle, GPU or browser job ran.
- D1 is for the implementer or the lead. A fresh independent pass reviews the remediation. The scratch trees and streams of this pass were deleted with Python `shutil.rmtree`.
