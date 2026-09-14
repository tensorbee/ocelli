# F-031 review, pass 4

**Reviewed**: the staged tree after pass 3's remediation,
`a72b9de9e40598cd60078da7c624273395187538`.
**Result**: 3 defects, 1 smell, 7 nitpicks

Pass 4's subject is pass 3's remediation and the whole crate with fresh eyes.
The reviewer was independent of the author, measured rather than read, and left
the tree unmutated with `lru.rs` hash-checked after each of its own mutations.

## Defects

### D1, the same allocation claim, in the two places nobody had looked

**Where**: `Admission`'s doc comment in `crates/ocelli-cache/src/lru.rs` and the
matching paragraph in `.claude/plans/F-031-design.md`.
**What**: both said the common path, an insert that fits with nothing displaced,
"is one value with three empty fields and no allocation at all". The subject is
the returned value, which is true of it, and the sentence reads as a claim about
the insert, which is false: measured over forty non-evicting, non-displacing
inserts, that path allocated six times.
**Why it is wrong**: `lru.rs` then contradicted itself a hundred lines later,
where `insert`'s own comment says it can allocate, and the `Admission` comment
is the one F-040 reads before deciding whether `insert` is a per-frame call.
**Fixed**: both now say the value allocates nothing and that this is a claim
about the return value rather than about `insert`, with a pointer to the method.

### D2, pass 3's absence check was false for one of the two phrases it named

**Where**: `.claude/reviews/F-031-implementation-pass-3.md`.
**What**: it reported "second lookup" gone from the code. It survives in
`evict_least_recently_used`'s corrected doc comment, wrapped across two `///`
lines, which is why a line-based grep reported absence.
**Why it is wrong**: the surviving sentence is the correct one, so the code is
right and the record was wrong about the tree it describes. A record that
overstates what a check covered is the same class of defect the check exists to
find.
**Fixed**: pass 3's record says what survives, and why the grep missed it.
Pass 4 found it by flattening whitespace before searching, which is the method
this crate's multi-line doc comments need.

### D3, pass 3's smell miscounted the list it replaced

**Where**: the same record.
**What**: "`docs/lld/cache.md` eight". It listed seven. Eight is the count in the
replacement text, nine public methods minus `insert`, carried back into the
description of what was replaced.
**Fixed**: the record says seven.

## Smells

### S1, the progress note was the fourth uncorrected copy

`.claude/scratch/F-031-progress.md` is gitignored, and `AGENTS.md` makes it the
next agent's first read. It still carried the enumerated allocation-free method
list, "`insert` allocates one map node", a test count of twenty, and a mutation
count of nine, all of which later passes had superseded. Every ledger written
downstream is read from there.

**Fixed**: the note now carries the rule, the measured allocation behaviour,
twenty-one tests, all ten mutations with the test each one reddened, and the
four review passes with their counts.

## Nitpicks

1. Pass 3's own reflow left a new ragged wrap in the plan's boundary bullet.
   **Reflowed.**
2. The allocation histogram is not key-order invariant in the way the LLD's
   hedge implied. A scattered order measures thirty-five, four and one against
   the monotone orders' thirty-four, five and one. **Re-measured here under all
   three orders and confirmed**, and `docs/lld/cache.md` now says the split is
   not a constant and gives both.
3. `tests/budget.rs` attributed the division by eight to `ocelli-codec`'s
   `frame_bits`, which returns bits and does not divide. **Corrected**, and the
   division is now attributed to this file.
4. `DecodedSeries`'s citation named PS3.3 C.7.6.3.1 for a byte length, and
   `Slices` is not a PS3.3 attribute. **Reworded** to cite the section for the
   four attributes it defines and to say the slice count is the fixture's.
5. A quoted HLD figure used `x` where section 7 writes `×`. **Corrected**, in a
   file that claims exact quotation elsewhere.
6. `Budgeted` is a new trait with no implementer outside tests, and
   `AGENTS.md`'s structural rule names only two HLD exceptions. **Answered in
   the plan**: the rule is about constructs this repository invents, and this is
   section 20's own signature transcribed, and the generic is what makes F-032,
   F-033, F-036 and F-040 implementations rather than forks.
7. Pass 3 listed `src/lib.rs` among files whose section 8 quotations were
   checked. It paraphrases and quotes nothing. **Corrected.**

## Verified clean

**The rule replacing the list is true, measured over every entry point.** A
counting global allocator exercised `new` on three tiers at budgets of zero, one
and `usize::MAX`, `Admission::default()`, `get` and `contains_key` on hits and
misses, `pressure`, `len`, `is_empty`, `would_admit` at six sizes, `remove` over
a hundred hits and a hundred misses in descending order to force node merges,
and a refused insert. All zero. Repeated with `String` keys, whose `Clone`
allocates, to catch a clone on a reading path. Still zero. `insert` is the only
method that allocates.

**The rule landed once per file and no enumeration survives.** The list that
remains in `docs/lld/cache.md` is about recency, a different fact, and is right.

**D-24 agrees with the code and with itself.** `would_admit`'s negation is
`budget == 0 || bytes > budget`, and both the behaviour column and the rationale
column now say that. `gate deviations` exit 0.

**Three fresh mutations, each red on its own test.** Deleting `release` from
`remove` reddens the `remove` test alone. Adding a `bytes > 0` clause to
`would_admit` reddens the two zero-byte tests and leaves the zero-budget test
green. Hardcoding a tier in `pressure` reddens the empty-cache test alone, which
is what shows the `CacheTier` round trip is executed rather than declared.

**no_std is compiled, not merely declared.** The integration tests link the
library built without `cfg(test)`, so every `cargo test` compiles the `no_std`
configuration of both modules.

**Arithmetic, casts, panics and termination**, checked again from scratch: zero
`as` casts, zero `unwrap`, `expect`, `panic!`, `unreachable!`, zero indexing,
zero `#[allow]`, zero `unsafe`. `self.used += incoming` cannot overflow.
`free()`'s saturation cannot fire under the invariant. The eviction loop removes
one entry per iteration or breaks. `incoming == 0` never satisfies
`0 > free()`, which is exactly why a zero-byte entry never evicts.

**Commands, each exit code read from the command itself.** `test` exit 0 with 21
tests over four binaries, `clippy` exit 0, and `gate prose unsafe nostd
deviations bindgen pins content backlog` exit 0 over eight gates.
