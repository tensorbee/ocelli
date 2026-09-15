# F-031 review, pass 1

**Reviewed**: the staged working tree on `work/f-031-claude`, tree
`d589382d0e33bf533586d19a2e0bbdbeb66e3329`, base 3b00890.
**Result**: 2 defects, 3 smells, 4 nitpicks

The pass was run by a reviewer independent of the author, over the staged diff,
the approved plan, HLD sections 20 and 8, and D-24. Every finding below carries
the command or the mutation that produced it. The reviewer left the tree
unmutated, checked by `git write-tree` before and after.

## Defects

### D1, the design plan claims the insert path is allocation-free, and it can allocate

**Where**: `.claude/plans/F-031-design.md`, the `## Boundary and tier` block.
**What**: "An insert that evicts nothing pushes nothing, and `Vec::new` does not
allocate until it is pushed to, so even that path is allocation-free." The `Vec`
half is true. `Lru::insert` also puts the incoming entry into a `BTreeMap`,
which allocates a node.
**Why it is wrong**: the plan is what a later story reads to decide whether
`insert` is safe to call per frame, and F-040 is the upload path. HLD section
20's first bullet and `AGENTS.md`'s performance rules are about exactly that
call. `docs/lld/cache.md` and the module doc did not make the claim, so the
defect was in the plan alone.
**Evidence**: a counting `GlobalAlloc` in a temporary integration test, created,
run and deleted: the first insert into an empty cache allocated once, and `get`,
`contains_key`, `pressure`, `len` and `would_admit` allocated nothing.

**That evidence was one insert, and pass 2 corrected the conclusion drawn from
it.** A `BTreeMap` takes a node at a time, so most non-evicting inserts allocate
nothing and an occasional one allocates twice. The finding stands, because the
plan's claim was that the path is allocation-free and it is not. The number does
not, and `docs/lld/cache.md` carries the distribution pass 2 measured.
**Fixed**: the plan's clause now says what allocates and why that is the reason
`insert` is not a render-loop call, and `docs/lld/cache.md` gained the measured
numbers. `Lru::insert`'s doc comment says it too, at the place a caller reads.

### D2, "a zero-budget cache admits nothing" was false, in a test's own name

**Where**: `crates/ocelli-cache/src/lru.rs`, test
`a_zero_budget_cache_admits_nothing_and_stays_empty`, and the same sentence as a
row of the plan's test table.
**What**: the test only offered a one-byte entry. `insert` refused on
`incoming > self.budget`, and `0 > 0` is false, so a zero-budget cache admitted
an unbounded number of zero-byte entries while reporting `is_empty()`.
`would_admit(0)` told a caller the same.
**Why it is wrong**: a test function name is a claim, and this one was
contradicted by the type it tested. HLD section 8 is one budget per tier, and a
tier given no bytes that fills with entries is the same defect as a budget that
does not describe memory, in the other direction.
**Evidence**: a temporary test inserted a thousand zero-byte values into a
zero-budget cache: `len=1000 used=0 is_empty=false`, nothing refused.
**Fixed**: `would_admit` is now `self.budget > 0 && bytes <= self.budget`, and
`insert` asks `would_admit` rather than repeating the comparison, so the two
cannot disagree. The test covers the zero-byte entry, a second test covers the
counterpart a tier with a budget must still admit, and both the plan's test
table and `docs/lld/cache.md` state the rule.

## Smells

### S1, three `saturating_sub` calls absorbed exactly the desync the type exists to prevent

`used` is the sum of the live entries' admitted bytes, so at each of the three
subtraction sites `used >= entry.bytes` holds and the saturation can only fire
after that invariant has already broken, clamping to zero and reporting less
memory than is held. **Fixed** by one private `release(bytes)` carrying a
`debug_assert!` and the saturation, used at all three sites, with the reason
written once instead of nowhere.

### S2, two unreachable arms in the eviction loop, one of which would hang

The `else { break }` after `least_recently_used` was correctly documented as
unreachable. The second lookup, `self.entries.remove(&victim)`, had an implicit
false arm that continued the loop, so if it were ever taken the loop would ask
the same question of the same map forever. **Fixed**: eviction is one private
`evict_least_recently_used` whose failure to remove what the scan found returns
`None` instead of continuing, so the loop breaks rather than asking the same
question of the same map again. This sentence said the second lookup was gone,
which pass 2 measured as false: there are still two, a scan and a
`remove_entry`, and what changed is what happens when they disagree.

### S3, the `must_use` message said the opposite of what dropping an `Admission` does

It read "drops the evicted values without releasing what they held". Dropping
`Admission` drops each `V` and runs its `Drop`, so it does release. The real
hazard is that the caller cannot tell an eviction from a replacement from a
refusal, which is D-24's whole point. **Fixed**, and the new message says that.

## Nitpicks

1. `contains_key_and_pressure_are_not_uses` read both methods, so a failure
   could not say which one was the use. **Split into two tests**, which the
   mutation run then showed was worth doing: making `contains_key` a use fails
   the `contains_key` test alone.
2. The plan's mutation 2, "make `contains_key` bump recency", cannot be applied
   without first changing the method to `&mut self`, because a method that
   cannot mutate cannot carry the defect. The plan now says so.
3. `CacheTier` derives `Hash` with no reader today. Left as the plan specifies
   it, because the plan's listing is the approved shape.
4. `Admission::refused` did not say what happens to a live value under the same
   key. `insert`'s doc comment now does: a refused insert changes nothing.

## Verified clean

**Plan conformance.** All nine public `Lru` methods are present with the plan's
signatures, the struct carries the plan's five fields with `budget` and `used`
spelled as HLD section 20 spells them, and `Entry<V>` is private. The plan's
transcriptions of sections 20 and 8 were diffed against `docs/hld/` character by
character and are exact. Nothing public was added that the plan did not
authorise: no `CacheSet`, no value type, no `wgpu`, no feature flag, no
`Box<dyn>`.

**Fixture arithmetic, recomputed independently twice.** 524,288 and 2,097,152
for the frames, 262,144 and 153,600 for the texture, 314,572,800 against
268,435,456 for the series, the 46,137,344 the series is over by, and the
108,544 in the failure message. The 300 to 512 row padding is wgpu's 256-byte copy alignment.
The `frame_bits` provenance claim was checked against
`crates/ocelli-codec/src/native.rs`, which computes the same product.

**Mutation testing.** The plan's six, plus three more the remediation made
necessary, all applied to the remediated tree, all observed red on their named
test, all reverted with the file hash checked. No test passed both ways. The
results are recorded in `.claude/scratch/F-031-progress.md`.

**Arithmetic and panics.** Zero `as` casts in the crate. Zero `unwrap`,
`expect`, `panic!`, `unreachable!` and zero indexing in `src/`. Zero `#[allow]`.
`self.used += incoming` cannot overflow, because the loop exit guarantees
`incoming <= budget - used`. `clock` uses `wrapping_add` and the 584-year figure
is correct.

**no_std.** `#![cfg_attr(not(test), no_std)]` retained, `extern crate alloc`
correct, no `std::` path outside `tests/`. `proptest` is dev-only and already a
workspace dependency with three other users.
