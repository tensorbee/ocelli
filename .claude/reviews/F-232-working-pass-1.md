# F-232 review, pass 1

**Reviewed**: `git diff 8fb69ae7 0d812e0b` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, clean at
start and end apart from this report. No unstaged paths reviewed.
**Author / reviewer**: GLM-5.3 in opencode wrote `0d812e0b`. Claude Opus 5.5
reviewed it as an independent subagent and wrote none of the reviewed material.
**Result**: 4 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **Trigger set (AC1 to AC5, AC7, AC8).** Fifty hand-written snippets were run
  through F-229's `strip_noise` and the new `scan`, with expectations taken
  from the Rust reference lexer rules rather than from the code: the five pass-8
  and pass-9 triggers, a string continuation, nested block comments, byte, raw
  byte, C and raw C strings, byte chars, lifetimes, labels, `'static`, `'_`,
  raw lifetimes, `r#unsafe`, `is_unsafe`, `unsafe` in a `macro_rules!` body, in
  `///`, `//!` and `/** */` doc comments and in `#[doc = "..."]`, unicode and
  `\u{}` chars, numbers with suffixes, a shebang, and brackets inside char
  literals within attributes. All five triggers and the nested comment are
  fixed. **Rejected**: one shape regressed (D1), see below.
- **Whole-tree agreement (AC9).** Old and new scanners give identical per-file
  `unsafe` line lists on all 382 tracked `.rs` files and no weakening hit, so
  104 and 0 are re-derived rather than coincident.
- **rustc reach (AC11 to AC13).** Plants in a `git archive HEAD` scratch copy
  with its own target dir, `ocelli-core`: lib, integration test, example,
  bench, build script and wasm32 lib all fail with "usage of an `unsafe`
  block ... requested on the command line with `-F unsafe-code`". Inner
  `#![allow(unsafe_code)]` and `#[allow(unsafe_code)]` on a `mod` give E0453.
  `-F unsafe_code -A unsafe_code` on the command line still errors, so a later
  rustflag cannot lower it. **Rejected**: doctests (D4).
- **lint_policy_check (AC15).** Table at `deny`, table absent, and the `unsafe`
  arm running another command each refuse. Table written as
  `{ level = "forbid", priority = -1 }` is accepted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `python3 -m unittest discover -s scripts/tests -p 'test_unsafe*'` | worktree HEAD | now | 0, 15 tests. No separate lint-policy unit file exists |
| same tests against `git show 8fb69ae7:scripts/unsafe_allowlist_check.py` | scratch copy | now | 8 FAIL, the 8 new tests (AC6 confirmed) |
| 9 scanner mutations (nesting, escape step, raw-as-escape, line comment off, char close, inner-only attr, raw ident, byte char off, unterminated blanking) | scratch copy | now | 8 red, byte-char-off survives (N2, equivalent) |
| 6 new `unsafe.*` probes against F-229's regex via a `scan` shim | scratch copy | now | all 6 HARNESS, "did not fire", so each is red for its reason |
| `guard_probe.py --profile deep --only` the 6 new, `unsafe.third-file`, the 3 unsafe lint-policy probes | worktree | now | all red as declared |
| `python3 scripts/unsafe_allowlist_check.py` | worktree | now | 0, 270 files, 104 and 0 |
| `python3 scripts/lint_policy_check.py` | worktree | now | 0 |
| `bin/ocelli.sh gate unsafe`, `gate guards`, `gate lint` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | 0, 0, 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | worktree | now | 0 |
| `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown` | worktree | now | 0 |
| `guard_census.py`, `--check-runbook` | worktree | now | 0, 0 watched by nothing, runbook current |
| `sync_agent_skills.py --check`, `prose_check.py`, `source_provenance_check.py` | worktree | now | 0, 0, 0 |
| rustc reach plants, see above, plus doctest, `#[unsafe(no_mangle)]`, spaced attribute, trybuild and vendor plants | scratch copy, own target dir, deleted after | now | table below |
| `lint-policy.clean-module-outside-the-member`, `.clean-cfg-attr-module-path` | main checkout, `sprint/s46` at `bbf624de`, unedited | now | both HARNESS, "3 `#[path]` module(s)", same as worktree |

What catches each plant:

| Plant | rustc (forbid table) | scanner |
|---|---|---|
| `unsafe` block in lib, test, example, bench, build.rs | refused | refused |
| same in lib, `--target wasm32-unknown-unknown` | refused | refused |
| `#![allow(unsafe_code)]` in lib root | E0453 | refused (weakening) |
| `#[allow(unsafe_code)] mod m` holding unsafe | E0453 plus unsafe block | refused, both lines |
| `# [allow(unsafe_code)] mod m` (space) | E0453 | **missed** (S1) |
| `#[unsafe(no_mangle)]` in lib | "declaration of a `no_mangle` function" | **missed** (D1), F-229's scanner refused it |
| `unsafe` in a `/// ``` ` doctest, `cargo test --doc` | **passes, 1 test ok** | **missed** (D4) |
| `unsafe` block in a trybuild ui file | not reached (`cargo test --test compile_fail` passes) | refused |
| `#[unsafe(no_mangle)]` in a trybuild ui file | not reached | **missed** (D1) |
| `unsafe` block in `vendor/ritk-codecs-0.6.0` | not reached | refused, count 1 against 0 |
| `#![allow(unsafe_code)]` in a vendored file | not reached | **missed** (D2) |
| `#[unsafe(no_mangle)]` in a vendored file | not reached | **missed**, count unchanged (D1) |

## Defects

### D1, `unsafe` inside an attribute is no longer reported

**Where**: `scripts/unsafe_allowlist_check.py:337`,
`elif word == "unsafe" and not attr_depth:`
**What**: every `unsafe` token between `#[` and its `]` is dropped.
`#[unsafe(no_mangle)]`, `#[unsafe(export_name = "g")]` and
`#[cfg_attr(all(), unsafe(no_mangle))]` give `([], [])`, where F-229's scanner
reported line 1 for each. The same suppression makes an unclosed `#[` fail
open: `#[cfg(x)\nunsafe fn f(){}` and `#[derive(\nfn g() { unsafe { 1 } }`
both return `([], [])` with no `LexError`, so a lexically broken trybuild file
hides every later `unsafe` in it.
**Why it is wrong**: HLD 27.2 R5. The workspace is edition 2024, where
`#[unsafe(no_mangle)]` is the only accepted spelling, and rustc's `unsafe_code`
refuses it ("declaration of a `no_mangle` function", measured). The story's
contract is "no new blind spot", and the docstring at line 26, the LLD at
`docs/lld/guards.md:1123`, the `refuses` text at `catalogue.py:7513` and the
`VENDORED_UNSAFE` comment at line 83 all say `unsafe` is reported "as a
keyword token", which it is in this position.
**Evidence**: probe table above, and the snippet run `unsafe_attr old=[1]
new=([], [])`.
**Scope and regression**: member targets are still covered by rustc. Trybuild
files and the vendored count are not: a vendored refresh adding
`#[unsafe(no_mangle)]` leaves 104 unmoved. Smallest regression: a unit test
that `#[unsafe(no_mangle)]` and `#[cfg_attr(x, unsafe(no_mangle))]` are
reported, one that an attribute still open at end of file is a `LexError`,
and a level-1 probe beside the five trigger probes.

### D2, the weakening refusal never runs on vendored files

**Where**: `scripts/unsafe_allowlist_check.py:386`, the vendor branch
`continue`s before the weakening loop. Claims at lines 31 to 34,
`catalogue.py:7669` (probe note) and `docs/lld/guards.md:1124`.
**What**: `#![allow(unsafe_code)]` planted in a `vendor/ritk-codecs-0.6.0`
file gives `OK` at exit 0. The docstring says the refusal "is what keeps the
trybuild files and the vendored packages, which no lint table reaches, honest",
the probe note repeats it, and the LLD says "in any tracked Rust file".
**Why it is wrong**: a false sentence in three places, AC14 says "any tracked
Rust file".
**Evidence**: plant table above.
**Scope and regression**: no runtime consequence today, because no lint table
reaches `vendor/` at all, so an allow there lowers nothing. Either refuse it
there with a unit test planting one under `vendor/`, or delete "and the
vendored packages" from all three places. The second is the smaller change.

### D3, the raw-string refusal cites the wrong rustc error

**Where**: `scripts/unsafe_allowlist_check.py:163`
**What**: "A closing quote followed by MORE hashes than the opening is rustc
error E0766". `rustc --explain E0766` is "A double quote byte string (`b"`)
was not terminated". rustc 1.97.1 on `r##"a"###` and `br##"a"###` prints
"too many `#` when terminating raw string" with no code. GLM's handoff measured
`r##"a"###b"##`, whose trailing `b"##` opens an unterminated byte string, so
the code it saw belonged to the junk after the literal. The refusal itself
also reads "unterminated raw string literal" for a literal that is terminated.
**Why it is wrong**: a false factual claim in a doc comment.
**Evidence**: `rustc --explain E0766`, and `rustc --edition 2021 h.rs` on the
two literals above.
**Scope and regression**: prose only. The fail-closed behaviour is right.
Replace the citation with rustc's text, or drop it.

### D4, unsafe in a doctest is caught by neither mechanism, and the prose says every target

**Where**: `docs/lld/guards.md:1115` "The lint covers every target of every
member", `scripts/lint_policy_check.py:262` and `:1804` "The table reaches
every target of every workspace member", `Cargo.toml:34 to 38`.
**What**: appending a three-line doc comment whose fenced block reads
`let _x = unsafe { core::ptr::read(&0u8) }` to `crates/ocelli-core/src/lib.rs` gives `cargo test -p
ocelli-core --doc` "test ... doc_planted ... ok", exit 0, and the scanner `OK`.
Cargo passes the lint table to rustc for the package's targets and not to the
doctest crates rustdoc compiles, and the scanner treats doc comments as
comments, which AC7 requires. `git grep` finds 88 doctest fences in tracked
Rust today, so this is compiled and executed code under `gate test`.
**Why it is wrong**: HLD 27.2 R5 keeps the unsafe audit surface to two files,
and the doctest is a third. The sentences above are false for the one target
kind `cargo test` builds that the plan's AC11 did not plant.
**Evidence**: the doctest row of the plant table.
**Scope and regression**: not introduced by this diff for the scanner, which
was always blind here, but the claim that one table line reaches every target
is new. The plan forbids crate source edits, so the options are a lead
decision: `#![doc(test(attr(forbid(unsafe_code))))]` in each lib root (a
later story), the scanner reading fenced code in doc comments, or at minimum
stating doctests as the declared residue in the four places above. Whichever
is chosen, plant one doctest to prove it.

## Smells

### S1, a space or comment between `#` and `[` evades the weakening refusal

**Where**: `scripts/unsafe_allowlist_check.py`, the `c == "#"` branch requires
`[` (after an optional `!`) immediately.
**What**: `# [allow(unsafe_code)]`, `#! [allow(unsafe_code)]` and
`#/**/[allow(unsafe_code)]` give `weakening == []`. They are valid
attributes: `rustc --edition 2024` compiled a crate with `#![deny(unsafe_code)]`
and `# [allow(unsafe_code)] mod m { ... unsafe { } ... }` with warnings only.
**Mechanism**: harmless in members today because the forbid gives E0453
(measured). The sequence recorded in the docstring moves the table to `deny`
for the first allowed file, and from then on this spelling silences rustc and
passes the scanner, so the refusal that sequence relies on is the one that
misses. Trybuild files are exposed now. Treat `#`, optional `!` and `[` as
tokens separated by whitespace and comments, with a unit test per spelling.

## Nitpicks

- **N1.** `#[cfg_attr(x, deny(unsafe_code), allow(dead_code))]` is refused as
  weakening, because names are pooled per attribute. Fail-closed and not in the
  tree.
- **N2.** The `word == "b" and nxt == "'"` branch is redundant: removing it
  leaves every test green because the `'` branch lexes the same literal next.
  Harmless.
- **N3.** The lint-policy refusal reads "carries `unsafe_code` at absent".
- **N4.** The too-many-hashes refusal says "unterminated" (part of D3).

## Verified clean

- The five pass-8 and pass-9 triggers, the string continuation and the nested
  comment, by unit test, by mutation and by probe against the old logic.
- No false positive on the 50-snippet set apart from N1, and zero per-file
  difference against F-229 over 382 files.
- `unsafe` in a `macro_rules!` body is reported.
- The forbid reaches lib, test, example, bench, build script and wasm32 lib,
  and cannot be lowered by an inner allow, an allow on a `mod` or a later `-A`.
  The trybuild ui files are not reached by the table, measured, so that
  sentence is true.
- `lint_policy_check.py` requires both halves, and accepts the inline-table
  spelling at forbid.
- `ci/guard-probe-budget.json` and the runbook match the census
  (`--check-runbook` 0). The runbook diff is regeneration and renumbering.
- `docs/lld/guards.md:1053` is true: `lint-policy.unsafe-gate-named-only-in-a-comment`
  is red on "must stay in the `unsafe` gate".
- README, `use.html` and `more.html` carry no unsafe sentence, so none is stale.
- **GLM's two failure claims.** The oracle `produce` tests fail on
  `tools/oracle/out/run.json` because this worktree has no `tools/oracle/out`,
  which is gitignored and present in the main checkout. That is the worktree's
  environment, not a tree failure, and not F-232's. The two `guards-deep`
  accept probes fail identically on `sprint/s46` at `bbf624de` in the main
  checkout, on "3 `#[path]` module(s)", from
  `examples/folder-viewer/src/held.rs:23`. Not F-232's.
- **GLM's proposed CLAUDE.md sentence.** Do not take it as written. It puts a
  state claim ("do not exist yet") in a file that keeps correcting stale state
  claims, it makes a third copy of the four-step sequence already in the
  scanner docstring and the LLD, and "over every tracked `.rs` file" inherits
  D2 and D4. After the fixes, something like this would do: "**No `unsafe`**
  outside `ocelli-wasm/src/ring.rs` and `ocelli-core/src/cast.rs`. rustc
  forbids it through `[workspace.lints.rust]` and
  `scripts/unsafe_allowlist_check.py` audits the files rustc does not reach.
  The first story that writes one of the two files follows the sequence in
  that script's docstring."

## Limitations and handoff

- No `bin/ocelli.sh wasm`, `wasm-decode`, `gate --floor` or `gate --sprint`
  were run on this pass. The `ocelli-wasm` feature combinations rest on GLM's
  receipt and on the workspace wasm32 check run here.
- rustc reach was planted in `ocelli-core` only, plus GLM's receipts for the
  other crates. Bin targets were not planted again here.
- No oracle, GPU, browser or corpus job.
- Next: GLM or the lead fixes D1 to D4 and S1, then a fresh pass 2 reviews
  the remediation.
