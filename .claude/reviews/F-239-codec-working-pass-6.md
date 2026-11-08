# F-239 review, pass 6, codec part

**Reviewed**: `work/f-239-codec`, range `925b546f..f7b99b85` (one commit, 3 files: `.claude/plans/F-239-design.md`, `.claude/reviews/F-239-codec-working-pass-5.md`, `scripts/guards/catalogue.py`), in the worktree `ocelli-wt-f-239-codec`, tip `f7b99b8535a2509fb727318d8f0b4cbfc8156372`. No staged or unstaged tracked change at review time. Untracked: the `.venv` symlink and the `tools/oracle/out` symlink only. Compared against `sprint/s46` at `b61cfe282bd1dd8bcea71d02456c9d60bdd94741`.
**Author / reviewer**: the commit is by the sprint lead, a Claude session. Reviewed by Claude as an independent `/microscope --working` pass 6. The reviewer wrote no part of this range. No JPEG 2000 codec source outside the vendored crates was read.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 5 S1, the notes conflicting with `sprint/s46`.** Not accepted from the ruling's wording. Both `Probe(...)` blocks were cut by exact line match from the tip and from `git show sprint/s46:scripts/guards/catalogue.py`, from the `Probe("<name>",` line up to the next `Probe(` line, and compared as strings and by sha256. Then `git merge-tree --write-tree` was run for the tip, for `925b546f` and for `ea3e5f6b` against `sprint/s46`, and the merged `catalogue.py` blob was read for conflict markers and for a duplicated or dropped note.
- **Pass 5 N1, the owner of the 300 seconds.** Checked against `Sandbox.run`'s signature and against `script()`, and every other `timeout` in `scripts/guard_probe.py` and `scripts/guards/` was searched for a second bound.
- **No behaviour change.** The catalogue at `925b546f` and at the tip were parsed with `ast` and compared with every `Probe` `note=` value replaced by one constant. Equal. The same comparison without the replacement is unequal, so the check discriminates. Comments do not reach the AST.
- **The copied notes are true on this tree, not only on s46.** `#[path]` appears six times under `git grep`, four of them in `vendor/ritk-codecs-0.6.0`, so "The tree follows two" was challenged. `scripts/lint_policy_check.py` on the tip prints `2 \`#[path]\` module(s) followed`, which are `tools/oracle/src/produce.rs:40` and `examples/folder-viewer/src/held.rs:23`. The vendored crates are not among its 16 members, so the sentence is true as a statement about what the check follows.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| block cut and compare, `lint-policy.clean-module-outside-the-member` | tip lines 12036 to 12054, s46 lines 12495 to 12513 | now | EQUAL, 1280 bytes each, sha256 prefix `343bafdc0ba6d22e` both |
| same, `lint-policy.clean-cfg-attr-module-path` | tip lines 12084 to 12099, s46 lines 12543 to 12558 | now | EQUAL, 1062 bytes each, sha256 prefix `138b4c1a82373cef` both |
| `git merge-tree --write-tree --name-only HEAD sprint/s46` | refs | now | 8 conflicted files, `catalogue.py` not among them |
| same for `925b546f` and `ea3e5f6b` | refs | now | 9 (with `catalogue.py`) and 8, the same 8 as the tip |
| merged tree `aabcc4d4`, `scripts/guards/catalogue.py` | merge-tree output | now | 14795 lines, 0 conflict markers, the s46 note sentence twice, the old `held` wording 0 times, each probe name once, the `Sandbox.run` comment once |
| `sed -n 265,274p scripts/guards/sandbox.py`, `catalogue.py:70` to `:73` | tip | now | `run(..., timeout: int = 300)`, and `script()` calls `box.run(listed)` with no timeout. No other `timeout` in `guard_probe.py` or `scripts/guards/` |
| `guard_probe.py --profile deep --only lint-policy.clean-module-outside-the-member` | worktree | now | 0. 1 accept green, 1 control green, 2.6 s |
| `guard_probe.py --profile deep --only lint-policy.clean-cfg-attr-module-path` | worktree | now | 0. 1 accept green, 1 control green, 2.5 s |
| `python3 scripts/guard_census.py` | worktree | now | 0. 1131 refusals in 79 files all claimed, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | worktree | now | 0 |
| `python3 -m unittest scripts.tests.test_guard_catalogue` | worktree | now | 0. 57 tests OK |
| `python3 scripts/prose_check.py` | worktree | now | 0. voice rules clean over 913 files |
| `python3 scripts/lint_policy_check.py` | worktree | now | 0. `2 \`#[path]\` module(s) followed` |
| `ast` comparison of `catalogue.py`, notes normalised, and raw | `925b546f` against tip | now | normalised equal, raw unequal |
| `git diff --stat 925b546f f7b99b85 -- ':!.claude'` | refs | now | `scripts/guards/catalogue.py` only, 11 insertions, 8 deletions |
| added lines and commit message scanned for U+2014, U+2013 and `; ` | refs | now | no dash. One `; ` hit in an added line, see below |

The single `; ` hit in the added lines is pass 5's quoted prose check string, "scanned for em-dash, en-dash and `; `", a code span naming the pattern rather than a prose semicolon, and `prose_check.py` is green over it.

## Defects

None.

## Smells

None.

## Nitpicks

- N1. `.claude/plans/F-239-design.md:584`, "present on `sprint/s46` too since F-X021. It is recorded for the lead to fix before the sprint push". True when written at 02:09:35, and two minutes later `sprint/s46` gained `b61cfe28` (02:11:08), which makes `rendered_pair` return `Option` and both tests take the reference-run-absent branch. On this branch the two tests still panic in a clean clone, and `tools/oracle/src/produce.rs` is not among the files that conflict with `sprint/s46`, so integration brings the fix in. The ruling is a dated record and does not need rewriting. A later ruling or the integration note can name `b61cfe28` as the fix.

## Pass 5 findings replayed

- **S1, closed.** Both probe blocks are byte-identical to `sprint/s46`, and `catalogue.py` no longer conflicts. The conflict set is back to the 8 files `ea3e5f6b` already had, so this branch adds none.
- **N1, closed.** The comment at `scripts/guards/catalogue.py:631` to `:633` credits the 300 seconds to `Sandbox.run`'s default, which `scripts/guards/sandbox.py:266` declares and `script()` reaches by passing no timeout. No other bound exists in the harness, so "the only bound" holds. The invoke still passes `timeout=1200` at `:634`.

## Files that still conflict with `sprint/s46`

All eight were already conflicted at `ea3e5f6b` and none comes from this range:

- `corpus/decode-digests.tsv`
- `crates/ocelli-codec/tests/registry.rs`
- `docs/hld/DEVIATIONS.md`
- `docs/lld/codecs.md`
- `docs/lld/corpus.md`
- `docs/runbooks/guard-verification.md`
- `scripts/corpus_synth.py`
- `scripts/tests/test_corpus_synth.py`

## Verified clean

- **Byte identity of the two probe blocks**, by exact cut and sha256, including expect `3`, polarity, `needs` and `profile`.
- **Merge cleanliness of `catalogue.py`**, by merge-tree at three points and by reading the merged blob.
- **No behaviour change.** Outside `.claude/` only `catalogue.py` changed, and its AST is equal once the two notes are normalised. No `.rs`, `.wgsl`, `vendor/`, gate, workflow or runbook file changed.
- **Every factual sentence the range adds**: the timeout comment, both copied notes (the two-module count measured by `lint_policy_check.py` on the tip), and the ruling's S1 and N1 lines. The ruling's third line is N1 above.
- **README, use.html and more.html** are not touched and describe nothing this range changed.

## Limitations and handoff

- The floor and the full deep profile were not rerun. The range changes two probe notes and one comment and the AST check shows no executable change, so only the two affected probes, the census, the runbook check, the catalogue unit tests and the prose check were run.
- No oracle, GPU or browser job was run.
- The eight remaining conflicts are for `/integrate-feature` and were not resolved here.
- Next owner: the lead. This pass is clean for the codec part's changed material. It is not sprint readiness, which waits for `gate --sprint` and the whole-sprint review.
