# F-029 review, pass 2

**Reviewed**: working tree after pass 1's remediation,
`crates/ocelli-pixel/src/lut.rs`, `error.rs`, `lib.rs`,
`crates/ocelli-pixel/tests/lut_chain.rs`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## What pass 1 raised, and what closed it

### D1, a test comment stated a false reason

Closed. The comment now says what is true and is more useful than what it
replaced: a reflection fixes the **midpoint of the output range**, PS3.3
C.11.2.1.3.2 maps the window centre exactly onto that midpoint so a
`LINEAR_EXACT` centre fixture would survive a double inversion, and
C.11.2.1.2's `c - 0.5` and `w - 1` move `LINEAR` off the fixed point. Re-derived
independently in Python from the three PS3.3 sections before the comment was
rewritten.

**Pass 1's own summary sentence was also wrong in the same way** and was
corrected in place rather than explained.

### S1, the display-range refusal was unreached

Closed by two fixtures rather than by deleting the guard, because the guard is
correct and the public constructor can reach it.
`the_presentation_stage_refuses_a_malformed_output_range_on_its_own` covers a
descending range, a `NaN` and an infinity.
`the_presentation_stage_reports_the_range_before_the_evidence` pins the order of
all three refusals, which was previously an accident of which check ran first.

### S2, the LUT entry range was mapped from bits in two places

Closed. `LutDescriptor` retains `largest_representable_value` from the bound
`LutDescriptor::new` already computed to refuse out-of-range LUT Data, and the
accessor is a field read. The entry-size-to-range mapping now exists once, inside
the validation that refuses every other entry size, so a future third entry size
cannot silently inherit the 16-bit range.

### S3, the test helpers used the `unwrap` family

Closed, and **pass 1's proposed fix was wrong and is recorded rather than
quietly replaced.** Pass 1 suggested a `match` ending in `panic!`. That does not
compile under this workspace: `Cargo.toml`'s `[workspace.lints.clippy]` denies
`unwrap_used`, `expect_used` **and** `panic`, tests included, so
`bin/ocelli.sh clippy ocelli-pixel` exited 101 on the suggestion.

That failure is the useful part. A helper returning a value directly has **no
legal way to report its own failure** in this crate, which is why
`tests/voi.rs` and `modality.rs` use the early-returning `require_ok!` macro.
The helpers now return `Result`, a third helper composes the whole chain so the
common case stays one call, and every call site goes through `require_ok!`. The
reason is written at the helpers rather than left to be rediscovered.

## Verified clean

- **All sixteen fixtures pass.** `cargo test -q -p ocelli-pixel --test lut_chain`
  reports `ok. 16 passed; 0 failed`.
- **Five mutations, each observed red, each reverted**, with the whole file
  restored from a byte copy and re-run green afterwards:

  | Mutation | Result |
  |----------|--------|
  | Resolution rule from override to exclusive-or | 15 passed, 1 failed |
  | `ymin + ymax - y` to `ymax - y` | 14 passed, 2 failed |
  | Presentation stage dropped from `LutChain::apply` | 9 passed, 7 failed |
  | Display-range refusal deleted from `PresentationTransform::new` | 14 passed, 2 failed |
  | `LutDescriptor`'s retained range corrupted | 14 passed, 2 failed |

- **One of those probes silently did nothing on its first attempt, and the
  script caught it.** The display-range guard's text occurs twice in the file,
  once in `VoiTransform::new` and once in `PresentationTransform::new`, so the
  first M4 attempt matched two sites. The probe asserts `count(old) == 1` and
  raised rather than editing, and the test run in the same command then printed
  `ok. 16 passed`, which reads exactly like "the guard is not needed". **Without
  the count assertion the probe would have reported the opposite of the truth.**
  Re-run against an anchor unique to `PresentationTransform::new`, the guard's
  deletion fails two tests by name.
- **Stages 1 and 2 are still untouched.** `git diff` shows no edit inside
  `apply_window` or `ModalityTransform`. The `c - 0.5` and `w - 1` pair, the
  `LINEAR_EXACT` arm without them, the `<=` low and `>` high comparisons and the
  `-4 * (x - c) / w` exponent are byte unchanged, and `tests/voi.rs` and
  `tests/modality.rs` are unmodified.
- **The whole crate is green**: 10, 4, 16, 5, 2, 5 and 0 tests across the seven
  targets, `bin/ocelli.sh clippy ocelli-pixel` exit 0, `bin/ocelli.sh gate fmt
  clippy unsafe nostd` ALL GREEN with `160 files checked, 2 permitted` and
  `7 no_std crate(s) reach no std feature`.
- **No `as` cast, no `unsafe`, no allocation, no `wasm-bindgen`, no pixel across
  a boundary** added by this change.
- **Structure.** `LutChain` resolves one flag from two sources, which no
  existing type does, and is not a forwarding wrapper.
  `PresentationLutEvidence` is one type replacing an `Option` plus a flag, so
  the illegal combination cannot be spelled. No new trait, generic or
  `Box<dyn>`.
