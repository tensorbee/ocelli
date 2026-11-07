# F-232 review, pass 3

**Reviewed**: `git diff 612cc362 6d95ce73` on `work/f-232-xai`, worktree
`/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-232`, HEAD
`6d95ce732bc7`, clean at start and end apart from this report. No unstaged
paths reviewed.
**Author / reviewer**: GLM in opencode wrote `6d95ce73` after "Lead rulings
after review pass 2" in `.claude/plans/F-232-design.md`. Claude Opus 5.5
reviewed it as an independent subagent and wrote none of the reviewed
material. The same model family wrote passes 1 and 2, which this pass replays
through the real scanner rather than trusts.
**Result**: 9 defects, 0 smells, 6 nitpicks

## Acceptance challenged

- **Lead ruling 1, fail closed in doc text.** Expectations came from rustdoc
  1.97.1 itself, the pinned `rust-toolchain.toml` channel and `rustc
  --version` in the worktree, on a scratch crate with the package lint table
  at `unsafe_code = "forbid"` and `cargo test --doc`. 122 doc items crossed
  six code bodies with thirteen doc contexts, plus 20 tags in both fence
  characters and a second batch of 32 tags. **Rejected**: rustdoc compiles 50 of
  the 122 items and the scanner passes them, and a 51st case, an escaped
  keyword, is in D6.
- **The five prose followers.** `unsafe extern "C"`, `pub unsafe extern "C"
  fn`, `unsafe impl` and a tab before `impl` are reported. `unsafe` then a
  newline then `{`, `unsafe` then a block comment then `{`, `unsafe` then a
  line comment, `#[unsafe(export_name = "...")]` and `unsafe` then a newline
  then `fn` all compile in a doctest and are not reported when they reach the
  prose rule (D2).
- **Lead ruling 2, rustdoc's info-string rule.** The cited page is the local
  1.97.1 copy of `write-documentation/documentation-tests.html`. The sentence
  `_fence_is_rust` presents as the page's rule is not on it, and rustdoc
  compiles `rust2015`, `rust2018`, `rust2021` and `rust2024` fences, which the
  scanner skips (D1).
- **Lead ruling 2, "no special case" for indented blocks, block quotes and
  split fences.** Holds only for a fence split by a plain `//` line. The
  premise that skipping is the only thing a fence can cause is false, because
  a fence the scanner fails to recognise hands real code to the weaker prose
  rule (D2, D3).
- **Lead ruling 3, one red-first test per tag class plus tilde, `#[doc]` and
  indented forms.** The tilde test stays green with tilde fences removed (D7).
- **Lead ruling 4, prose states the rule and nothing wider.** Rejected for
  five sentences (D8). The two `#[path]` probe notes are now right.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `python3 -m unittest discover -s scripts/tests -p 'test_unsafe*'` | worktree at `6d95ce73` | now | 0, 32 tests |
| 38 mutations of the new logic plus the N1 and N2 replays, each in a fresh `shutil` copy of `scripts/` | scratchpad, deleted after | now | 16 red, 24 survive (D7, N1, N5 below) |
| `scan` per file, `612cc362` scanner against `6d95ce73` scanner | all 382 tracked `.rs` files | now | 0 files differ |
| `python3 scripts/unsafe_allowlist_check.py` | worktree | now | 0, 270 files, 104 and 0 |
| `bin/ocelli.sh gate unsafe` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | 0, ALL GREEN |
| `guard_probe.py --profile deep --only` for `unsafe.third-file`, `unsafe.after-quote-char`, `unsafe.weakening-attribute` and the two lint-policy `#[path]` accept probes | worktree | now | all OK as declared |
| `guard_census.py`, `--check-runbook`, `prose_check.py`, `sync_agent_skills.py --check` | worktree | now | 0, 0, 0, 0 |
| `python3 scripts/lint_policy_check.py` | worktree | now | 0, "2 `#[path]` module(s) followed" |
| rustdoc matrix, 122 items, `cargo test --doc` | scratch crate, edition 2024, rustc 1.97.1, deleted after | now | table below |
| rustdoc tag batch, 32 tags | second scratch crate, same toolchain, deleted after | now | table below |
| four doctests appended to `crates/ocelli-core/src/lib.rs`, real scanner and `cargo test -p ocelli-core --doc --offline` | `git archive 6d95ce73` in the scratchpad with its own `git init`, `CARGO_TARGET_DIR=<wt>/target/review-p3`, both deleted after | now | rustdoc 4 of 4 ok, scanner reports only the `no_run` one at line 42, exit 1 |
| local rustdoc book, `~/.rustup/toolchains/1.97.1-aarch64-apple-darwin/share/doc/rust/html/rustdoc/` | pinned toolchain's own docs | now | quoted in D1 |

Doc contexts against code bodies. Each cell is what rustdoc did and then what
the scanner did. "brace" is a `let` binding of `unsafe { ... }` on one line,
"nl" splits `unsafe` and `{` over two lines, "cmt" puts a block comment
between them, "linecmt" a line comment, "attr" is `#[unsafe(export_name =
"...")]` on an `extern "C" fn`, and "nlfn" is `unsafe` then a newline then
`fn`.

| Context | brace | nl, cmt, linecmt, attr, nlfn |
|---|---|---|
| plain backtick fence | ok, reported | ok, reported |
| indented block after a paragraph | ok, reported | ok, **missed** |
| fence inside `>` | ok, reported | ok, **missed** |
| fence opened on a `- ` or `1. ` list marker line | ok, reported | ok, **missed** |
| fence opened on the `/*!` line | ok, reported | ok, **missed** |
| `#[cfg_attr(all(), doc = "...")]` lines | ok, **missed** | ok, **missed** |
| fence markers written `\x60\x60\x60` in `#[doc]` | ok, reported | ok, **missed** |
| fence opener written `concat!("``", "`")` | ok, reported | ok, **missed** |
| previous item ends inside an unclosed `text` fence | ok, reported | ok, **missed** |
| closing marker indented four spaces inside a raw string | ok, refused (LexError) | ok, refused (LexError) |
| `#[doc = r#"..."#]` | ok, reported | ok, reported |
| `#![doc = "...\n..."]` | ok, reported | ok, reported (attr: LexError) |

Further single cases, all compiled and run by rustdoc unless stated: an
unclosed fence and a fence split by a plain `//` line are reported, a fence
supplied through `macro_rules!` and `#[doc = $s]` is missed, a fence in a file
read through `#[doc = include_str!("../inc.md")]` is missed, and `#[doc]`
holding `\x75nsafe { ... }` is missed.

Tags, with the "nl" body so only a fence reading can report it:

| Tag | rustdoc | scanner |
|---|---|---|
| `no_run`, `edition2021`, `edition-2021`, `standalone_crate`, `{.rust}`, `{class=rust}`, `ignore-wasm32`, `rust,text` | compiled | reported |
| `should_panic`, `should_panic(boom)`, `compile_fail`, `compile_fail,E0133`, `test_harness` | compiled (the test fails for its own reason) | reported |
| `ignore` | ignored | reported |
| `rust2015`, `rust2018`, `rust2021`, `rust2024` | **compiled**, with an "unknown attribute" warning | **skipped** (D1) |
| `rust2020`, `rust1`, `rustx`, `rust_2021`, `rust-2021`, `Rust2021`, `Rust`, `no-run`, `should-panic`, `compile-fail`, `test-harness`, `text`, `wgsl`, `"text"`, `text(rust)`, `E+133`, `rust2021,text` | not collected | skipped |
| `E0133`, `{rust}`, `custom`, `custom,rust` | not collected | reported (fail closed, N4) |

## Defects

### D1, rustdoc compiles `rust` edition tags that the fence rule skips

**Where**: `scripts/unsafe_allowlist_check.py:353` `RUSTDOC_WORDS` and `:396`
`_fence_is_rust`, and its docstring at `:399`.
**What**: a fence tagged `rust2015`, `rust2018`, `rust2021` or `rust2024` is
compiled and run by rustdoc 1.97.1, which only warns "unknown attribute
`rust2021`" with a hint toward `edition2021`. The scanner treats each as an
unknown word with no `rust` word and drops the fence.
**Why it is wrong**: the lead ruling makes rustdoc's rule the rule, and a
compiled doctest holding `unsafe` is outside HLD 27.2 R5's two files. The
docstring says "The rule is rustdoc's own, at RUSTDOC_DOC_PAGE for the pinned
toolchain: unknown attributes are treated as markers for code samples written
in other programming languages". That sentence is not on the 1.97.1 page, and
the page documents neither the edition alias nor `test_harness`, so the
documented list alone cannot be the rule.
**Evidence**: tag batch above. Real tree: a `rust2021` fence holding an
`unsafe` block in `ocelli-core`'s lib ran "ok" under `cargo test -p
ocelli-core --doc`, and the scanner did not report it.
**Scope and regression**: every lib target's doc text. Repair: recognise the
`rust` plus edition forms rustdoc accepts, measured on the pinned toolchain
rather than read from the page, and replace the quoted sentence with what was
measured. Regression: one test per alias class with a body only a fence
reading reports, red on the current `_fence_is_rust`.

### D2, the prose rule misses compiled forms, and four compiled contexts reach only the prose rule

**Where**: `_prose_unsafe` at `:506`, the module docstring at `:40` to `:45`,
and the lead ruling's "needs no special case" sentence, which the code follows.
**What**: an indented code block, a fence inside a block quote and a fence
opened on a list marker line are compiled by rustdoc and are prose to
`_fence_code`, because `_fence_open` only looks at a line beginning with a
fence character. Prose is scanned line by line for `unsafe` followed on the
same line by spaces or tabs and then `{`, `fn`, `impl`, `trait` or `extern`.
So `unsafe` then a newline then `{`, `unsafe` then a comment then `{`,
`#[unsafe(...)]` and `unsafe` then a newline then `fn` in any of those
contexts compile and pass. 20 of the matrix's misses are these three contexts.
**Why it is wrong**: R5 and lead ruling 1. The ruling's own reason, "skipping
is the only thing a fence can cause", is false: an unrecognised fence demotes
code from the full tokenizer to the five-follower rule.
**Evidence**: matrix rows two to four. Real tree: an indented block with
`unsafe` and `{` on consecutive lines in `ocelli-core` ran "ok" and the scanner
passed it.
**Scope and regression**: lib doc text. Two independent repairs, either
sufficient for the contexts measured here: scan prose with Rust trivia between
`unsafe` and its follower, newlines and comments included, and add the
attribute form `unsafe(` after `#[` or `#![`. Or recognise indented blocks,
block quotes and list items as rustdoc's CommonMark parser does. The tree's
seven doc lines naming the word have six followed by a closing backtick and
one by the word `identifier`, so the first repair is expected to add no false positive, to
be confirmed by the same per-file comparison this pass ran. Regression: a test
per context with the "nl" and "attr" bodies.

### D3, a fence opened on the `/*!` line is not recognised

**Where**: `_doc_block_text` at `:564` to `:581`.
**What**: the body handed back by `_block_comment` begins after `/*`, so a
`/*!` comment's first content line starts with `!`. Only a leading `*` is
stripped at `:577`, so "! ```no_run" is prose. The `/**` opener works because
its `*` is stripped. Three consequences, all measured: a compiled `/*!`
doctest with `unsafe` and `{` on two lines passes, a `text` fence opened there
reports its content as a false `unsafe {` hit, and that fence's closing marker
opens an untagged fence which swallows the rest of the file's doc text, so a
later doc line holding one `"` is refused as an unterminated string literal.
**Why it is wrong**: rustdoc reads the fence (pass 2 measured it, this pass
compiled the "nl" body there). The test at
`scripts/tests/test_unsafe_allowlist_check.py:433` says "A fence may open on
the `/*!` line itself, and its tag class decides Rust-ness like any other
opener's". It passes only because its body is `unsafe {`, which the prose rule
reports. It stays green with `_fence_is_rust` returning False for every
info string, and goes red only when the prose rule itself is removed.
**Scope and regression**: any `/*!` doc whose first line opens a fence. Strip
the `!` the way the `*` is stripped, and make the test body one only a fence
reading reports.

### D4, doc text is one document per file, not per item

**Where**: `scan`, the single `doc` list at `:661` consumed at `:763`.
**What**: rustdoc closes an unclosed fence at the end of the item's doc.
The scanner joins every item's doc text in the file, so a `text` fence left
open on one item consumes the next item's opener as its closer, and that
item's compiled fence becomes prose. 5 of the matrix's misses are this case.
**Why it is wrong**: lead ruling 1. The module docstring's "a fence still open
when the doc text ends runs to its end, as markdown closes it" is true of the
file and false of rustdoc's per-item document.
**Evidence**: matrix row "previous item ends inside an unclosed `text`
fence".
**Scope and regression**: any file with two documented items. Flush the
collected doc text at each item boundary, at minimum at every non-doc,
non-attribute token, and keep the plain `//` split joined, which rustdoc joins.
Regression: the two-item case with the "nl" body.

### D5, `#[cfg_attr(..., doc = "...")]` is not doc text, and the comment says it is not

**Where**: `scan`, the comment and condition at `:679` to `:684`, and the two
raw-string branches that copy the condition.
**What**: `attr_names == ["doc"]` excludes the `cfg_attr` payload, and the
string is then skipped as a literal, so a doctest written through `cfg_attr`
is invisible even in the `unsafe {` form, the one shape every other context
still catches.
**Why it is wrong**: rustdoc expands `cfg_attr` before collecting doc text and
ran every `cfg_attr` doctest in the matrix. The comment "a `cfg_attr`
wrapper's payload is not" is false. `#[cfg_attr(feature = "x", doc = ...)]`
is an ordinary idiom, so this is the likeliest of the misses after D1.
**Evidence**: matrix row, all six bodies missed. Real tree: four `cfg_attr`
doc lines in `ocelli-core` ran "ok" and the scanner passed them.
**Scope and regression**: read a `doc = <string>` inside `cfg_attr` at any
depth as doc text, which fails closed for a predicate that is false. A test
with the "brace" body, red today.

### D6, doc attribute values are not decoded as rustdoc decodes them

**Where**: `_plain_doc_lines` at `:583` and its docstring, and `scan` for
non-literal values.
**What**: only `\n` is decoded. `\x60\x60\x60` fence markers are compiled by
rustdoc and are prose to the scanner, `\x75nsafe { ... }` is compiled and
passes even in the brace form, and a `\"` left as written is lexed by the
fence scan as a quote, so a `#![doc]` fence holding an attribute with a string
is refused as an unterminated string literal. `#[doc = include_str!(...)]`,
`#[doc = concat!(...)]` and a `macro_rules!` body's `#[doc = $s]` all supply
doc text the scanner never sees, and rustdoc ran each.
**Why it is wrong**: the docstring says "Other escapes are left as written:
no rule reads them", and the fence scan and fence detection both read them.
The tree has no non-literal doc value and no escape other than `\n` in a doc
string, so the cost of refusing them today is zero.
**Scope and regression**: decode every Rust string escape, line continuation
included, before splitting, and either refuse a doc attribute whose value is
not a string literal or name it in the docstring and LLD as the declared
residue. Tests for `\x75`, `\x60` and one non-literal value.

### D7, tests that pass with the behaviour they name removed

**Where**: `scripts/tests/test_unsafe_allowlist_check.py:404` (tilde),
`:433` (`/*!`), and the suite against the mutations below.
**What**: most new tests use a body of `unsafe {`, which the prose rule
reports wherever the fence logic puts the line, so only a fence wrongly
classified as non-Rust can turn them red. Surviving mutations, each green on
all 32 tests: tilde fences removed from `_fence_open`, the `*` unindent
removed (red in pass 2, masked now), the `rust` override in `_fence_is_rust`,
the `ignore-` prefix, quote stripping and `(...)` stripping in
`_attribute_word`, the error code and `test_harness` and `custom` words, two
backticks opening a fence, a closer of any length, a closer with trailing
text, the `\n` split in `#[doc]`, both raw doc string branches, each of `fn`,
`impl`, `trait` and `extern` as followers, and the LexError line remap in
`_fence_hits`.
**Why it is wrong**: microscope section 2 makes a test that passes both ways
a defect, and lead ruling 3 asked for a red-first test for tilde. Removing
the `rust` override, `ignore-` or parenthesis stripping is fail open on
fences rustdoc compiles (`rust,text`, `ignore-x86_64`, `should_panic(msg)`).
**Scope and regression**: switch the fence tests to a body the prose rule
cannot report, such as `unsafe` and `{` on two lines, and add one test per
surviving mutation that is fail open. The fail-closed survivors may stay
unpinned if the docstring says they are conservative.

### D8, the coverage sentences claim more than the scanner does

**Where**: module docstring `:40` ("Every line of doc text is scanned as
code") and `:44` ("the forms that compile"), the `scan` docstring at `:641` to
`:644` ("reported wherever its fence sits and however the fence is written"),
`scripts/lint_policy_check.py:267` to `:270` and `:1814` to `:1816` ("which is
what covers the doctests", "which covers the doctests"), and
`docs/lld/guards.md:1126` to `:1130`.
**What**: each is falsified by D1 to D6, and "scanned as code" contradicts
the next sentence, which says outside a fence only five followers count.
**Why it is wrong**: lead ruling 4, every sentence claiming doctest coverage
says exactly the rule and nothing wider.
**Scope and regression**: rewrite after D1 to D6 land, stating the rule and
any declared residue, and delete "however the fence is written".

### D9, the `unsafe.third-file` note still describes a regex

**Where**: `scripts/guards/catalogue.py:7561` to `:7564`.
**What**: "It deliberately does NOT test `is_unsafe` or `unsafe` in a doc
comment, which are facts about the current regex rather than about R5." There
is no regex since F-232's tokenizer, and `unsafe` in doc text is now an R5
rule this story added.
**Why it is wrong**: a false sentence beside the probe it explains. It
predates F-232 (F-X009) and this story made it false. Not mirrored into the
runbook, so `--check-runbook` is unaffected.
**Scope and regression**: delete the clause or say the tokenizer's cases are
covered by the unit tests and the `unsafe.after-*` probes.

## Smells

None.

## Nitpicks

- **N1, carried from pass 2.** `line = probe_line` replaced by a no-op still
  leaves every test green. Wrong line numbers after a `#` and `[` on
  different lines, never a miss. **Does not block.**
- **N2, carried N4 from pass 2.** Doc fences in `tests/`, `examples/` and bin
  files are scanned though rustdoc never runs them. Fail closed, zero tree
  effect, and the new docstring no longer gives the lib-only reason, so the
  sentence half is gone. **Does not block.**
- **N3, carried N5 from pass 2.** A level-1 catalogue probe for the attribute
  shape remains a preference, and pass 1's N1 and N2 stand. **Does not
  block.**
- **N4.** `custom`, `custom,rust`, `{rust}` and a bare `E0133` are not
  collected by rustdoc 1.97.1 and are scanned as Rust. Fail closed. The
  `RUSTDOC_WORDS` comment says the error codes come from the release's
  LangString parse, which this pass could not read offline, and measured a
  bare code as not Rust.
- **N5.** Removing either word boundary in `_prose_unsafe` leaves every test
  green. False positive direction only.
- **N6.** A closing marker indented four spaces inside a fence closes it in
  the scanner and not in CommonMark. Every valid Rust shape measured was
  refused as unterminated, never passed, so it is a false refusal only.

The four leftovers named for ruling: N1, N4 and N5 from pass 2 are N1, N2 and
N3 here and do not block. The "current regex" wording blocks as D9.

## Verified clean

- **Pass 2 replayed.** D1's tag classes `no_run`, `should_panic`,
  `compile_fail`, `edition2021`, `standalone_crate` and `{.rust}`, the tilde
  fence, the `#[doc]` spread and the fence split by `//` are reported, and the
  real-tree `no_run` plant is refused at line 42 with exit 1. The block quote,
  indented block and `/*!` rows are reported only in the brace form (D2, D3).
  D2 closed, `git grep '#\[path'` outside `vendor/` gives `produce.rs:40` and
  `held.rs:23` and both notes now say so. S1 closed, the unclosed-fence tail is
  red under mutation. N2 closed by the `////` case. N3 closed by the weakening
  case.
- **False positives.** Zero per-file difference over all 382 tracked `.rs`
  files between `612cc362` and `6d95ce73`, and the tree's seven doc lines
  naming the word are none of them followed by a reported form.
- **Raw doc strings.** `r"..."`, `r#"..."#` and `r##"..."##` in `#![doc]` and
  `#[doc]` are reported at the right file line.
- **Mutations red.** The `edition` prefix, `no_run`, `should_panic`,
  `compile_fail`, `ignore`, `standalone_crate`, brace dropping, the `{`
  follower, the tab skip, the plain `#[doc]` collection, the unclosed tail,
  the prose rule itself, non-Rust fences read as prose, and every fence as
  Rust or as non-Rust.
- **Prose executed.** The catalogue `#[path]` notes, the indented-block test
  comment, the unclosed-fence test comment (red as it says), the `{.rust}`
  comment (compiled, measured), and the LLD and lint-policy sentences against
  D8.
- README, `use.html` and `more.html` carry no `unsafe` sentence.
- Scratch crates, the archive and `target/review-p3` were deleted with
  `shutil`. The archive had its own `git init`.

## Limitations and handoff

- No `gate --floor`, `gate --sprint`, clippy, wasm or GPU job. The diff is
  Python and prose.
- rustdoc's LangString source was not read. The tag rule is measured, which is
  what the lead ruling's contract needs, but the measured list is bounded by
  the 52 tags tried.
- HTML blocks and nested list depth were not exercised.
- Next: GLM fixes D1 to D9, then a fresh pass 4 replays this matrix.
