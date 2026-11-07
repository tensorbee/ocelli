# F-232 review, pass 8

**Reviewed**: `git diff 5582c0bc 93d2acdf` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, HEAD
`93d2acdf`, one commit editing `.claude/plans/F-232-design.md` (7 lines) and
adding the pass 7 report. Clean at start. This report is the only new path.
**Author / reviewer**: the sprint lead wrote `93d2acdf`. Claude Opus 5.5
reviewed it as an independent subagent and wrote none of the reviewed
material. The same model family wrote passes 1 to 7, so independence is per
changed material, not per model.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- Pass 7 S1, N1 and N2, each against the edited sentence and the code.
- Every edited sentence executed against `scripts/lint_policy_check.py`,
  `scripts/unsafe_allowlist_check.py`, the 16 member library roots and
  `docs/lld/guards.md`.
- The whole plan read once more for a sentence that is superseded but reads
  as current, or that contradicts the final design: the workspace table at
  forbid on every target, doctests forbidden by the crate-root attribute
  checked at its fixed position with the root read as rustc reads it and the
  header limited to printable ASCII and tab, and the scanner reading doc text
  as comment text.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff 5582c0bc 93d2acdf -- .claude/plans/F-232-design.md` | range | now | edits at lines 91, 148, 149, 197, 216, 231, 234, 251 |
| `python3 scripts/lint_policy_check.py` | worktree `93d2acdf` | now | exit 0, "16 library root(s) forbid it in their doctests" |
| `cargo metadata` lib targets, then `git diff 870b90cc HEAD` per root | merge base with `sprint/s46` to HEAD | now | 16 roots, each gains exactly `#![doc(test(attr(forbid(unsafe_code))))]` in `151d961a`. The other `+` lines in `ocelli-codec` are F-229's |
| read `carries_doctest_forbid`, `unprintable_head_line`, `_rust_lines`, `library_root_problems` | worktree | now | the printable scan stops at the first line that is neither blank nor `//`, which is the attribute line when present and the place it belongs when absent |
| `git log 870b90cc..HEAD -- scripts/tests/test_guard_readers.py` | worktree | now | three F-232 commits, `151d961a`, `b5dcee19`, `2158d01d` |
| `python3 -m unittest -v ...EveryLibraryRootForbidsUnsafeDoctests` | worktree | now | 11 tests, all ok, each calling `lint_policy_check` |
| `git show 870b90cc:scripts/tests/test_guard_readers.py \| grep -c lint_policy` | merge base | now | 22, so the file tested `lint_policy_check` before F-232 too |
| `python3 -m unittest scripts.tests.test_guard_readers scripts.tests.test_unsafe_allowlist_check` | worktree | now | 108 tests OK |
| `python3 scripts/unsafe_allowlist_check.py` | worktree | now | exit 0, 270 files, vendored 104 and 0 |
| `grep -n` the three AC15 probe names in `scripts/guards/catalogue.py` | worktree | now | 3 hits |
| catalogue `unsafe.third-file` note, `lint-policy.doctest-forbid-removed` and `-behind-a-separator` probes | worktree | now | present and saying what the plan says |
| scanner docstring lines 40 to 49, LLD lines 1109 to 1161 | worktree | now | doc text is comment text, doctests left to the attribute |
| `bin/ocelli.sh gate prose` | worktree | now | ALL GREEN |

## Defects

### D1, AC15 says `lint_policy_check.py` has no unit tests

**Where**: `.claude/plans/F-232-design.md:120`, and the unit row of the
Tests table at line 180.
**What**: AC15's test column ends "`lint_policy_check.py` has no unit test
file, so its probes are its tests". The final tree has unit tests for it
that F-232 wrote: `EveryLibraryRootForbidsUnsafeDoctests` in
`scripts/tests/test_guard_readers.py`, 11 tests calling
`carries_doctest_forbid`, `unprintable_head_line`, `library_root_problems`
and `library_roots`. The plan's own write set at line 148, edited in this
range, names that file. The Tests table's unit row names only
`scripts/tests/test_unsafe_allowlist_check.py`, so the doctest attribute
check, the position rule and the printable header rule have no row in the
inventory or the Tests table, and the one sentence about where
`lint_policy_check.py` is tested says the wrong thing.
**Why it is wrong**: the microscope severity table counts a false claim in
prose as a defect. It also contradicts line 148 in the same document. The
sentence was not fully true at plan time either: at the merge base
`test_guard_readers.py` already imported `lint_policy_check` and tested its
TOML reader (22 references).
**Evidence**: the `git log`, `unittest -v` and `git show` rows above.
**Scope and regression**: prose only, no behaviour. A later author who
trusts AC15 changes `lint_policy_check.py` and runs only the probes. The
smallest repair is to delete the clause "`lint_policy_check.py` has no unit
test file, so its probes are its tests" and add `test_guard_readers.py` to
the unit row at line 180 for the doctest attribute rules.

## Smells

None.

## Nitpicks

- **N1.** The in-scope list at line 78 and the evidence invalidation
  sentence at line 164 predate the pass 3 ruling. Line 78 names neither the
  16 member library roots nor `scripts/tests/test_guard_readers.py`, and
  line 164 does not name a change to a root's attribute line or to the
  doctest branch of `lint_policy_check.py`. The write set at lines 148 and
  149 now records both, so the scope is recorded once. **Does not block.**
- **N2.** The first rejected alternative at line 135 counts against crate
  root attributes that "it writes a line into crate roots that F-209, ...
  and F-229 also write". After the pass 3 ruling F-232 writes one line into
  every member library root anyway, as line 154 says. The rejection still
  stands on its first reason, 21 of 143 roots, so this is a stale argument
  and not a stale decision. **Does not block.**

## Verified clean

- **Pass 7 S1 is closed.** Line 216 now says "Withdrawn by the ruling after
  review pass 3", and line 231 names both "The pass 1 ruling that doctests
  are read as code, and the pass 2 rulings on doc text". Line 234 no longer
  claims the attribute is exercised on every floor run and states that the
  workspace has no doctests today. The scanner docstring and the LLD agree.
- **Pass 7 N1 is closed.** Line 91 names every member library root and the
  attribute position refusal, and `lint_policy_check.py` does refuse a root
  without the attribute at that position. Lines 148 and 149 name
  `test_guard_readers.py`, which F-232 changed in three commits, and the 16
  roots, each of which gained exactly one attribute line. Line 197's added
  clause matches LLD lines 1119 to 1125.
- **Pass 7 N2 is closed.** Line 251's added clause matches
  `unprintable_head_line`, which stops at the first line that is neither
  blank nor `//`, and the refusal message's "at or above the place".
- **No other ruling reads as current while superseded.** Pass 1's attribute,
  whitespace and `#[path]` bullets stand and the catalogue carries them. Pass
  2's `#[path]` probe note sentence is not about doc text and stands, and
  the probe note says one module landed and the planted module is the third.
  Pass 4 and pass 5 agree with the code. Decision 4 and AC7 already treat a
  doc comment as a comment.
- **Line 30**, `REQUIRED_RUST = {"unsafe_code": "deny"}` and the "one of
  the two" rule, sits under "Normative source, transcribed" as the state the
  story changes, and line 130 says the departure paragraph is rewritten. It
  is now `forbid` at line 315 and "requires all three". Read as a
  transcription, not as a current claim.

## Limitations and handoff

- No cargo build, doctest plant or probe profile was run, because the range
  changes prose only and no code changed since `2158d01d`.
- Next owner: the lead for D1, then a fresh pass over that change.
