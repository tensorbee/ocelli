# F-041 review, pass 4

**Reviewed**: working tree, staged set `git write-tree` =
`f96cf2cbbe5ec9d0b2aa930855242daf715ae221`, remediation of pass 3.
Independent review. Reviewer did not write the code.
**Machine**: `arm64` Darwin, `Apple M4 Max`, Metal, resolved tier **A**.
**Result**: 1 defect, 0 smells, 3 nitpicks

**This pass is NOT clean.**

Pass 3's D1 and S1 are both fixed and all five nitpicks are fixed, each verified
by measurement. Nothing regressed: all fifteen mutations are still red.

What blocks is one sentence I had not read closely enough in three earlier
passes. The **heading** of `voi_linear_at_width_one_pins_the_lower_boundary_operator`
still carries a universal claim about operator observability, and it is false.
It is the same shape of claim as the three the shader has already retracted, and
this is the first pass that tested it rather than reading past it. That is my
miss as much as anyone's: passes 1 to 3 attacked the upper operator and took the
lower one's "only" on trust.

---

## Defects

### D1, "a window of width one is the only input that pins the LOWER boundary operator" is false

**Where**: `crates/ocelli-render/tests/voi_shader.rs:676-677`, the heading of
`voi_linear_at_width_one_pins_the_lower_boundary_operator`.

**What**:

> **A window of width one, which is the only input that pins the LOWER
> boundary operator.**

**Why it is wrong**: it is a universal claim over all windows, and legal windows
with `w != 1` pin the lower operator too. Pass 2 falsified the upper operator's
unobservability by showing that `fl(c' + w'/2) - c'` need not equal `w'/2` once
`c'` is large relative to `w'/2`. The identical argument applies to
`fl(c' - w'/2) - c'`, and it is the same construction with the sign flipped. The
lower side has one extra protection the upper side lacks, that `0.0 * range +
ymin` is exactly `ymin` for any range, so it fails only through the quotient and
not through the range round-trip. It still fails.

Measured on the Metal adapter, three legal LINEAR chains, all accepted by
`VoiTransform::new`, baseline against `<=` mutated to `<`, at `x` exactly equal
to `c' - w'/2`:

| c | w | lower breakpoint | `<=` as written | `<` | CPU |
|---|---|---|---|---|---|
| 1536.5 | 1.0003662109375 | 1535.999755859375 | 0.0 | **-42.500004** | 0 |
| 23695.754 | 1.0174913 | 23695.246 | 0.0 | **13.604357** | 0 |
| 16495.264 | 1.0019568 | 16494.762 | 0.0 | **-127.01843** | 0 |

The third is half of full scale below `ymin` on a declared range of `[0, 255]`.
In every case `lower < upper` strictly, so this is not the `w = 1` collapse under
another name, and `VoiTransform::new` printed no refusal.

**Not a corner case.** Over 1,500,000 random legal LINEAR parameter pairs, `c`
in plus or minus 30000 and `w` in `[1, 5000]` with range `[0, 255]`,
**1,240,936 of them, about 83 percent**, give `body(lower) != ymin`. As with the
upper operator the magnitude falls with width and the large divergences
concentrate near `w = 1`, which is why all three measured cases have a width
just above one.

**What is true, and is presumably what the heading meant**: `w = 1` is the only
width at which the divergence is a **NaN**, because `w' = w - 1 = 0` only there
and the division by zero needs `w' == 0` exactly. Everywhere else the divergence
is a finite wrong number, which is worse in this project's terms. The body of the
test and its other four paragraphs are all correct, including the paragraph pass
3 added. Only the heading over-reaches, and deleting one word closes it:
"**A window of width one, which pins the LOWER boundary operator.**"

**Why it is a defect and not a nitpick**: it is the fourth universal statement
about operator observability in this file, the fourth to be falsified by
measurement, and it sits on the heading of the one test whose job is that
operator. It is exactly what pass 4's first question asked me to look for.

**Evidence**:

```
baseline
CONSTRUCTED: c=1536.5 w=1.0003662 wp=0.00036621094 lower=1535.9998 upper=1536.0002 lower<upper=true
CONSTRUCTED: gpu at [lower, upper] = [0.0, 297.50003]   cpu = [0, 297.50003]
SEARCH-2:    c=16495.264 w=1.0019568 lower=16494.762
SEARCH-2:    gpu at [lower, upper] = [0.0, 382.01843]   cpu = [0, 382.01843]

with `if (x <= cp - wp / 2.0)` mutated to `<`
CONSTRUCTED: gpu at [lower, upper] = [-42.500004, 297.50003]   cpu = [0, 297.50003]
SEARCH-1:    gpu at [lower, upper] = [13.604357, 241.39563]    cpu = [0, 241.39563]
SEARCH-2:    gpu at [lower, upper] = [-127.01843, 382.01843]   cpu = [0, 382.01843]
```

The construction, for reproducibility: take `c'` a value whose neighbourhood
grid is uniform, here `1536 = 1.5 * 2^10` with `ulp = 2^-13`, and `w' = 3 * ulp`.
Then `c' - w'/2` is exactly halfway between two representable values, rounds to
even, and `(L - c')/w'` is `-2/3` rather than `-1/2`.

---

## Smells

None.

---

## Nitpicks

### N1, the uncaptured-error-handler rewrite left an orphaned `// NOTHING`

`tests/voi_shader.rs:875-881`. The sentence is now correct, and the rewrap is
not:

```rust
    // here and none is needed for that. Asking for the derived bind group layout confirms
    // NOTHING
    // further, measured in the F-041 review's second pass, so it is not asked
```

A 94-character line followed by a one-word line.

### N2, the `+ ymin` rewrite left an orphaned `/// modality`

`tests/voi_shader.rs:759-762`. "Hand-computed: stored 2106," then `/// modality`
alone, then the next line. The arithmetic is right and the wrap is not.

### N3, "a negative width under SIGMOID produces a plausible 127.5" describes one input, and the truth is worse

`src/voi.rs`, the `VoiParams` hazard paragraph. 127.5 is what it produces at
`x == c`. Across a frame a negative width **mirrors the sigmoid**, so it renders
a complete, smooth, correctly shaped but INVERTED image. Measured, soft-tissue
chain with `width` forced to -400, inputs `-460, -60, 39, 40, 41, 140, 540`:

```
SIGMOID width -400 = [253.29333, 186.41994, 128.1375, 127.5, 126.8625, 68.580055, 1.706677]
SIGMOID width -1   = [255.0, 255.0, 250.41351, 127.5, 4.5864835, 0.0, 0.0]
```

An inverted study is the paragraph's own argument at full strength, and it is
stronger than a single plausible grey value. The rest of the sentence is exact.

---

## Verified clean

### Item 1, no observability claim survives except D1's

`grep -rn -i -e "observab" -e "pin the upper" -e "at every legal width" -e
"exactly .ymax. in .f32" -e "asymmetric comparison"` over `crates` and `docs`
returns, inside F-041's files, only `shaders/voi.wgsl:98`, which is the true
historical sentence "THREE EARLIER VERSIONS OF THIS COMMENT ARGUED ABOUT WHICH
OPERATOR IS OBSERVABLE AND ALL THREE WERE WRONG". The `pixel-pipeline.md:290`
hit is F-030's unrelated use of the word.

I then read every sentence containing "operator" in all eight F-041 files rather
than relying on the phrasings. Ten hits. Nine are statements about coverage or
about the standard and are correct. The tenth is D1.

The pass-3 replacement paragraph at `voi_shader.rs:697-700` is correct in all
four clauses: it pins the lower operator, no test pins the upper one, the name
says `lower` for that reason, and no claim is made about why. The dangling
reference to the shader is gone.

### Item 2, the renamed test's doc is accurate

Both measurable claims hold.

| Claim | Measured |
|---|---|
| "All four operator mutations leave it green" | LINEAR `<=` to `<`: red only on `voi_linear_at_width_one...`. LINEAR `>` to `>=`, LINEAR_EXACT `<=` to `<`, LINEAR_EXACT `>` to `>=`: green everywhere. **True** |
| "it is red when LINEAR loses its `c - 0.5` and `w - 1`" | mutation E fails four tests including `the_voi_boundary_values_and_the_two_upper_bounds_differ_on_the_gpu`. **True** |

The name `the_voi_boundary_values_and_the_two_upper_bounds_differ_on_the_gpu`
matches what the assertions do. The new paragraph "At 239 itself LINEAR does not
clamp: the comparison is `>`, so the body runs and returns `ymax` by continuity.
What differs at 239 is that LINEAR has reached `ymax` and LINEAR_EXACT has not"
is correct for these parameters: `(239 - 39.5) / 399` is exactly 0.5, so the
body gives exactly 255, and LINEAR_EXACT gives `255 * 399 / 400 = 254.3625`.

### Item 3, the `params.ymin..=params.ymax` change did not weaken anything that matters

Two questions, both measured rather than reasoned.

**Is it still evaluated?** Yes. Tightening it to
`(params.ymin..=(params.ymax * 0.4))` turns the sweep **RED**, so it runs on all
4096 values times three functions times two photometric interpretations.

**Can a uniform carrying a WRONG range still be caught, now that the bound is
read from that same uniform?** Yes, and on the floor. Mutating `from_chain` to
multiply `ymax` by four:

```
X2: from_chain inflates ymax by 4
  FLOOR=RED  from_chain_reads_section_18_4s_eight_parameters,
             ymin_and_ymax_come_from_the_voi_stages_declared_range
  GPU  =RED  4 further tests
```

It is narrower than the literal in exactly one respect, that the in-range
assertion itself can no longer independently detect a wrong range. That class has
redundant coverage in two floor tests and four GPU tests, and for the assertion's
own stated purpose, "the output stays inside the **declared** range", reading
from the uniform the shader was given is the more faithful bound. The change is
an improvement, and the comment justifying it is accurate.

### Item 4, the five nitpick rewrites

| Nitpick | Verdict |
|---|---|
| "panics through wgpu's error scope" | **Fixed and correct.** The failure path is wgpu's default uncaptured-error handler, measured in pass 3 as `wgpu error: Validation Error` followed by a panic, and no scope is pushed. Wrap damage is N1 |
| "only SIGMOID gives one" | **Fixed.** The `w = 0` scoping is explicit and correct, NaN width does NaN all three, and none is reachable through `from_chain`. The negative-width clause understates, which is N3 |
| in-range bound reads from the uniform | **Fixed**, see item 3 |
| "LINEAR clamps its upper bound at 239" | **Fixed and correct** |
| long doc lines | **Fixed in substance.** The count in your message is off: seven lines exceed 92 across the four F-041 files, not three. `voi_shader.rs:39, 640, 690` are the three you named, and `voi_params.rs:24, 212, 243, 375` are four more. All seven are rustfmt's own output or the doc-link path, so leaving every one of them is the right call. **Do not shorten the `crates/ocelli-pixel/tests/voi.rs::...` path**: it names a real test and a shortened form stops being a usable reference. Only the count was wrong, and it is in your message rather than in the tree, so it is not a finding |

### Item 5, no regression, fifteen mutations

Every mutation red in an earlier pass is still red, on the same tests or more.

| Mutation | Floor | GPU |
|---|---|---|
| LINEAR lower `<=` to `<` | green | **RED** `voi_linear_at_width_one...` |
| LINEAR loses `- 0.5` and `- 1.0` | green | **RED**, 4 tests |
| inversion `voi.ymax - d` | green | **RED**, the non-zero-range row |
| `voi.invert` ignored | green | **RED**, inversion row and sweep |
| SIGMOID `-4.0` to `-2.0` | green | **RED**, the `255/(1+e)` row and sweep |
| delete LINEAR lower clamp | green | **RED**, sweep and width-one |
| delete LINEAR upper clamp | green | **RED**, 18.3 rows, sweep and width-one |
| stage 1 to `let m = stored;` | green | **RED** `stage_one_applies...` |
| WGSL struct `ymin`/`ymax` swapped | **RED** | not needed |
| WGSL struct `slope`/`intercept` swapped | **RED** | not needed |
| WGSL struct `center`/`width` swapped | **RED** | not needed |
| WGSL struct `fn_kind`/`invert` swapped | **RED** | not needed |
| `fn_kind` swaps 0 and 1 | **RED** | not needed |
| `from_chain` `ymin`/`ymax` hardcoded | **RED** | not needed |
| `VoiParams` field pairs reordered, three ways | **RED** | not needed |
| drag test narrow centre 40 to 50 | green | **RED**, the equality assertion is live |
| `VOI_WGSL` syntactically broken | green | **RED**, including the render-pipeline test |

The renamed test did not lose coverage in the rename: mutation E still names it.

### Carried forward and re-checked

The three window formulas against PS3.3 C.11.2.1.2, C.11.2.1.3.2 and
C.11.2.1.3.1. `c - 0.5` and `w - 1` in LINEAR and neither in LINEAR_EXACT, in
separate functions. SIGMOID's `-4`. Inversion `voi.ymin + voi.ymax - d`. Stage
order 1, 2, 3, applied once each. The section 18.3 rows with D-13, the boundary
rows, `255/(1+e)` and the `[16, 235]` reflection to 70.75. The 32-byte layout and
its eight offsets. `from_chain`'s two sequence refusals. No route by which the
shader could double-invert, re-select a window or apply a sequence. Measured sweep
divergence reproduces at `0.000030517578`, which is two `f32` ULP at 255.

### Lints, policy and gates

No `as` cast, no `.unwrap()`, no `.expect(`, no `panic!`, no `unreachable!` and
no em-dash added by this delta. Gates green on this tree: `fmt`, `clippy`,
`prose` (305 files), `unsafe`, `device`, `nostd`, and `gate gpu` ALL GREEN with
9 shader tests, 9 device tests and the lib's.

### Tree

`git write-tree` = `f96cf2cbbe5ec9d0b2aa930855242daf715ae221`, identical to the
starting hash, `git diff --stat` empty. `git status` was checked after every
mutation batch and none leaked. The temporary probe file is deleted. Nothing was
committed and nothing was fixed.

---

## Observation outside F-041's scope, recorded so it is not lost

This is **not** a finding against F-041 and must not hold the story. It is the
thing three passes of measurement have converged on and it belongs in the
backlog.

At exactly one input per window, `c' - w'/2` and `c' + w'/2`, a legal LINEAR
chain returns a display value **outside its own declared `[ymin, ymax]`**, on the
CPU and the GPU identically, because PS3.3's formula evaluated in `f32` does not
reproduce its own breakpoint. Measured magnitudes so far: 297.50003 and
382.01843 against a declared `ymax` of 255, and -127.01843 against a declared
`ymin` of 0. The clamps are present and do not prevent it, because the clamp
comparison and the body disagree about where the breakpoint is.

`ocelli-pixel` owns this arithmetic, F-018 wrote it, and it is a faithful
transcription of the specification, so F-041 changing it would be the second copy
section 18 forbids. What it deserves is a backlog row against `ocelli-pixel` to
decide whether the stages should clamp their own output to the declared range as
a final step. Until then the sweep's new in-range assertion is the standing
guard, correctly scoped to parameters where the case does not arise.

---

**F-041 pass 4: 1 defect, 0 smells, 3 nitpicks. NOT clean.**

The defect is one word in one heading. The three nitpicks are two bad line wraps
and one understated sentence, and none of them blocks. A pass 5 over that should
be very short.
