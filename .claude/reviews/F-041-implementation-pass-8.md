# F-041 review, pass 8

**Reviewed**: working tree, staged set `git write-tree` =
`0bfe8414cf66f72eacf981a388309c8a8d38ed91`, remediation of pass 7.
Independent review. Reviewer did not write the code.
**Machine**: `arm64` Darwin, `Apple M4 Max`, Metal, resolved tier **A**.
**Result**: 0 defects, 0 smells, 0 nitpicks

# THIS PASS IS CLEAN.

Both scoped edits landed, both took the better of the two options I offered, and
nothing else moved. Nineteen mutations behave exactly as in passes 5, 6 and 7.
`gate --floor` is ALL GREEN over 26 gates and `gate gpu` is ALL GREEN. The tree
is byte-identical going out to what it was coming in.

**I have no other open concern about this story.**

---

## Defects

None.

## Smells

None.

## Nitpicks

None.

---

## Verified clean

### The two edits, and only the two edits

`git diff 936fea73 0bfe8414 -- crates/ docs/` is one hunk of nine lines in
`crates/ocelli-render/tests/voi_shader.rs`. No other tracked file moved.

**Pass 7's D1, the count, is dropped rather than corrected.** "across four
frames" became "over several frames". That is the option I recommended and the
stronger one: a corrected "five" would have been right today and stale the first
time anyone adds a sixth frame, which is the failure mode `CLAUDE.md` names. The
sentence now carries the two endpoints, which are measurements, and no
cardinality, which was bookkeeping.

**Pass 7's N1, the colon, is a full stop.** The paragraph now reads:

> **No rate is quoted for how common that is.** One was, 83 per cent, with no
> sampling frame. `docs/lld/pixel-pipeline.md` records why a rate without its
> frame is refused here. Measured over several frames, the same quantity ranges
> from 6.0 to 99.9 per cent, so it describes the sampler rather than the code.
> The counterexamples establish "not the only width" on their own and need no
> population behind them.

Every clause checks out.

| Clause | Verdict |
|---|---|
| "One was, 83 per cent, with no sampling frame" | true, it was there through pass 6 |
| "`docs/lld/pixel-pipeline.md` records why a rate without its frame is refused here" | **true and now correctly scoped.** `pixel-pipeline.md:280-281` says "quoting one without its frame is the shape `CLAUDE.md` names as this repository's repeating failure". That is exactly the principle being credited, and no measurement is attributed to it |
| the LLD is not credited with the numbers | confirmed. `grep -n "6\.0\|99\.9\|frames" docs/lld/pixel-pipeline.md` returns nothing, and the sentence no longer promises otherwise |
| "ranges from 6.0 to 99.9 per cent" | re-derived from scratch in pass 7, two fresh seeds, 1,000,000 draws per cell, `min = 6.0%`, `max = 99.9%` |
| "so it describes the sampler rather than the code" | sound, and the added "rather than the code" is a sharper statement of the point than the version it replaces |
| "The counterexamples establish 'not the only width' on their own" | true, three measured legal chains from pass 4 |

The edit introduces no new line over 82 characters and no orphaned doc fragment.
A sweep of the four F-041 source files for one-word comment or doc lines returns
only the legitimate WGSL code-fence lines.

### No regression, nineteen mutations

Every mutation is identical to passes 5, 6 and 7. Each was reverted by its own
handler and `git status` was checked after every batch.

| Mutation | Floor | GPU |
|---|---|---|
| LINEAR lower `<=` to `<` | green | **RED** `voi_linear_at_width_one...` |
| LINEAR upper `>` to `>=` | green | green, expected, recorded in the LLD finding |
| LINEAR_EXACT lower `<=` to `<` | green | green, expected |
| LINEAR_EXACT upper `>` to `>=` | green | green, expected |
| LINEAR loses `- 0.5` and `- 1.0` | green | **RED**, 4 tests |
| inversion `voi.ymax - d` | green | **RED** |
| `voi.invert` ignored | green | **RED**, 2 tests |
| SIGMOID `-4.0` to `-2.0` | green | **RED**, 2 tests |
| delete LINEAR lower clamp | green | **RED**, 2 tests |
| delete LINEAR upper clamp | green | **RED**, 3 tests |
| stage 1 to `let m = stored;` | green | **RED** |
| in-range bound tightened | green | **RED** |
| WGSL struct `ymin`/`ymax` swapped | **RED** | not needed |
| WGSL struct `slope`/`intercept` swapped | **RED** | not needed |
| WGSL struct `center`/`width` swapped | **RED** | not needed |
| WGSL struct `fn_kind`/`invert` swapped | **RED** | not needed |
| `fn_kind` swaps 0 and 1 | **RED** | not needed |
| `from_chain` `ymin`/`ymax` hardcoded | **RED** | not needed |
| `VoiParams` field pairs, three ways | **RED** | not needed |
| drag test narrow centre 40 to 50 | green | **RED** |
| `VOI_WGSL` syntactically broken | green | **RED** |

### The whole diff, final sweep

`crates/ocelli-pixel/src/lut.rs` is **byte-identical** to the version reviewed in
pass 1, confirmed by an empty diff between the two trees, and
`cargo test -p ocelli-pixel` is green across all eight targets. The four
accessors are `const fn` matches returning held state and add no arithmetic.

Across the entire `git diff d33ed44`: no `unsafe`, no `.unwrap()`, no
`.expect(`, no `panic!`, no `as` cast of any integer or float type, no em-dash
and no en-dash. The only occurrences of the word `unsafe` are two lines of
`Cargo.toml` comment explaining why `bytemuck` is used instead of a transmute.

### The arithmetic, one last time

Re-checked against the specification rather than against the comments above it.
The three window functions transcribe PS3.3 C.11.2.1.2, C.11.2.1.3.2 and
C.11.2.1.3.1 exactly. `c - 0.5` and `w - 1` in LINEAR, neither in LINEAR_EXACT,
in two separate functions with no shared branch. Lower bound `<=`, upper bound
`>`, in both. SIGMOID's constant is `-4`. Inversion is
`voi.ymin + voi.ymax - d`, PS3.3 C.11.6, not `ymax - d`. Stages 1, 2 and 3 apply
in order and once each.

The section 18.3 fixture rows and D-13's correction of row one, the eight
boundary rows, SIGMOID at `255/(1+e)`, the `[16, 235]` reflection to 70.75 and
the width-one threshold rows were all recomputed by hand as exact rationals in
earlier passes and none has moved. Measured sweep divergence reproduces at
`0.000030517578`, two `f32` ULP at 255, against an asserted `1e-4`.
`SWEEP_TOLERANCE` is `1e-4` and `FIXTURE_TOLERANCE` is `0.001`, the latter still
identical to `crates/ocelli-pixel/tests/voi.rs:6`.

The central design claim holds. The uniform carries no photometric
interpretation, no window multiplicity, no sequence and no presentation
evidence, so the shader has nothing from which to re-decide anything, and I
found no path to a double inversion, a re-selected window or an applied
sequence.

### Gates

```
bin/ocelli.sh gate --floor     ALL GREEN  26 gate(s)
bin/ocelli.sh gate gpu         ALL GREEN  9 shader tests, 9 device tests, the lib's
cargo test -p ocelli-pixel     green, 8 targets
```

### Tree

`git write-tree` = `0bfe8414cf66f72eacf981a388309c8a8d38ed91`, identical to the
starting hash, `git diff --stat` empty so the working tree matches the index.
Every mutation reverted, no probe file left behind, nothing committed and
nothing fixed.

---

## The eight passes, for the record

| Pass | Defects | Smells | Nitpicks |
|---|---|---|---|
| 1 | 5 | 4 | 4 |
| 2 | 3 | 2 | 5 |
| 3 | 1 | 1 | 5 |
| 4 | 1 | 0 | 3 |
| 5 | 1 | 0 | 3 |
| 6 | 0 | 2 | 4 |
| 7 | 1 | 0 | 1 |
| **8** | **0** | **0** | **0** |

The findings that mattered, in order of what they would have cost:

- **Pass 1 D1**, the LINEAR `<=` at a width of one returning NaN, and the false
  continuity argument that had been used to justify not testing it. Four
  successive versions of that argument were falsified before it was deleted.
- **Pass 1 D2**, the shader's Modality stage executed by no test.
- **Pass 1 D3**, the only floor-runnable WGSL layout guard defeated by the
  file's own prose header.
- **Pass 4 D1**, the mirror-image claim about the lower operator, which three
  passes had taken on trust while attacking the other one.

Everything after pass 4 was prose accuracy, and the last four passes turned up
no arithmetic defect at all.

### Two things to carry into completion, neither blocking

1. **The plan deviations live only in `.claude/scratch/F-041-progress.md`**,
   which `.gitignore` excludes. Four of them: no entry point in the shader, a
   tier-A compute harness, `ocelli-core` as a dev dependency, and `bytemuck`
   activated for the first time. They need to reach `AS_BUILT.md` at
   `/complete-feature` or they leave no tracked record.
2. **The open finding against `ocelli-pixel`'s own arithmetic** is recorded in
   `docs/lld/pixel-pipeline.md` with the parameters to reproduce it, and the
   decision not to create an F-ID inside this story is right. It needs the
   operator's scheduling decision, which is what your closing report is for.

---

**F-041 pass 8: 0 defects, 0 smells, 0 nitpicks. CLEAN.**

Proceed to `/verify` and `/complete-feature`.
