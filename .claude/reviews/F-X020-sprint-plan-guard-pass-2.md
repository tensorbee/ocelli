# F-X020 review, pass 2

**Reviewed**: working tree against `1e18647`, including pass-1 remediation
**Result**: 0 defects, 0 smells, 0 nitpicks

## Pass-1 finding

Pass 1 found that `scripts/tests/test_gen_sprint_plan.py` was not executed by a
required gate. That defect is closed.

`bin/ocelli.sh gate guards` now invokes the generator suite by its exact file
pattern. `guards` is part of `--floor`, `.github/workflows/ci.yml` invokes the
gate directly, and `scripts/ci_floor_check.py` confirms that every command in
the arm is required on pull requests and pushes. A clean gate run visibly ran
the four generator tests between the 49 catalogue tests and the 71 reader
tests.

## Adversarial mutation

I changed the production `--check` branch temporarily so that a successful
check rewrote the checked plan before returning success. The catalogue probes
still reached their declared outcomes, then the required `guards` gate failed
in:

```text
test_check_reads_explicit_paths_without_rewriting_the_plan
AssertionError: b'mutation rewrote the checked plan\n' != <original bytes>
```

The command exited 1 with `FAILED guards`. This proves the new suite is not
merely present in the arm text and that its read-only assertion contributes to
the gate result. I restored the production file exactly, confirmed there was
no unstaged diff, and reran the four tests green.

## Full feature inspection

- A bare invocation checks for an existing destination before rendering or
  writing it, returns 1, names the protected path, and names `--check` and
  `--force` as the explicit modes.
- An absent destination still takes the bootstrap path. `--force` is the only
  mode that replaces an existing destination. Argparse makes it mutually
  exclusive with `--check`.
- `--check` reads the injected plan path and returns the existing structural
  checker result without writing.
- The four temporary-directory tests preserve and compare bytes rather than
  inferring non-mutation from an exit status.
- The new standing probe plants prose that allocation data cannot reconstruct,
  requires the bare writer to refuse it, and uses `--force` as the green
  control. The unit suite independently checks the replacement bytes, so a
  no-op control cannot substitute for deliberate regeneration.
- The sprint-plan probe budget moves from 14 to 15. The generated runbook and
  the catalogue agree, and the census remains green.
- The dependency-injection seam adds no trait, generic parameter, forwarding
  wrapper, patient data, unsafe code, pixel arithmetic, runtime hot path, wasm
  boundary or tolerance change.

The actual tracked `SPRINT_PLAN.md` had SHA-256
`c4b737b75d9aa4f243a23322d690a9daa12a2cb386f310ecd15146c20baad94b`
before and after a bare invocation. The invocation exited 1 with the protected
path and both explicit alternatives in its output.

## Commands and exit codes

| Command | Exit | Evidence |
|---------|------|----------|
| `python3 -B -m unittest discover -v -s scripts/tests -p test_gen_sprint_plan.py` | 0 | 4 tests passed |
| `bin/ocelli.sh gate guards` | 0 | 1 gate green, 147 refusal probes, 37 controls, generator suite ran 4 tests |
| `bin/ocelli.sh gate guards` with the temporary production rewrite | 1 | read-only generator test failed, gate reported `FAILED guards` |
| `python3 scripts/guard_census.py --check-runbook` | 0 | 623 refusal sites claimed, 233 probes, runbook current |
| `python3 scripts/gen_sprint_plan.py --check` | 0 | 177 planned F-IDs, 18 milestone summaries and 72 goals agree |
| `python3 scripts/ci_floor_check.py` | 0 | all 25 floor gates and every arm command are invoked by CI |
| `python3 scripts/gen_sprint_plan.py` | 1 | existing plan refused and its digest stayed unchanged |
| `bin/ocelli.sh gate backlog prose` | 0 | both gates green |
| `git diff --cached --check` | 0 | no whitespace errors |

The `guards` output still names the pre-existing declared G-04 handoff defect.
It is unrelated to F-X020 and was not introduced or changed by this feature.
