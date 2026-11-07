# F-232 review, pass 5

**Reviewed**: `git diff 4959de30 b5dcee19` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, HEAD
`b5dcee19`, one commit, clean at start and end apart from this report. No
unstaged paths reviewed.
**Author / reviewer**: a Claude implementer wrote `b5dcee19` after "Lead
ruling after review pass 4" in `.claude/plans/F-232-design.md`. Claude Opus
5.5 reviewed it as an independent subagent and wrote none of the reviewed
material. The same model family wrote passes 1 to 4 and the implementer
rounds, so independence is per changed material, not per model.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- **The ruling's first bullet**, that a root is accepted only when the
  attribute is the first line after its leading `//!`, `//` and blank lines,
  and that those three line kinds open nothing, so the accepted line is Rust
  source. Challenged with an independently derived case: Python's
  `str.splitlines()` and `Path.read_text()` do not agree with rustc on what a
  line is. rustc ends a line comment at LF only. `splitlines()` also splits at
  CR, VT, FF, the three information separators `\x1c` to `\x1e`, NEL, U+2028
  and U+2029, and `read_text()` turns a bare CR into LF before that. A `//`
  comment holding one of those characters and then the attribute text is one
  comment to rustc and two lines to the reader.
- **Attempts the brief named**, each measured on a real member root with a
  planted `unsafe` doctest: BOM, CRLF line endings, a shebang line, trailing
  whitespace, removal, a `[lib] path` pointing at a second root, and the two
  pass 4 D1 shapes replayed through the real check.
- **The 16 roots against `cargo metadata`**, every lib-kind target compared
  with `library_roots`.
- **N1's new test**, by removing the line commit it names.
- **Composition with the sprint branch**: read-only `git merge-tree` of
  `b5dcee19` and `sprint/s46` at `636fdcc1`, and a simulated resolution that
  loses the line in `examples/folder-viewer/src/lib.rs`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `python3 scripts/lint_policy_check.py` | worktree at `b5dcee19` | now | exit 0, "16 library root(s) forbid it in their doctests" |
| `library_roots` against every lib-kind target of `cargo metadata --no-deps --locked --offline` | worktree | now | 16 and 16, sets equal, all 16 accepted by `carries_doctest_forbid`, none holds CR, VT, FF, `\x1c` to `\x1e`, NEL, U+2028, U+2029 or a BOM |
| planted doctest `let _x = unsafe { core::ptr::read(&0u8) };` on `pub fn planted_f232`, root `crates/ocelli-cache/src/lib.rs`, then `lint_policy_check.py` and `cargo test -p ocelli-cache --doc --offline` | `git archive b5dcee19` scratch copy, `CARGO_TARGET_DIR=target/f232-pass5-scratch` | now | control (attribute as committed): lint exit 0, doctest FAILED "usage of an `unsafe` block" |
| same, attribute line replaced by `// F-232.` + separator + attribute, for U+2028, U+2029, bare CR, FF, VT, NEL, `\x1c`, `\x1d`, `\x1e` | scratch copy | now | all nine: reader True, lint exit 0, `cargo test --doc` exit 0, "1 passed". D1 |
| U+2028 shape with the plant above `mod tests`, then `cargo fmt --check`, `cargo clippy -p ocelli-cache --all-targets -- -D warnings`, `unsafe_allowlist_check.py` | scratch copy | now | fmt exit 0 after one `cargo fmt` that left the U+2028 in place, clippy clean, scanner exit 0 "270 files checked", lint exit 0, doctest "1 passed" |
| BOM, trailing space, shebang line before the docs | scratch copy | now | reader False, lint exit 1, doctest FAILED with the unsafe error. Effective roots refused, fail closed |
| CRLF whole file | scratch copy | now | reader True, lint exit 0, doctest FAILED. Accepted and effective |
| attribute removed | scratch copy | now | lint exit 1, doctest "1 passed" |
| pass 4 D1 string shape and block comment shape, planted doctest | scratch copy, `ocelli-cache` root | now | lint exit 1 naming the root, doctest "1 passed" for both. Refused, and the refused roots really forbid nothing |
| `[lib] path = "src/other.rs"` in `ocelli-seg`, new root without the line | scratch copy | now | lint names `crates/ocelli-seg/src/other.rs` |
| seven reader mutations against `EveryLibraryRootForbidsUnsafeDoctests` | scratch copy | now | the pass 4 reader RED (3 failures, the three new shapes), `strip()` before `==` RED, `in` for `==` RED, keep scanning past the first line RED, skip `#![` lines RED, skip `/*` lines RED. The candidate repair below GREEN |
| candidate repair, `source.split("\n")` with `removesuffix("\r")` | scratch, called directly and on a file | now | refuses all nine separators, accepts CRLF. Through `read_text()` the bare CR shape is still accepted, through `read_bytes().decode("utf-8")` refused |
| `line = probe_line` replaced by `pass` | scratch copy of `scripts/unsafe_allowlist_check.py` | now | `test_line_count_survives_trivia_inside_an_attribute_opener` RED, finding reported at `:2:`. Restored, 21 OK |
| `test_guard_readers.py`, `test_unsafe_allowlist_check.py`, `test_guard_catalogue.py` | worktree | now | 81 OK, 21 OK, OK |
| `guard_probe.py --only lint-policy.doctest-forbid-removed`, `--profile floor` | worktree | now | red for its reason. Floor 437 refusal red, 44 accept green, 101 controls, exit 0 |
| `guard_census.py`, `--check-runbook`, `bin/ocelli.sh gate prose`, `cargo fmt --all --check` | worktree | now | all exit 0 |
| `cargo test --workspace --doc --offline` | worktree, default target dir | now | 16 Doc-tests sections, each "running 0 tests" |
| `git merge-tree --write-tree b5dcee19 sprint/s46` | read-only | now | conflicts in `examples/folder-viewer/src/lib.rs` (attribute against `mod link; mod measure;`) and the runbook |
| folder-viewer root with the line dropped, as taking the sprint side would | scratch copy | now | lint exit 1, message names the file, the exact line and where it must sit |

## Defects

### D1, the reader's line is not rustc's line, so a `//` comment can carry the accepted text

**Where**: `scripts/lint_policy_check.py`, `carries_doctest_forbid`,
`for line in source.splitlines():`, and its caller in `main`,
`root_source = library.read_text(encoding="utf-8")`. The docstring sentence
"Those three line kinds open no string and no block comment, so the line that
follows them is Rust source and not literal text", the module docstring, the
LLD sentence "refuses a member library root ... that does not carry that exact
line as the first line after its leading `//!`, `//` and blank lines" and the
refusal message rest on the same reading.
**What**: rustc ends a `//` comment at LF only. `splitlines()` also ends a
line at CR, VT, FF, `\x1c`, `\x1d`, `\x1e`, NEL, U+2028 and U+2029, and
`read_text()` has already turned any bare CR into LF. So

```
// F-232.<U+2028>#![doc(test(attr(forbid(unsafe_code))))]
```

is one comment to rustc and, to the reader, a `//` line followed by the exact
attribute. The root forbids nothing and is accepted.
**Why it is wrong**: HLD 27.2 R5 and the ruling after pass 3, "the lint policy
check refuses a member library root without that exact attribute", restated
by the ruling after pass 4 as "accepted only as the first line after the
crate's leading `//!`, `//` and blank lines". The attribute here is not on any
line of its own in the source rustc reads.
**Evidence**: on a `git archive b5dcee19` copy, the attribute line of
`crates/ocelli-cache/src/lib.rs` rewritten with each of the nine separators
and an `unsafe` doctest planted: reader True, `lint_policy_check.py` exit 0
"16 library root(s) forbid it", `cargo test -p ocelli-cache --doc` exit 0
"1 passed", nine of nine. The U+2028 shape also passes `cargo fmt --check`,
`cargo clippy -D warnings` and `unsafe_allowlist_check.py`. With the committed
line the same doctest FAILED "usage of an `unsafe` block".
**Scope and regression**: present on `b5dcee19`, every member library root
exposed, all 16 clean today. It needs a deliberate invisible character, so it
is a fail-open route rather than an accident, and it is the same class as
pass 4 D1, text the reader takes for the attribute. Smallest repair: read the
root as `library.read_bytes().decode("utf-8")` (or `open(..., newline="")`),
so a bare CR survives, and iterate `source.split("\n")` with
`line.removesuffix("\r")` so CRLF stays accepted. Measured in scratch: that
refuses all nine, accepts CRLF, and leaves the current unit suite green. The
`read_text()` half matters, because with `split("\n")` alone the bare CR shape
is still accepted through `main`. Regression: one case per separator in
`test_the_attribute_is_read_only_in_the_crate_header`, `// x` + separator +
the attribute, red on the current reader, plus a bare CR case through the real
check on a planted root file so the read is covered as well as the split.

## Smells

None.

## Nitpicks

- **N1.** Two added lines run past the wrap of their neighbours,
  `docs/lld/guards.md` "`cargo test --workspace` in the floor then compiles
  under it. `scripts/unsafe_allowlist_check.py` stays in the `unsafe`" and the
  refusal f-string "leading `//!`, `//` and blank lines. The
  [workspace.lints.rust] table ". The file has older long lines too. **Does
  not block.**
- **N2.** Effective roots are refused when they start with a BOM, carry
  trailing whitespace on the line or open with a shebang, each measured. Fail
  closed and within the ruling's "exactly as written". **Does not block.**

## Verified clean

- **Pass 4 D1 closed.** Both shapes, and the `#![cfg_attr(...)]` line above
  the attribute, are refused by the real check on a member root, and the
  doctest passing under them confirms those roots forbid nothing. The pass 4
  reader is RED on the three new test cases.
- **Pass 4 N1 closed.** The new test goes red when the line commit is
  removed, reporting `:2:` instead of `:3:`, as its comment says.
- **Pass 4 N2 closed.** "The workspace has no doctests today" is true, 16
  sections of 0. The floor's `test` gate is `cargo test --workspace`
  (`bin/ocelli.sh` line 107), so "compiles under it" holds for the first
  doctest written.
- **Reader mutations.** Six of six behavioural mutations go red. Weaker level,
  indented, trailing comment, trailing space and spaced `#![ doc` are refused.
- **Roots.** All 16 match the lib-kind targets of `cargo metadata`, a
  `[lib] path` redirect is followed to the new root and refused, and a `#[path]`
  module is a submodule that inherits the root attribute (pass 4).
- **Prose executed.** The module docstring, `carries_doctest_forbid`
  docstring, refusal message, catalogue note, LLD paragraph and both test
  docstrings and comments were executed against the measurements above. All
  hold except the sentences D1 names. "Both compile and neither forbids
  anything" for the pass 4 shapes is measured. README, `use.html` and
  `more.html` mention neither `unsafe` nor doctests.
- **Merge with the sprint branch.** The folder-viewer conflict is the
  attribute against two new `mod` lines. Keeping both is the resolution.
  Taking the sprint side alone is refused with a message that names the file,
  the exact line and the position it must take, which tells a later author
  what to write and where. Placing the `mod` lines above it makes it an inner
  attribute after an item, which rustc rejects, so neither wrong resolution is
  silent.
- **Gates run.** Unit suites, floor probe profile, census, runbook, prose and
  fmt all exit 0.

## Limitations and handoff

- No GPU, oracle or browser job. `gate --floor` and `gate --sprint` were not
  run as wholes. The floor probe profile, the guards unit suites, census,
  fmt and prose were.
- The doctest effect is measured on host only, as `cargo test --workspace`
  runs it.
- Scratch tree copy and its `target/f232-pass5-scratch` lived in the session
  scratchpad and were removed with Python `shutil`. `cargo test --workspace
  --doc` used the worktree's default target dir, build artefacts only.
- Next owner: the implementer for D1, then a fresh pass over that change.
