# F-031 review, pass 9

**Reviewed**: the staged tree after pass 8's remediation,
`c16e658accb2999eb70e14d6edaf6b3b2a8a44a7`.
**Result**: 1 defect, 0 smells, 1 nitpick

The reviewer was independent of the author, applied pass 8's exact mutant, tried
twenty-four more, ran the floor gate and the eight-gate set, and left the tree
unmutated.

## Defects

### D1, a uniqueness claim that the property suite falsifies in ten cases

**Where**: `docs/lld/cache.md`, the plan's new test row, and the pass 8 record,
all saying the new test is the only one that replaces a live key while the cache
is under pressure.
**What**: both proptest properties reach that state routinely. Their keys are
drawn from eight values against a budget up to two thousand, so a replacement
under pressure is common rather than rare.
**Why it is wrong**: the charitable reading, that "test" means "deterministic
test", is closed by a sentence eight lines above in the same document that says
"the unit and property suites both pin it". Within that document "test" already
includes the properties.
**Evidence**: a probe asserting the state never arises failed after ten
successes at the default case count, with a minimal input of three inserts. The
strict `used == budget` reading is rarer, arising three times in two thousand
cases and not at all in the default two hundred and fifty-six, which is a margin
rather than a requirement.
**Fixed by deleting the claim rather than explaining it**, which is what the
review command asks for. The LLD and the plan row now state the property that is
true and was measured: deferring the release keeps the accounting correct,
evicts a stranger, and fails that one test. The pass 8 record says the property
suite does reach the state and does not notice the mutant, and why.

## Smells

None.

## Nitpicks

1. Reading `V::bytes()` a second time at the admission site, in place of the
   value already computed, survives the suite. It is not provably equivalent, a
   `V` that changed between two reads inside one statement would desynchronise
   the accounting, but that is a stranger value than the one the doc comment
   claims to survive, which is a `V` that changes **after** admission, and all
   three post-admission sites are covered. **Left as it is**, recorded so a
   later pass knows it was weighed.

## Verified clean

**Pass 8's smell is genuinely closed.** Its exact mutant, the deferred release
of the displaced bytes, fails `replacing_a_live_key_at_a_full_budget_evicts_nothing`
and nothing else, at the assertion that `evicted` is empty, with the other
twenty lib tests passing. The integration suites passed the mutant in five runs
of five at 4096 cases, which is the reason the unit test had to exist.

**The new test is correct by hand-derivation from the rule.** `filled()` leaves
three 100-byte entries at ticks 0, 1 and 2 with `used` at the whole budget.
Replacing one releases 100 first, so the loop's test is 100 against 100 and
fails, nothing is evicted, and `used` returns to the budget. Every assertion is
the rule rather than an accident, and the two that separate "evicted nothing"
from "evicted a stranger" are the length and the survivors. An equal-sized
replacement is the right minimal case, because a larger one would legitimately
need to evict.

**Every count reproduces.** Twenty `unit` rows against twenty tests in
`lru.rs`, four tests in `budget.rs` over three `fixture` rows, two `property`
rows against two proptest functions, twenty-one in the lib binary with F-001's
scaffold test, and twenty-seven over three test binaries. Cargo reports a fourth
target, the doc tests, which compiles no binary and runs none. Every record in
this chain that says "four binaries" means those four targets.

**Twenty-four further mutations**, thirteen killed and ten of the eleven
survivors proved equivalent with a reason given for each rather than a shrug.
Two findings worth keeping: the `const {}` assertion in `budget.rs` kills a
256 MB for 256 MiB substitution **at compile time**, so it is a live guard, and
deleting `release`'s `debug_assert!` is inert alone but combined with releasing
the wrong bytes it leaves the deterministic suite green, so the assertion and
the property test are two independent guards on the same rule and neither is
decorative.

**Arithmetic re-derived.** All eight fixture constants, and the tightest wasm32
intermediate, `512 * 512 * 600 * 16 = 2,516,582,400`, is under `u32::MAX`.

**Commands, exit codes read from the command itself.** `test` exit 0 with 27
tests, `clippy` exit 0, `gate --floor` exit 0 with 22 passed
and 4 environmental skips, and `gate prose unsafe nostd deviations bindgen pins
content backlog` exit 0 over eight gates with 24 deviations and every citation
resolving.
