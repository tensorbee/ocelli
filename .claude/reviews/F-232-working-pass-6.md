# F-232 review, pass 6

**Reviewed**: `git diff d8030f58 2158d01d` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, HEAD
`2158d01d`, one commit, clean at start and end apart from this report. No
unstaged paths reviewed.
**Author / reviewer**: a Claude implementer wrote `2158d01d` after
`.claude/reviews/F-232-working-pass-5.md`. Claude Opus 5.5 reviewed it as an
independent subagent and wrote none of the reviewed material. The same model
family wrote passes 1 to 5 and the implementer rounds, so independence is per
changed material, not per model.
**Result**: 0 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **Pass 5 D1 replayed**: all nine separators Python's `splitlines` ends a
  line at and rustc does not end a `//` comment at, each written as
  `// F-232.` + separator + the attribute in place of the attribute line of
  `crates/ocelli-cache/src/lib.rs`, with an `unsafe` doctest planted, through
  the real `scripts/lint_policy_check.py` and `cargo test -p ocelli-cache
  --doc`.
- **Every other way the check and rustc could disagree** on the line before
  or holding the attribute that the brief named, and the ones derived from
  rustc's lexer: an outer `///` doc comment above, `//!` and `//` lines holding
  `/*`, a quote or `r#"` text, a tab-only blank line, CRLF for the whole file
  and for the attribute line alone, CR CR LF, a trailing tab, a NUL, invalid
  UTF-8 above and below the attribute, a BOM, a later crate-level
  `doc(test(attr(allow)))` and `doc(test(attr(warn)))`, an item-level
  `doc(test(attr(allow)))` on the planted item, an inner `allow` in the
  doctest, and `--cap-lints allow` or `-A unsafe_code` reaching rustdoc
  through `[build] rustdocflags` and `RUSTDOCFLAGS`.
- **Common real layouts**: non-ASCII text below the attribute line, and the
  16 member roots on both this branch and `sprint/s46` at `5451cb7d` run
  through `unprintable_head_line`.
- **Would the new tests fail if the code were wrong**: seven mutations of the
  reader against `EveryLibraryRootForbidsUnsafeDoctests`, and the new probe
  run against the pass 5 reader.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| control, attribute as committed, planted doctest `let _x = unsafe { core::ptr::read(&0u8) };` | `git archive 2158d01d` scratch copy, `CARGO_TARGET_DIR` under its `target/` | now | lint exit 0 "16 library root(s) forbid it", doctest exit 101 "usage of an `unsafe` block" |
| pass 5 D1, separators CR, VT, FF, `\x1c`, `\x1d`, `\x1e`, NEL, U+2028, U+2029 | scratch copy | now | nine of nine: lint exit 1 "line 18, at or above the place ... holds a character other than printable ASCII and tab", doctest exit 0 "1 passed", so each refused root really forbids nothing |
| attribute removed | scratch copy | now | lint exit 1 "does not carry", doctest "1 passed" |
| `/// outer doc` above the attribute | scratch copy | now | lint exit 0, but rustc refuses the crate "an inner attribute is not permitted following an outer doc comment", so nothing builds. Fail closed by the compiler |
| `//! /* opened`, `// /* opened`, `// " r#" opened`, `//! r#" opened`, a tab-only line above | scratch copy | now | lint exit 0, doctest exit 101 with the unsafe error. Accepted and effective, so a `//` line opens nothing rustc honours |
| CRLF whole file, CRLF attribute line only | scratch copy | now | lint exit 0, doctest exit 101 unsafe error. Accepted and effective |
| CR CR LF attribute line, NUL in a `//` line above, BOM | scratch copy | now | lint exit 1 naming line 18, 18 and 1. Doctest still fails, so these are effective roots refused, fail closed |
| trailing tab on the attribute | scratch copy | now | lint exit 1 "does not carry", doctest fails. Fail closed |
| invalid UTF-8 in a `//` line above, and in a comment below | scratch copy | now | lint exit 1 "could not be read as UTF-8 text", rustc "stream did not contain valid UTF-8". Both refuse |
| later `#![doc(test(attr(allow(unsafe_code))))]` | scratch copy | now | lint exit 1 "re-allows", doctest fails |
| later `#![doc(test(attr(warn(unsafe_code))))]`, item `#[doc(test(attr(allow(unsafe_code))))]`, doctest `#![allow(unsafe_code)]` | scratch copy | now | doctest exit 101 in all three, the forbid holds. The inner doctest allow is also refused by lint |
| non-ASCII `//!` and `//` with U+2028 below the attribute | scratch copy | now | lint exit 0, doctest fails. The printable rule stops at the attribute line as claimed |
| `[build] rustdocflags = ["--cap-lints", "allow"]`, `["-A", "unsafe_code"]`, `RUSTDOCFLAGS="--cap-lints allow"` | scratch copy, `.cargo/config.toml` | now | doctest exit 101 with the unsafe error in all three. Neither flag reaches the injected forbid |
| seven reader mutations, `EveryLibraryRootForbidsUnsafeDoctests` | scratch copy | now | `read_text` for the byte read RED (bare CR file test), `splitlines` RED, printable check removed RED, printable widened to any char above space RED, CR not stripped RED, gate removed from `carries_doctest_forbid` RED, printable check moved below the first non-comment line GREEN (N1) |
| `guard_probe.py --only lint-policy.doctest-forbid-behind-a-separator` | worktree, and scratch copy with `b5dcee19`'s `lint_policy_check.py` | now | worktree red for its declared reason, exit 0. Pass 5 reader: HARNESS FAIL "exited 0. The guard it aims at did not fire" |
| `test_guard_readers` (87), `test_unsafe_allowlist_check`, `test_guard_catalogue` | worktree | now | OK, OK, OK |
| `lint_policy_check.py`, `guard_census.py`, `--check-runbook`, `bin/ocelli.sh gate prose` | worktree | now | exit 0, 0, 0, ALL GREEN |
| `guard_probe.py --profile floor` | worktree | now | 437 refusal red, 44 accept green, 101 controls, exit 0 |
| `unprintable_head_line` on all 16 `src/lib.rs` member roots at `sprint/s46` `5451cb7d` | read-only `git show` | now | None for all 16 |
| `git merge-tree --write-tree 2158d01d sprint/s46` | read-only | now | conflicts in `examples/folder-viewer/src/lib.rs` and the runbook only, as in pass 5 |
| Python separator set, every code point for which `("a"+c+"b").splitlines()` splits | local Python | now | LF, CR, VT, FF, `\x1c` to `\x1e`, NEL, U+2028, U+2029, matching the docstring's list exactly |

## Defects

None.

## Smells

### S1, the new test cites a lead ruling the tree does not record

**Where**: `scripts/tests/test_guard_readers.py:1523`,
`test_any_unprintable_character_above_it_is_refused`, docstring "Fail closed,
by the ruling after pass 5".
**What**: `.claude/plans/F-232-design.md` carries "Lead rulings after review
pass 1", "pass 2", "Lead ruling after review pass 3" and "pass 4", each landed
as its own commit before the implementer round that followed it. There is no
section and no commit for pass 5, in this worktree or in any other
`ocelli*` worktree's copy of the plan. Pass 5 itself recommended only the LF
split and the byte read, not the printable ASCII rule.
**Why it matters**: the printable rule is the part of this round that
refuses ordinary heads nothing exploits, a `//! café` or a
`// Copyright ©` line (measured as refused, N3). The next author who hits that
refusal will look up the ruling the test names and find none, so the fail
closed choice reads as an implementer's own and is the obvious thing to relax
to "refuse the nine named separators", which reopens pass 5 D1 for any
separator nobody named. The ledger of rulings exists to stop exactly that.
**Evidence**: `grep -n -i "pass 5\|printable" .claude/plans/F-232-design.md`
prints nothing, `git log -- .claude/plans/F-232-design.md` ends at
`4959de30 F-232, lead ruling after review pass 4`.
**Scope and regression**: prose only, behaviour is correct. Smallest repair
is for the lead to record the ruling after pass 5 in the plan, which is the
established place, or for the docstring to drop the citation. No test change.

## Nitpicks

- **N1.** `unprintable_head_line` checks the attribute line itself, and the
  LLD says a character there is "refused naming the line", but no test holds
  a non-printable character on the attribute line. Moving the printable check
  below the non-comment return stays green. Behaviour does not change,
  because such a line can never equal the ASCII `DOCTEST_FORBID` and is
  refused as "does not carry", so only the message is unpinned. **Does not
  block.**
- **N2.** The `carries_doctest_forbid` docstring is not a raw string, so
  `` `\x1c` to `\x1e` `` is stored as the two control characters themselves
  and `help()` prints them. The source reads correctly. **Does not block.**
- **N3.** A root whose header holds any non-ASCII character, a `©` in a
  licence line or an accented word in `//!` text, is now refused, measured by
  the unit test and through the real check for a BOM. Fail closed, no member
  root on this branch or on `sprint/s46` is affected, and the refusal names the
  line. **Does not block**, recorded so the cost is visible beside S1.

## Verified clean

- **Pass 5 D1 closed.** All nine separators are refused through the real check
  on a member root, naming line 18, and the doctest passing under each confirms
  those roots forbid nothing, so each refusal is correct and not merely
  conservative.
- **The narrowing class looks exhausted for the shapes tried.** With the head
  limited to printable ASCII and tab and split at LF with one CR removed, the
  reader's lines are rustc's lines, a `//` line is a line comment to rustc
  whatever it holds, and a `//` or blank line opens no string or block
  comment, so by induction from line 1 the first other line is Rust source.
  Every shape tried either agrees with rustc on an effective root (accepted,
  doctest refused), refuses a root that forbids nothing, refuses an effective
  root (fail closed), or is a root rustc itself will not compile (`///`
  above, invalid UTF-8). I found no shape where the check accepts and the
  planted doctest runs. rustdoc flags and later or item-level
  `doc(test(attr(...)))` do not lower the injected forbid.
- **Roots.** `LIBRARY_KINDS` covers every kind `cargo metadata` reports here,
  16 doctest-carrying lib targets, all accepted. `library_roots` and the
  `[lib] path` route are unchanged by this round and were measured in pass 5.
- **Tests discriminate.** Six of seven reader mutations go red, the seventh is
  N1. The new probe goes red for its declared reason and the harness fails it
  against the pass 5 reader.
- **Prose executed.** The module docstring, `carries_doctest_forbid` and
  `library_root_problems` docstrings, refusal message, catalogue probe note,
  LLD paragraph and the test comments were run against the measurements above.
  "A text-mode read turns a bare CR into LF before any split sees it" is the
  M1 mutation's red. "The unit tests carry one case per separator" holds, nine
  subtests. The splitlines list matches Python exactly. The only unbacked
  sentence is S1's citation. The LLD rewrap of the `unsafe_allowlist_check.py`
  sentence closes pass 5 N1 for that line. The budget bump 31 to 32 matches
  the one new lint-policy probe and the census and runbook checks pass. README,
  `use.html` and `more.html` mention neither `unsafe` nor doctests.
- **Composition.** The sprint branch has moved to `5451cb7d`. The merge still
  conflicts only in the folder-viewer root and the runbook, as pass 5
  described, and every sprint-side root header is printable ASCII.

## Limitations and handoff

- No GPU, oracle or browser job. `gate --floor` and `gate --sprint` were not
  run as wholes. The floor probe profile, the guards unit suites, census,
  runbook and prose were.
- The doctest effect is measured on host only, against `ocelli-cache`. Other
  roots share the code path, not the measurement.
- The scratch copy lived in the session scratchpad outside the worktree. Its
  `target/` was removed once with `rm -rf` and the rest with Python `shutil`,
  not `shutil` throughout as the brief asked. Nothing in the worktree was
  removed.
- Next owner: the lead for S1, recording the ruling after pass 5 or asking
  for the citation to go, then a fresh pass over that change. The
  implementation itself needs no further remediation from this pass.
