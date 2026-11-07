# F-232 review, pass 4

**Reviewed**: `git diff 0b6d3fde 151d961a` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, HEAD
`151d961a`, one commit, clean at start and end apart from this report. No
unstaged paths reviewed.
**Author / reviewer**: a Claude implementer wrote `151d961a` after "Lead
ruling after review pass 3" in `.claude/plans/F-232-design.md`. Claude Opus
5.5 reviewed it as an independent subagent and wrote none of the reviewed
material. The same model family wrote passes 1 to 3 and the implementer
round, so independence is per changed material, not per model.
**Result**: 1 defect, 0 smells, 4 nitpicks

## Acceptance challenged

- **The ruling's central claim**, that `#![doc(test(attr(forbid(unsafe_code))))]`
  at a crate root makes every compiled doctest holding `unsafe` fail. Derived
  independently in a scratch workspace on 1.97.1, edition 2024 (so merged
  doctests are in play), each crate carrying the package table
  `unsafe_code = "forbid"` as the Ocelli workspace does, each shape built twice,
  with and without the attribute. A wrong mechanism would show as a "with" run
  that passes. None did for any compiled shape, and every "without" run passed,
  which also re-proves that the lint table does not reach a doctest.
- **Breaking attempts the brief named**: a doctest that overrides it (inner,
  hidden `# #![allow]`, `cfg_attr` allow, outer allow on `fn main`, item and
  module `doc(test(attr(allow)))`, a second root `doc(test(attr(allow)))`
  after the forbid and before it), a submodule file's doctests, `cfg_attr`
  forms of the root attribute, a proc-macro crate, a bin-only package, and a
  library whose root is `src/root.rs`.
- **The presence check** against cargo metadata of that scratch workspace and
  of the real tree, against six mutations of its reader, and against spellings
  that compile with the same effect and spellings with none.
- **The scanner removal**, by running the `0b6d3fde` scanner and the
  `151d961a` scanner over every tracked `.rs` file and comparing per file.
- **Composition with the sprint branch**: a read-only `git merge-tree` of
  `151d961a` and `sprint/s46`, which has moved to `636fdcc1`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test --doc -p <shape>` over 50 shapes x 2, scratch workspace `ws` | scratch crates, `rust-toolchain.toml` copied from the tree, `CARGO_TARGET_DIR=target/f232-pass4-scratch` | now | with the attribute: every compiled shape exit 101 "usage of an `unsafe` block", E0453 added for every override. Without: exit 0 "1 passed". Exceptions below |
| shapes red with, green without | plain, `rust`, `rust2015` `rust2018` `rust2021` `rust2024`, `edition2015` to `edition2024`, `edition2021,no_run`, `no_run`, `rust,no_run`, `standalone_crate`, `should_panic`, `test_harness` with a `#[test]`, tilde, indented, block quote, list marker, `/**` and `/*!` opener, `unsafe` before a newline and before a comment, `unsafe fn`, `unsafe impl`, `unsafe trait`, `unsafe extern`, `#[unsafe(export_name)]`, `#[unsafe(no_mangle)]`, explicit `fn main`, `cfg_attr` doc, `\x75nsafe` escape, `concat!`, `include_str!`, `macro_rules!` doc, a local `macro_rules!` holding `unsafe`, submodule file with `//!` and nested `mod` doctests (2 of 2 failed), `no_crate_inject` beside it | now | as above |
| overrides | inner `allow`, `warn`, `expect`, hidden `# #![allow]`, `#![cfg_attr(all(), allow)]`, `#[allow]` on `fn main`, `#[doc(test(attr(allow)))]` on an item, on an outer `mod`, `#![doc(test(attr(allow)))]` in a module file, root allow after the forbid | now | exit 101, E0453 each. Root allow BEFORE the forbid: exit 101, unsafe error |
| `ignore` and `compile_fail` | scratch | now | `ignore`: 0 passed 1 ignored both ways. `compile_fail`: passes with the attribute, fails without |
| crate-exported `macro_rules!` expanding to `unsafe`, called from a doctest | scratch | now | passes WITH the attribute, the external macro lint exemption. The macro body is a keyword token in library source, which the scanner refuses, see Verified clean |
| root attribute spellings | scratch `ws2` | now | effective: `cfg_attr(all(), ...)`, `cfg_attr(doc, ...)`, `cfg_attr(doctest, ...)`, `doc(test(no_crate_inject, attr(forbid(...))))`. No effect: `cfg_attr(any(), ...)`, `cfg_attr(not(doc), ...)`, `deny` level with an inner allow |
| proc-macro, bin-only, `[lib] path = "src/root.rs"` | scratch `ws2` | now | proc-macro and `src/root.rs`: red with, green without. bin-only: `cargo test --doc` "no library targets", `cargo test` exit 0, doc fences in `main.rs` never compiled |
| `library_roots` on `cargo metadata` of `ws2` | scratch | now | every lib, proc-macro and `src/root.rs` root found, bin-only absent |
| `carries_doctest_forbid` on each `ws2` root | scratch | now | accepts the plain line and refuses every `cfg_attr`, combined and `deny` form. Also ACCEPTS the two no-effect shapes of D1 |
| `python3 scripts/lint_policy_check.py` | tree at `151d961a` | now | exit 0, "16 library root(s) forbid it in their doctests" |
| attribute removed from `tools/oracle`, `examples/folder-viewer`, `ocelli-wasm` roots | `git archive 151d961a` scratch copy | now | exit 1 each, naming that root |
| D1 reproduction on a real member | scratch copy, `ocelli-cache` root | now | lint check exit 0, scanner exit 0, `cargo test -p ocelli-cache --doc` "1 passed" with `unsafe { core::ptr::read(&0u8) }` in the doctest. Control with the plain line: "usage of an `unsafe` block", FAILED |
| six reader mutations in the scratch copy | `lint_policy_check.py` | now | `in` for `==`, `strip()` before `==`, whole-file read, `proc-macro` dropped from kinds, `lib` only, header attribute skip removed: `EveryLibraryRootForbidsUnsafeDoctests` RED for all six |
| old and new scanner `scan()` per file | 382 tracked `.rs` files of `151d961a`, vendored included | now | 0 files differ |
| five scanner mutations of retained behaviour | scratch copy | now | `unsafe` in attributes, weakening append, trivia after `#`, raw string with hashes, line comment with a quote: suite RED for all five |
| `cargo test --workspace --doc` | worktree, default target dir | now | 16 Doc-tests sections, each "running 0 tests" |
| `guard_probe.py --only` the four lint-policy probes the round added or touched | worktree | now | doctest-forbid-removed red, unreadable-library-root red, unreadable-crate-root red, clean-crate-root-outside-the-member green |
| `guard_probe.py --profile floor`, `--profile deep`, `--self-test` | worktree | now | 437 refusal red, 44 accept green, 101 controls. 494 refusal red, 59 accept green, 106 controls. Self-test OK |
| `test_guard_readers.py` 81, `test_unsafe_allowlist_check.py` 20, `test_guard_catalogue.py` | worktree | now | OK |
| `guard_census.py`, `--check-runbook`, scanner worktree and `--staged`, `cargo fmt --all --check`, `gate prose` | worktree | now | all exit 0. Scanner "270 files checked, 2 permitted", 104 and 0 |
| crate root diff shape | `git diff -U0 0b6d3fde 151d961a -- crates examples tools` | now | 16 added attribute lines, 9 added blank lines, nothing removed |
| `git merge-tree --write-tree 151d961a sprint/s46` | read-only | now | conflicts in `examples/folder-viewer/src/lib.rs` and the runbook. Every `src/lib.rs` in the result carries the line |

## Defects

### D1, the presence check accepts the attribute inside a string or block comment opened on a header attribute line

**Where**: `scripts/lint_policy_check.py`, `carries_doctest_forbid`, the skip
`if line.startswith("#![") and line.rstrip().endswith("]")`, and its
docstring "so a weaker level, another spelling, a block comment or a string
holding the text are all refused".
**What**: a header line that starts with `#![` and ends with `]` is skipped as
a one-line attribute without checking that it closes. If it opens a string or
a block comment, the next line can be the exact attribute text inside that
literal, and the reader accepts it. Two shapes, both compile:

```
#![doc = "]
#![doc(test(attr(forbid(unsafe_code))))]
"]
```

```
#![allow(dead_code)] /* ]
#![doc(test(attr(forbid(unsafe_code))))]
*/
```

**Why it is wrong**: HLD 27.2 R5 and the ruling's second bullet, "the lint
policy check refuses a member library root without that exact attribute".
The attribute in either shape is text, not an attribute, so doctests holding
`unsafe` compile and run while the check says the root forbids them. The
docstring's "a block comment or a string holding the text are all refused" is
false for both.
**Evidence**: both shapes in scratch crates, `cargo test` "1 passed" with an
`unsafe` doctest, `carries_doctest_forbid` True. On a `git archive 151d961a`
copy, the string shape in `crates/ocelli-cache/src/lib.rs` plus one `unsafe`
doctest gave lint check exit 0 "16 library root(s) forbid it", scanner exit 0,
and `cargo test -p ocelli-cache --doc` "1 passed". With the plain line the
same doctest FAILED with "usage of an `unsafe` block".
**Scope and regression**: present on `151d961a`. Every member library root is
exposed, and the tree is clean today. It needs a deliberate shape, so it is a
fail-open route rather than a likely accident, but it is exactly the class the
docstring claims to refuse. Smallest repair: stop skipping inner attributes
above the line and require the attribute as the first line after the run of
blank and `//` lines, which is where all 16 roots already carry it, or skip a
header attribute line only when it holds no `"` and no `/*`. Regression: the
two shapes above as cases in `test_the_attribute_is_read_only_in_the_crate_header`,
red on the current reader.

## Smells

None.

## Nitpicks

- **N1, carried from pass 2 N1 and pass 3 N1.** `line = probe_line` replaced
  by a no-op in the scanner's `#` branch still leaves all 20 tests green,
  re-run now. Wrong line numbers only, never a miss. **Does not block.** Third
  pass carrying it, worth the operator's ruling.
- **N2.** The workspace has zero doctests, measured now, 16 sections each
  "running 0 tests". "`cargo test --workspace`, which the floor runs, compiles
  the doctests" in the LLD and in the `lint-policy.doctest-forbid-removed`
  note is true but compiles nothing today. What the floor actually exercises
  is the presence check, and the rustc refusal is evidenced by scratch
  measurement. A short "none exist today" would keep the sentence from being
  read as coverage. **Does not block.**
- **N3.** Spellings with the same effect are refused: `cfg_attr(all(), ...)`,
  `cfg_attr(doc, ...)`, `cfg_attr(doctest, ...)` and
  `doc(test(no_crate_inject, attr(forbid(unsafe_code))))` all made the
  doctest fail in scratch and all fail the reader. Fail closed and named by
  the docstring's "another spelling". **Does not block.**
- **N4, the plan, outside the range.** `.claude/plans/F-232-design.md` still
  says "No crate source file is written" and its write set lists none, while
  the ruling at `0b6d3fde` makes the story write 16 crate roots. The trial
  merge with `sprint/s46` at `636fdcc1` conflicts in
  `examples/folder-viewer/src/lib.rs`, so whoever integrates resolves a root.
  Losing the line there is refused by the check (measured by removing it from
  that root). For the lead, not the implementer. **Does not block.**

## Verified clean

- **Mechanism.** Every compiled doctest shape pass 3 listed, and the extra
  ones in the Evidence table, fails with the attribute and passes without it,
  on 1.97.1, edition 2024 merged doctests included. Every override route is
  E0453, a module or item `doc(test(attr(allow)))` included, so a submodule
  cannot escape. A root `allow` placed before the forbid does not win. A
  proc-macro library and a library rooted at `src/root.rs` behave like any
  other library. A bin-only target has no doctests at all.
- **What compiles but does not trip it.** A doctest calling a macro the crate
  exports, whose body holds `unsafe`, passes with the attribute, because
  unsafe_code is not reported inside an external macro. Every such body is in
  member library source and is a keyword token the scanner refuses, so it is
  not a gap in this tree. The same exemption applies to the workspace table
  for ordinary code. `ignore` is not compiled, `compile_fail` passes because
  the forbid makes it fail, as the LLD says.
- **`library_roots`.** Finds all 16 roots from `cargo metadata`, keys match
  `cargo_metadata_members` (the real tree prints 16), proc-macro and
  non-default path found, bin-only excluded. Every lib kind on the tree is
  `lib`, or `cdylib` and `rlib` for `ocelli-wasm`, as its docstring says. All
  tracked `Cargo.toml` outside `vendor/` are members, so no tracked library
  escapes the member walk.
- **`carries_doctest_forbid`.** Exact line only, header only, every reader
  mutation goes red in the unit tests. Apart from D1, no no-effect spelling
  was accepted: `deny`, `warn`, spaced, trailing comment, indented, below an
  item, in a block comment starting its own line and in a raw string after
  an item are all refused.
- **Probes.** `lint-policy.doctest-forbid-removed` and
  `lint-policy.unreadable-library-root` red for their fragments,
  `clean-crate-root-outside-the-member` still green with the attribute in its
  moved root. Budget `lint-policy` 29 to 31 matches the two new refusal sites
  and the census passes. The runbook is current.
- **Scanner removal.** Per-file `scan()` output is identical between the old
  and new scanner over all 382 tracked `.rs` files, so the tracked-tree result
  is unchanged. Attributes, raw strings, weakening, trivia after `#` and
  line-comment quotes are each still pinned by a red-on-mutation test. Only
  doc-text tests were removed, and two were added.
- **Prose executed.** The scanner module and `scan` docstrings, the lint
  policy docstring, main comment, refusal message and `library_roots`
  docstring, the two catalogue docstrings and three notes, the guard `refuses`
  text and the LLD paragraph, against the measurements above. Only the
  docstring sentence in D1 is false. The `unsafe.third-file` note no longer
  names a regex, which closes pass 3 D9. README, `use.html` and `more.html`
  mention neither `unsafe` nor doctests.
- **Crate roots.** 16 files, each only the attribute line plus a blank line
  where rustfmt layout needs it, `cargo fmt --all --check` exit 0. Clippy with
  `-D warnings` on a scratch root carrying the attribute is clean.
- **Pass 1 to 3 replay.** Pass 3 D1 to D6 are closed by the mechanism, each
  shape re-measured above. D7 is closed because the tests it named are
  removed with the code. D8 closed except D1's sentence. D9 closed. Pass 2
  N2 to N5 and pass 3 N2 and N4 to N6 are moot with the fence code gone.
  Pass 1 D1 to D3 and S1 and pass 2 D2 are still pinned (mutations above,
  deep probes green).

## Limitations and handoff

- No GPU, oracle or browser job. `gate --floor` and `gate --sprint` were not
  run as wholes. The floor and deep probe profiles, the guards gate's unit
  suites, census, fmt and prose were.
- wasm32 doctests are not run by cargo, so the attribute is measured on host
  only. That matches what `cargo test --workspace` runs.
- `cargo test --workspace --doc` was run in the worktree's default target
  directory, build artefacts only. Scratch crates and the scratch tree copy
  lived in the session scratchpad with `target/f232-pass4-scratch`, removed
  with Python `shutil` at the end.
- Next owner: the implementer for D1, then a fresh pass over that change. The
  lead for N1, carried three passes, and N4.
