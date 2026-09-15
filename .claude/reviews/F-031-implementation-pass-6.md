# F-031 review, pass 6

**Reviewed**: the staged tree after pass 5's remediation,
`00dbbafb46d4b1a09d4326d92451a792bb80804f`.
**Result**: 5 defects, 2 smells, 3 nitpicks

Pass 5 closed one mutation gap. This pass hunted that class systematically
instead of opportunistically, with a catalogue of **fifty-three mutations**, one
per public method and per branch of `insert`, each applied by exact string
replacement, the suite run from the command's own exit code, and the file
restored with its SHA-256 re-checked. Four more gaps fell out, and none of them
is reachable by any proptest case count.

**That is the lesson of this pass.** Five earlier passes read the tests and
agreed they looked thorough. A mutation catalogue disagreed, and it was right.

## Defects

### D1, the documented eviction order was pinned by nothing

**Where**: `Admission::evicted`'s doc comment and `docs/lld/cache.md`, both
saying "least recently used first".
**What**: reversing the vector before returning it, and independently pushing at
the front, both left the suite green at 256 proptest cases and at 8192. The root
cause is sharper than a missing assertion: **no deterministic test evicted more
than one entry**, so there was no order to get wrong anywhere the suite looked.
**Why it matters**: D-24's own rationale says F-032 turns these into JS events,
so this is the order events reach the shell in.
**Fixed**: `the_evicted_vector_is_least_recently_used_first` inserts an entry
needing two victims and asserts both the order and the survivor. Both mutations
now fail that test alone.

### D2, the read-once `V::bytes()` capture was executed by nothing

**Where**: `Entry::bytes`, and the three sites that give bytes back.
**What**: replacing `entry.bytes` with `entry.value.bytes()` in `remove`, in the
eviction path and in the replacement path each left the suite green. Every
`Budgeted` in the suite is immutable, so no case count can tell a cache that
kept the admitted number from one that re-reads it.
**Why it matters**: the paragraph on `Lru` spends five lines on exactly this
hazard, and `docs/lld/cache.md` repeats it. It was a claim with no test.
**Fixed**: a `Budgeted` over a `Cell<usize>` and three tests, one per release
site. Each mutation now fails its own test.

### D3, "a refused insert changes nothing" was half-tested

**Where**: `Lru::insert`'s doc comment.
**What**: the value half was pinned. The recency half was not, so bumping a live
key's tick on the refusal path was green at 256 cases and at 8192.
**Fixed**: `a_refused_insert_does_not_change_the_eviction_order` offers a live
key a value the budget cannot hold and then checks which entry leaves next.

### D4, pass 5's record stated a false mechanism for its own finding

**Where**: `.claude/reviews/F-031-implementation-pass-5.md`.
**What**: "The property test does catch it, but only at `PROPTEST_CASES=4096`
... so at the default 256 the suite is green." Re-measured with the unit
assertion removed and the regressions file cleared, the property alone went red
in **eight runs of twelve** at the default. It is a coin flip, not a threshold.
**Why it is wrong**: pass 5 saw one green run and generalised it, which is the
shape passes 2 and 4 corrected in others. The remedy it chose, a deterministic
unit assertion, is right and is load-bearing. Only the evidence was wrong.
**Fixed**: the record says what was measured.

### D5, the progress note carried a sentence pass 5's own finding falsified

**Where**: `.claude/scratch/F-031-progress.md`, "No mutation left the suite
green, so no test was added for a gap." Pass 5's defect was exactly such a
mutation, and seven of them exist by the end of this pass.
**Fixed**: the note carries the seven that were green, the test each one
produced, and the confirmation that each is now red on that test alone.

## Smells

### S1, the crate's only loop had no deterministic fixture

Separate from D1 and its root cause: nothing named the multi-eviction case, so
how many entries leave and where `used` lands after several removals were pinned
only by the property's aggregate accounting. **Fixed** by the same new test.

### S2, a proptest regressions file was left in the worktree

`crates/ocelli-cache/tests/invariants.proptest-regressions`, written by an
earlier pass's mutation run. It is gitignored, so it never reaches the tree, but
proptest replays its seeds, which makes this worktree's suite behave differently
from a clean clone's and makes a flaky property look deterministic. It is why
reproducing D4 needed it removed first. **Deleted.**

## Nitpicks

1. The scattered-order histogram in `docs/lld/cache.md` did not name its
   permutation, so it could not be re-taken, and pass 6's own scattered order
   measured a third split. **Fixed**: the permutation is named and both
   measurements are given, which strengthens the point that the split is not a
   constant.
2. Every frame fixture used one sample per pixel, so dropping `Samples per
   Pixel` from the product left the suite green. **Fixed**: the frame test also
   asserts an 8-bit three-sample frame of the same geometry at 786,432 bytes,
   and the mutation now fails it.
3. The plan's `Admission` listing omitted the ordering the code and the LLD both
   state. The recurring shape in its silent form. **Fixed.**

## Verified clean

**Fifty-three mutations**, with the green ones examined rather than counted.
Thirty-three killed by the unit suite, most of them by a single named test. Four
fixture mutations, three killed and the fourth reported above. Eight equivalent
mutants confirmed as such and left alone, which is itself evidence: the
eviction loop's `break`, the `remove_entry` fallback, both saturations,
`Vec::with_capacity`, `is_empty` by `len`, the post-increment tick, and a clock
bump on refusal are all unobservable by construction, and each has a doc comment
saying why.

**Pass 5's diff, sentence by sentence.** The key-clone measurement reproduced
exactly, one allocation with a `u32` key and two with a `String`. The `is_empty`
assertion is load-bearing. The plan's "the type asserts" rewording is right, no
signature carries that constraint. The progress note's counts agreed with the
plan and with `lru.rs`.

**Arithmetic recomputed independently**, all eight fixture numbers, and the
256-byte alignment that makes 300 pad to 512.

**Normative text diffed against `docs/hld/`**: section 20's block and four
bullets, section 8's two paragraphs, section 7's bricking bullet, exact
including the multiplication sign.

**Overflow, termination, structure and the boundary** re-checked from scratch
and unchanged: no `as` cast, no panic route, no indexing, no `#[allow]`, no
`unsafe`, no `std::` in `src/`, no `wasm-bindgen`, no `wgpu`, no pixel type.

**Commands, exit codes read from the command itself.** `test`, `clippy`, `fmt`
and eight gates all exit 0.
