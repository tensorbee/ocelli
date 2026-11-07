# F-232 review, pass 2

**Reviewed**: `git diff 155b30ea d514da30` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, clean at
start and end apart from this report. No unstaged paths reviewed.
**Author / reviewer**: GLM in opencode wrote `d514da30` after the lead rulings
in `.claude/plans/F-232-design.md`. Claude Opus 5.5 reviewed it as an
independent subagent and wrote none of the reviewed material. The same model
family wrote pass 1, which this pass replays rather than trusts.
**Result**: 2 defects, 1 smell, 5 nitpicks

## Acceptance challenged

- **Lead ruling 1, `unsafe` inside an attribute.** `#[unsafe(no_mangle)]`,
  `#[unsafe(link_section = ".x")]` and `#[cfg_attr(all(), unsafe(export_name =
  "g"))]` are reported on their own lines. An attribute still open at end of
  file raises "unterminated attribute" naming the line of the `#`. Pass 1's
  trybuild and vendored plants are both refused now, the vendored one by the
  count moving from 0 to 1.
- **Lead ruling 2, doctests are code.** Expectations were taken from rustdoc's
  own behaviour, measured on a scratch crate carrying `unsafe_code = "forbid"`
  with one item per fence shape and `cargo test --doc`, not from the scanner's
  docstring. **Rejected**: the scanner's rule is narrower than rustdoc's (D1).
- **Lead ruling 3, weakening in every spelling.** `# [`, `#! [`, `#/**/[`,
  `#` then a line comment then `[` on the next line, spaces inside the
  brackets, a comment before the lint name, a `reason`, `cfg_attr` and nested
  `cfg_attr` with `expect`, and an attribute after a non-attribute `[` on the
  same line are all refused. `#[allow(UNSAFE_CODE)]` and `#[allow(warnings)]`
  were measured under rustc 1.97.1 with `#![deny(unsafe_code)]`: the first is
  an unknown lint and the second does not lower a deny, so neither is a
  missing spelling.
- **Lead ruling 4, the two `guards-deep` probes.** Justified. The tree
  follows two `#[path]` modules, `tools/oracle/src/produce.rs:40` and
  `examples/folder-viewer/src/held.rs:23` (F-210), so a planted module is the
  third. The expect is right and the note beside it is not (D2).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `python3 -m unittest discover -s scripts/tests -p 'test_unsafe*'` | worktree at `d514da30` | now | 0, 20 tests |
| 20 scanner mutations, each in a fresh `shutil` copy of `scripts/` | scratchpad, deleted after | now | 15 red on the intended test, 5 survive (S1, N1 to N3, one equivalent) |
| `scan` per file, `155b30ea` scanner against `d514da30` scanner | all 382 tracked `.rs` files | now | 0 files differ, so no new false positive and no new hit |
| `python3 scripts/unsafe_allowlist_check.py` | worktree | now | 0, 270 files, 104 and 0 |
| `python3 scripts/lint_policy_check.py` | worktree | now | 0, "2 `#[path]` module(s) followed" |
| `bin/ocelli.sh gate unsafe` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `guard_probe.py --profile deep --only` each of the 8 `unsafe.*` probes and the 2 corrected lint-policy accept probes | worktree | now | all red or green as declared |
| `guard_census.py`, `--check-runbook` | worktree | now | 0, 0 |
| `prose_check.py`, `sync_agent_skills.py --check` | worktree | now | 0, 0 |
| rustdoc fence matrix, 25 items, `cargo test --doc` | scratch crate, edition 2024, package lint table at forbid, deleted after | now | table below |
| real-tree plants, real scanner | `git archive d514da30` in the scratchpad with its own `git init`, deleted after | now | table below |
| `no_run` fence holding an `unsafe` block appended to `crates/ocelli-core/src/lib.rs`, `cargo test -p ocelli-core --doc --offline` | same archive, `CARGO_TARGET_DIR=<wt>/target/review-p2`, deleted after | now | "doc_planted (line 39) - compile ... ok", exit 0 |
| `rustc` on `r##"a"###` | scratchpad, rustc 1.97.1 | now | "too many `#` when terminating raw string", matching the new docstring |
| bin target holding an `unsafe` block | scratch crate, package lint table at forbid | now | refused with `-F unsafe-code` |

What rustdoc compiles against what the scanner reports, each item holding one
`unsafe` block read from a `u8` reference:

| Fence shape | rustdoc | scanner |
|---|---|---|
| untagged, `rust`, ` rust`, `rust,no_run` | compiled | reported |
| `no_run` | compiled, not run | **missed** |
| `should_panic`, `edition2021`, `standalone_crate`, `{.rust}` | compiled and run | **missed** |
| `compile_fail` | compiled (the test fails because the code compiles) | **missed** |
| `~~~` fence | compiled and run | **missed** |
| indented code block after a paragraph | compiled and run | **missed** |
| fence spread over `#[doc = "..."]` attributes, or mixing `///` with `#[doc]` | compiled and run | **missed** |
| `/*! ```` with the fence on the opening line | compiled and run | **missed** |
| fence inside a `>` block quote | compiled and run | **missed** |
| fence split by a plain `//` line | compiled and run | **missed** |
| `/** ```` on the opening line, fence in a list item, hidden `# ` line, two fences in one comment, unclosed fence | compiled and run | reported |
| `ignore`, `text`, `wgsl` | not compiled | not reported |

Real-tree plants:

| Plant | scanner |
|---|---|
| untagged doctest in `ocelli-core` lib | refused at the code line |
| `no_run`, `should_panic`, `~~~` and `#[doc]` doctests in `ocelli-core` lib | **OK at exit 0** (D1) |
| `#[unsafe(no_mangle)]` in a trybuild ui file | refused (pass 1 D1 closed) |
| `# [allow(unsafe_code)]` in a trybuild ui file | refused (pass 1 S1 closed) |
| `#[unsafe(no_mangle)]` in `vendor/ritk-codecs-0.6.0` | refused, count 1 against 0 (pass 1 D1 closed) |
| `#![allow(unsafe_code)]` in `vendor/ritk-codecs-0.6.0` | OK, as the corrected prose now says (pass 1 D2 closed) |

## Defects

### D1, the doctest coverage is narrower than rustdoc, and the prose claims it covers the doctests

**Where**: `scripts/unsafe_allowlist_check.py`, `_fence_code`, where
`lang in ("", "rust")` tests only the first token of the info string and only
backtick fences open a block. Claims at the module docstring, lines 42 to 43 ("Fences tagged
`text` or another language are comment text"), `scripts/lint_policy_check.py:268`
in the docstring and `:1812` in the comment ("which is what covers
the doctests", "which covers the doctests"), `docs/lld/guards.md:1119` ("the lint does not reach the doctests ... which the script audits"), and
the comment at
`scripts/tests/test_unsafe_allowlist_check.py:324`.
**What**: rustdoc treats a fence as Rust unless its info string carries a word
rustdoc does not know and no `rust` word, so `no_run`, `should_panic`,
`compile_fail`, `edition2021`, `standalone_crate` and `{.rust}` are all Rust,
and the scanner reads every one of them as comment text. `~~~` fences,
indented code blocks, fences written through `#[doc = "..."]`, a fence opened
on the `/*!` line, a fence in a block quote and a fence split by a plain `//`
line are doctests the scanner also misses.
**Why it is wrong**: lead ruling 2 says doctests are code and that the
documents say the scanner covers them, HLD 27.2 R5 keeps the audit surface to
two files, and the story's contract is no new blind spot behind a sentence
claiming coverage. `no_run` is the ordinary tag for a doctest that needs a
device or a file, so it is the likeliest shape in this repository.
**Evidence**: the rustdoc matrix above, and the real-tree `no_run` plant:
`cargo test -p ocelli-core --doc` compiled it at exit 0 under the workspace
forbid, and `scripts/unsafe_allowlist_check.py` printed OK at exit 0 on the
same tree. The tree today holds only untagged, `rust`, `text` and `wgsl`
fences, so no current file is missed.
**Scope and regression**: every lib target's doc comments. Smallest repair:
decide Rust-ness the way rustdoc's info-string parse does (Rust unless an
unknown word appears without `rust`, the known words being `rust`, `ignore`
and its `ignore-` forms, `should_panic`, `no_run`, `compile_fail`,
`test_harness`, `standalone_crate`, `edition` forms, error codes and `{...}`
attribute sets), accept `~~~` openers, and either read indented blocks and
`#[doc]` strings or name them in the docstring and LLD as the declared
residue. A unit test per tag class (`no_run`, `should_panic`, `edition2021`)
and one `~~~` case, each red on the current `_fence_code`.

### D2, the corrected probe notes miscount the `#[path]` modules

**Where**: `scripts/guards/catalogue.py`, the notes of
`lint-policy.clean-module-outside-the-member` and
`lint-policy.clean-cfg-attr-module-path`.
**What**: both notes still say "`produce` already follows one path module, so
the planted module is the second", and the new sentence says "a third
`#[path]` module had landed since the probe was written". One module landed,
`examples/folder-viewer/src/held.rs:23` from F-210, which makes it the second
in the tree and the planted module the third.
**Why it is wrong**: a false sentence beside the number it explains.
**Evidence**: `git grep -n '#\[path' -- '*.rs'` outside `vendor/` gives the
two files above, and the unplanted `lint_policy_check.py` prints "2 `#[path]`
module(s) followed".
**Scope and regression**: prose only, both probes are green and correct.
Rewrite each note to say the tree follows two path modules, `produce` and
`held.rs`, so the planted module is the third, and delete the "had landed"
sentence.

## Smells

### S1, the unclosed-fence branch is fail-open if removed and no test reaches it

**Where**: `scripts/unsafe_allowlist_check.py`, the tail of `_fence_code`
that appends a fence still open when the comment ends.
**What**: replacing its condition with `False` leaves all 20 tests green.
rustdoc compiles an unclosed fence (measured, one test ok), and the scanner
reports it today, so the behaviour is right and unguarded.
**Mechanism**: the next edit to `_fence_code`, which D1 makes likely, can drop
the branch and reopen a doctest blind spot with a green suite. One unit test
with a doc comment whose fence never closes, asserting the code line, closes
it.

## Nitpicks

- **N1.** Replacing `line = probe_line` with a no-op in the `#` branch leaves
  every test green. The effect would be wrong line numbers after an attribute
  whose `#` and `[` sit on different lines, not a miss. A test with a newline
  between `#` and `[` followed by an `unsafe` would pin it.
- **N2.** Treating `////` as a doc line leaves every test green. Fail-closed
  and absent from the tree.
- **N3.** Dropping the weakening hits found inside a doctest fence leaves
  every test green. Harmless, since no table reaches a doctest and its
  `unsafe` is reported anyway.
- **N4.** Fences in the doc comments of `tests/`, `examples/` and bin files
  are read as code although rustdoc never runs them, and the docstring's reason
  ("rustdoc compiles and runs them") is true only for lib targets. Fail-closed,
  zero effect on the tree (no per-file change).
- **N5.** Pass 1 suggested a level-1 catalogue probe beside the five trigger
  probes for the attribute shape. The lead rulings did not require one and
  the unit tests drive `main()` over a staged index, so this stays a
  preference. Pass 1's N1 (pooled names refuse a `cfg_attr` holding a deny
  and an unrelated allow) and N2 (redundant byte-char branch) remain as they
  were.

## Verified clean

- **Pass 1 replayed.** D1 closed: attribute `unsafe` reported in members,
  trybuild and vendored files, and an unterminated attribute refused, with
  each new test red under its mutation. D2 closed by the smaller option: the
  three sentences now say vendored files are audited by count, and a vendored
  allow is OK as they say. D3 closed: the citation is rustc's text, measured on
  1.97.1, and the refusal no longer says "unterminated". D4 partly closed,
  untagged and `rust` fences in line and block doc comments are reported at
  the right file line, the remainder is D1 here. S1 closed for every spelling
  tried. N3 and N4 closed.
- **False positives.** Zero per-file difference over all 382 tracked `.rs`
  files between the pre-fix and post-fix scanner, including the tree's 3 untagged,
  5 `rust`, 40 `text` and 2 `wgsl` fences, vendored ones counted. The whole-tree run
  stays at 270, 104 and 0.
- **Mutations.** 15 of 20 red on the intended test: attribute suppression,
  the end-of-file attribute refusal, trivia after `#`, trivia after `!`,
  block comments in trivia, line and block doc fences returning nothing, the
  `rust` tag, the untagged fence, every fence as Rust, `//!` recognition, the
  block offset, the fence start line, the vendored branch and the `*` unindent.
- **New prose executed.** The LLD sentence on the targets the lint reaches
  (doctest unreached, bin reached, measured now, the rest from pass 1), the
  scanner and LLD claim that `unsafe` is reported inside an attribute, the
  unterminated attribute refusal, the vendored-allow sentence in the docstring,
  LLD and catalogue note, the rustc message, and runbook rows 504 and 507,
  which `--check-runbook` confirms are generated from the catalogue.
- **The two probe expect changes** are measured green and justified by
  `held.rs` (F-210), apart from the note text in D2.
- **GLM's bare `git checkout` in a scratch copy.** The worktree is clean
  against `d514da30`, `git status --ignored` shows only build and dependency
  output, HEAD's reflog since `8fb69ae7` holds the three F-232 commits and no
  checkout, the only stash is S17's, and `sprint/s46` in the main checkout is
  clean. Nothing tracked in the worktree was affected. A bare `git checkout`
  with no arguments only reports status in any case.
- README, `use.html` and `more.html` carry no `unsafe` sentence, so none is
  stale.
- Every scratch copy and the `target/review-p2` directory were deleted with
  `shutil`. The scratch archive had its own `git init`, so its restores never
  reached the worktree.

## Limitations and handoff

- No `gate --floor`, `gate --sprint`, `bin/ocelli.sh wasm`, clippy or wasm32
  check on this pass, since the diff touches only Python and prose. The rustc
  reach of the lint rests on pass 1 plus the bin plant here.
- Indented code blocks and `#[doc]` strings were measured once each on the
  scratch crate, not exhaustively.
- No oracle, GPU, browser or corpus job.
- Next: GLM fixes D1, D2 and S1, then a fresh pass 3 reviews the remediation.
