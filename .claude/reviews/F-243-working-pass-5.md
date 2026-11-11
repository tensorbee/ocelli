# F-243 review, pass 5

**Reviewed**: `work/f-243-xai`, range `ca8956ab..c8f4c2ab`, tip `c8f4c2ab`, worktree `ocelli-wt-f-243`, clean against the tip when the pass began. Untracked `.venv` and `tools/oracle/out` only. The range is `84347bf0` (pass 4's review file, the plan's use.html bullet at `:352` and its ruling line `:387`) and `c8f4c2ab` (a rewrap of the RESET paragraph in `docs/lld/codecs.md`).
**Author / reviewer**: written by the sprint lead, a Claude session. Reviewed by an independent Claude reviewer that wrote none of the range, the plan or the rulings. The rulings were taken as given.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **No code change.** `git diff --name-only ca8956ab..c8f4c2ab` lists three Markdown files only, and the same diff restricted to `*.rs`, `*.py`, `*.toml`, `*.ts`, `*.js`, `*.wgsl`, `*.sh`, `*.json` and to `vendor`, `crates`, `packages`, `scripts`, `tools` and `bin` is empty.
- **The plan's new use.html bullet, executed against use.html.** `use.html:631` says JPEG-LS frames decode to CharLS 2.4.4's samples "on every undamaged stream of a generated corpus that CharLS opens, 149 of 150", refuses a scan that runs out of data or leaves non-zero or more than 7 bits after its last sample, and refuses a non-default LSE preset as `CodecError::UnsupportedPreset` naming the parameter. The bullet's three clauses, the corpus-scoped agreement, "a truncated or over-long scan is refused" and the preset refusal, each correspond to a clause of that paragraph. The bullet no longer carries a line number, so it cannot go stale on one.
- **The corpus, measured now.** I decoded all 150 streams of `jpegls_charls_streams.tsv` with CharLS 2.4.4 through imagecodecs 2026.10.10 from `.venv`. CharLS opens 149 and refuses one, `jpegls_final_ff_one_padded_u8`, which `generate_jpegls.py:169` lists in `RUN_HAND_SCANS`, the hand-built scans. So "149 of 150, the other a hand-built stream both refuse" holds on CharLS's side. Ocelli's side is the `tests/jpegls.rs` corpus test, green now, which asserts each undamaged stream CharLS opens decodes to CharLS's digest. The range changes no decoder, so pass 4's independent Ocelli decode of the 149 stands as the prior receipt.
- **Plan `:387`, executed.** "The documents say what is measured: every undamaged stream of the generated corpus that CharLS opens decodes as CharLS does." `use.html:631`, `docs/lld/codecs.md:139` to `:140` and `PATCH-PROVENANCE.md:26` to `:28` each say exactly that, scoped to the generated corpus. `docs/lld/codecs.md:114` states it without the corpus scope, which is N1 below.
- **The codecs.md rewrap changed no words.** `git diff --word-diff=porcelain ca8956ab..c8f4c2ab -- docs/lld/codecs.md` shows no added or removed word, and the file's whitespace-split tokens at `ca8956ab` and `c8f4c2ab` are byte-identical under `cmp`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff --name-only`, then restricted to code suffixes and code trees | `ca8956ab..c8f4c2ab` | now | three `.md` files, code restriction empty |
| `git diff --word-diff=porcelain` and a token `cmp` of `codecs.md` | both commits | now | no word added or removed, tokens identical |
| added lines scanned for U+2014, U+2013 and `;` | range diff | now | none |
| `imagecodecs.jpegls_decode` over the 150 streams | `jpegls_charls_streams.tsv` at tip, CharLS 2.4.4 | now | 149 open, 1 refuse, `jpegls_final_ff_one_padded_u8` |
| `cargo test -p ocelli-codec` | worktree, `CARGO_TARGET_DIR=$PWD/target` | now | 0, `tests/jpegls.rs` 33 passed, every binary green |
| `bin/ocelli.sh gate pins` | worktree | now | 0, ALL GREEN |
| `python3 scripts/prose_check.py` | worktree | now | 0, 968 files clean |
| `python3 scripts/deviation_check.py` | worktree | now | 0, 32 deviations |
| whole-tree `git grep` for CharLS agreement phrasings | tip, `.claude/reviews` excluded | now | see Verified clean, and N1 |
| Ocelli over the 149 and the 136 `ins:00` cases | pass 4 scratch test | prior receipt, decoder unchanged | 149 byte-identical, 136 refused |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1.** `docs/lld/codecs.md:113` to `:115`, "as CharLS was measured to, since every undamaged stream that CharLS opens decodes to its samples", carries the "that CharLS opens" qualifier the lead's ruling after pass 3 prescribed for this file, but not the generated-corpus scope that `:139` in the same section, use.html, PATCH-PROVENANCE and plan `:387` give it. It is not shown false, the section's own "What is and is not claimed" bullet scopes it, and the wording is the ruling's, so it does not block. If the lead wants plan `:387` to describe every document exactly, "since every undamaged stream of the generated corpus that CharLS opens decodes to its samples" closes it. Pre-existing, words unchanged in this range.
- **N2.** `docs/lld/codecs.md:117` is now "On the original", 17 columns, where the rewrap stopped one line early. The one-word "The" line of pass 4's N2 is gone, so this is the same shape moved down three lines. Rendered Markdown reflows it.

## Verified clean

- **Pass 4 D1, closed.** `plan:352` no longer says JPEG-LS frames decode as CharLS decodes them. It now scopes the claim to undamaged streams of the generated corpus that CharLS opens, which is what use.html says and what CharLS measured now. The stale `:630` line number is gone.
- **Pass 4 N1, closed.** `plan:387` now says "of the generated corpus" in place of "in both corpora". Pass 2's corpus is cited by no document, confirmed by the grep, so the sentence matches the documents apart from N1 above.
- **Pass 4 N2, closed as reported.** The one-word "The" line is gone and the paragraph's words are unchanged. A short line remains at `:117`, N2 above.
- **Whole-tree grep.** Patterns for "as CharLS does/decodes/gives", "same samples as CharLS", "CharLS's samples", "every undamaged stream", "decodes like/identically to CharLS", "bit-exact" near CharLS and "JPEG-LS frames/streams decode". Outside `.claude/reviews`, every hit is corpus-scoped (`use.html:631`, `codecs.md:139`, `PATCH-PROVENANCE.md:26`, `plan:247` and `:327` inside rows naming the generated corpus, `plan:352`, `plan:387`), the RESET floor stated as CharLS's behaviour (`PATCH-PROVENANCE.md:71`, the D-20 row), the reported frame's expected result (`plan:147`), a test assertion message looping over the corpus only (`tests/jpegls.rs:1281`), the dated rulings (`plan:377`, `:383`, `:385`, `:392`), N1, or unrelated HTJ2K and F-028 history. No unqualified claim that JPEG-LS frames, or every undamaged stream, decode as CharLS does.
- **README, more.html, D-20.** `README.md:186` to `:191` and the D-20 row make no unqualified agreement claim. more.html has no CharLS sentence. None changed in this range.
- **Policy.** No dwv, Horos, Grok or CharLS source was opened. CharLS ran only as a tool through imagecodecs. Nothing tracked was edited. This file is the only write and is uncommitted.

## Limitations and handoff

- Ocelli's decode of the 149 and its refusal of the 136 were not re-run outside the green corpus test. The range changes no code, so pass 4's independent decode stands as a prior receipt.
- No damaged-stream sweep and no mutation of a test were repeated, for the same reason. AC14, AC18 and AC13's comparison stay owed as in pass 3. No oracle, GPU or browser job ran.
- This is a clean bounded review of the range, not completed acceptance or sprint readiness. N1 and N2 are the lead's to take or leave.
