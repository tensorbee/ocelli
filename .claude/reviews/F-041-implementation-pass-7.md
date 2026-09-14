# F-041 review, pass 7

**Reviewed**: working tree, staged set `git write-tree` =
`936fea73c401697de5d8b6fe38843e87c5cc57ce`, remediation of pass 6.
Independent review. Reviewer did not write the code.
**Machine**: `arm64` Darwin, `Apple M4 Max`, Metal, resolved tier **A**.
**Result**: 1 defect, 0 smells, 1 nitpick

**This pass is NOT clean, and it is one word away from clean.**

Both pass-6 smells are deleted and both deletions are correct. Both nitpicks are
deleted. Nineteen mutations, no regression. Every gate green.

You asked me whether to reproduce the 6.0-to-99.9 range before it shipped. **The
range is right and I re-derived it from scratch. The count beside it is not: the
table has five frames, not four.**

---

## Defects

### D1, "across four frames" is five

**Where**: `crates/ocelli-render/tests/voi_shader.rs:682`.

**What**:

> ... measured across four frames the same quantity ranges from 6.0 to 99.9 per
> cent, so it describes the sampler.

The pass-6 report's table has five rows.

**Why it is wrong**: it is a count in prose beside the thing it counts, which
`CLAUDE.md` names four separate times as this repository's repeating failure,
and it is wrong. It also shipped unverified by its author, which your own
question 2 anticipated.

**Evidence**: I re-measured the whole table from scratch with two fresh seeds and
1,000,000 draws per cell, rather than copying pass 6.

```
How many frames are in that table? 5

  seed=20260915
    c~U(-30000,30000) w~U(1,5000)      n=1000000   82.7%
    c~U(-20000,20000) w~U(1,2)         n=1000000   99.9%
    c~U(-20000,20000) w~U(100,5000)    n=1000000   76.5%
    c~U(-100,100)    w~U(1,2)          n=1000000   95.7%
    c~U(-100,100)    w~U(1,5000)       n=1000000    6.0%

  seed=777
    c~U(-30000,30000) w~U(1,5000)      n=1000000   82.7%
    c~U(-20000,20000) w~U(1,2)         n=1000000   99.9%
    c~U(-20000,20000) w~U(100,5000)    n=1000000   76.5%
    c~U(-100,100)    w~U(1,2)          n=1000000   95.8%
    c~U(-100,100)    w~U(1,5000)       n=1000000    6.0%

min across all runs = 6.0%   max = 99.9%
```

**So: 6.0 and 99.9 are confirmed**, stable across seeds, and the conclusion the
sentence draws from them is sound. Only "four" is wrong, and "five" fixes it.
Dropping the count entirely and writing "measured across several frames" fixes it
better, because then there is no number to go stale when someone adds a sixth.

---

## Smells

None.

---

## Nitpicks

### N1, the colon reads as attributing the 6.0-to-99.9 measurement to `pixel-pipeline.md`, which does not contain it

`voi_shader.rs:680-684`: "`docs/lld/pixel-pipeline.md` records why that shape is
refused here: measured across four frames the same quantity ranges from 6.0 to
99.9 per cent".

`grep -n "6\.0\|99\.9\|frames" docs/lld/pixel-pipeline.md` returns nothing. The
LLD records the **principle**, that a rate moving with the frame is a fact about
the sampler, and it records 28.3 and 50.0 for a different quantity. The
6.0-to-99.9 measurement lives only in this doc comment and in
`.claude/reviews/`. A reader following the pointer will not find the numbers the
colon appears to promise.

The claim before the colon is true, so this is wording rather than a broken
reference, and it is the same shape as the dangling pointer pass 3 had to fix. A
full stop instead of the colon closes it.

---

## Verified clean

### Item 1, both deletions removed what they were meant to and nothing dangles

**S1.** The unframed 83 per cent is gone from the sentence, which now reads "the
F-041 review's fourth pass measured **three legal chains** with `w` near but not
equal to 1". Three is right: pass 4 measured `c = 1536.5`, `c = 23695.754` and
`c = 16495.264`. The claim the sentence supports, "It is NOT the only width at
which the lower operator is observable", now rests on those three counterexamples
and on nothing else, which is what "need no population behind them" says.

**S2.** `src/voi.rs` is one block, lines 97 to 102. The second paragraph is gone
and its one non-duplicated clause, "Constructing one field by field is
constructing a uniform nothing validated", was carried into the survivor. That
clause and "the type cannot stop a caller assembling a uniform field by field"
are adjacent but not redundant: the first is about what the type cannot prevent,
the second about what you get if you do it. No duplication remains anywhere in
the block.

**Nothing dangles from either deletion.** "None of these is reachable through
`VoiParams::from_chain`" still has its antecedents, the `w = 0` two-tone
threshold, the `w = 0` SIGMOID NaN and the negative-width inversion, all three
described immediately above it. The deleted "So the shape of the wrongness
depends on which invalid width arrives" was load bearing for nothing, as you
said. `grep` for orphaned one-word comment or doc lines across both edited files
returns only the three legitimate WGSL code-fence lines.

### Item 2, the no-rate paragraph, clause by clause

| Clause | Verdict |
|---|---|
| "One was, 83 per cent, with no sampling frame" | true, it was there through pass 6 |
| "`docs/lld/pixel-pipeline.md` records why that shape is refused here" | true, the principle is recorded there. The numbers are not, which is N1 |
| "the same quantity ranges from 6.0 to 99.9 per cent" | **independently re-derived, two fresh seeds, 1,000,000 draws per cell. Confirmed** |
| "so it describes the sampler" | sound, and the mechanism holds: the rate is driven by the `w` distribution, since `w'` must be small relative to `c'` for `fl(c' - w'/2)` to drop bits |
| "The counterexamples establish 'not the only width' on their own" | true, three measured chains |
| "across four frames" | **D1** |

### Item 3, no unframed rate or unquantified population claim survives

Every percentage anywhere in the F-041 diff, four occurrences, is inside one of
the two paragraphs whose purpose is to disqualify it:

```
voi_shader.rs   "One was, 83 per cent, with no sampling frame"
voi_shader.rs   "ranges from 6.0 to 99.9 per cent, so it describes the sampler"
pixel-pipeline  "An earlier version of this paragraph said 83 per cent ..."
pixel-pipeline  "... 28.3 per cent and this author measured it at 50.0 per cent"
```

None is offered as a property of the arithmetic. I also swept the diff for
unquantified population adjectives, `exotic`, `common`, `rare`, `typical`,
`usually`, `most`, `often`, `frequently`, `widespread`, `pervasive`, `corner
case`, `population`. Four hits, all legitimate: "the single most commonly
mis-ported detail in DICOM viewers" is transcribed from HLD section 18.2's own
prose, and the other three are the meta-sentences that refuse to quote a rate.
"neither is exotic" is gone from `pixel-pipeline.md`.

### Item 4, no regression, nineteen mutations

Every mutation behaves exactly as in passes 5 and 6.

| Mutation | Floor | GPU |
|---|---|---|
| LINEAR lower `<=` to `<` | green | **RED** `voi_linear_at_width_one...` |
| LINEAR upper `>` to `>=` | green | green, expected, see the LLD finding |
| LINEAR_EXACT lower `<=` to `<` | green | green, expected |
| LINEAR_EXACT upper `>` to `>=` | green | green, expected |
| LINEAR loses `- 0.5` and `- 1.0` | green | **RED**, 4 tests |
| inversion `voi.ymax - d` | green | **RED** |
| `voi.invert` ignored | green | **RED**, 2 tests |
| SIGMOID `-4.0` to `-2.0` | green | **RED**, 2 tests |
| delete LINEAR lower clamp | green | **RED**, 2 tests |
| delete LINEAR upper clamp | green | **RED**, 3 tests |
| stage 1 to `let m = stored;` | green | **RED** |
| WGSL struct, four adjacent-pair swaps | **RED** | not needed |
| `fn_kind` swaps 0 and 1 | **RED** | not needed |
| `from_chain` `ymin`/`ymax` hardcoded | **RED** | not needed |
| `VoiParams` field pairs, three ways | **RED** | not needed |
| in-range bound tightened | green | **RED** |
| drag test narrow centre 40 to 50 | green | **RED** |
| `VOI_WGSL` syntactically broken | green | **RED** |

### The wrap claim

The pass-7 delta adds **no** line over 82 characters, so the rewrap is complete.
Across the whole F-041 diff two long lines remain in tracked prose: the
`crates/ocelli-render/tests/voi_shader.rs::the_shader_agrees_with_ocelli_pixel_over_a_sweep`
doc path you named, at 91, and `docs/lld/feature-availability.md`'s table row at
411. The table row is a markdown cell and this repository's LLD tables routinely
run past 400 characters, so it is not a wrap violation and I would leave both.

### Carried forward and re-checked

The three window formulas against PS3.3 C.11.2.1.2, C.11.2.1.3.2 and
C.11.2.1.3.1. `c - 0.5` and `w - 1` in LINEAR and neither in LINEAR_EXACT, in
separate functions. SIGMOID's `-4`. Inversion `voi.ymin + voi.ymax - d`. Stage
order 1, 2, 3, applied once each. The section 18.3 rows with D-13, the boundary
rows, `255/(1+e)` and the `[16, 235]` reflection to 70.75. The 32-byte layout and
its eight offsets. `from_chain`'s two sequence refusals. No route by which the
shader could double-invert, re-select a window or apply a sequence. Measured
sweep divergence reproduces at `0.000030517578`. Tolerances unchanged at `1e-4`
and `0.001`, the latter still matching `crates/ocelli-pixel/tests/voi.rs:6`.

### Lints, policy and gates

No `as` cast, no `.unwrap()`, no `.expect(`, no `panic!`, no `unreachable!` and
no em-dash added by this delta. Gates green on this tree: `fmt`, `clippy`,
`prose`, `unsafe`, `device`, `nostd`, and `gate gpu` ALL GREEN.

### Tree

`git write-tree` = `936fea73c401697de5d8b6fe38843e87c5cc57ce`, identical to the
starting hash, `git diff --stat` empty. `git status` was checked after every
mutation batch and none leaked. No probe file was left behind. Nothing was
committed and nothing was fixed.

---

**F-041 pass 7: 1 defect, 0 smells, 1 nitpick. NOT clean.**

The defect is the word "four" where the measurement has five frames, and the
nitpick is one colon. The arithmetic, the shader, the tests, the tolerances and
the LLD finding are all correct and measured, and nothing outstanding touches a
pixel or a test. **A pass 8 over those two edits should be the last one.**
