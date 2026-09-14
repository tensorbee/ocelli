# F-031 ready

**Branch**: `work/f-031-claude`
**Base**: `3b00890488f586af40ac4924004e32d3d387e0ee`
**Head**: `27c3a76b48be8b28a3d2f03bc2125a864061e8f6`
**Files touched**: `crates/ocelli-cache/{Cargo.toml,src/lib.rs,src/lru.rs,src/tier.rs,tests/budget.rs,tests/invariants.rs}, Cargo.lock, docs/lld/{README.md,cache.md}, docs/hld/DEVIATIONS.md, .claude/plans/F-031-design.md, .claude/reviews/F-031-implementation-pass-{1..10}.md`
**Review**: `F-031 implementation review pass 10, clean with zero defects and zero smells`
**Verify tree**: `6404ff2e8abaa07ffd88e26979c1f0ae76942ab2`

## What the integrator needs to know

**Two shared files are touched and both are additive.** `docs/lld/README.md`
gains one row at the end of its index table. `docs/hld/DEVIATIONS.md` changes
inside row D-24 only, one clause in the behaviour column and one in the
rationale column, because the implemented refusal condition is `would_admit`,
which is `bytes` above the whole budget **or** a budget of zero, and the row as
approved named only the first half. No other row is touched and the count stays
at 24.

**No sprint ledger was touched.** `CURRENT_SPRINT.md`, `BACKLOG.md`,
`SPRINT_TRACKER.md`, `AS_BUILT.md` and `CHANGELOG.md` are unmodified, and sprint
state was not written, because this worktree has none. The backlog row is still
`pending` and the sprint state still says `claimed`.

**The verification is `profile=feature` with `corpus=absent`**, which is honest
rather than convenient: `corpus/data` is gitignored, so a worker worktree has no
corpus to check. `gate --floor` is green at the verified tree, 22 passed and 4
skipped, and all four skips are absent prerequisites the runner names, an
absent `node_modules` for `lint`, `types` and `packages`, and an unimportable
pydicom for `corpus-tests`. The canonical worktree has both, so the sprint
verification will cover them.

**The story ran ten review passes**, each by a reviewer independent of the
author. Pass 10 is clean. The chain is worth reading for one reason: passes 1 to
5 read the tests and agreed they looked thorough, and pass 6 ran a mutation
catalogue instead and found four documented rules that no test would have
noticed breaking. Nine mutations left a green suite across the ten passes, and
nine tests exist because of them.
