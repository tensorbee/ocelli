# F-031 review, pass 3

**Reviewed**: the staged tree after pass 2's remediation,
`9096537035ce1547850c7c459a69e21a3d4ea227`.
**Result**: 1 defect, 1 smell, 4 nitpicks

Pass 3's subject is pass 2's remediation. The reviewer was independent of the
author, re-measured rather than re-read, and left the tree unmutated.

**The finding that matters is not any one sentence.** Three passes have now
produced one shape: a claim corrected where it was found and left standing in
another file that carries it. That is worth telling the operator, and this
pass's remediation attacks the shape rather than the instance.

## Defects

### D1, the allocation number was corrected in three places and left in the fourth

**Where**: `.claude/reviews/F-031-implementation-pass-1.md`, D1's evidence line.
**What**: it still read "one allocation during a non-evicting insert" as an
unqualified measurement, which pass 2 measured as false for 35 of 40 inserts.
Pass 2 listed three files to correct and its own predecessor was not among them.
**Why it is wrong**: a review record is prose whose every factual sentence is
checkable, and pass 2 had already established the remedy by correcting the same
record's S2 in place.
**Evidence**: re-measured independently under three key orders, ascending,
descending and scattered. All three gave the same histogram over forty
consecutive non-evicting inserts: 34 allocating nothing, 5 once, 1 twice.
**Fixed**: pass 1's evidence now says what it measured, one insert into an empty
cache, and says that pass 2 corrected the conclusion drawn from it. The finding
itself stands, because the plan's claim was that the path is allocation-free.

## Smells

### S1, the allocation-free method set was written out three times at three lengths

`src/lru.rs` listed six methods, `docs/lld/cache.md` seven, the plan four. All
three were true, and all three were hand-maintained enumerations of the same
fact in a crate where every review pass has found the same failure mode. The
next method added to `Lru` would have been added to one of them.

**Fixed by replacing the list with a rule**: `insert` is the only method on
`Lru` that allocates. That is one sentence, it is true, it says why it is a rule
rather than a list, and a new method is covered by it without an edit. All three
files now carry the rule and `docs/lld/cache.md` keeps the measurement.

## Nitpicks

1. "the byte budget working **as specified**" in `docs/lld/cache.md` implied
   HLD section 20 specifies entry-count behaviour. It specifies none, which the
   plan says explicitly. **Reworded** to say section 20 specifies no entry count
   and this crate adds none.
2. A ragged wrap in the same paragraph, left by an earlier edit. **Reflowed.**
3. `docs/hld/DEVIATIONS.md` D-24's rationale column still framed a refusal as an
   entry too large for the budget, after the behaviour column had been corrected
   for the zero-budget half. **Fixed**, so the row agrees with itself.
4. `DecodedSeries`'s `Budgeted` implementation carried no citation where
   `DecodedFrame`'s carried PS3.3 C.7.6.3.1 for the same product. **Added.**

## Verified clean

**Every pass 2 correction, checked by the absence of the old text across all
tracked files** rather than in the diff. "exceeds the whole budget" has no
occurrence anywhere. "eviction record" survives twice, both of them in-place
notes saying what the text used to say. "only for an empty map" is gone from the
code and the LLD. **"second lookup" is not**, and pass 3 reported it as gone
because the phrase wraps across two `///` lines and a line-based grep missed it.
Pass 4 found that by flattening the whitespace first. The surviving sentence is
the corrected one, which says there are two lookups and what happens when they
disagree, so the code is right and this record was wrong.

**The 34, 5 and 1 measurement reproduced exactly**, on the first attempt, under
ascending and descending key orders, with the node split at insert index 11
stable and the later ones moving. Pass 4 then measured a scattered order at 35,
4 and 1, so the histogram is not a constant either, and `docs/lld/cache.md`
says so rather than carrying one run as though it were the rule.

**D-24.** The file changed by one line in pass 2 and one clause in pass 3, and
nothing else in it is disturbed. The row's condition matches
`would_admit`'s negation exactly. `python3 scripts/deviation_check.py` exit 0,
24 deviations, every citation resolving.

**The two newest tests are load-bearing and separable.**
`entries_of_no_bytes_never_evict_anything` fails alone under a mutation that
makes a zero-byte entry evict, and its `len() == 11` is three from `filled()`
plus eight. `would_admit_agrees_with_insert_at_the_budget_boundary` fails alone
under `<=` to `<`, and dropping the zero-budget clause reddens the zero-budget
test alone.

**The files no earlier pass had read in full.** `tests/invariants.rs`, whose
model is driven by what `insert` reports rather than by a second copy of the
eviction policy, and whose generated space includes both a zero budget and a
zero-byte entry. `tests/budget.rs`, every constant recomputed by hand and none
read back from the implementation. `src/lib.rs`, which paraphrases section 8 and
quotes nothing, and `src/tier.rs`, whose three quotations match
`docs/hld/06-memory-and-cache.md` word for word.

**Doc comments cross-checked against each other and against the HLD.**
`release`'s claim that the release profile leaves debug assertions off was
checked against HLD section 15.2's manifest, which sets `opt-level`, `lto`,
`codegen-units`, `panic` and `strip` and never `debug-assertions`, and against
`bin/ocelli.sh`, which builds wasm in release. The glam cross-reference in
`ocelli-core` resolves and records the same reasoning.

**Commands, each exit code read from the command itself.**
`bin/ocelli.sh test ocelli-cache` exit 0, 21 tests over four binaries.
`bin/ocelli.sh clippy ocelli-cache` exit 0. `bin/ocelli.sh gate prose unsafe
nostd deviations bindgen pins` exit 0, all six green. Zero `as` casts, zero
`unwrap`, `expect`, `panic!`, `unreachable!` and zero indexing in `src/`, zero
`#[allow]` in the crate.
