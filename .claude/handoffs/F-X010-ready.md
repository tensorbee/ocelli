# F-X010 ready for integration

**Branch**: work/f-x010-codex
**Base**: 408c86656755375df3ff9ec685a90b72da9437d1
**Head**: e58a3f3724f9391b8d5a075085dd301af863a224, the reviewed and verified feature commit
**Review**: `.claude/reviews/F-X010-ci-floor-equivalence-pass-2.md`, zero defects, zero smells and zero nitpicks
**Verify tree**: 588db5f942233a2c314a17a9f854e75b07ae4868, feature profile with corpus passing

## What landed

- Visible multi-command floor arms require a named gate invocation per event.
- Single-command arms retain exact argument-vector equivalence.
- Blocked event and condition paths are refused before invocation spelling is
  evaluated, preserving independent reachability checks.
- The CI workflow invokes the two-command `backlog` arm by name while retaining
  the existing area jobs and the D-04 exclusions.
- `--sprint` and `--all` share one selector path.
- Reader tests and generated guard probes cover split-step, reordered-step,
  split-job, parser-shape, reachability and named-in-area-job behavior.

## Files touched

- `.claude/plans/F-X010-design.md`
- `.claude/reviews/F-X010-ci-floor-equivalence-pass-1.md`
- `.claude/reviews/F-X010-ci-floor-equivalence-pass-2.md`
- `.github/workflows/ci.yml`
- `bin/ocelli.sh`
- `docs/lld/README.md`
- `docs/lld/guards.md`
- `docs/runbooks/guard-verification.md`
- `scripts/ci_floor_check.py`
- `scripts/guards/catalogue.py`
- `scripts/tests/test_guard_readers.py`

## Review and evidence

Pass 1 identified that the stronger named-invocation refusal masked legacy
reachability and parser probes. The remediation established blocked-path
precedence and isolated the legacy builders so each probe remains load-bearing.
Pass 2 independently reviewed that final staged tree and returned clean.

The targeted 21-probe compatibility audit passed. The reader and catalogue
suite passed 129 tests. The complete floor guard profile also passed.

## Verification

The exact staged tree passed all 25 floor gates:

`backlog, bench, bindgen, ci, clippy, content, corpus-tests, deviations, device, errors, fmt, guards, lint, native, nostd, packages, panic, pins, prose, provenance, skills, test, types, unsafe, wasm`

The separate corpus gate verified all 91 manifest rows with zero missing or
mismatched files and a green metadata audit. The verification ledger records
`profile=feature`, `corpus=pass` and generator `Codex` for the verify tree above.

## Integrator notes

This worker did not update `AS_BUILT.md`, `SPRINT_TRACKER.md`, sprint totals or
state, `BACKLOG.md`, or `CHANGELOG.md`. It did not integrate or push. The
integrator must confirm that the handoff head is the reviewed feature commit,
verify ancestry from the stated base, apply the canonical sprint bookkeeping
and consume this handoff.
