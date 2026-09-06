# F-X010, CI floor equivalence, and the identical --sprint and --all gate profiles

**Status**: approved
**Epic ref**: Y1.5
**Sprint**: S04
**Estimate**: 2w

## Normative source, transcribed

_Transcriptions preserve the words in the HLD, with em-dashes and prose
semicolons normalised because plans are prose-checked. The tracked HLD remains
authoritative._

### `docs/hld/08-validation-architecture.md`, section 11

> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture. In production, shadow mode renders both libraries and
> alerts on divergence - the oracle running against real clinical traffic,
> and the same corpus a regulatory submission would want to see.

### `docs/hld/12-workspace-and-build.md`, section 15.3

> Decision D2 is worthless unless it is enforced. This runs on every pull
> request.

### `docs/hld/DEVIATIONS.md`, D-04 compensating controls

> Outside the bootstrap exception below, `/verify` runs the oracle locally
> and `push` is refused without it.

> CI asserts the ledger, without a GPU. The CI floor re-reads the ledger entry
> for the pushed head and fails when it is missing, stale or red.

> A GPU corpus run is available on manual dispatch for a release or when a
> divergence is suspected, so the expensive path exists and is simply not
> automatic.

> `--sprint` and `--all` are now the same set of gates.

## What the specification does not cover

The HLD does not define command-level equivalence between a named gate and a
CI job that invokes its constituent commands. It also does not require two
different command-line spellings for the same complete gate set.

## Approach

Keep the per-area CI jobs because their failure names are useful. Preserve the
existing exact argument-vector comparison, complete-arm check, and explicitly
stronger addition list. Close the remaining ordering gap by requiring a CI
step to invoke `bin/ocelli.sh gate NAME` whenever that gate arm contains more
than one executable command. A named runner invocation preserves the arm's
`&&` ordering and exit semantics while retaining the surrounding job's useful
name. Single-command arms may continue to use exact direct commands. Continue
to fail closed on shell or YAML shapes the reader cannot prove. Add focused
probes for a multi-command arm split across steps, split across jobs, or
reordered, plus a control where the named gate runs inside the same per-area
job.

Collapse the duplicate `--sprint` and `--all` selector bodies in
`bin/ocelli.sh` into one shared case arm. Retain both public spellings because
the workflow and release commands communicate different intent even while
their gate sets are identical. Add a reader test asserting both spellings
select the same declared set through the one implementation path.

Anticipated write set: `scripts/ci_floor_check.py`,
`scripts/tests/test_guard_readers.py`, `bin/ocelli.sh`,
`scripts/guards/catalogue.py`, `ci/guard-probe-budget.json`, and
`docs/lld/guards.md`.

Two measured corrections joined that set during implementation:

- `.github/workflows/ci.yml` must replace the direct backlog commands with a
  named gate invocation. The guard cannot adopt the new rule while its real
  control tree still uses the state that rule refuses.
- `docs/runbooks/guard-verification.md` is generated from the catalogue. The
  split-step probe was reshaped and reorder plus split-job probes were added,
  so the runbook table must be rendered in the same change.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Multi-command arms count only through a named gate invocation, while single-command arms retain exact argv equivalence | `scripts/tests/test_guard_readers.py` |
| guard mutation | Splitting or reordering a multi-command arm makes the CI gate red | `scripts/guards/catalogue.py` through `scripts/guard_probe.py` |
| shell | `--sprint` and `--all` select the same complete gate set from one case arm | runner tests and `bin/ocelli.sh gate --list` readers |

## Parity surface covered

None.

## Deviations

D-04, already recorded. No new deviation.

## LLD impact

Update `docs/lld/guards.md` with the exact equivalence rule and the shared
complete-profile selector.

## Open questions

None. The plan keeps the useful CI job split and makes equivalence mechanical.
