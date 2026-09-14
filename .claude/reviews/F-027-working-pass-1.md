# F-027 review, pass 1

**Reviewed**: working tree. `vendor/openjph-core-0.1.0/` (50 published files),
`crates/ocelli-codec/src/htj2k.rs`, `lib.rs`, `Cargo.toml`, the workspace
`Cargo.toml`, `examples/decode_htj2k.rs`, `examples/verify_htj2k.rs`,
`tests/htj2k.rs`, `tests/fixtures/generate_htj2k.py` and its three fixtures,
`scripts/pin_and_size_check.py`, `scripts/unsafe_allowlist_check.py`,
`scripts/guards/catalogue.py`, `ci/guard-probe-budget.json`,
`ci/bench-baseline.json`, `bin/ocelli.sh`, `tools/bench/`,
`docs/SOURCE-POLICY.md`, `docs/spikes/A1-*.md`, `docs/lld/benchmarks.md`, and
the removal of `tools/spikes/a1-htj2k/` and `tools/spikes/x013-htj2k-route/`
**Result**: 1 defect, 2 smells, 0 nitpicks. All three were closed during the
pass and each is recorded as found.

## Defects

### D1, the design plan's `.201` progression rule was wrong

**Where**: `.claude/plans/F-027-design.md`, Approach, step 5: "`.202`
additionally requires progression order RPCL, read from COD's `SGcod`
progression byte, and `.201` requires anything else. **Without the progression
check the two UIDs are interchangeable**, and a `.202` claim would be
unfalsifiable."

**What**: The second half is false. PS3.5 A.4.10 requires RPCL for `.202`.
A.4.9 constrains **nothing** about progression for `.201`. So a `.202`
codestream is also a valid `.201` codestream, and the two are not mutually
exclusive. Implementing the plan as written would have refused conformant
`.201` files whose encoder happened to choose RPCL.

**Why it is wrong**: It would have produced a refusal for a file the standard
permits, which is the opposite failure from the one the story is about but is
still a wrong answer about a real file.

**Evidence**: Caught by the fixture generator before any adapter code ran. It
asserted the plan's expectation and refused:

```text
htj2k_corpus_lossy.j2c: progression order is RPCL, wanted anything else
```

Reading the three corpus rows' own COD segments:

| row | UID | CAP | progression | transform |
|-----|-----|-----|-------------|-----------|
| `htj2k_lossless` | `.201` | yes | LRCP | reversible 5/3 |
| `htj2k_lossless_rpcl` | `.202` | yes | RPCL | reversible 5/3 |
| `htj2k_lossy` | `.203` | yes | RPCL | irreversible 9/7 |

`.203` is RPCL too, so progression does not partition the three at all. What
separates `.201` and `.202` from `.203` is the **wavelet transform**, and RPCL
is a one-directional requirement on `.202` alone.

**Closed** by implementing the standard rather than the plan: three `Mode` arms
instead of two, `.201` requiring reversibility only, `.202` requiring
reversibility and RPCL, `.203` constraining neither.
`the_rpcl_constraint_is_one_directional_because_ps3_5_makes_it_so` asserts both
directions, including the one that matters, that `.201` **accepts** the RPCL
row. The generator records the reasoning where the fixtures are built.

## Smells, both closed in this pass

### S1, the codestream-side component check was unreached

**What**: `header.components != 1 || header.multiple_component_transform != 0`
could not fire. No multi-component HTJ2K corpus row exists, and a three-sample
descriptor is refused by `validate_descriptor` before the codestream is read.

**Evidence**: replacing the condition with `false` left the suite green at 9
passed.

**Closed** by splicing a three-component SIZ into the `.201` fixture inside the
test, which is the smallest edit that makes a structurally valid
multi-component main header from a single-component one. Every earlier check
including CAP still passes, so the component refusal is the one that fires.
Re-probed: removing it now fails a test.

### S2, vendoring put 104 `unsafe` constructs inside `git ls-files`

**What**: `scripts/unsafe_allowlist_check.py` reads `git ls-files`, and
`docs/SOURCE-POLICY.md` said it "never sees a dependency". A **vendored**
dependency is in `git ls-files`, so the gate went red on the unmodified tree.
`ritk-codecs` did not expose this because it contains no unsafe at all.

**Why it is a smell rather than a defect**: the obvious repair, excluding
`vendor/`, would leave R5 passing mechanically while its stated purpose was
weakened. R5's payoff is that a device-submission reviewer reads two files to
audit every unsafe line, and a vendored package carrying unsafe makes that
false whether it is tracked or resolved from a registry.

**Evidence**: `python3 scripts/unsafe_allowlist_check.py` exited 1, naming
`vendor/openjph-core-0.1.0/src/coding/simd/neon.rs:15` first.

**Closed** by recording the count rather than excluding the directory. The
guard now refuses a vendored package whose count has moved, one with no record
at all, and a record naming a package absent from the tree, and its OK line
prints the per-package figure. That is strictly stronger than the state before
F-027, where a dependency's unsafe was invisible either way. The audit itself,
nine files and which paths are reachable on wasm32, is in
`docs/SOURCE-POLICY.md`, re-counted rather than quoted from F-X013.

## Nitpicks

None.

## Two findings the probe harness produced, which are the reason it exists

Both were in work written during this pass, and neither would have been found
by reading.

**The redistribution gate had no reachable green state.** `pins.openjph-notice`
is an accept probe: it plants a `LICENSE` file and requires the check to pass.
It failed, because `check_openjph_vendor` then reported
`openjph-core carries an unrecorded file: LICENSE`. **Obtaining the notice, the
only action that is supposed to satisfy the gate, would have tripped a
different refusal in the same script.** A gate with no reachable green state is
not a gate. Closed by making the four notice filenames permitted additions, with
the reason recorded at the constant.

**A probe declared the wrong refusal.** `pins.openjph-inventory-root` expected
`openjph-core published file changed`, and changing a source file together with
its inventory row actually fails on the inventory **digest**, which is rooted
outside the vendor tree. The harness reported "got the exit status it wanted,
but not at the declared refusal", which is exactly the check that separates a
run that went the right way from one that went the right way for another reason.

## Verified clean

- **Provenance was re-verified independently, not copied.** The archive fetched
  on 2026-09-13 has SHA-256
  `c8b96ed12b3d41623a771af4af8131abf353bc822b7a567c6ef3b35ab967a36d`, matching
  what F-X013 and `docs/SOURCE-POLICY.md` recorded. The crates.io API reports
  `BSD-2-Clause`, not yanked, 4,863 downloads against F-X013's 3,703, and the
  packaged VCS revision is `7ed6d6d110d994ec740aacaa90a78b2e807c4c24` under path
  `openjph-core`. The fifty published files contain no `LICENSE`, `LICENCE`,
  `COPYING`, `NOTICE` or `README`, which is the condition that stays open.
- **The vendored package is byte-identical to the archive.** No patch, which the
  pins gate enforces by requiring every inventory row to match. The published
  manifest already yields the graph section 15.2 wants: `openjph-core` plus
  `thiserror`, no Rayon, no C toolchain.
- **Nine mutations, each observed red, each reverted**, with the source
  restored from a byte copy and the suite re-run green afterwards:

  | Mutation | Result |
  |----------|--------|
  | CAP marker check removed | 9 passed, 1 failed |
  | RPCL constraint defeated | 9 passed, 1 failed |
  | Reversibility dropped from `.201` | 9 passed, 1 failed |
  | COD progression byte index wrong | 9 passed, 1 failed |
  | COD transform byte index wrong | 2 passed, 8 failed |
  | SIZ precision check removed | 9 passed, 1 failed |
  | Codestream component check removed | 9 passed, 1 failed |
  | Signedness check removed | 9 passed, 1 failed |
  | Sample conversion narrowed to `i8` | lib target, 7 passed, 2 failed |

  The last one is green on the integration target and red on the lib target,
  and that is correct rather than a gap: narrowing `i16` to `i8` changes nothing
  for unsigned samples, because the `u16` arm still converts them exactly. It
  matters only for negative samples, which no corpus row carries, so the unit
  test that sweeps `-32_768 ..= 65_535` is the thing that can see it.
- **CAP is what makes a codestream HTJ2K, and it is falsifiable.** The Part 1
  `.90` fixture is the same synthetic ramp, so its dimensions, precision and
  reversibility all match and every other check passes. All three HTJ2K
  decoders refuse it, and removing the CAP check makes one of them accept it.
- **Both lossless syntaxes reproduce the uncompressed reference exactly**, and
  that reference is `scripts/corpus_synth.py`'s ramp, which comes from no codec.
- **`.203` reproduces F-X013's measurement exactly.** The production adapter's
  output digest is
  `ce4a2bb9d75b897292a4e9e9e7455447e976f5ff1e18b4ecb3219bb17d4d062c`, which is
  precisely what the spike recorded for `D_native`, `D_wasm` and `D_simd`. That
  is the spike's result reproduced through an entirely different code path, the
  production `Decoder` boundary rather than a throwaway cdylib.
- **The `.203` assertion was corrected during the pass and the correction is in
  the test's own comment.** The plan said to pin F-X013's "41 of 6144"
  divergence. That figure is the candidate against **OpenJPH 0.31.0's** output,
  which is not in this tree: `ojph_expand` is an external tool the spike ran
  once. Against the uncompressed ramp the irreversible decode differs at 5,377
  of 6,144 samples, which is what a lossy codec does. What is pinned instead is
  the candidate's own digest, which is a stronger statement and needs no
  tolerance: if the decode ever changes this fails, and the OpenJPH comparison
  would have to be re-measured rather than assumed.
- **F-X013's central claim is now standing rather than a spike result.**
  `bin/ocelli.sh gate native` steps 9 to 11 run `verify_htj2k` natively, as
  plain wasm and as `+simd128` wasm, over all three syntaxes, and all three
  targets execute it. The spike that measured this once is deleted.
- **No `as` cast.** `grep -n ' as '` over `crates/ocelli-codec/src/htj2k.rs`
  finds none. `exact_f32` converts through `i16::from` and `u16::from`, both
  lossless, and refuses anything outside their union rather than rounding.
  `i32 as f32` would round silently on exactly the values that function exists
  to catch.
- **No `unsafe` added by this repository**, and the dependency's surface is now
  a recorded number rather than an invisible one.
- **D-21's largest instance is named where it happens.** `pull` returns a fresh
  `Vec<i32>` per row, so a 64-row frame costs at least 65 allocations. The rows
  are validated completely before the caller's output is touched, and the write
  is one `copy_from_slice`, asserted by a surviving `0xa5` fill on every error
  path.
- **The benchmark is real and its band is the tightest of the three codecs.**
  Median 0.1685 ms over the `.203` corpus frame, fifteen-sample calibration with
  extremes 3.74 per cent below and 2.43 above, and a declared 5 per cent band.
  No protocol changed to achieve it.
- **The spike harnesses are deleted and their answer files say so**, keeping
  every measurement. The `E2.6` misnumbering in both A1 files and in
  `A2-jpeg-ls.md` is corrected once at the top of each rather than rewritten
  throughout, because the record is what it is and a reader needs the referent
  before the body.
- **The whole floor is green**, 26 gates, including `guards` with the four new
  probes and `bench` with the new subject.
