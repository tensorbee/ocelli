# F-031 review, pass 5

**Reviewed**: the staged tree after pass 4's remediation,
`a692e408241330d0a224792bc9f05fef3754631c`.
**Result**: 2 defects, 1 smell, 3 nitpicks

The reviewer was independent of the author, swept the recurring shape
mechanically with `///` and `//!` wraps flattened before searching, ran the
whole floor gate, and left the tree unmutated.

**`bin/ocelli.sh gate --floor` exit 0 at this tree**, 22 passed and 4 skipped.
All four skips are environment prerequisites the runner names, `lint`, `types`
and `packages` for an absent `node_modules`, and `corpus-tests` for an
unimportable pydicom. None is F-031's and none is a pass reported as one.

## Defects

### D1, the one test gap the five passes had not found

**Where**: `Lru::is_empty` in `crates/ocelli-cache/src/lru.rs`.
**What**: changing it from `self.entries.is_empty()` to `self.used == 0` left
the whole suite green. That is a real behavioural change: a cache holding a
zero-byte entry would report itself empty, which is exactly the case
`docs/lld/cache.md` spends a paragraph on.
**Why it is wrong**: HLD 27.2 R2 and this repository's mutation rule. A method
whose mutation no test notices is coverage that is counted forever and means
nothing.
**Evidence**: the mutation applied, `cargo test -p ocelli-cache` exit 0, 21
passed. **The property test catches it sometimes**, which is worse than never:
pass 6 re-measured it at the default 256 cases and got red in eight runs of
twelve. Pass 5 recorded that as a case-count threshold, 4096, from one green run
at the default. It is a coin flip, and the reachable case needs a zero-byte
entry drawn while no other live entry has non-zero bytes.
**Fixed**: one assertion, `assert!(!cache.is_empty())`, in
`a_tier_with_a_budget_admits_an_entry_of_no_bytes`, which is the only state in
which `used == 0` and `is_empty()` part company. Re-applying the mutation now
fails that test alone: 14 passed, 1 failed.

### D2, pass 4's rewrite of the progress note introduced a new wrong count

**Where**: `.claude/scratch/F-031-progress.md`.
**What**: "the fifteen unit tests the plan's test table lists". The table lists
fourteen and `lru.rs` holds fourteen. Fifteen is the lib binary's count, which
includes `src/lib.rs`'s scaffold test from F-001.
**Why it is wrong**: it is the recurring shape again, introduced by the
remediation whose purpose was to fix a wrong count, in the file pass 4 itself
called the one every downstream ledger is read from.
**Fixed**: the note says fourteen, and says why the binary reports fifteen.

## Smells

### S1, the eviction measurement generalised from one key type

`docs/lld/cache.md` said an evicting insert allocates once, for the `Vec` push.
`evict_least_recently_used` also clones the victim's key, so a key that owns
memory costs one more. Measured at one with a `u32` key and two with a `String`
key. That is the same generalise-from-one-measurement shape passes 1 and 2
already corrected once.

**Fixed**: the LLD carries both numbers and the reason, and
`evict_least_recently_used`'s doc comment says what the clone is for, which is
ending the scan's borrow of the map before the removal, and why `K: Clone` is on
the type. It matters for F-032 and F-033, whose keys are frame identities and
are unlikely to be `Copy`.

## Nitpicks

1. `docs/hld/DEVIATIONS.md` D-24 carries the fifth instance of the sentence pass
   4 corrected elsewhere: "an insert that fits with nothing displaced returns
   three empty fields and `Vec::new` does not allocate". **Left as written**,
   deliberately. Parsed strictly it claims only that the widening costs the
   common path nothing and that the returned value is free, both of which are
   true, and it is a normative register row where an edit for a nitpick is a
   worse trade than the ambiguity. Recorded here so a later pass does not read
   it as the defect recurring.
2. A ragged wrap left in `docs/lld/cache.md` by pass 4's own reflow.
   **Reflowed**, and the paragraph split in two where it had grown two subjects.
3. The plan said `refused` and `evicted` being mutually exclusive is something
   "the type asserts". The type asserts nothing. **Corrected** to say the order
   inside `insert` guarantees it and both suites pin it, which is what
   `docs/lld/cache.md` already said.

## Verified clean

**The recurring shape, swept mechanically** across all four earlier records, the
plan, `docs/hld/DEVIATIONS.md`, `docs/lld/cache.md`, `docs/lld/README.md`, all
three `src/` files, both test files, `Cargo.toml` and the progress note, with
doc-comment line wraps flattened first. Allocation claims, the refusal
condition, what `Admission` carries, test counts, mutation counts and per-pass
findings agree everywhere except the two places named above.

**The four earlier records reproduce against the trees they name.** Each
headline result line matches its own sections. The mutation arithmetic
reconciles: six in the plan, nine by pass 1, ten by pass 2 with the progress
note's table carrying exactly ten rows, and three more from pass 4.

**The plan's test table, row by row and by name.** Fourteen `unit` rows against
fourteen tests in `lru.rs` in order, three `fixture` rows against four tests in
`budget.rs` because the first row states two things, two `property` rows against
two proptest functions. No orphan test and no unimplemented row.

**Every HLD quotation verified character for character** after flattening:
section 8's sentences in `tier.rs` and the plan, section 7's bricking bullet in
`budget.rs` including the multiplication sign, and section 20's bullets and
`insert` comment in the plan.

**Fixture arithmetic recomputed by hand once more**, all eight numbers, and the
compile-time assertion on the 46,137,344 the series is over by.

**Substantive re-check from scratch.** Zero `as` casts, zero `unwrap`,
`expect`, `panic!`, `unreachable!`, `todo!`, zero indexing, zero `#[allow]`,
zero `unsafe`, zero `std::` in `src/`. `used` correct on all five mutating
paths. `self.used += incoming` cannot overflow. Both saturations are unreachable
under the invariant and the `debug_assert` is stripped from the release profile.
The eviction loop terminates on every input and its break is unreachable.
`no_std` is compiled rather than declared, because the integration tests link
the non-`cfg(test)` build. No `wasm-bindgen`, no pixel type, no `wgpu`, no
device, and all three tiers declared n/a with a reason.

**A second fresh mutation**, moving the repeated-key removal below the eviction
loop, is caught by `returned_entries_leave_and_the_rest_stay` alone, on the
assertion that an eviction is never the incoming key.

**Structural rules.** The trait question is answered in the plan and the same
reasoning covers the generic parameter. No `Box<dyn>`, no forwarding wrapper, no
feature flag. Nine public methods, five fields, `Entry<V>` private, nothing
public the plan did not authorise.
