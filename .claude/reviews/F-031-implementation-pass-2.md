# F-031 review, pass 2

**Reviewed**: the staged tree after pass 1's remediation,
`ab1bde21be1dfefac0f724e6fdcd5e79fd7bd55c`.
**Result**: 4 defects, 1 smell, 3 nitpicks

Pass 1's remediation is what this pass is about, because a fix nobody reviewed
is as likely to be wrong as the code it fixed. The reviewer was independent of
the author and left the tree unmutated, checked by `git write-tree` before and
after.

Three of the four defects are the same shape: a correction that landed in the
code and in the LLD and not in every other place the sentence lived. **A claim
that lives in four files is corrected in four files or not at all.**

## Defects

### D1, the "eviction carries its tier" falsehood was fixed in two places of three

**Where**: `.claude/plans/F-031-design.md`, the `CacheTier` paragraph.
**What**: it still read "It is on `Lru::new`, on `Pressure` and on every
eviction record, so an eviction leaving this crate already says which budget it
came from." An eviction record is a `(K, V)` pair, and `Admission` has no tier
field.
**Why it is wrong**: the plan is the approved, normative artefact, so the
sentence that pass 1 deleted from `src/tier.rs` and rewrote in
`docs/lld/cache.md` was still standing in the document the other two are
supposed to agree with.
**Fixed**: the plan now says `Admission` does not carry it, and why that is the
right answer rather than an omission.

### D2, the measured allocation number was wrong, and so was the mechanism

**Where**: `docs/lld/cache.md`, `.claude/plans/F-031-design.md` and
`Lru::insert`'s doc comment.
**What**: "one allocation for a non-evicting insert", and "the incoming entry
becomes a map node". A `BTreeMap` node holds several entries, so there is no
node per entry and most non-evicting inserts allocate nothing at all. Pass 1
measured one insert into an empty map and generalised it.
**Why it is wrong**: it is a measurement, and a measurement stated wrongly is
worse than none, because the next reader will not re-take it.
**Evidence**: re-measured here with a counting `GlobalAlloc` over forty
consecutive non-evicting inserts into one cache, temporary test deleted
afterwards and the tree hash confirmed: 34 allocated nothing, 5 allocated once,
1 allocated twice. Reading methods all zero, on hits and misses:
`get`, `remove`, `contains_key`, `pressure`, `len`, `is_empty`, `would_admit`,
and a refused insert. An evicting insert allocated once, for the `Vec` push.
**Fixed**: all three places now say the map takes a node at a time, carry the
measured distribution, and say that where the ones fall depends on key order,
so the number to carry away is that it is neither always zero nor always one.
The conclusion, that `insert` is not a render-loop call, is unchanged.

### D3, the new zero-budget rule made every description of `refused` false

**Where**: `Admission::refused`'s doc comment, `docs/lld/cache.md`'s type table,
the plan's `Admission` listing, and `docs/hld/DEVIATIONS.md` row D-24.
**What**: all four said `refused` holds the entry "because its `bytes` exceeds
the whole budget". After pass 1's fix, a zero-byte entry into a zero-budget
cache is refused and its bytes exceed nothing.
**Why it is wrong**: D-24 is a normative register row, and a register that
describes a condition the code does not implement is worse than a missing row.
The remediation's own new test asserts the contradicting case.
**Fixed**: all four now say the budget cannot hold it, name `Lru::would_admit`
as the condition, and give both halves of it. The behaviour was not re-decided:
the plan's test table requires a zero-budget cache to admit nothing, and it now
does, for every entry size.

### D4, `evict_least_recently_used`'s doc described a different function

**Where**: `crates/ocelli-cache/src/lru.rs`, and the same sentence in pass 1's
review record.
**What**: "`None` only for an empty map", and "no second lookup that could
disagree with the first". There are still two lookups, a scan for the smallest
tick and a `remove_entry` for the key it found, and the `?` on the second is
exactly the arm where they disagree.
**Why it is wrong**: what pass 1 actually fixed is the consequence, not the
second lookup. The old code continued the loop when the removal failed, which
would have spun forever. The new code returns `None` and the caller breaks.
**Fixed**: the doc comment says that, and pass 1's record is corrected in place
rather than left asserting something pass 2 measured as false.

## Smells

### S1, the zero-budget rule's stated reason applied at every budget

The rationale was "a disabled tier accepts an unbounded number of zero-byte
entries while reporting itself empty". True, and not specific to a zero budget:
a full cache accepts them too, because `0 > 0` is false and a zero-byte entry
never makes room. Measured: a budget of 300 already holding 300 bytes accepted a
thousand more zero-byte entries and evicted nothing. A reader of that paragraph
would reasonably conclude that a non-zero budget bounds what a tier holds.

**Fixed**, and the fix is a test rather than a sentence.
`entries_of_no_bytes_never_evict_anything` fills a budget and adds eight
zero-byte entries, and nothing leaves. `docs/lld/cache.md` now says the budget
bounds bytes and not entries, cites that test, and says what the property
depends on: `Budgeted::bytes` reporting what an entry costs, which is the rule
at the top of the same file.

## Nitpicks

1. `would_admit_agrees_with_insert_at_the_budget_boundary` restated the
   pre-remediation rule as its expected answer. **Changed** to a table of
   `(bytes, admitted)` written out per case, so the expectation is stated rather
   than recomputed from the rule under test.
2. The `debug_assert!` in `release` is the first in a shipped Ocelli crate.
   It does not breach HLD section 23: the release profile leaves debug
   assertions off, so the shipped artefact carries the saturation and not the
   assertion. **A line saying so** is now in the doc comment, next to the
   reasoning `ocelli-core` already records for glam's assertions.
3. Pass 1's record did not account for two edits the remediation made that no
   finding drove, the plan's stale `Default` derive and the `tier.rs` rewrite.
   Both are noted here instead.

## Verified clean

**No fix landed twice.** Checked by diffing the pass 1 tree against this one
directly, then counting the occurrences of thirteen distinct new sentences
across the four edited files. Each occurs once, except the split test row, which
occurs twice by design.

**Mutation testing, ten mutations on the final tree**, each applied by exact
string replacement, the crate's tests run, the file restored and the hash
checked. Every one went red on its named test and no test passed both ways:
the plan's six, the three the remediation made necessary, and one more for the
eviction loop's boundary. Two of them, the zero-budget clause and the zero-byte
counterpart, fail alone, which is what shows the new tests are load-bearing.
`pressure_is_not_a_use` and `contains_key_is_not_a_use` each fail alone under
the mutation that belongs to them, which is what splitting them bought.

**The zero-budget rule's consistency.** `insert` delegates to `would_admit`, so
there is one comparison and no path where one admits what the other refuses.
`budget` is immutable after `new`, so a `&self` answer cannot go stale before
the insert. Both proptest properties generate `budget` from zero and `bytes`
from zero, so both cases are inside the generated space.

**Termination and the accounting invariant.** The eviction loop removes one
entry from a finite map per iteration or breaks, so it terminates on every
input. `used` is the sum of the live entries on all five mutating paths, and the
`release` assertion is reached by every removal and never fired across both
proptest properties in the debug profile.

**The plan's test table against the tests that exist.** Fourteen `unit` rows
against fourteen tests in `lru.rs`, three `fixture` rows against four tests in
`budget.rs`, where the first row states two things, and two `property` rows
against two proptest functions. No orphan test and no unimplemented row.

**Commands, each exit code read from the command itself.**
`bin/ocelli.sh test ocelli-cache` exit 0, 21 tests over four binaries.
`bin/ocelli.sh clippy ocelli-cache` exit 0. `bin/ocelli.sh gate prose
deviations` exit 0, with the D-24 edit in the staged tree.

**What pass 1 did not look at**: the dev-dependency, `docs/lld/README.md`'s row,
the crate docs and `Cargo.lock`'s single added edge. All correct.
