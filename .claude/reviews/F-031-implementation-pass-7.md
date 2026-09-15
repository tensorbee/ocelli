# F-031 review, pass 7

**Reviewed**: the staged tree after pass 6's remediation,
`35ce5c2fe888e14f95be78ed92a7f7ab6fa94cb1`.
**Result**: 1 defect, 0 smells, 3 nitpicks

Pass 6 added five tests and closed seven mutation gaps. This pass reviewed that
work as code, re-ran all seven gap mutations, hunted twenty-six more, and ran
the floor gate. The reviewer was independent of the author and left the tree
unmutated.

**`bin/ocelli.sh gate --floor` exit 0**, 22 passed and 4 skipped, all four skips
environmental, an absent `node_modules` and an unimportable pydicom.

## Defects

### D1, a true claim lost the word that made it true, in two files

**Where**: `docs/lld/cache.md` and the plan's new test-table row, both saying
`the_evicted_vector_is_least_recently_used_first` is "the only test that evicts
more than one entry".
**What**: the property test reaches multi-eviction routinely. Measured by adding
`prop_assert!(admission.evicted.len() <= 1)` to its insert arm and running it
twelve times with fresh seeds: red in twelve of twelve.
**Why it is wrong**: pass 6's own record states it correctly, with the word
"deterministic", and both documents dropped it when restating the finding. That
is the recurring shape again, and this time the copies were written from memory
of the finding rather than from the record.
**Fixed**: both say "deterministic", and both now add what the property test
does and does not assert, which is that it evicts several routinely and says
nothing about their order.

## Smells

None.

## Nitpicks

1. **The `Shifting` fixture had an unasserted precondition.** Making `restate`
   a no-op, or `Shifting::bytes` a constant, left the suite green, because
   nothing asserted that the value really reported a different number after
   admission. The three read-once tests were genuinely red on the three real
   mutants, so the coverage was real, but it rested on a fixture nothing
   checked. **Fixed**: each of the three asserts the restatement took, and the
   no-op mutation now fails all three.
2. **The order test was weaker than its name.** Its `get(&"c")` was inert,
   because "c" was already the most recent, and the expected `[a, b]` happened
   to equal `BTreeMap` key order, so the test could not tell recency order from
   key order. **Fixed**: it uses `get(&"b")` and expects `[a, c]`, which is not
   key order. Evicting by map order rather than by tick now fails it, where
   before it was caught only by another test.
3. **The second scattered histogram was unnamed**, which reinstated half of the
   objection pass 6 had just fixed. **Fixed**: `(i * 3) % 41` is named beside
   `(i * 37) % 101`, so both are reproducible.

## Verified clean

**All seven of pass 6's gap mutations are red, each on its intended test.**
Re-run here: the reversed vector and the front push on the order test, the three
`V::bytes()` re-reads each on their own release-site test, the refusal bumping
recency on the refusal-order test, and the dropped `Samples per Pixel` on the
frame fixture.

**The five new tests were checked as code, not run as ritual.** The tick order
was worked out by hand for each before anything was run. `filled()` leaves ticks
0, 1 and 2, and `restate`'s `get` bump is accounted for in all three tests that
call it: in two of them no eviction happens so recency is irrelevant, and in the
third the reads that follow put the order back, exactly as its comment says.
**No test depends on an accident of ordering.**

**Twenty-six further mutations** beyond pass 6's catalogue, covering both
`would_admit` operators, the loop boundary, a frozen clock, `get` not setting
the tick, evicting the most recently used, evicting by map order, `release(0)`,
`displaced = None`, a wrong tier in `pressure`, `len` off by one, and two
structural reorderings of `insert`'s three phases. Every one red with a named
killer. The reordering that releases the displaced value after the eviction loop
is caught by the property test alone, measured red in twelve runs of twelve, so
it is not a gap.

**Pass 6's measurements reproduce.** Its D1 mutation is green on the
pre-remediation tree at 8192 cases, confirmed by checking out that tree's two
files. Its D4 coin flip reproduced exactly, red in eight runs of twelve.

**Arithmetic.** No `as` cast. `used += incoming` cannot overflow. Both
saturations are unreachable. All eight fixture constants recomputed by hand,
including the new `512 * 512 * 3 * 8 / 8 = 786_432`, and the series product fits
a `u32`, so it is safe on wasm32.

**Prose against the tree.** F-032 is confirmed as the story that surfaces
eviction events to JS, which is what the ordering claim rests on. All five new
plan rows match their tests by name and by content. The edited fixture row's
numbers hold. No pass 6 text landed twice.

**no_std, boundary, tier, structure, panics and termination** re-checked and
unchanged.
