# F-232 review, pass 9

**Reviewed**: `git diff 93d2acdf cac5cf47` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, HEAD
`cac5cf47`, one commit editing `.claude/plans/F-232-design.md` (5 lines
changed, 1 added) and adding the pass 8 report. Clean at start. This report
is the only new path.
**Author / reviewer**: the sprint lead wrote `cac5cf47`. Claude Opus 5.5
reviewed it as an independent subagent and wrote none of the reviewed
material. The same model family wrote passes 1 to 8, so independence is per
changed material, not per model.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- Pass 8 D1, N1 and N2, each against the edited sentence and the tree.
- Every edited sentence executed: the in-scope list at line 78, the AC15
  row at line 120, the first rejected alternative at line 135, the evidence
  invalidation sentence at line 164 and the new unit row at line 181.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff 93d2acdf cac5cf47 -- .claude/plans/F-232-design.md` | range | now | edits at lines 78, 120, 135, 164 and an added line 181 |
| `python3 -m unittest -v scripts.tests.test_guard_readers.EveryLibraryRootForbidsUnsafeDoctests` | worktree `cac5cf47` | now | 11 tests, all ok |
| `python3 scripts/lint_policy_check.py` | worktree | now | exit 0, "144 cargo target root(s) seeded", "16 library root(s) forbid it in their doctests" |
| `cargo metadata --no-deps --offline`, targets of workspace members | worktree | now | 144 targets: 15 `lib`, 1 `cdylib rlib`, 5 `bin`, 114 `test`, 9 `example`. 16 library roots |
| `grep -cF '#![doc(test(attr(forbid(unsafe_code))))]'` per library root | worktree | now | 1 in each of the 16 roots |
| `git diff --name-status --diff-filter=AD 870b90cc HEAD -- '*.rs'` | merge base to HEAD | now | `crates/ocelli-codec/tests/jpegxl.rs` added, F-229's, which is the 114th test target |
| `grep -n` "no unit test" and "its probes are its tests" over the plan, LLD, `lint_policy_check.py`, catalogue | worktree | now | no hit |
| plan lines 230 to 252, the pass 3, 4 and 5 rulings | worktree | now | presence, position, LF-only read and printable header, matching the test names |
| `bin/ocelli.sh gate prose` | worktree | now | ALL GREEN |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1.** Line 135's added sentence says "the 21-of-143 reason against this
  alternative stands". The tree now has 144 member targets, because F-229's
  `crates/ocelli-codec/tests/jpegxl.rs` merged in as the 114th test target,
  and `lint_policy_check.py` prints "144 cargo target root(s) seeded". The
  21 lib and bin roots are still 21 (15 `lib`, 1 `cdylib rlib`, 5 `bin`),
  so the reason stands as a proportion and the label names the plan-time
  count of fact 2 at line 49. The sentence is a pointer to an earlier
  argument, not a fresh measurement. **Does not block.**
- **N2.** The AC15 test cell at line 120 now ends "updated." with a full
  stop, left over from the deleted clause. No other cell in that column ends
  with one. **Does not block.**

## Verified clean

- **Pass 8 D1 is closed.** The clause "`lint_policy_check.py` has no unit
  test file, so its probes are its tests" is gone, and no other tracked
  document repeats it. Line 181 adds a unit row naming
  `scripts/tests/test_guard_readers.py`, class
  `EveryLibraryRootForbidsUnsafeDoctests`, which exists and runs 11 ok
  tests. Each part of "presence, position, the root read as rustc reads it
  and the printable header" has a test by name:
  `test_a_root_without_it_is_refused`,
  `test_the_attribute_is_read_only_in_the_crate_header` and
  `test_the_attribute_after_the_crate_docs_is_found`,
  `test_a_separator_rustc_does_not_end_a_comment_at_is_refused` and
  `test_crlf_line_endings_are_accepted`,
  `test_any_unprintable_character_above_it_is_refused` and
  `test_only_the_lines_down_to_the_attribute_must_be_ascii`. "after the
  pass 3 to pass 5 rulings" matches the three rulings at lines 230 to 252.
- **Pass 8 N1 is closed.** Line 78 names `scripts/tests/test_guard_readers.py`
  and "one attribute line in each of the 16 member library roots", and
  `cargo metadata` lists 16 library roots, each holding the attribute once.
  Line 164 names the doctest branch of `lint_policy_check.py` and a member
  library root's header, which is what `library_root_problems` and
  `unprintable_head_line` read.
- **Pass 8 N2 is closed.** Line 135 now says F-232 writes the doctest
  attribute into every library root and that the line does what no root
  lint can. `carries_doctest_forbid`'s docstring records that the workspace
  table does not reach a doctest and that the injected attribute does, and
  a `#![forbid]` in a crate root is a lint of that crate, which a doctest,
  compiled as its own crate, does not inherit.

## Limitations and handoff

- No cargo build, doctest plant or probe profile was run, because the range
  changes prose only and no code changed since `2158d01d`.
- Next owner: the lead. This pass blocks nothing.
