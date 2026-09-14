# F-031 review, pass 8

**Reviewed**: the staged tree after pass 7's remediation,
`8cff2976a8106c69e543a3163091afe8a473d309`.
**Result**: 0 defects, 1 smell, 1 nitpick

The reviewer was independent of the author, tried thirty-six mutations, ran the
floor gate, and left the tree unmutated. **Every claim pass 7 made reproduced,
including the two most likely to be loose**, the twelve-of-twelve reordering
measurement and the named permutation's histogram.

## Defects

None.

## Smells

### S1, the one rule in `insert`'s doc comment that no test guarded

**Where**: `Lru::insert`'s doc comment, "a repeated key releases its previous
value before eviction begins, so the incoming value never competes with the
value it replaces".
**What**: a finer mutant than any earlier pass tried. Keep the removal where it
is and defer only the release until after the eviction loop. The net accounting
is unchanged, so every invariant still holds and only the eviction count moves.
Green in thirteen runs of thirteen, ten at the default case count and three at
8192.
**Why it is not equivalent**: with three 100-byte entries in a 300-byte budget,
replacing one of them evicts nothing and ends at three entries. The mutant
evicts a stranger and ends at two.
**Why the property test cannot see it**: its model is driven by what `insert`
reports, so an over-eviction is reported faithfully and the model follows it.
The gap was that **no deterministic test replaced a live key while the cache was
under pressure**. The two that replace a key both had room to spare. The
property tests do reach that state, within ten cases, and do not notice the
mutant for the reason above, which pass 9 measured and this paragraph originally
overstated as no test reaching it at all.
**Why it matters**: on the GPU tier a stranger is a texture discarded that did
not need to go, and nothing would have said so.
**Fixed**: `replacing_a_live_key_at_a_full_budget_evicts_nothing`, which fills
the budget and replaces one entry. The mutant now fails that test alone, and
`docs/lld/cache.md` states the rule beside the refusal rule it mirrors.

## Nitpicks

1. `would_admit`'s zero-budget guard mutated from `budget > 0` to `budget > 1`
   survives at 8192 cases. It is not equivalent, because a budget of one byte
   behaves differently, but a one-byte tier is not a configuration anything
   creates, and the guard's two real boundaries, zero and the budget itself, are
   each killed by their own test. **Left as it is**, recorded here so a later
   pass knows it was considered rather than missed.

## Verified clean

**The restructured order test is correct by the rule**, worked out by hand:
`filled()` leaves ticks 0, 1 and 2, `get(&"b")` moves b to 3, and an insert
needing 200 bytes evicts a then c. `[a, c]` follows from least recently used
first and is not `BTreeMap` key order, which is `[a, b]`. All three order
mutations are red on it, and the pre-remediation form was reconstructed to
confirm that map-order eviction used to pass it, which is what pass 7 claimed.

**The three `Shifting` assertions do what pass 7 said.** Making `restate` a
no-op, and making `Shifting::bytes` a constant, are each red on all three
read-once tests now and were each green on the pre-remediation tree, which
confirms the gap was real and is closed.

**Both "deterministic" sentences are true.** Instrumenting `insert` with an
assertion that at most one entry is evicted fails exactly one deterministic
test, the order test, and passes the whole fixture suite, while the property
test trips it in six runs of six. The property test iterates `evicted` and
asserts membership only, never order, which is what the LLD now says.

**Every number in the pass 7 diff reproduced.** Ascending 34, 5 and 1,
descending the same, `(i * 37) % 101` at 35, 4 and 1, and `(i * 3) % 41` at 36,
3 and 1, stable across key types. All eight fixture constants recomputed by hand
including the new 786,432. The series product fits a `u32`, so it is safe on
wasm32.

**Thirty-six mutations**, thirty killed, four proved equivalent and named, two
surviving and both reported above. Coverage included both `would_admit`
operators, the loop boundary, a single-eviction loop, `bytes: 0`, `used += 0`,
`release(0)` at all three sites, `displaced = None`, a frozen clock, `get` not
ticking, `max_by_key`, map-order eviction, `len` off by one, `is_empty` by
`used`, a hardcoded tier, swapped `free()` operands, an empty `evicted` push,
both phase reorderings, and four fixture mutations.

**Commands.** `bin/ocelli.sh test ocelli-cache` exit 0, `clippy` exit 0, and
`gate --floor` exit 0 with 22 passed and 4 environmental skips.
