# F-031 review, pass 10

**Reviewed**: the staged tree after pass 9's remediation,
`3e5ec5f168fa03c8fcfc9ede7d570041f9d18b5e`.
**Result**: 0 defects, 0 smells, 2 nitpicks

**Clean.** Ten passes, every one by a reviewer independent of the author, every
finding remediated before the next pass, and no finding surviving from any
earlier pass.

## Defects

None.

## Smells

None.

## Nitpicks

Both were corrected after the pass, in the pass 9 record only. **No code, test,
LLD, plan or deviation text changed**, so everything this pass verified about
the tree still holds, and a reader can confirm that from the diff.

1. Pass 9's evidence read "the strict `used == budget` form failed at two
   hundred thousand cases", which invites the reading that it needed that many.
   Measured here: the strict state arises three times in two thousand cases and
   not at all in the default two hundred and fifty-six, while the loose state
   arises at the first case. **Corrected** to say what the margin is.
2. "twenty-seven over four binaries" is three test binaries plus a doc-test
   target that compiles no binary and runs none. **Corrected**, with a note that
   every record in this chain saying "four binaries" means those four targets.

## Verified clean

**The three replacement sentences are true, measured.** The mutant the LLD names,
deferring only the release of the displaced bytes until after the eviction loop,
fails `replacing_a_live_key_at_a_full_budget_evicts_nothing` and nothing else at
the default case count, and the property suite stays green at two hundred
thousand cases three times over and at one million once. "Leaves the accounting
correct and evicts a stranger" holds literally: the mutant reports `evicted` as
one stranger, `used` at 200 and `len` at 2, and 200 is exactly the sum of the two
survivors. "Nothing else" is workspace-wide, since no crate depends on
`ocelli-cache` yet.

**The false uniqueness claim is gone everywhere.** A whitespace-flattened sweep
of all 803 tracked files returns one hit, inside pass 9's own heading where it is
quoted as the defect. Every surviving sentence containing "only" was then checked
individually, and each is true: the order test is the only deterministic test
evicting more than one entry, `would_admit` is the only place the admission
condition is written, and `refused` is non-empty only when `evicted` is empty.

**Pass 9's record is true sentence by sentence**, including the "eight lines
above" claim, which was checked against the pre-remediation file, and the
generator bounds.

**Both of pass 9's findings reproduce.** Substituting 256 MB for 256 MiB fails to
compile, with `error[E0080]` from the `const {}` assertion, so that guard is
live. Releasing the wrong bytes fires the `debug_assert!`, and deleting the
assertion as well leaves the deterministic suite green with only the accounting
property red, so the assertion and the property are two independent guards on the
same rule.

**Every measured number in the LLD reproduces on this toolchain**: the four
allocation histograms, the evicting insert at one allocation with a `u32` key and
two with a `String`, and zero for every reading method on hits and misses.

**Citations resolve.** D-24's quotation of section 20 is verbatim against
`docs/hld/17-cache-and-allocation.md`, `tier.rs`'s three quotations are verbatim
against `docs/hld/06-memory-and-cache.md`, F-032, F-033, F-036 and F-040 exist in
the backlog with the roles claimed, and `docs/lld/benchmarks.md` exists.

**Commands, exit codes read from the command itself.** `bin/ocelli.sh test
ocelli-cache` exit 0 with 27 tests, `bin/ocelli.sh clippy ocelli-cache` exit 0,
and `bin/ocelli.sh gate --floor` exit 0, 22 passed and 4 skipped, every skip an
absent `node_modules` or an unimportable pydicom rather than a gate of this
story's.

## What the ten passes cost and bought

The counts fell 2, 4, 1, 3, 2, 5, 1, 0, 1, 0 in defects. **Two shapes produced
almost all of it.**

The first is a claim corrected where a pass found it and left standing in another
file carrying the same sentence. It took five passes to converge, and what
finally ended it was replacing an enumeration with a rule rather than correcting
the enumeration again.

The second is the one that matters. Passes 1 to 5 read the tests and agreed they
looked thorough. Pass 6 ran a mutation catalogue instead and found four gaps,
pass 8 found a fifth with a finer mutant, and every one of them was a documented
rule that no test would have noticed breaking. **Nine mutations left a green
suite across the chain, and nine tests now exist because of them.** A green suite
is not evidence. A mutation that survives it is.
