# F-029 review, pass 1

**Reviewed**: working tree, `crates/ocelli-pixel/src/lut.rs`,
`crates/ocelli-pixel/src/error.rs`, `crates/ocelli-pixel/src/lib.rs`,
`crates/ocelli-pixel/tests/lut_chain.rs`
**Result**: 1 defect, 3 smells, 0 nitpicks

## Defects

### D1, a test comment states a false reason

**Where**: `crates/ocelli-pixel/tests/lut_chain.rs`,
`monochrome1_inverts_exactly_once_away_from_the_window_centre`, the comment
"A symmetric window cannot tell one inversion from two, because the centre is
its own reflection"

**What**: The sentence is false as written. The value that is its own reflection
is the midpoint of the **output** range, not the window centre, and the two are
not the same point. `LINEAR` maps the window centre to `127.819549` because of
`c - 0.5` and `w - 1`, and the midpoint of `0 ..= 255` is `127.5`, so the window
centre reflects to `127.180451` and is perfectly distinguishable. The
`monochrome1_inverts_exactly_once_at_the_window_centre` fixture directly above
relies on exactly that and passes, so the code is right and only the stated
reason is wrong.

**Why it is wrong**: `/microscope` severity table, "a claim in prose that is
false" is a defect. It is worse than a stray sentence here, because it tells the
next reader that a centre-of-window fixture proves nothing, and the test
immediately above it is a centre-of-window fixture. Someone acting on the
comment would delete a working test.

**Evidence**: The three functions re-derived from PS3.3 C.11.2.1.2, C.11.2.1.3.2
and C.11.6.1.2 in Python, independently of the Rust:

```text
LINEAR        at the WINDOW CENTRE x=40: y=127.819549  reflected=127.180451  distinguishable
LINEAR_EXACT  at the WINDOW CENTRE x=40: y=127.500000  reflected=127.500000  SELF-INVERSE
LINEAR        at x=-60: y=63.909774  reflected=191.090226
```

**The true statement the comment was reaching for** is about `LINEAR_EXACT`, not
about symmetry in general: `LINEAR_EXACT` maps the window centre exactly onto the
output midpoint, so a `LINEAR_EXACT` centre fixture would be self-inverse and
could not separate one inversion from two. `LINEAR`'s half-and-one adjustment
moves it off that fixed point. The `-60` input avoids the fixed point for both
functions, which is the property worth stating.

## Smells

### S1, `PresentationTransform::new`'s display-range refusal is unreached

**Where**: `crates/ocelli-pixel/src/lut.rs`, `PresentationTransform::new`, the
`!ymin.is_finite() || !ymax.is_finite() || ymin > ymax` guard

**What**: The guard is present, looks authoritative, and no test in the crate
reaches it. `LutChain::new` is the only caller in the tree, and it sources its
range from `VoiTransform::output_range`, which returns either a window range
`VoiTransform::new` has already validated or a descriptor range derived as
`(0.0, 255.0)` or `(0.0, 65_535.0)`. Neither can be non-finite or descending, so
via `LutChain` the guard cannot fire. It is reachable only through the public
`PresentationTransform::new`, which nothing exercises.

**Why it is wrong**: `/microscope` section 4, "things that exist and that
nothing executes", and this is that class exactly. It is a smell rather than a
defect because the refusal is correct when it does fire. It will be counted as
coverage forever otherwise.

**Evidence**: The guard was deleted and the whole crate suite re-run.

```text
cargo test -q -p ocelli-pixel
test result: ok. 10 passed; 0 failed
test result: ok. 4 passed;  0 failed
test result: ok. 14 passed; 0 failed      <- lut_chain, still green
test result: ok. 5 passed;  0 failed
test result: ok. 2 passed;  0 failed
test result: ok. 5 passed;  0 failed
```

Nothing went red. The guard was restored.

### S2, the LUT entry range is mapped from bits in two places

**Where**: `crates/ocelli-pixel/src/lut.rs`, `LutDescriptor::new`'s
`largest_value` match, and `LutDescriptor::largest_representable_value`

**What**: `new` already computes `8 => 255.0, 16 => 65_535.0` to bound LUT Data.
`largest_representable_value` computes the same mapping a second time, and it
spells the second arm `_ => 65_535.0` because a method on a constructed
descriptor cannot fail. If a third entry size were ever accepted by `new`, the
catch-all would silently return the 16-bit range for it.

**Why it is wrong**: `AGENTS.md`'s structural test, does this increase the
number of places a reader must look. One fact, PS3.3 C.11.2.1.1's declared
output range, has two definitions that are only equal by inspection.

**Evidence**: The 16-bit arm is genuinely covered, so this is a smell and not
dead code. Changing `65_535.0` to `4_294_967_295.0`:

```text
a_voi_lut_sequence_inverts_about_its_descriptor_range --- FAILED
test result: FAILED. 13 passed; 1 failed
```

The defect this smell predicts is the one the mutation cannot show, because it
needs a future third entry size to exist.

### S3, the test helpers use the `unwrap` family

**Where**: `crates/ocelli-pixel/tests/lut_chain.rs`, `identity_modality` and
`window`, both ending `.unwrap_or_else(|_| unreachable!(...))`

**What**: `crates/ocelli-pixel/tests/voi.rs` and `modality.rs` both avoid this
by returning early through the `require_ok!` macro. These two helpers return a
value rather than `()`, so `require_ok!` does not fit, and the substitute
reached for `unwrap_or_else`. The failure message on a broken helper is
`internal error: entered unreachable code`, which names neither the constructor
that failed nor its `PixelError`.

**Why it is wrong**: `/implement-feature` section 3 denies `unwrap` and `expect`
in this codebase. The lint is scoped to crate sources rather than to test
targets, so clippy is green and the habit is still the one the project rejected.
A plain `match` with a `panic!` carrying the error is the same failure with a
usable message.

**Evidence**: `bin/ocelli.sh clippy ocelli-pixel` exits 0 with the helpers as
written, so no gate catches this and a reader is the only thing that will.

## Nitpicks

None.

## Verified clean

- **The three VOI formulas are untouched.** `git diff` over
  `crates/ocelli-pixel/src/lut.rs` shows no edit inside `apply_window`. The
  `c - 0.5` and `w - 1` pair, the `LINEAR_EXACT` arm without them, the `<=` low
  and `>` high comparisons and the `-4 * (x - c) / w` exponent are byte
  unchanged. **F-029 did not reimplement stages 1 or 2**, which was the story's
  main risk.
- **`crates/ocelli-pixel/tests/voi.rs` and `modality.rs` are unmodified**, so
  F-018's section 18.3 fixtures still judge the same code.
- **The section 18.3 rows survive composition.**
  `hld_section_18_3_rows_survive_composition_through_the_chain` asserts all four
  rows for both linear functions through `LutChain`, with D-13's `0.000` in row
  one, and passes.
- **Inversion applies exactly once**, and the assertion is load-bearing. Three
  mutations were run and each was observed red, then reverted:

  | Mutation | Result |
  |----------|--------|
  | Resolution rule from override to exclusive-or | `an_explicit_presentation_shape_decides_alone` FAILED, 13 passed 1 failed |
  | `ymin + ymax - y` to `ymax - y` | 12 passed, 2 failed |
  | Presentation stage dropped from `LutChain::apply` | 7 passed, 7 failed |

  The third failing seven of fourteen rather than one or two is the signal that
  the inversion fixtures are not concentrated in a single test.
- **The asymmetric fixture is real.** `monochrome1_inverts_exactly_once_away_
  from_the_window_centre` uses `-60` HU, where one inversion gives `191.090` and
  two give `63.910`. Under `LINEAR` the window centre also separates the two, at
  `127.180` against `127.820`, which is what D1 above corrects the stated reason
  for. The
  non-zero-`ymin` fixture at `16 ..= 236` is the one that separates
  `ymin + ymax - y` from `ymax - y`, and `165` against `181` is asserted
  explicitly.
- **Every fixture's expected value cites a PS3.3 section and shows its
  arithmetic** in the comment above it. None was copied from program output.
  Checked by re-deriving `71` and `181` by hand from C.11.2.1.3.2 and C.11.6.1.2
  independently of the source.
- **Boundary and tier.** No `wasm-bindgen`, no pixel crosses a boundary, no
  allocation added anywhere. `LutChain::map_into` writes into caller storage and
  refuses a length mismatch before the first write, asserted by a surviving
  `Display(77.0)` sentinel. `bin/ocelli.sh gate unsafe` reports
  `no unsafe outside the allow-list (160 files checked, 2 permitted)`.
  `bin/ocelli.sh gate nostd` reports `7 no_std crate(s) reach no std feature`,
  so `ocelli-pixel` keeps its posture and the new code uses no `std` item.
- **Structure.** `LutChain` is not a forwarding wrapper: it resolves the single
  inversion flag from two possible sources, which is work no existing type does,
  and `inverts()` exists because HLD section 18.4's uniform carries one `u32`.
  `PresentationLutEvidence` replaces an `Option<PresentationLutShape>` plus a
  separate sequence flag, which is one type instead of two fields with an
  illegal combination. No new trait, no new generic, no `Box<dyn>`.
- **No `as` cast was added.** `grep -n ' as ' crates/ocelli-pixel/src/lut.rs`
  finds none in the new code.
