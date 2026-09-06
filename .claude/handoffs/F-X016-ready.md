# F-X016 ready, adapter fallback after device creation failure

**Branch**: work/f-x016-codex
**Base**: 1e186479476431f98b1e711f0584df6aa2d80293
**Head**: ad957008e5afdb9a55a85894ef689353a3917a09, the reviewed and verified feature commit. The branch tip is one handoff-only commit above it because this file cannot name the commit that contains it.
**Review**: `.claude/reviews/F-X016-adapter-fallback-pass-2.md`, microscope pass 2, clean with 0 defects, 0 smells and 0 nitpicks. Pass 1 is retained at `.claude/reviews/F-X016-adapter-fallback-pass-1.md` and its one defect was remediated before pass 2.
**Verify tree**: e9b408e18bcde64ccc28f0598da2550279e0fd5d, feature profile with 21 floor gates passed, four prerequisite-dependent floor gates skipped, and `corpus=absent`, recorded with `Ocelli-Generated-By: codex`.

## What landed

Tier resolution ranks every candidate and attempts them in preference order
until one device opens. All A candidates precede B candidates. Device type
breaks ties within a tier, and enumeration order breaks exact ties.

Every failed device request retains the adapter facts and wgpu diagnostic
text. `NoDevice` now means every candidate was attempted and failed. A later
success carries the failed attempts that preceded it.

Classification, reported limits and override construction use only the
adapter whose device opened. A failed A adapter cannot make tier A
constructible after a B fallback opens. Probing stops after the first success,
then the existing benchmark and software-renderer rules classify that adapter.

The production-used asynchronous attempt loop has deterministic tests for a
failed first candidate followed by a successful second candidate and for
all-failed exhaustion. It adds no mock trait.

## Files touched

- `.claude/reviews/F-X016-adapter-fallback-pass-1.md`
- `.claude/reviews/F-X016-adapter-fallback-pass-2.md`
- `crates/ocelli-native/src/lib.rs`
- `crates/ocelli-render/src/caps.rs`
- `crates/ocelli-render/src/lib.rs`
- `crates/ocelli-render/src/probe.rs`
- `crates/ocelli-render/tests/classify_is_total.rs`
- `docs/lld/tier-resolution.md`
- `docs/sprints/BACKLOG.md`

The native file is outside the anticipated plan write set because its public
`TierSignals` test fixture had to move to the new `ProbeOutcome` shape. The
backlog change is the feature-start transition from `pending` to
`in-progress`. No sprint total or completion record is changed here.

## Review and mutation evidence

Pass 1 found that the new tests constructed outcomes directly and did not
exercise continuation in the production attempt loop. The remediation moved
the ordering, continuation, first-success termination and exhausted
termination into the production-used `attempt_candidates` function.

Inserting an early `break` after the first failed attempt made both focused
loop tests exit 101. Each observed only candidate `[0]` instead of `[0, 1]`.
The mutation was reverted, both tests passed, and microscope pass 2 returned
clean.

The earlier feature mutations also went red independently. Reversing candidate
order, dropping the first failed record, and allowing a failed A adapter to
construct an A override after B opened each failed its focused test and was
reverted.

## Verification

`bin/ocelli.sh gate --floor` exited 0 with these 21 gates green:

`fmt`, `clippy`, `test`, `bindgen`, `unsafe`, `pins`, `nostd`, `errors`,
`panic`, `bench`, `provenance`, `prose`, `content`, `backlog`, `deviations`,
`skills`, `wasm`, `native`, `device`, `ci`, `guards`.

The same run named `lint`, `types`, `packages` and `corpus-tests` as skipped
because their prerequisites were absent. They are not recorded as passed.

`bin/ocelli.sh gate corpus` verified coverage over 91 manifest rows, including
44 real rows and all 16 transfer syntaxes, then exited 1 because all 91 ignored
corpus files are absent from this worker tree. The feature profile permits and
records that state as `corpus=absent`.

The committed feature tree carries:

```text
Ocelli-Verify: profile=feature gates=backlog,bench,bindgen,ci,clippy,content,deviations,device,errors,fmt,guards,native,nostd,panic,pins,prose,provenance,skills,test,unsafe,wasm corpus=absent tree=e9b408e18bcd
Ocelli-Generated-By: codex
```

## Integrator notes

This prepare pass does not update `AS_BUILT.md`, `SPRINT_TRACKER.md`, sprint
totals, sprint state, `CHANGELOG.md` or the backlog row to `done`. Those remain
integrator work. Nothing was pushed or integrated.
