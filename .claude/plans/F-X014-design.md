# F-X014, Close the two guard holes F-X009 declared, and watch the refusals its census leaves to nothing

**Status**: approved
**Epic ref**: Y1.9
**Sprint**: S04
**Estimate**: 2w

## Normative source, transcribed

_The source uses em-dashes. They are written as hyphens here because the prose
gate covers plans. The tracked HLD remains authoritative._

From `docs/hld/24-agent-code-standards.md`, section 27:

> Given agent-assisted delivery, review bandwidth - not generation speed - is
> the binding constraint, and reviewing generated code is slower per line than
> reviewing a colleague's because intent cannot be inferred. These rules exist
> to move as much of that burden as possible onto machines.

From section 27.2:

| **#** | **Rule** | **Why** |
|----|----|----|
| R1 | Never re-type file content from tool output; edit in place with a script | Tool output truncates, and the truncation is silent |
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R6 | Provenance trailer on every commit | Cheap now; a retrofit across sixty thousand lines is not, and a device pathway may require it |

From section 27.3, with the source punctuation normalised:

> That a new test would actually fail if the code were wrong. Mutate one
> constant, re-run, confirm it goes red.

## What the specification does not cover

The HLD requires machine-enforced rules but does not define the repository
guard census, its mutation sandbox, the set of `no_std` crates, the handoff
Markdown grammar, or how benchmark-runner refusals are exercised. Those are
repository workflow decisions.

The current census reports G-02 and G-04 as declared defects, nine uncovered
sites in `bench.runner`, and six limits owned by this story. A limit that
cannot be exercised in the tracked sandbox will remain an explicit limit with
its environmental reason, but will no longer cite F-X014 as if this story
could remove that environment constraint. This does not convert a limit into
coverage.

## Approach

1. Replace G-02's self-selecting crate set with an explicit expected `no_std`
   set in `scripts/no_std_check.py`. Compare the expected set with the crates
   that declare the attribute in both directions before resolving dependency
   graphs. Deleting the attribute from one crate must therefore refuse with
   that crate named.
2. Give the handoff fields one named tuple in `scripts/sprint_workflow.py` and
   parse field values as Markdown code spans or plain text. Keep the exact
   branch-prefix check after unwrapping one matching pair of backticks. Add the
   five-field handoff template to the command documentation so the validator's
   contract is no longer implicit.
3. Export the benchmark runner's subject-execution seam without adding a
   wrapper. Add a node test that exercises every `parseArgs` refusal and the
   pending-subject runner refusal directly. Add that suite to the existing
   `bench` gate.
4. Convert the `bench.runner` catalogue entry from uncovered to standing-test
   coverage. Remove G-02 and G-04 from `DEFECTS` only after their existing
   defect probes pass. Rewrite the six irreducible limit notes to name the
   actual missing browser, install, private input, or dependency graph, with
   no fictitious story owner. Do not claim those limits are covered.
5. Record the reduced uncovered count and `sweep_complete: true` only after the
   census observes the benchmark suite. Each fix is first observed red by its
   existing probe or new test.

Anticipated implementation write set, exactly:

- `scripts/no_std_check.py`
- `scripts/sprint_workflow.py`
- `.claude/commands/complete-feature.md`
- `.agents/skills/complete-feature/SKILL.md`
- `tools/bench/run.mjs`
- `tools/bench/tests/run_test.mjs`
- `bin/ocelli.sh`
- `scripts/guards/catalogue.py`
- `ci/guard-probe-budget.json`
- `docs/lld/guards.md`
- `docs/lld/benchmarks.md`

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
| unit | One missing `no_std` declaration is refused and the full expected set is accepted | existing `nostd.loses-a-crate` guard probe and `scripts/no_std_check.py` |
| unit | A correct backticked branch is accepted and a wrong branch is refused | existing handoff guard probes |
| unit | Every benchmark argument refusal and the pending-subject runner refusal is reached | `tools/bench/tests/run_test.mjs` |
| mutation | G-02 and G-04 go red before repair, then their declared-defect probes force removal of the declarations | `scripts/guard_probe.py` |
| conformance | The census has no uncovered entry and every remaining limit states its real external constraint | `python3 scripts/guard_census.py` |

No pixel or geometry arithmetic is added, so no DICOM fixture applies.

## Parity surface covered

None. Appendix B has no row whose `Covered by` value is Y1.9.

## Deviations

None. This closes enforcement gaps without changing the HLD contract.

## LLD impact

- `docs/lld/guards.md`
- `docs/lld/benchmarks.md`

## Open questions

None.
