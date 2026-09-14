# F-041 review, pass 6

**Reviewed**: working tree, staged set `git write-tree` =
`fdd5c7018b055178fd69b24f6ac28dbe9b4b60e4`, remediation of pass 5.
Independent review. Reviewer did not write the code.
**Machine**: `arm64` Darwin, `Apple M4 Max`, Metal, resolved tier **A**.
**Result**: 0 defects, 2 smells, 4 nitpicks

**This pass is NOT clean.**

Pass 5's D1 is fixed, and the way it was fixed is the right one: the percentage
was deleted rather than corrected, and **your sampler is correct**, which I
reproduced rather than took on trust. N3 is fixed and exact.

Two things block, and both are the same shape as each other.

- The rule the LLD just wrote down, that a frame-dependent rate must not be
  quoted without its frame, is broken four files away by the one percentage
  still standing in this story, and that one swings from **6.0 to 99.9 per
  cent** with the frame, a wider range than the 28.3-to-50.0 that justified the
  deletion.
- Pass 5's N1, the duplicated `from_chain` paragraph, is reported fixed and is
  not fixed. It is measurably worse: the two paragraphs overlapped on two claims
  before and now overlap on four.

---

## Defects

None.

---

## Smells

### S1, the story states a rule about unframed percentages in one file and breaks it in another

**Where**: `crates/ocelli-render/tests/voi_shader.rs:682`, against
`docs/lld/pixel-pipeline.md:271-283`.

The LLD now says, in bold:

> **NO PERCENTAGE IS GIVEN HERE, DELIBERATELY.** ... A rate that moves by that
> much with the sampling frame is a fact about the sampler, and quoting one
> without its frame is the shape `CLAUDE.md` names as this repository's
> repeating failure.

`voi_shader.rs:682`, in the paragraph about the immediately neighbouring
question, still says:

> ... and 83 per cent of randomly drawn legal parameter pairs make the operator
> observable.

No frame. And that quantity is **more** frame-dependent than the one that was
deleted, not less. Measured, same predicate as pass 4's, `body(L) != ymin`:

| Frame | lower operator observable |
|---|---|
| `c ~ U(-30000, 30000)`, `w ~ U(1, 5000)` | 82.7% |
| `c ~ U(-20000, 20000)`, `w ~ U(1, 2)` | **99.9%** |
| `c ~ U(-20000, 20000)`, `w ~ U(100, 5000)` | 76.5% |
| `c ~ U(-100, 100)`, `w ~ U(1, 2)` | 95.8% |
| `c ~ U(-100, 100)`, `w ~ U(1, 5000)` | **6.0%** |

**6.0 to 99.9 per cent.** The deleted figure moved 28.3 to 50.0 and that was
judged enough to delete it. This one moves across almost the whole range, and
a reader has no way to know which end they are being handed.

It is a smell rather than a defect because the number is not false under the
frame I measured it in, 82.7 against a stated 83. It blocks because it will
become a defect the first time anyone re-measures it, and because a repository
that writes a rule and violates it in the same story has written nothing.

**The sentence does not need the number.** Its job is to support "It is NOT the
only width at which the lower operator is observable", and the three measured
counterexamples in pass 4 do that completely. Deleting the clause is the
cheapest fix and is the one the LLD's own paragraph argues for.

### S2, pass 5's N1 is reported fixed and is not fixed, and the duplication is now twice the size

**Where**: `crates/ocelli-render/src/voi.rs:98-102` and `104-107`.

The handover says "The duplicated 'not reachable through `from_chain`' is one
block now." It is two blocks, consecutively, and the first was expanded until it
duplicates every claim in the second:

| Claim | lines 98-102 | lines 104-107 |
|---|---|---|
| `from_chain` is the safe route | "None of these is reachable through `VoiParams::from_chain`" | "`VoiParams::from_chain` is the only route that cannot reach any of this" |
| why | "because `VoiTransform::new` refuses every width outside its function's domain before a chain exists" | "because `VoiTransform::new` validated the width before the chain existed" |
| everyone uses it | "the constructor that reads a validated chain is the one every test and every caller uses" | "and it is the route every test and every caller uses" |
| field-by-field is unvalidated | "the type cannot stop a caller assembling a uniform field by field" | "Constructing one field by field is constructing a uniform nothing validated" |

Before this pass the two paragraphs overlapped on two of these. They now overlap
on four. The first paragraph also declares "That is the whole of the mitigation",
after which the second adds nothing at all.

Both copies are true, so this is a smell and not a defect. It blocks for the
reason the microscope gives for duplicated comment blocks: nothing catches them,
and the next edit to one leaves the other stale. **This review has already hit
that exact failure once**, when pass 2's D1 lived in two files and pass 3 fixed
one of them.

---

## Nitpicks

### N1, `pixel-pipeline.md:288` is 92 characters in a file that wraps at 79

"sides overshoot together and by the same amount. The arithmetic is this crate's,
from F-018." This is the third consecutive pass in which a remediation edit
leaves one ragged line behind.

### N2, `src/voi.rs:87-88` is left ragged by the deletion

```
/// `x == c`. So the shape of the wrongness depends on which
/// invalid width arrives.
```

A short line and a two-word continuation, the same class as the `// NOTHING` and
`/// modality` fragments from passes 3 and 4.

### N3, the NaN-width fact was deleted rather than folded, and the connective lost an antecedent

The handover says the NaN-width clause "is folded into one sentence ahead of the
negative-width paragraph". It is not anywhere: `grep -n NaN` over the four F-041
source files finds no statement that a NaN width produces NaN. Losing it is fine,
it was the least interesting of the three cases. What is left behind is the
connective: "So the shape of the wrongness depends on which invalid width
arrives" now follows two sentences that vary the **function** at `w = 0`, not the
width, and is completed only by the paragraph after it. It reads as a
non-sequitur where it stands.

### N4, "neither is exotic" is now the only unquantified population claim in a section that just purged its population claims

`docs/lld/pixel-pipeline.md`, second paragraph: "Both are accepted by
`VoiTransform::new` and neither is exotic." A width of `1.0003662109375` is
legal, which the sentence before it already says, but it is not a width any
clinical object carries, and "exotic" is doing the rhetorical work the deleted
percentage used to do. The section is stronger without the clause: the worked
example is exact and reproduces, which is what the section itself says its claim
is.

Related, and not worth its own row: the paragraph gives 28.3 and 50.0 per cent
without their frames while arguing that frames matter. It disclaims both numbers
in the same breath, so it is self-consistent. For the record the frames are
`c ~ U(-30000, 30000)`, `w ~ U(1, 5000)` for 28.3 and
`c ~ U(-20000, 20000)`, `w ~ U(1, 2)` for 50.0, both n = 1,500,000.

---

## Verified clean

### Item 1, the finding section is true in every clause, and the deletion left nothing dangling

The old sentence ended "so this is a population rather than a corner". That
inference went with the percentage. `grep -rn -i -e "population" -e "rather than
a corner"` over `crates/ocelli-render` and `docs/lld/pixel-pipeline.md` finds
only F-037's unrelated use of "population" for device loss in `caps.rs`,
`gpu.rs` and `probe.rs`. Nothing now rests on the deleted figure.

Every clause of the section, measured on the adapter at the parameters it gives:

| Clause | Measured |
|---|---|
| "at one input per window, on the CPU and the GPU identically" | true. 0 out-of-range values on the lower side in 400,000 pairs |
| centre `1024.5`, width `1.0003662109375` accepted by `VoiTransform::new` | ACCEPTED |
| `c' = 1024`, `w' = 0.00036621094`, upper rounds to `1024.0002` | exact |
| body is **297.50003**, overshoot 42.5 | exact |
| "At the parameters above the quotient is `0.6666667`" | exact. New this pass and correct |
| "**Only the UPPER breakpoint can escape**" with the `<=` / `>` reason | true, and the reason given is sufficient for breakpoints. The stronger statement also holds: for the first `f32` above `lower` the quotient is already above `-0.5`, so the lower side cannot escape anywhere, which I proved in pass 5 and re-confirmed here |
| "the lower side never leaves the range" | 0 of 400,000 |
| "to within two `f32` ULP over F-041's sweep" | sweep maximum reproduces at `0.000030517578`, which is two ULP at 255 |
| "at these particular parameters the two agree bit for bit, `0x4394c001` on both" | exact, measured in pass 5 |
| "no tolerance was widened" | `SWEEP_TOLERANCE` still `1e-4`, `FIXTURE_TOLERANCE` still `0.001`, still matching `ocelli-pixel/tests/voi.rs:6` |
| "F-041 did not touch it" | `crates/ocelli-pixel/src/lut.rs`'s diff is four accessors and no arithmetic |

### Item 2, your sampler is correct and I reproduced it

Not taken on trust. I implemented the predicate independently, ran your frame and
mine, and varied the frame to find out which parameter drives the spread.

```
PREDICATE: shipped code returns a value outside [0,255] at x == upper

  author's frame   c~U(-20000,20000) w~U(1,2)        n=1500000  out of range  50.0%   worst  127.376
  reviewer's frame c~U(-30000,30000) w~U(1,5000)     n=1500000  out of range  28.3%   worst   13.031
  author's frame, different seed                     n=1500000  out of range  49.9%   worst  127.376
  c~U(-2000,2000)  w~U(1,2)                          n= 400000  out of range  49.8%   worst  119.539
  c~U(-100,100)    w~U(1,2)                          n= 400000  out of range  46.9%   worst   57.955
  c~U(-20000,20000) w~U(1,100)                       n= 400000  out of range  48.4%   worst   88.973
  c~U(-20000,20000) w~U(100,5000)                    n= 400000  out of range  22.3%   worst    0.002
```

**Your 50.0 per cent reproduces exactly and is stable across seeds.** Both
numbers measure the same predicate. The spread is driven almost entirely by the
`w` distribution and barely at all by `c`: at `w ~ U(1, 2)` the rate is about 50
per cent whether `c` spans 200 or 40,000, and at `w ~ U(100, 5000)` it falls to
22 per cent. The reason is mechanical. The phenomenon needs `w'` small enough
relative to `c'` that `fl(c' + w'/2)` drops bits, and once it does the sign of
the perturbation is close to a coin flip, which is the 50 per cent. Wider `w`
draws include many where nothing is lost.

**The worst overshoot moves with the frame too**, 0.002 to 127.4, so both the
rate and the magnitude are facts about the sampler. That is a stronger version of
the section's own argument than the section makes, and it is why deleting the
number was right.

### Item 3, the three rewrites

| Rewrite | Verdict |
|---|---|
| duplicated `from_chain` block | **Not fixed.** S2 |
| superseded "plausible 127.5" clause | **Fixed.** The clause is gone. The NaN-width clause went with it, see N3 |
| "to within two `f32` ULP over F-041's sweep" plus the bit-for-bit | **Fixed and exact.** The sweep bound reproduces, and `0x4394c001` on both sides is my pass-5 measurement verbatim |
| the 2e-5 addition | **Correct, and conservative.** The pairwise sums are `255.00002` at the extremes, whose exact `f32` value is `255.00001525878906`. The deviation is `1.52587890625e-05`, which is **exactly one ULP at 255** and is inside the stated 2e-5. The claim is true and could be tightened to one ULP |

The "photographic negative" framing is arithmetically exact, as recorded:
`255/(1+e^-u) + 255/(1+e^u)` is identically 255, so negating the width computes
precisely PS3.3 C.11.6's inversion.

### Item 4, no regression, nineteen mutations

Every mutation behaves exactly as in pass 5.

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

### Carried forward and re-checked

The three window formulas against PS3.3 C.11.2.1.2, C.11.2.1.3.2 and
C.11.2.1.3.1. `c - 0.5` and `w - 1` in LINEAR and neither in LINEAR_EXACT, in
separate functions. SIGMOID's `-4`. Inversion `voi.ymin + voi.ymax - d`. Stage
order 1, 2, 3, applied once each. The section 18.3 rows with D-13, the boundary
rows, `255/(1+e)` and the `[16, 235]` reflection to 70.75. The 32-byte layout and
its eight offsets. `from_chain`'s two sequence refusals. No route by which the
shader could double-invert, re-select a window or apply a sequence.

### Lints, policy and gates

No `as` cast, no `.unwrap()`, no `.expect(`, no `panic!`, no `unreachable!` and
no em-dash added by this delta. Gates green on this tree: `fmt`, `clippy`,
`prose`, `unsafe`, `device`, `nostd`, and `gate gpu` ALL GREEN.

### Tree

`git write-tree` = `fdd5c7018b055178fd69b24f6ac28dbe9b4b60e4`, identical to the
starting hash, `git diff --stat` empty. `git status` was checked after every
mutation batch and none leaked. No probe file was left behind. Nothing was
committed and nothing was fixed.

---

**F-041 pass 6: 0 defects, 2 smells, 4 nitpicks. NOT clean.**

Both smells are deletions rather than rewrites. S1 is one clause in
`voi_shader.rs:682`, S2 is one of two adjacent paragraphs in `src/voi.rs`. The
arithmetic, the tests and the LLD finding are all now correct and measured, and
nothing outstanding touches a pixel.
