# F-X001 ready, the feature availability contract

**Branch**: work/f-x001-codex
**Base**: 0eae7420b9104c44d44b97206794da350f09ef88
**Head**: d48e7a825e46d7b10365538d11434be4aace1f37, the reviewed and verified feature commit. The branch tip is one handoff-only commit above it because this file cannot name the commit that contains it.
**Review**: `.claude/reviews/F-X001-feature-availability-pass-1.md`, microscope pass 1, clean with 0 defects, 0 smells and 0 nitpicks.
**Verify tree**: d10123642b4f8d4d8450e32ffbc627f1d8551b7a, feature profile with 21 floor gates passed, four prerequisite-dependent floor gates skipped, and `corpus=absent`, recorded with `Ocelli-Generated-By: codex`.

## What landed

`docs/lld/feature-availability.md` defines `Available`, `Degraded` and
`Unavailable` as the three answers every tier-gated feature declares.
Required and resolved tiers remain distinct roles. Only `Degraded` carries a
named fallback, and that path must preserve the specified result rather than
quietly substituting a different answer.

The contract starts from the existing F-004 and F-X016 tier resolution,
software evidence and adapter fallback. It reuses `ErrorCode::Unavailable`
700 and the in-process `ComputeError::Unavailable` example. It adds no second
resolver or error implementation.

Unavailable features remain visible in the shell with the feature, missing
capability, resolved tier and one truthful action named. Diagnostic adapter
evidence remains separate from user-facing text.

Stable feature identity, numeric tier encoding and boundary operands remain
F-101's work when the producer and consumer exist. F-X005 applies this
contract to volume rendering. No speculative registry, production code, shell
implementation, LUT path or pixel arithmetic was added.

## Files touched

- `.claude/reviews/F-X001-feature-availability-pass-1.md`
- `docs/lld/README.md`
- `docs/lld/errors.md`
- `docs/lld/feature-availability.md`
- `docs/lld/gpu-ownership.md`
- `docs/lld/tier-resolution.md`
- `docs/sprints/BACKLOG.md`

The backlog change is the feature-start transition from `pending` to
`in-progress`. No sprint total or completion record is changed here.

## Review and tests

Microscope pass 1 returned clean. It confirmed that the three states are
distinct, fallback is confined to `Degraded`, existing mechanisms keep their
single owners, no stable operand or registry was invented, and all LLD links
and contribution metadata agree.

The approved plan's code-test rows are conditional on stable operands. They do
not apply because this story deliberately introduces none. The story performs
no pixel or geometry arithmetic, so no fixture applies.

## Verification

`bin/ocelli.sh gate --floor` exited 0 with these 21 gates green:

`fmt`, `clippy`, `test`, `bindgen`, `unsafe`, `pins`, `nostd`, `errors`,
`panic`, `bench`, `provenance`, `prose`, `content`, `backlog`, `deviations`,
`skills`, `wasm`, `native`, `device`, `ci`, `guards`.

The same run named `lint`, `types`, `packages` and `corpus-tests` as skipped
because their prerequisites were absent. They are not recorded as passed.

`bin/ocelli.sh gate corpus` verified coverage over 91 manifest rows, including
44 real rows and all 16 transfer syntaxes, then exited 1 because all 91 ignored
corpus files are absent from this worker tree. The feature profile records that
state as `corpus=absent`.

The committed feature tree carries:

```text
Ocelli-Verify: profile=feature gates=backlog,bench,bindgen,ci,clippy,content,deviations,device,errors,fmt,guards,native,nostd,panic,pins,prose,provenance,skills,test,unsafe,wasm corpus=absent tree=d10123642b4f
Ocelli-Generated-By: codex
```

## Integrator notes

This prepare pass does not update `AS_BUILT.md`, `SPRINT_TRACKER.md`, sprint
totals, sprint state, `CHANGELOG.md` or the backlog row to `done`. Those remain
integrator work. Nothing was pushed or integrated.
