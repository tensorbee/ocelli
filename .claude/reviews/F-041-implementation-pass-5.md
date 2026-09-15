# F-041 review, pass 5

**Reviewed**: working tree, staged set `git write-tree` =
`8f76ecf4f5c324253cde025114184e3b7ac842fe`, remediation of pass 4.
Independent review. Reviewer did not write the code.
**Machine**: `arm64` Darwin, `Apple M4 Max`, Metal, resolved tier **A**.
**Result**: 1 defect, 0 smells, 3 nitpicks

**This pass is NOT clean**, and what blocks is one number in one new sentence.

Pass 4's D1 is fixed and the fix is better than a deletion: the heading now says
what `w = 1` uniquely is, and the paragraph under it states the general case
correctly. All three nitpicks are fixed. Nothing regressed across nineteen
mutations. The new `pixel-pipeline.md` finding section is accurate in every
clause I could measure **except one**, and that one is the statistic carrying its
"population rather than a corner" argument.

---

## Defects

### D1, the 83 per cent in the new LLD finding is the rate of a different phenomenon, and overstates this one by three times

**Where**: `docs/lld/pixel-pipeline.md`, "An open finding against this crate's
own arithmetic, found by F-041", third paragraph.

**What**:

> The F-041 review measured the operators diverging on 83 per cent of randomly
> drawn legal parameter pairs, so this is a population rather than a corner.

The section's own subject, in its opening bold sentence, is "**A legal LINEAR
chain can return a value outside its declared output range**". The "so" clause
attaches 83 per cent to that. A reader takes away that 83 per cent of legal
windows produce an out-of-range pixel.

**Why it is wrong**: 83 per cent is the rate at which the **lower** operator's
two spellings disagree, which is what pass 4 measured and reported. In the code
as shipped the lower comparison is `<=`, so at `x == c' - w'/2` the clamp fires
and the body is never evaluated there. **The lower side never leaves the range at
all.** The number therefore comes from the one phenomenon that cannot cause the
finding the section documents.

Measured over the same sampling frame, `c ~ U(-30000, 30000)`,
`w ~ U(1, 5000)`, range `[0, 255]`, n = 1,500,000 LINEAR parameter pairs:

| Population | Rate |
|---|---|
| lower operator observable, `body(L) != ymin` | 1,241,294, **82.8%** |
| upper operator observable, `body(U) != ymax` | 976,203, **65.1%** |
| either operator observable | 1,272,391, 84.8% |
| **shipped code returns out of range at `x == U`** | 424,432, **28.3%** |

and, separately over 400,000 pairs, the number of cases where the shipped code
returns a value below `ymin` anywhere on the lower side: **0**.

So three things are wrong in one sentence. "The operators" plural is at 82.8 and
65.1 per cent respectively and neither is 83. The figure used is the lower
operator's, which is irrelevant to out-of-range output. And the rate for the
section's actual subject is **28.3 per cent**, so the LLD overstates its own
finding by a factor of about three.

**A fourth, smaller problem**: no sampling frame is given, so the number cannot
be reproduced from the document. `CLAUDE.md` is explicit about this shape, that a
number in prose beside the thing it counts is the failure this repository keeps
having to correct, and that the command which prints it is what carries the
weight.

**The same number is correct in the other file.**
`crates/ocelli-render/tests/voi_shader.rs:682` says "83 per cent of randomly
drawn legal parameter pairs make **the operator** observable", singular, in a
paragraph about the lower operator. That is 82.8 per cent and it is right. Only
the `pixel-pipeline.md` use transfers it to a different question.

**The fix is small**: cite 28.3 per cent with its sampling frame, or drop the
percentage entirely and keep the worked example, which is exact, reproducible and
already in the section.

**Evidence**: the table above, and

```
shipped code: values below ymin at the f32 just above L: 0
  (0 means the lower side never leaves the range)
```

---

## Smells

None.

---

## Nitpicks

### N1, the "not reachable through `from_chain`" claim now appears twice, back to back

`crates/ocelli-render/src/voi.rs`, the `VoiParams` hazard block. The paragraph
added by this pass ends:

> None of these is reachable through [`VoiParams::from_chain`], because
> `VoiTransform::new` refuses every width outside its function's domain before a
> chain exists.

and the pre-existing paragraph immediately after it begins:

> [`VoiParams::from_chain`] is the only route that cannot reach any of this,
> because `VoiTransform::new` validated the width before the chain existed, and
> it is the route every test and every caller uses.

Same claim, same reason, adjacent. This is the duplicated-block shape
`.claude/commands/microscope.md` warns about, arrived at by insertion rather than
by re-application. Both sentences are true. One of them should go.

### N2, the superseded "plausible 127.5" clause is still standing, immediately ahead of its own correction

Same block, four lines earlier: "a negative width under SIGMOID produces a
plausible 127.5, so the shape of the wrongness depends on which invalid width
arrives." The new paragraph then gives the full and better account. The weaker
sentence was not removed, so the reader meets the understatement first. The
guidance is to prefer deleting a wrong sentence to explaining it, and pass 5 did
the explaining without the deleting.

### N3, "reproduces `LutChain::apply` to two `f32` ULP" is weaker than the truth and broader than the measurement

`docs/lld/pixel-pipeline.md`, the finding section. Two `f32` ULP is the maximum
divergence over the sweep's 4096 inputs at one window, not a property of
`LutChain::apply`. At the parameters the section itself cites the two sides are
**bit for bit identical**, both `0x4394c001`, which is a stronger and simpler
statement of "both sides overshoot together" and needs no tolerance at all.

---

## Verified clean

### Item 1, no universal or observability claim survives, headings included

I did not grep phrasings this time. I enumerated **every occurrence of the word
"only"** in the four F-041 source files, twenty hits, and read each in context.
Nineteen are about tier B's fragment-only rule, `from_chain` being the only
validated route, `ymax - y` being correct only when `ymin` is zero, the text
check being the only WGSL check on the floor, and similar. All correct. The
twentieth is the new corrected sentence at `voi_shader.rs:684`, "because
`w' = 0` only there", which is true of an `f32` width.

The new heading is "**A window of width one, where `<` instead of `<=` returns
NaN**", which is a statement about one width and not about all widths. The
paragraph under it is correct in every clause: it is not the only observable
width, the earlier heading said "the only input", the mechanism is
`fl(c' - w'/2) - c'` not being exactly `-w'/2`, the lower-operator rate is 83 per
cent, and what `w = 1` uniquely is is the NaN.

### Item 2, the LLD finding section, clause by clause on the adapter

Every measurable clause reproduces at the exact parameters the section gives.

| Clause | Measured |
|---|---|
| "centre `1024.5`, width `1.0003662109375`, range `[0, 255]`" accepted by `VoiTransform::new` | **ACCEPTED**, no refusal |
| "`c' = 1024`, `w' = 0.00036621094`" | `cp=1024 wp=0.00036621094` |
| "the upper breakpoint `fl(c' + w'/2)` rounds to `1024.0002`" | `upper=1024.0002` |
| "The body evaluated there is **297.50003** against a declared `ymax` of 255" | `gpu=297.50003` |
| "an overshoot of 42.5" | `42.50003` |
| "`(fl(c' + w'/2) - c') / w'` is not exactly `0.5`" | `0.6666667` |
| "The comparison `x > c' + w'/2` is false at that input" | confirmed, the body ran |
| "at one input per window" | **correct**, and non-obvious. The lower comparison is `<=`, so at `x == L` the clamp fires and the body is never evaluated there, and for the first `f32` above `L` the quotient is already above `-0.5`. Measured: 0 out-of-range values on the lower side in 400,000 pairs. Only the upper breakpoint can escape |
| "on the CPU and the GPU identically", "both sides overshoot together" | **bit for bit**, `gpu bits 0x4394c001`, `cpu bits 0x4394c001` |
| "evaluating the comparison in a wider type" would fix it | true. In `f64` the breakpoint is `1024.0001831054688` and `x > breakpoint` is true, so the clamp fires and `ymax` is returned |
| "It is not fixed here and no tolerance was widened to hide it" | true. `SWEEP_TOLERANCE` is still `1e-4`, `FIXTURE_TOLERANCE` still `0.001`, and `0.001` still matches `crates/ocelli-pixel/tests/voi.rs:6` |
| "F-041 did not touch it" | true, `crates/ocelli-pixel/src/lut.rs`'s diff is four accessors and no arithmetic |
| the 83 per cent | **D1** |

### Item 3, the negative-width paragraph and the two repaired fragments

The paragraph is right, and stronger than it claims. Measured, soft-tissue chain
with `width` forced to `-400`, inputs `-460, -60, 39, 40, 41, 140, 540`:

```
SIGMOID  w=+400: [1.706677, 68.580055, 126.8625, 127.5, 128.1375, 186.41994, 253.29333]
SIGMOID  w=-400: [253.29333, 186.41994, 128.1375, 127.5, 126.8625, 68.580055, 1.706677]
sums (should all be 255): [255.00002, 255.0, 255.0, 255.0, 255.0, 255.0, 255.00002]
any NaN in the flipped frame? false
```

"253.3 at one end, 127.5 at the centre, 1.7 at the other" is exact. "No NaN, no
clamp and no discontinuity" is confirmed. "A photographic negative of the correct
one" is not loose language, it is arithmetically exact: the pairwise sums are
`ymin + ymax` to within 2e-5, because
`255/(1+e^-u) + 255/(1+e^u)` is identically 255, so negating the width computes
precisely the inversion PS3.3 C.11.6 defines.

Both repaired fragments are correct. `// NOTHING` is reflowed into "Asking for
the derived bind group layout confirms NOTHING further", and `/// modality` into
"Hand-computed: stored 2106, modality `2106 * 2 - 1024 = 3188`". No orphaned
comment or doc fragment remains in either file.

### Item 4, no regression, nineteen mutations

| Mutation | Floor | GPU |
|---|---|---|
| LINEAR lower `<=` to `<` | green | **RED** `voi_linear_at_width_one...` |
| LINEAR upper `>` to `>=` | green | green, expected, see the LLD finding |
| LINEAR_EXACT lower `<=` to `<` | green | green, expected |
| LINEAR_EXACT upper `>` to `>=` | green | green, expected |
| LINEAR loses `- 0.5` and `- 1.0` | green | **RED**, 4 tests including the renamed one |
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
| in-range bound tightened | green | **RED**, the assertion is still live |
| drag test narrow centre 40 to 50 | green | **RED** |
| `VOI_WGSL` syntactically broken | green | **RED**, including the render-pipeline test |

The renamed test still carries its coverage: mutation E names
`the_voi_boundary_values_and_the_two_upper_bounds_differ_on_the_gpu`.

### Carried forward and re-checked

The three window formulas against PS3.3 C.11.2.1.2, C.11.2.1.3.2 and
C.11.2.1.3.1. `c - 0.5` and `w - 1` in LINEAR and neither in LINEAR_EXACT, in
separate functions. SIGMOID's `-4`. Inversion `voi.ymin + voi.ymax - d`. Stage
order 1, 2, 3, applied once each. The section 18.3 rows with D-13, the boundary
rows, `255/(1+e)` and the `[16, 235]` reflection to 70.75. The 32-byte layout and
its eight offsets. `from_chain`'s two sequence refusals. No route by which the
shader could double-invert, re-select a window or apply a sequence. Measured
sweep divergence reproduces at `0.000030517578`.

### Lints, policy and gates

No `as` cast, no `.unwrap()`, no `.expect(`, no `panic!`, no `unreachable!` and
no em-dash added by this delta. Gates green on this tree: `fmt`, `clippy`,
`prose`, `unsafe`, `device`, `nostd`, `deviations`, `backlog`, and `gate gpu`
ALL GREEN.

### Tree

`git write-tree` = `8f76ecf4f5c324253cde025114184e3b7ac842fe`, identical to the
starting hash, `git diff --stat` empty. `git status` was checked after every
mutation batch and none leaked. The temporary probe file is deleted. Nothing was
committed and nothing was fixed.

---

## On the backlog row

**Your call is right and I would not change it.** Creating an F-ID moves the
backlog's identity and the sprint plan's arithmetic, both of which the `backlog`
gate checks, and neither is a reviewer's or an implementer's decision to take
inside a story. Recording the finding in the LLD with the numbers to reproduce it
and surfacing it to the operator in the closing report is the correct shape. Fix
D1's statistic and that section becomes a complete, checkable handover.

---

**F-041 pass 5: 1 defect, 0 smells, 3 nitpicks. NOT clean.**

The defect is one percentage in one sentence of a section that is otherwise
exact, and the correct number, 28.3 per cent, is in this report with its sampling
frame. The three nitpicks are two redundant sentences and one understatement, and
none blocks. A pass 6 over that should be very short.
