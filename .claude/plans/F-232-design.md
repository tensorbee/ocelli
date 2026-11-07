# F-232, Unsafe guard soundness

**Status**: approved
**Epic ref**: E38.35
**Sprint**: S46
**Estimate**: 2w

Story row, `docs/sprints/BACKLOG.md`: "Unsafe guard soundness: unsafe_code forbidden by rustc through the workspace lint table on every target of every workspace member, and scripts/unsafe_allowlist_check.py rebuilt on a left-to-right Rust tokenizer with a planted unsafe test for each of the five triggers the F-229 review found". Layer Build, depends on F-229, milestone M20.

`allocation.json` notes: "Found by F-229's eighth and ninth reviews. Answered by the operator on 2026-11-06: fixed in S46, the guard made sound against every trigger."

## Normative source, transcribed

### HLD 27.1, `docs/hld/24-agent-code-standards.md`

> \[workspace.lints.rust\]
>
> unsafe_code = "deny" \# allow-listed per file, see 27.2

### HLD 27.2 R5, same file

| # | Rule | Why |
|----|----|----|
| R5 | No unsafe outside the allow-list (ocelli-wasm/src/ring.rs, ocelli-core/src/cast.rs) | Keeps the audit surface to two files |

### The declared substitution, `scripts/lint_policy_check.py` module docstring

"`unsafe_code = "deny"` is NOT in `Cargo.toml`. HLD 27.2 R5 is enforced instead by `scripts/unsafe_allowlist_check.py` over the whole tree, in the `unsafe` gate and in the CI floor. That substitution is stronger than the lint for the thing R5 actually asks for, because a per-file `#![allow(unsafe_code)]` would silence the lint and would not silence the script. This check requires ONE of the two and names which one it found, so removing the script without adding the lint is refused."

`REQUIRED_RUST = {"unsafe_code": "deny"}` (line 296), `STRENGTH = {"allow": 0, "warn": 1, "deny": 2, "forbid": 3}` (line 455). The check accepts the lint when `STRENGTH.get(lint_level, 0) >= 2`, otherwise the script in the `unsafe` gate, otherwise refuses. A separate check refuses an inner `allow` or `expect` of any lint named in `REQUIRED_CLIPPY` or `REQUIRED_RUST`.

### The operator's answer, `docs/sprints/CURRENT_SPRINT.md`

"F-229's eighth review found that `scripts/unsafe_allowlist_check.py`'s string pattern opens a false string after a `'"'` character literal, after a raw string holding a quote or ending in a backslash, and after a line comment with an unpaired quote, and blanks the code that follows. An `unsafe` block planted at `crates/ocelli-dicom/src/dicomweb.rs:418` passed the guard. No audited count changes. ... **Answered by the operator on 2026-11-06:** fixed in S46. A new story makes the guard sound against every trigger above."

### The findings, `../ocelli-wt-f-229/.claude/reviews/`

Pass 8, line 54: "`STRING` begins a match at any `"`. A char literal `'"'` or `'\"'`, a raw string holding a `"` or ending in `\`, and a line comment with an unpaired `"` each open a false string. That string runs to the next real literal's opening quote, and everything between is blanked as string."

Pass 8, line 56: "I planted an unsafe block at `crates/ocelli-dicom/src/dicomweb.rs:418`, inside `media_type_parameter`, between the `trim_matches('"')` calls at 412 and 425, in a full `git archive HEAD` copy. `--staged` printed OK with exit 0, while the control plant in `ocelli-pixel` was refused. The lexer comparison finds the same blind span in `ocelli-render/src/caps.rs` 2486-2487, `ocelli-wasm/src/canvas.rs` 558, `examples/example-server/src/reply.rs` 138-226, `series.rs` 208-325 and `examples/folder-viewer/src/preview.rs` 1334-1371."

Pass 9, line 13: "The scanner finds 0 after `'"'`, after `'\"'`, after `r"a\"`, after `r#"a " b"#` and after `// say "`. That is five triggers."

## Facts measured for this plan

Measured on 2026-11-06 on `sprint/s46` at `1ba258e`, toolchain 1.97.1, in scratch target directories under the session scratchpad. Nothing in the tree was edited to measure.

1. **Neither allow-listed file exists.** `ls crates/ocelli-wasm/src/ring.rs crates/ocelli-core/src/cast.rs` reports both absent. The scanner's docstring already says "Neither file has to exist yet. The allow-list is a permission, not a requirement."
2. `cargo metadata --no-deps` lists **16 workspace packages** with **16 lib, 5 bin, 113 test and 9 example targets**. Every member carries `[lints] workspace = true`. Root `Cargo.toml` has `[workspace.lints.clippy]` and no `[workspace.lints.rust]` table. `git grep unsafe_code` finds no lint attribute in any Rust source.
3. **Every target already compiles under `-F unsafe_code`.** A `RUSTC_WRAPPER` that appends `-F unsafe_code` only to rustc invocations whose source is under `crates/`, `tools/oracle/` or `examples/` gave exit 0 for `cargo check --workspace --all-targets` (16 workspace packages checked, 126 distinct crate names given the flag), for `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown`, for `cargo check -p ocelli-wasm --target wasm32-unknown-unknown --all-targets`, with `--features render-worker`, and with `--no-default-features --features decode-worker`. So `#[wasm_bindgen]` expansions do not trip the lint. A positive control crate with one `unsafe` block failed under the same wrapper with "usage of an `unsafe` block".
4. A plain `RUSTFLAGS="-D unsafe_code"` is not a usable measurement, because it also reaches the path dependency `vendor/openjph-core-0.1.0`, which failed with 74 errors. A workspace lint table does not reach it, because `vendor/` is excluded from the workspace.
5. The scanner checks 267 files today: `OK: no unsafe outside the allow-list (267 files checked, 2 permitted), vendored audit surface unchanged: openjph-core-0.1.0 104, ritk-codecs-0.6.0 0`. Its `STRING` regex is `r'"(?:[^"\\]|\\.)*"'`, run after `BLOCK_COMMENT` and before `LINE_COMMENT`. There is no character literal, raw string or nested comment handling. `scripts/tests/test_unsafe_allowlist_check.py` has five tests on `sprint/s46` and none tests the tokenizer. F-229's branch adds `re.S` to `STRING` and a sixth test.
6. Seven trybuild files under `crates/*/tests/ui/` (four in ocelli-core, two in ocelli-compute, one in ocelli-render) are compiled by trybuild, not as cargo targets of their package, so no lint table reaches them. The scanner covers them through `git ls-files`.

## What the specification does not cover

Every item is a lead decision under the operator's delegation of 2026-11-06.

1. **The rustc mechanism is `unsafe_code = "forbid"` in `[workspace.lints.rust]`.** This is the recommended option, forbid enforced by rustc at every crate root, reached through the lint table. Cargo passes the table's lints to rustc for every target of every member that inherits it, so the one table line is a forbid at all 143 crate roots of fact 2, including the 113 test and 9 example roots that a `#![forbid]` written in `lib.rs` and `main.rs` would miss. Fact 3 shows the tree compiles under it today. A forbid cannot be relaxed by an inner `allow`, so no source file can switch it off.
2. **The two allowed crates get no exception today**, because fact 1 shows the allowed files do not exist and there is no module to give a narrow `#[allow(unsafe_code)]`. The first story that writes `ring.rs` or `cast.rs` makes that change by design: it moves the table to `deny`, adds `#![forbid(unsafe_code)]` to every other crate root, puts `#[allow(unsafe_code)]` on that one `mod` declaration, and teaches `lint_policy_check.py` that one exception. This plan writes that sequence into the scanner's docstring and the LLD so it is not rediscovered.
3. **Both mechanisms are required from now on.** `lint_policy_check.py` stops accepting either one and requires the table at `forbid` and the script in the `unsafe` gate, because the script covers the vendored audit and the trybuild files that the lint does not reach, and the lint covers what any lexer can still miss.
4. **The scanner keeps its job and gets a real tokenizer.** One left-to-right pass over the source recognises line comments, doc comments, nested block comments, string literals, byte strings, C strings, raw strings of any hash count with their byte and C forms, character and byte literals, lifetimes and labels, and raw identifiers, and reports `unsafe` only as a keyword token. `r#unsafe` is an identifier and is not reported. An unterminated literal or comment is a refusal naming the file and line, never a blank to the end of the file.
5. **The scanner also refuses a lint attribute that weakens `unsafe_code`.** `allow`, `expect` or `warn` of `unsafe_code`, inner or outer, in any tracked Rust file. Under fact 2 rustc refuses these too, and the script refusal is what keeps the trybuild files and a future `deny` table honest.
6. **A differential test against rustc's own lexer is not added.** The planted tests and a whole-tree comparison recorded once in the handoff carry the evidence. A second lexer in the test suite would be a second copy to maintain.

## Problem and reproducer

In a scratch copy of the tree, plant `let _p = unsafe { core::ptr::read(&0u8) };` after line 417 of `crates/ocelli-dicom/src/dicomweb.rs`, between the two `trim_matches('"')` calls, which is pass 8's reproduction.

```bash
python3 scripts/unsafe_allowlist_check.py     # OK, exit 0, which is the defect
```

Expected after this story: the scanner names the planted line, and `cargo check -p ocelli-dicom` refuses it with "usage of an `unsafe` block" independently.

## Scope, API and compatibility

In scope: `Cargo.toml` `[workspace.lints.rust]`, `scripts/unsafe_allowlist_check.py`, `scripts/tests/test_unsafe_allowlist_check.py`, `scripts/lint_policy_check.py`, the `unsafe` and `lint-policy` guard entries and probes in `scripts/guards/catalogue.py`, `ci/guard-probe-budget.json`, `docs/runbooks/guard-verification.md`, `docs/lld/guards.md`.

Non-goals: the vendored counts, which stay 104 and 0 and are re-derived by the new tokenizer rather than re-recorded. Writing `ring.rs` or `cast.rs`. `cargo deny` advisories, which the operator accepted as a known risk.

No public API change. The scanner's command line, its `--staged` mode and its OK line keep their shape.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| every `cargo` build, check, clippy and test of a member | `[workspace.lints.rust] unsafe_code = "forbid"` | all targets of all 16 members | rustc error on any `unsafe` or weakening attribute |
| `gate unsafe`, CI floor | `unsafe_allowlist_check.py` | every tracked `.rs`, worktree or index | OK line, or a refusal naming file and line |
| pre-commit hook | the same script, `--staged` | index blobs | the same |
| `gate guards`, first command | `lint_policy_check.py` | `Cargo.toml` and member manifests | refusal if the table is weaker than `forbid` or the script leaves the gate |
| `gate guards` | catalogue probes | floor sandbox | each probe's expected fragment |

## State transitions and invariants

Not applicable. Both checks are functions of the tree. The invariant across consumers: a tracked Rust file holding an `unsafe` keyword outside the allow-list is refused by the scanner, and if it is part of a member target it is refused by rustc as well.

## Ownership, commit and recovery

Not applicable. No runtime resource.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Trigger 1. A source with `'"'` followed by an `unsafe` block is refused, naming the block's line | `test_unsafe_after_quote_char_literal` | A fix that adds `re.S` or a char regex but keeps three passes | Implementation |
| AC2 | Trigger 2. The same after `'\"'` | `test_unsafe_after_escaped_quote_char_literal` | A char rule that handles `'"'` only | Implementation |
| AC3 | Trigger 3. The same after `r"a\"` | `test_unsafe_after_raw_string_ending_in_backslash` | Treating `\` as an escape inside a raw string | Implementation |
| AC4 | Trigger 4. The same after `r#"a " b"#`, and after `br##"x "# y"##` | `test_unsafe_after_raw_string_holding_a_quote` | A raw rule with a fixed hash count | Implementation |
| AC5 | Trigger 5. The same after `// say "` and after `/// say "` | `test_unsafe_after_line_comment_with_a_quote` | Stripping strings before line comments | Implementation |
| AC6 | Each test in AC1 to AC5 fails against the `sprint/s46` scanner and against F-229's | old-source RED recorded in the handoff | Tests written against the new tokenizer only | Implementation, recorded |
| AC7 | No false positive. `unsafe` inside a string, a raw string, a byte string, a char-adjacent string, a line comment, a doc comment, a nested block comment `/* /* unsafe */ unsafe */`, and as `is_unsafe` and `r#unsafe`, is not reported. A lifetime `'a` followed by `unsafe` on the next line IS reported | `test_no_keyword_inside_literals_or_comments`, `test_lifetime_is_not_a_char_literal` | A tokenizer that reads `'a` as an unterminated char and blanks the next line | Implementation |
| AC8 | An unterminated string, raw string or block comment is a refusal naming file and line, exit 1 | `test_unterminated_literal_is_refused` | Blanking to end of file, the regex's failure shape | Implementation |
| AC9 | Whole-tree agreement. The new scanner reports 0 outside the allow-list on the final tree, and re-derives 104 for `openjph-core-0.1.0` and 0 for `ritk-codecs-0.6.0` with no edit to `VENDORED_UNSAFE` | `gate unsafe` | A tokenizer that changes the vendored count and re-records it to pass | Implementation |
| AC10 | The six blind spans of pass 8, `dicomweb.rs` 412 to 425, `caps.rs` 2486 to 2487, `canvas.rs` 558, `reply.rs` 138 to 226, `series.rs` 208 to 325 and `preview.rs` 1334 to 1371, each get a planted `unsafe` in a scratch copy and each is refused | recorded in the handoff, line numbers re-read on the base tree | A fix proved only on synthetic snippets | Implementation, recorded |
| AC11 | rustc forbids. `Cargo.toml` carries `[workspace.lints.rust] unsafe_code = "forbid"`. A planted `unsafe` block in a lib, a bin, an integration test and an example target, each in a scratch copy, fails `cargo check --workspace --all-targets` with "usage of an `unsafe` block" | recorded in the handoff, four plants | A forbid in `lib.rs` only, which misses the test and example roots | Implementation, recorded |
| AC12 | rustc refuses relaxation. A planted `#![allow(unsafe_code)]` in a lib root fails `cargo check` with error E0453 | recorded in the handoff | `deny`, which an inner `allow` silences | Implementation, recorded |
| AC13 | wasm32. `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown` and `bin/ocelli.sh wasm` and `bin/ocelli.sh wasm-decode` pass with the table in place | `gate native`, `gate wasm` | A forbid that breaks `#[wasm_bindgen]` expansion | Implementation |
| AC14 | The scanner refuses `allow`, `expect` or `warn` of `unsafe_code`, inner or outer, in any tracked Rust file, including a trybuild file | `test_weakening_attribute_is_refused` and probe `unsafe.weakening-attribute` | Refusing the inner form only | guards |
| AC15 | `lint_policy_check.py` requires both. Table at `deny` or absent is refused with a message naming `forbid`. The script removed from the `unsafe` gate is refused even with the table present | probes `lint-policy.unsafe-code-not-forbidden` and `lint-policy.unsafe-script-dropped-beside-the-lint`, and the existing `lint-policy.unsafe-gate-named-only-in-a-comment` updated. `lint_policy_check.py` has no unit test file, so its probes are its tests | Keeping "one of the two", which lets the script go once the lint exists | guards |
| AC16 | Guard registration. The `unsafe` guard's `refuses` text names the tokenizer, the weakening attribute and the five triggers. A level-1 probe per trigger, `unsafe.after-quote-char`, `unsafe.after-escaped-quote-char`, `unsafe.after-raw-backslash`, `unsafe.after-raw-quote`, `unsafe.after-comment-quote`, each with a control. `ci/guard-probe-budget.json` re-recorded, runbook regenerated, `python3 scripts/guard_census.py` reports the unwatched bucket empty | `gate guards` | A tokenizer fix with no probe per trigger | guards |
| AC17 | `bin/ocelli.sh gate --floor` and `gate clippy` pass on the final tree | gates | A forbid that turns a clippy-only target red | Implementation |

## Approach

1. Build on `sprint/s46` after F-229 is integrated, so the scanner and its sixth test are F-229's.
2. Write AC1 to AC5, AC7 and AC8 as tests and record them red on the unchanged scanner (AC6).
3. Replace `strip_noise` with a tokenizer function returning the line numbers of `unsafe` keyword tokens and the line numbers of weakening attributes. Keep `rust_files`, `--staged`, the vendored counting and every message.
4. Add the table line to `Cargo.toml`, run AC11 to AC13 in a scratch copy.
5. Change `lint_policy_check.py` to require both, with probes. Update its docstring's departure paragraph, which no longer describes a departure.
6. Probes, budget, runbook, census, LLD.

Rejected alternatives:

- `#![forbid(unsafe_code)]` in each of the 21 lib and bin roots. It covers 21 of 143 roots, and it writes a line into crate roots that F-209, F-211, F-214, F-223, F-225, F-226, F-227, F-228 and F-229 also write, for a weaker result.
- `deny` in the table, the HLD's literal text. An inner `allow` silences it, and the tree needs no allow today.
- Keeping the regexes with a char literal pattern added. Pass 9 shows five triggers and the regex approach found three, so another pattern is another blind spot.

## Dependencies, write ownership and resources

Declared: F-229, which owns `scripts/unsafe_allowlist_check.py`, its test and `Cargo.toml` dependency lines in flight. **F-232 starts after F-229 is integrated into `sprint/s46`.**

Write set:

```text
Cargo.toml                                       [workspace.lints.rust], one table
scripts/unsafe_allowlist_check.py, scripts/tests/test_unsafe_allowlist_check.py
scripts/lint_policy_check.py
scripts/guards/catalogue.py, ci/guard-probe-budget.json, docs/runbooks/guard-verification.md
docs/lld/guards.md
```

No crate source file is written. That keeps F-232 clear of every in-flight story that writes a crate root. The catalogue, budget and runbook are shared with every story that adds a probe, so F-232 re-records the budget and regenerates the runbook on the tree it is based on, after F-229. **F-233 follows F-232** for the same three files. No GPU, browser or corpus.

## Verification and evidence reuse

```bash
bin/ocelli.sh gate unsafe guards
bin/ocelli.sh gate native wasm clippy
python3 scripts/guard_census.py
```

Final profile: `bin/ocelli.sh gate --sprint`. Evidence is invalidated by any change to the tokenizer, the table, `lint_policy_check.py`'s unsafe branch, or a vendored package.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none, and this story is the rule's enforcement
- Tier A (WebGPU): n/a, a source lint and a script
- Tier B (WebGL2): n/a, a source lint and a script
- Tier C (CPU): n/a, a source lint and a script

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC1 to AC5, AC7, AC8, AC14, the tokenizer over planted snippets | `scripts/tests/test_unsafe_allowlist_check.py` |
| guard probe | AC14 to AC16 in the floor sandbox, AC15 included | `scripts/guards/catalogue.py` |
| integration | AC9, AC13, AC17 through the real gates | `gate unsafe`, `gate native`, `gate wasm`, `gate clippy` |
| recorded | AC6, AC10, AC11, AC12 in scratch copies | handoff |

No `fixture` row is owed. No pixel or geometry arithmetic.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no row for source policy.

## Deviations

None. The table implements HLD 27.1's row at a stronger level, and R5's allow-list is unchanged.

## LLD impact

`docs/lld/guards.md`, the `unsafe_code` paragraphs at lines 1045 and 1104: the substitution becomes both mechanisms, the tokenizer replaces the three regexes, the five triggers and their probes, and the sequence the first story writing an allowed file must follow.

## User-facing documentation impact

No change to `README.md`, `examples/folder-viewer/src/use.html` or `examples/folder-viewer/src/more.html`. None of them describes the unsafe rule or its guard, measured by `grep -n unsafe` over the three files.

## Open questions

None open.

- Q1, rustc enforcement through crate-root attributes or the lint table. Answered: lead decision under the operator's delegation of 2026-11-06. Forbid, the recommended option, through `[workspace.lints.rust]`, decision 1 and facts 2 and 3.
- Q2, an exception for ocelli-core and ocelli-wasm. Answered: lead decision under the operator's delegation of 2026-11-06. None today because the allowed files do not exist, and the sequence for the first one is recorded, decision 2 and fact 1.
- Q3, keep the scanner beside the lint. Answered: lead decision under the operator's delegation of 2026-11-06. Yes, with a tokenizer and both mechanisms required, decisions 3 to 5 and fact 6.

### Lead rulings after review pass 1

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-232-working-pass-1.md`.

- **`unsafe` inside an attribute is reported.** `#[unsafe(no_mangle)]` and its `cfg_attr` forms are `unsafe` code that rustc's lint refuses, so the scanner reports them, and an unclosed `#[` is an error rather than a silent skip.
- **Doctests are code.** The scanner reads the Rust code fences of doc comments as code and reports `unsafe` there, because no lint table reaches a doctest. The documents say exactly which targets the lint reaches and that the scanner covers doctests.
- **A weakening attribute is found with any whitespace or comment between `#` and `[`.**
- **The two `guards-deep` lint-policy probes are corrected here.** Their expected `#[path]` count is the one measured on the tree, because the floor and the sprint gate run them.

### Lead rulings after review pass 2

Made by the sprint lead on 2026-11-07 after `.claude/reviews/F-232-working-pass-2.md`, which measured that rustdoc compiles fences the scanner reads as prose.

- **Fail closed in doc text.** Every line of doc text, meaning `///`, `//!`, `/** */`, `/*! */` and the string of a `#[doc = ...]` or `#![doc = ...]` attribute, is scanned as code for the `unsafe` keyword, except inside a fence that rustdoc would not compile. Prose that only names the word is not a finding, so the scanner reports `unsafe` in doc text only where it is followed by `{`, `fn`, `impl`, `trait` or `extern`, the forms that compile.
- **A fence is non-Rust only by rustdoc's rule.** Backtick and tilde fences are both read. A fence is skipped only when its info string carries a word rustdoc does not recognise as a Rust attribute and no `rust` word. The recognised list is taken from rustdoc's documentation for the pinned toolchain, and the implementer cites the page. An indented block, a block quote or a fence split by a plain comment line needs no special case under this rule, because skipping is the only thing a fence can cause.
- **The unclosed-fence branch is pinned by a test.** One red-first test per recognised tag class, plus tilde, `#[doc]` and indented forms.
- **Prose states the rule.** Every sentence that claims doctest coverage says exactly this rule and nothing wider. The two `#[path]` probe notes say one module landed and the planted module is the third.
