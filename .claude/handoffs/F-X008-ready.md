# F-X008 ready, one parity target and packaged wasm licences

**Branch**: work/f-x008-codex
**Base**: deb7a3e, the worker branch base.
**Head**: 6bab1a06981001ae4f950b25491def642e6254a8, the reviewed feature
commit. The branch tip is one commit above it and adds only this handoff file,
because a file cannot carry the hash of the commit containing itself. Merge the
tip.
**Review**: `.claude/reviews/F-X008-parity-licence-pass-3.md`, the third
independent microscope pass, reports 0 defects, 0 smells and 0 nitpicks. Passes
1 and 2 are included in the feature commit with their remediations.
**Verify tree**: cf9a2d087be77b226d8f0b72122d766eb1461fdf, recorded with
`profile=feature`, `corpus=pass` and `Ocelli-Generated-By: codex`.
**Files touched**: `.agents/skills/parity/SKILL.md`,
`.claude/commands/parity.md`, all three F-X008 reviews, `bin/ocelli.sh`,
`ci/guard-probe-budget.json`, the three licence links under
`crates/ocelli-wasm/`, `docs/lld/build-targets.md`, `docs/lld/guards.md`,
`docs/lld/oracle.md`, `docs/runbooks/guard-verification.md`,
`docs/sprints/BACKLOG.md`, `scripts/gen_sprint_plan.py`,
`scripts/guard_probe.py`, `scripts/guards/catalogue.py`,
`scripts/guards/sandbox.py`, `scripts/pin_and_size_check.py`,
`scripts/tests/test_guard_catalogue.py`,
`scripts/tests/test_pin_and_size_check.py`, and
`tools/oracle/tests/pins_test.mjs`.

## What landed

The exact 5.8.2 package pins remain the executable parity authority. The
canonical parity command, generated Codex adapter, sprint-plan generator
preamble and hand-curated current sprint plan now name 5.8.2 and D-11. The
consumer test opens all three operational documents so any one drifting to the
nonexistent 5.8.9 target is a named failure.

The wasm crate carries relative links to the repository `LICENSE`,
`LICENSE-MIT` and `LICENSE-APACHE` files. The sandbox recreates a relative link
only when it resolves inside both the source repository and destination
sandbox, and refuses absolute links or an escape from either root. The two
escape fixtures use different source and destination depths, so each
containment condition is watched independently.

The generated wasm package must contain regular MIT and Apache files whose
bytes match the repository originals. A resolving symlink is refused even when
it reaches the correct grant. Focused tests separately cover a missing source,
a missing package entry, a packaged symlink and differing bytes. Catalogue
probes drive the missing-entry and resolving-symlink branches through the real
`--with-size` command.

## Review and mutation evidence

- Pass 1 found the sandbox escape. Absolute and relative escapes were then
  refused and mutation-tested.
- Pass 2 found four independent coverage defects. Each protection was disabled
  alone and only its matching test failed: the current sprint consumer, source
  containment, destination containment, regular package files, and the absent
  repository grant diagnostic.
- Pass 3 re-read the twice-remediated staged tree and found no defect, smell or
  nitpick.
- The guard census records 630 refusals in 61 files. The new package-symlink
  refusal has a real catalogue probe, and the generated runbook matches the
  catalogue.

## Verification

`bin/ocelli.sh gate --floor` exited 0. It reported 24 passed gates and one
declared `corpus-tests` prerequisite skip in the disposable worktree. The
tooling suite's standard-library half ran 19 tests with no failure, error or
skip.

`bin/ocelli.sh gate corpus` then ran with the documented `OCELLI_PYTHON`
interpreter and the clone's existing ignored corpus store. It verified all 91
manifest rows, with none missing or mismatched, and the metadata audit agreed
on modality, transfer syntax and tolerance class. No corpus file was copied,
tracked or added to a prompt.

The ledger records the gates that actually ran, including the floor's
`corpus-tests` invocation and the separate passing `corpus` gate. The feature
commit carries the hook-generated trailers for that exact tree.

## Prepare-only boundary

No sprint totals, AS_BUILT entry, tracker row, done status, push or integration
was performed. The integrator owns the shared sprint completion records. The
ignored corpus link used for verification was removed before this handoff was
written.
