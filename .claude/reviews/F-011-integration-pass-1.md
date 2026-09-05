# F-011 review, integration pass 1

**Reviewed**: `2ffabe3`, committed directly to `sprint/s03` in serial mode.
**Reviewer**: the integrator, independent of the implementing agent.
**Result**: 0 defects, 0 smells, 0 nitpicks.

## The check that closes the sprint's central question

The sprint opened with a derivation: `LINEAR(x) - LINEAR_EXACT(x) =
255 * (x + 160) / 159600` peaks at 0.6375 of a display code, so a whole-frame
swap between the two functions can never exceed one code after quantisation,
and HLD 25.1's maximum-difference rule therefore **passes the project's own
headline defect everywhere**. That was derived twice independently, by me and by
the implementing agent, and it is why the operator added a signed-mean bias
bound to 25.1.

**I proved by mutation that the bound is what catches it.** With
`MONOCHROME_SIGNED_MEAN_BIAS` at its recorded 0.1, all twenty catalogue
mutations are detected. Widened to 0.7:

```
plus-one-on-two-fifths-of-the-image   NOT DETECTED
  real__ct_cmb_mml__00000001 is pass and fail was declared
compare: 20 mutations, 1 not detected        exit 1
```

Restored to 0.1, `20 mutations, 0 not detected`, exit 0, and `git diff` on
`tolerance.rs` is clean. So the one mutation that carries the LINEAR against
LINEAR_EXACT signature is failed by the bias bound and by nothing else, which is
the whole reason the bullet was added. The analytic argument and the mechanism
now agree.

## Defects

None.

## Smells

None.

## Verified clean

**Run-level verdict, re-run by me.** `bin/ocelli.sh compare` exit 0:
`98 views: 70 pass, 0 fail, 28 unmeasured, 0 absent`, with `weak: 22`,
`unstated-threshold: 5`, `decimated: 2`, identity over 98 views green, and
`20 mutations, 0 not detected`. The `weak` qualifier reaches volume views,
including both AXIAL reformats at informative fraction zero, which is what
F-X007's low-information finding required.

**Deviation D-13 is honoured.** No fixture anywhere asserts
`LINEAR_EXACT(-160) = 1.594`. `tests/voi_divergence_fixture.rs` carries the
table with `0.000`, quotes the arithmetic
`((-160 - 40) / 400 + 0.5) * 255 = 0.000`, and uses the other three rows of
18.3 unchanged at 127.819, 127.500, 63.910 and 63.750.

**Casts, allows and unwraps.** Zero of each in the new Rust. **My own grep said
otherwise twice and was wrong both times**: every `as` hit is the English word
inside a string or comment, and the single `#[allow]` hit is a doc comment
stating that none is added. `frame.rs` routes every count that becomes a float
through `u32::try_from` and `f64::from`, which is exact over the whole of `u32`,
and a count too large to convert is an error rather than a wrong number.

**The 89 stack digests did not move**, per row against the baseline taken at
`e1bcf53`, and `renderParamsSha256` is unchanged.

**The view list is the union of declared frame lists**, not `rows[]`. This is
the trap F-X007 named: a comparator reading `rows[]` alone would have compared
89 of 98 views and reported success. An undeclared `.raw` in the directory is
refused, and that refusal is in the mutation catalogue.

**Attribution is a measurement rather than a constant.**
`candidate-image-slope-changed` and `reference-image-slope-changed` are the same
damage on opposite sides and produce opposite attributions, using the sidecar's
two independent readings of the same metadata. Both directions are in the
catalogue, which is what stops rung 2 being a rule that always answers "ours".

**What the story says it cannot prove, and says at the site.**
`docs/lld/comparator.md` does not claim the bias bound is calibrated. Two
refusals are named as honest gaps that cannot be exercised yet, one waiting on a
SIGMOID corpus row in F-X012. `INFORMATIVE_FRACTION_FLOOR` is declared a
judgement rather than a derivation, with the measured separation recorded as an
observation after the fact and explicitly not as the reason for the number.

## Carried forward

Eleven plan contradictions are reported in the story's own record rather than
absorbed, including that a volume-reformat sidecar carries no `row` block at
all, so class is resolved through the members' stack sidecars, and that a
reformat has no derivable image rectangle so its bias bound is slightly looser.
Both are consequences of F-X007's shape landing after the plan was written.
