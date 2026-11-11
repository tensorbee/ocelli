# F-232 review, pass 7

**Reviewed**: `git diff 2158d01d 5582c0bc` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, HEAD
`5582c0bc`, one commit adding the "Lead ruling after review pass 5" section
to `.claude/plans/F-232-design.md` and the pass 6 report. Clean at start and
end apart from this report.
**Author / reviewer**: the sprint lead wrote `5582c0bc`. Claude Opus 5.5
reviewed it as an independent subagent and wrote none of the reviewed
material. The same model family wrote passes 1 to 6, so independence is per
changed material, not per model.
**Result**: 0 defects, 1 smell, 2 nitpicks

## Acceptance challenged

- Every sentence of the new section against `scripts/lint_policy_check.py`
  as it stands, `library_root_problems`, `unprintable_head_line`,
  `_rust_lines`, `_printable` and `carries_doctest_forbid`.
- Whether the docstring at `scripts/tests/test_guard_readers.py:1523` now
  resolves to a recorded ruling.
- Whether any earlier section of the plan states, as current, a design the
  final tree no longer has.
- Whether the plan, `docs/lld/guards.md`, the module docstrings of both
  scripts, the catalogue entries and the refusal messages agree on the final
  design: the workspace table at forbid on every target, doctests forbidden
  by the crate-root attribute checked at its fixed position, and the scanner
  not reading doc text.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| read `library_root_problems` | worktree `5582c0bc` | now | `library.read_bytes().decode("utf-8")`, matching "decoded from its bytes as UTF-8" |
| read `_rust_lines` | worktree | now | `source.split("\n")` then `removesuffix("\r")`, matching "split on LF only, with one trailing CR removed" |
| read `_printable`, `unprintable_head_line` | worktree | now | tab or `" " <= c <= "~"`, checked on every line up to and including the first line that is neither blank nor `//`, which is where the attribute must be. Matches "down to and including the attribute line" |
| refusal message in `library_root_problems` | worktree | now | "line {line}, at or above the place ... holds a character other than printable ASCII and tab", so the refusal names the line as the section says |
| `grep -n "ruling after pass 5"` against the plan headings | worktree | now | test docstring cites "the ruling after pass 5", plan heading at line 245 is "Lead ruling after review pass 5". Resolves |
| `python3 scripts/lint_policy_check.py` | worktree | now | exit 0, "16 library root(s) forbid it in their doctests" |
| `python3 -m unittest scripts.tests.test_guard_readers` | worktree | now | 87 tests OK |
| `bin/ocelli.sh gate prose` | worktree | now | ALL GREEN, 896 files |
| fence info strings in member sources, `git grep '```'` | worktree | now | 24 `///` and 3 `//!` `text` openers, 1 `wgsl`, closers balance. No Rust fence, so "the workspace has no doctests" still holds |
| plan, every ruling section read whole | worktree | now | see S1 |
| LLD lines 1109 to 1161, scanner docstring lines 40 to 49 and 317 to 319, catalogue `unsafe.third-file` note, lint-policy `refuses` text and both doctest probe notes | worktree | now | all agree with the final design |

## Defects

None.

## Smells

### S1, two superseded rulings in the plan still read as current

**Where**: `.claude/plans/F-232-design.md:215` and `:233`.
**What**: line 215, under "Lead rulings after review pass 1", says
"**Doctests are code.** The scanner reads the Rust code fences of doc
comments as code and reports `unsafe` there ... The documents say exactly
which targets the lint reaches and that the scanner covers doctests." The
pass 3 ruling at line 230 withdraws only "The pass 2 rulings on doc text",
so by its own words the pass 1 bullet still stands, and it contradicts line
234, "The scanner stops reading doc text", the scanner docstring ("Doc text
is comment text here") and the LLD ("reads doc comments and doc strings as
comment and string text and leaves doctests to the attribute"). Line 233,
under pass 3, says `cargo test --workspace` "compiles the doctests, so the
attribute is exercised on every floor run". Pass 4's third bullet corrected
that claim in the prose because the workspace has no doctests, measured
again in this pass, but the pass 3 sentence was left standing beside it.
**Why it matters**: this is the same failure pass 6 S1 named, read from the
other side. The ruling ledger is where a later author looks to learn which
design is current, and at the moment it holds a live ruling that the scanner
covers doctests, which is the design pass 3 abandoned as "a chase with no
end". A later author reading the ledger top to bottom can take line 215 as
licence to put fence reading back into the scanner.
**Evidence**: `grep -n "withdrawn" .claude/plans/F-232-design.md` prints only
line 230, which names pass 2. `grep -n "Doctests are code"` prints line 215
with no superseding marker.
**Scope and regression**: prose only, no behaviour. The smallest repair is
for the lead to widen line 230 to "The pass 1 and pass 2 rulings on doc text
are withdrawn", or to mark line 215 as withdrawn by pass 3 and line 233's
last sentence as corrected by pass 4. The new pass 5 section itself
supersedes nothing and contradicts nothing above it.

## Nitpicks

- **N1.** The consumer and provider map at line 88 to 92, the write set
  block at lines 145 to 151 and the LLD impact at line 196 predate the pass
  3 ruling and do not name the crate-root attribute or the member library
  roots. Line 153 does state that F-232 writes one attribute line into every
  member library root, so the scope is recorded, only not in the table and
  block a reader scans first. **Does not block.**
- **N2.** The pass 5 section says the rule covers "lines, down to and
  including the attribute line". The code stops at the first line that is
  neither blank nor `//`, which is the attribute line only on a root that
  carries it. On a root without it the rule covers the line where it should
  have been. The refusal message words this correctly, "at or above the
  place ... must be". **Does not block.**

## Verified clean

- **The new section is true against the code.** UTF-8 decode of the bytes,
  LF split with one trailing CR removed, printable ASCII and tab down to and
  including the attribute position, a refusal naming the line. The reason
  given, nine separators on which Python and rustc disagree, is the set pass
  6 measured.
- **Pass 6 S1 is closed.** The test docstring's "the ruling after pass 5"
  now resolves to the plan section at line 245, and that section records
  the fail closed choice and forbids narrowing it to a list of separators,
  which is what S1 asked to be written down.
- **Plan, LLD, docstrings, catalogue and messages agree on the final design**
  apart from S1. The table at forbid reaches every lib, bin, test, bench,
  example and build script target. Doctests are forbidden by
  `#![doc(test(attr(forbid(unsafe_code))))]` accepted only as the first line
  after the leading `//!`, `//` and blank lines. The scanner reads doc text
  as comment text. The `lint-policy` entry's `refuses` text, the two
  doctest probe notes and the `unsafe.third-file` note say exactly this.
- **Pass 6 report.** Read whole. It is a review record and makes no claim
  about the tree beyond its own evidence table, which this pass did not
  re-execute except where listed above.

## Limitations and handoff

- No cargo build, doctest plant or probe profile was run, because the range
  changes prose only and pass 6 measured the code at `2158d01d`, which this
  range does not touch.
- Next owner: the lead for S1, then a fresh pass over that change.
