# S11 whole-sprint review, pass 4

**Reviewed**: staged tree `325168bb0feeb5c5bde3c89d91d96d7b48ce624e`, on
`sprint/s11` at HEAD `1974802`. Remediation is uncommitted and staged.
**Delta reviewed**: `git diff 292052e8593c0d44 325168bb0feeb5c5b`, 5 files, of
which `.claude/reviews/S11-sprint-pass-3.md` is 288 of the 303 insertions. The
substantive delta is 15 lines across `bin/ocelli.sh`,
`crates/ocelli-render/src/voi.rs`, `scripts/guards/catalogue.py` and the
runbook it renders.
**Scope**: pass 3's defect and three nitpicks, the five questions asked, a
regression sweep, and a fresh mutation.
**Machine**: working GPU, resolves tier A.
**Result**: **0 defects, 0 smells, 0 nitpicks**

**This pass is CLEAN.** Push it.

---

## Verdict on each pass-3 finding

| Pass 3 | Status |
|--------|--------|
| D1, pass 2 stamped with the tree that fixed it | **Closed**, and the mechanism verified working again |
| N1, "the reason such a gate exists at all" | **Closed.** Now says what D-04 says |
| N2, last copy of the retired formulation in `voi.rs` | **Closed.** Split the way the other three sites split it |
| N3, `--list` does not print one number | **Closed.** Command now pipes through `grep -c`, and I ran it |

---

## The five questions, answered

### 1. Is `sprint_reviews[1].tree` correct, and does staleness detection behave?

**Both yes, and I checked the whole list rather than the one field.**

| Pass | Counts recorded | Tree recorded | Tree that pass reviewed |
|------|-----------------|---------------|-------------------------|
| 1 | 5 / 3 / 6 | `98c2462347bd...` | `98c2462347bd...` |
| 2 | 2 / 2 / 4 | `e8f6f4e343065bbd...` | `e8f6f4e343065bbd...` |
| 3 | 1 / 0 / 3 | `292052e8593c...` | `292052e8593c...` |

All three trees are right and all three count triples match what I reported.

**The staleness line is back**, which is the evidence rather than the claim:

```
$ python3 scripts/sprint_workflow.py close-preflight
  latest sprint review pass 3 reports 1 defects and 0 smells
  latest sprint review tree 292052e8593c is stale for current tree 325168bb0fee
```

That line was absent at pass 3 and is present now, on an unchanged code path,
because the field it reads is no longer lying. The guard that stops a review of
a superseded tree from closing a sprint works again.

### 2. Are the three nitpick fixes true?

**All three, verified by execution rather than by reading.**

**N1**, `scripts/guards/catalogue.py:7994-7996` and the rendered copy at
`docs/runbooks/guard-verification.md:706`, now reads "...marks YES in its GPU
column, **and deviation D-04 is why such a gate is excluded from the floor.**
That column is not in the declared-constant ratchet."

D-04's row in `docs/hld/DEVIATIONS.md:21` is "CI runs no GPU build and no GPU
test." That is exactly a reason for exclusion from the floor and is not a reason
a gate exists, so the sentence now says what the deviation says. It also agrees
with the two other places that state it: `.claude/WORKFLOW.md` ("`gpu` is
excluded from `--floor` under deviation **D-04**") and
`scripts/ci_floor_check.py` ("Deviation D-04 is that CI has no GPU, so nothing
in CI may run them"). The looseness pass 3 recorded is gone rather than
re-argued.

The render moved with it, `1 1` on numstat, and re-rendering from the current
catalogue into a copy reproduces the runbook byte for byte, so the two are in
sync. The copy was restored and the tree re-verified.

**N2**, `crates/ocelli-render/src/voi.rs:7-12`, now reads "**The Rust here
computes no LUT value**: every field of [`VoiParams`] is taken from a
[`LutChain`] that already resolved it. **The WGSL here does evaluate the three
window formulas**, because section 18.4's uniform hands a shader `center`,
`width` and `fn_kind`. What neither half does is make a LUT DECISION, and the
shader is handed no input from which it could re-make one."

Checked against the code, not against the other docs:

- "The Rust here computes no LUT value" is true. `from_chain` is six accessor
  reads, a total match `fn_kind`, and a `u32::from(bool)`. No arithmetic.
- "every field of `VoiParams` is taken from a `LutChain` that already resolved
  it" is true for all eight fields, including `invert`, which comes from
  `LutChain::inverts`, and `fn_kind`, which encodes the `VoiFunction` the chain
  selected.
- "The WGSL here does evaluate the three window formulas" is true,
  `voi.wgsl:56-85`.
- "What neither half does is make a LUT DECISION" is true and is enforced by
  what the uniform omits: no Photometric Interpretation, no window multiplicity,
  no sequence.

**N3**, `bin/ocelli.sh:307-309`. The command is now `cargo test -p
ocelli-render -- --ignored --list | grep -c ': test$'`. I ran it on this tree:

```
18
```

Which matches the count that was deleted, and matches the gate's own run,
3 plus 6 plus 9. "across every binary in the crate" is true of `cargo test -p`.

### 3. Did N2 introduce a fourth wording that disagrees?

**There are four sites and none disagrees.** Here they are side by side, because
the useful answer to this question is the map rather than a yes.

| Site | What it says the shader evaluates | Parameters it cites |
|------|-----------------------------------|---------------------|
| `voi.wgsl:29-32` | the three window functions | `center`, `width`, `fn_kind` |
| `voi.rs:7-12` | the three window formulas | `center`, `width`, `fn_kind` |
| `lib.rs:19-25` | the three window formulas | `center`, `width`, `fn_kind` |
| `pixel-pipeline.md:17-22` | stage 1, all three window functions, stage 3 | `slope`, `intercept`, `center`, `width`, `fn_kind` |

The first three are identical in scope and justification. The fourth is a
superset and is in the LLD, which is the document that owns the full stage
account. A superset is not a contradiction, and all four agree on the load
bearing half: the shader evaluates, the shader does not decide.

**No site is gratuitous**, which is the thing worth saying given how much this
idea has been rewritten. `voi.wgsl` is the code. `voi.rs` exports it. `lib.rs`
describes the crate's fourth module, which was missing entirely until pass 1.
`pixel-pipeline.md` is the LLD whose ownership paragraph was false. And none can
rot independently in a dangerous direction: if the shader's arithmetic changed,
all four would become wrong together and the nine GPU tests would go red first.

I am raising no finding on the duplication. Doing so would be the fourth
correction of a comment on thinning grounds, which is what
`voi.wgsl:97-104` exists to warn against.

### 4. Did anything regress?

**No, and the delta is what you said it was.**

- `git diff 292052e 325168bb` over `*.rs`, `*.wgsl` and `*.toml`, with comments
  and blank lines filtered, is **empty**. The only Rust file in the delta is
  `voi.rs` and the change is its module header.
- `crates/ocelli-pixel/`, `crates/ocelli-cache/` and
  `crates/ocelli-render/shaders/` are **byte-identical to HEAD** across all four
  remediation rounds. No arithmetic has been touched at any point in this review
  loop.
- `cargo test -p ocelli-render -p ocelli-cache -p ocelli-pixel`: all green, 18
  ignored as expected.
- `cargo doc -p ocelli-render --no-deps`: **zero** warnings, holding from pass 3.
- `bin/ocelli.sh gate gpu`: ALL GREEN, 3 plus 6 plus 9.
- `bin/ocelli.sh gate prose guards ci fmt clippy`: ALL GREEN, 5 gates.
- `python3 scripts/guard_census.py`: exit 0, 858 refusals, 0 watched by nothing.
- `.claude/verify-ledger.json` holds `325168bb0feeb5c5bde3c89d91d96d7b48ce624e`
  at `profile=sprint`, 31 gates including `oracle`, `corpus=pass`, and
  `.claude/scratch/S11-run.json` carries the matching fourth verification row.

**A fresh mutation, on a part of the sprint I had not mutated myself.**
`Lru::would_admit`, `budget > 0 && bytes <= budget` reduced to `bytes <=
budget`, which is the exact clause deviation D-24's row was edited during
implementation to add:

```
test lru::tests::a_zero_budget_cache_admits_nothing_and_stays_empty ... FAILED
test result: FAILED. 20 passed; 1 failed
```

Red, on the named test, and nothing else. Reverted, `git write-tree` back to
`325168bb0feeb5c5bde3c89d91d96d7b48ce624e`.

### 5. Is this pass clean?

**Yes. Zero defects, zero smells, zero nitpicks. Push it.**

---

## On the pass count, since you raised it

You were right to continue and right to surface it, and there is a mechanical
detail that supports the judgement.

`/run-sprint` says two things that pull against each other. Its termination rule
is that the run ends when "the review loop hits `--max-review-passes` with
actionable findings and the sprint is marked blocked". Its review-loop section
says "**End the loop on substance, not on a count**". At pass 3 the loop was at
the cap with one actionable finding, so the letter said block and the substance
said continue.

`/microscope` settles it and is the more specific document: "**Repeat until
clean. There is no pass limit and that is deliberate.** ... A cap would either
land known defects or stall the sprint on a judgement the cap cannot make."
Blocking S11 over a mis-stamped field in a gitignored ledger would have been
the cap making exactly the judgement it cannot make.

**Two things worth handing to the operator.**

First, `max_review_passes` is written at `sprint_workflow.py:155` by `init` and
**read by nothing**. `grep -rn "max_review_passes"` finds the one write site and
no reader in `scripts/` or `.claude/commands/`. So the "marked blocked" branch of
the termination rule has no mechanism behind it. Nothing enforced the cap and
nothing detected the overrun. That is not an S11 defect and it is not in this
sprint's diff, but it is a rule that reads as enforced and is not, which is the
class this repository cares about. It belongs in a backlog row rather than in a
sprint review, and creating one is the operator's scheduling decision.

Second, the trajectory is the argument for having continued: 5 defects, then 2,
then 1, then 0, with the findings changing every pass rather than repeating.
`/microscope` names that pattern precisely: "A rising pass count with changing
findings is progress. The same finding surviving three passes is not."

---

## What four passes actually established

Recorded here because a clean pass and a pass that looked at nothing produce
identical reports, and this one is the sign-off.

**The code was never wrong.** Across four passes and eleven findings, not one
required a change to a `.rs` or `.wgsl` file's logic. Every finding was prose, a
comment, a documentation index or a ledger field. `ocelli-pixel`,
`ocelli-cache` and `shaders/voi.wgsl` are byte-identical to the tree the sprint
delivered.

**What was independently verified over the four passes**, not re-read from the
per-story reviews:

- The WGSL LUT chain diffed term by term against `crates/ocelli-pixel/src/lut.rs`
  for `c - 0.5` and `w - 1` in LINEAR and neither in LINEAR_EXACT, `<=` at the
  low end and `>` at the high end in both, SIGMOID without clamps, and stage 3
  as `ymin + ymax - d` rather than `ymax - d`.
- Two mutations, one per sprint half. `let cp = c - 0.5;` to `let cp = c;` took
  five GPU tests red with hand-computed expected values in the messages.
  `would_admit` losing its zero-budget clause took its named test red.
- The measured divergence `0.000030517578` reproduced by execution, and the
  `ocelli-pixel` overshoot finding recomputed independently in `f32` to 297.5 at
  the stated parameters.
- All four cache byte fixtures recomputed. 524,288, 262,144 against 153,600 with
  a 108,544 difference, 314,572,800 against 256 MiB.
- The recorded wasm budget of 16,388 against a measured 16,455, inside the 5 per
  cent tolerance, with no re-baseline.
- The boundary rules: `wasm-bindgen` confined, no pixels across the boundary,
  one device-creating crate, and four `submit(` sites of which none is in a
  frame because there is no frame yet.
- All three stories declaring all three tier rows, and the sprint's four named
  defect classes each checked rather than assumed.
- 12,120 added lines scanned for `unsafe`, `unwrap`, `expect`, `panic!`, `as`
  casts, em-dashes and prose semicolons. Clean.

**What remains true and unclosed, by design rather than by omission**: tier B
has never been exercised by anything in this repository, and both F-037 and
F-041 now say so in the same terms. The oracle still has no Ocelli renderer to
compare against, which is D7 holding. Both are recorded in `AS_BUILT.md` and
neither is a finding.

**Tree state**: unchanged. `git write-tree` returns
`325168bb0feeb5c5bde3c89d91d96d7b48ce624e`. The only working-tree addition is
this report. The runbook copy and the `lru.rs` mutation were both reverted and
the tree re-verified after each.
