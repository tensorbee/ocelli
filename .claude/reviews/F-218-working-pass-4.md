# F-218 review, pass 4

**Reviewed**: worktree `ocelli-wt-f-218`, branch `work/f-218-xai`, base and HEAD `ab3d11cc7586069aa95bb02762b576acb4dc3fbb`. Staged tree `891e11f71ff0592855983021aba9e2fe46052d1d`, unchanged since pass 1. The pass 1 to 3 remediation is unstaged, plus untracked `scripts/tests/test_unreachable_injections.py`. `tools/oracle/out` is untracked and out of scope. Pin: SHA-1 of `git diff` is `f658d99be7852cbf0820e006e6d438c6b93a3429`, and of `git diff` followed by the untracked injection test is `4a4435e30b8da112ddf332780e091c0ddf71fa80`. Both are identical at the start and the end of this pass.
**Author / reviewer**: implementation by an external coding agent, remediation of pass 3 by the sprint lead (Claude). Reviewer is a fresh independent Claude agent that wrote none of the diff, the plan, the earlier passes or the pass 3 fix. The only worktree file this pass wrote is this review. Mutations were temporary edits to `scripts/package_check.py`, restored from a byte copy and confirmed with `cmp` after each run. Gate runs wrote ignored build output (`packages/*/dist`) as `gate packages` always does.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- **Pass 3 D1, D2, D3**: each fix read against its trigger, and the D3 claims executed by mutation.
- **The regenerated runbook**: compared against the staged copy with row numbers stripped, and the result traced to the catalogue diff.
- **The overclaim sweep**: every added line in `git diff ab3d11c` and the untracked test grepped for `prove`, `proves`, `proved`, `proof`, plus number words and digits in prose. Pass 3 grepped only `proved`, which is why D1 below survived it.

## Evidence

All executed now. node v24.13.0, Python 3.14.5, macOS.

| Command / mutation | Input | Exit and observation |
|--------------------|-------|----------------------|
| `guard_probe.py --only` on `packages.unreachable-trace`, `packages.smoke-requires-dcmjs`, `packages.hook-register-hooks`, unmutated | worktree | OK each, refusal red, control green |
| M3: `problems += unreachable_problems(consumer, _unreachable_rows())` deleted from `check_consumer` | temporary edit, restored, `cmp` clean | exit 1, HARNESS FAIL, "exited 0. The guard it aims at did not fire". The probe goes red, as the D3 fix now says |
| M6: `createRequire(import.meta.url)("dcmjs")` line removed from `SMOKE_SCRIPT` | same | exit 1, control `smoke_wiring_problems, healthy` exits 1, "the smoke script does not require dcmjs" |
| M7: `registerHooks` removed from `TRACE_REGISTER` (import and call renamed to `register`) | same | exit 1, control `hook healthy` exits 1, "the resolve hook does not use registerHooks" |
| Runbook, staged vs working, row numbers stripped | `git show :docs/runbooks/guard-verification.md` | exactly four added rows (`packages.unreachable-trace`, `packages.smoke-requires-dcmjs`, `packages.hook-register-hooks`, `advisories.exception-scope-padded`) and the `package_check.py` limit line, which is the catalogue's `limit` text verbatim. Every other change is renumbering. No change outside the BEGIN/END markers |
| `guard_census.py --check-runbook` on a checkout of the index | staged tree | exit 0, so the staged runbook was the staged catalogue's render and the diff is fully implied by the catalogue diff |
| `bin/ocelli.sh gate packages` | worktree | exit 0, injection test `Ran 1 test in 23.8s OK` |
| `bin/ocelli.sh gate guards` | worktree | exit 0 |
| `bin/ocelli.sh gate advisories` | worktree | exit 0, `OK: advisory check, exception adm-zip@0.5.17` |
| `guard_census.py`, `--check-runbook` | worktree | exit 0 both, 1054 refusals in 78 files, all claimed |
| `prose_check.py`, `source_provenance_check.py` | worktree | exit 0 both, 826 and 1648 files |
| `python3 -m unittest scripts.tests.test_advisory_check` | worktree | 48 tests OK |
| Pin after all mutations | worktree | both SHA-1 values above unchanged |

## Defects

### D1, two comments still say the packages gate proves the rows

**Where**: `scripts/advisory_check.py:117`, "the same function `package_check` uses to choose the rows it proves." and `scripts/tests/test_advisory_check.py:131`, "a padded row is not a row it proves".
**What**: both say `package_check` proves the unreachable rows. This is the claim pass 2 D1 removed from three documents and pass 3 D1 removed from `docs/lld/guards.md`, which now says "checked by `gate packages` ... on the paths that gate runs". Both lines are in the unstaged remediation (`git diff -U0 -- scripts/advisory_check.py scripts/tests/test_advisory_check.py | grep proves`), and the same hunk deleted the staged "while `gate packages` proves no shipped entry point", so the remediation removed one "proves" and added two.
**Why it is wrong**: microscope section 3. The gate checks a bounded set of paths. Pass 2 measured six edits that leave `check_consumer` at 0 problems, and `typescript-packaging.md` states the run-time specifier bound. Pass 3's sweep grepped `proved` only.
**Scope and regression**: two comments. Smallest fix: "the rows it checks" and "a row it checks". A re-sweep with `grep -nE 'prove[sd]?\b'` over the added lines should then show only the base vocabulary below.

## Smells

None.

## Nitpicks

- **N1**: `docs/lld/guards.md:626` is 105 columns after the D1 edit. The paragraph was not rewrapped. Lines 829 and 1086 in the same file are already over 80, so the file has no enforced width.
- **N2**: the `packages.smoke-requires-dcmjs` note at `scripts/guards/catalogue.py:10703` still says removing the require "leaves this control failing", while the limit pass 3 rewrote now says "turns those probes red". It is true (M6), and pass 3's D3 concern was the "green" sentences, so this does not block. One vocabulary would read better.

## Verified clean

- **Pass 3 D1 closed.** `docs/lld/guards.md:625-626` says "checked by `gate packages` on a real consumer install, on the paths that gate runs". That is true and matches the bound in `typescript-packaging.md`.
- **Pass 3 D2 closed.** The false "four" sentence is gone. The comment that is left, "Widening PROOFS would accept a proof name the packages gate does not run", is true: `exception_problems` refuses any `proof` not in `PROOFS` (probe `advisories.exception-proof`).
- **Pass 3 D3 closed and executed.** "turns this probe red" (M3) and "turns those probes red" (M6, M7) hold. The `_unreachable_trace_snippet` docstring, "leaves the refused run with nothing to print", matches M3's "exited 0".
- **Runbook**: only the changes the catalogue implies.
- **Counts in prose**: the five recorded constants (`REFUSED`, `KNOWN`, `FIELDS`, `SCOPES`, `PROOFS`, five `Constant("advisories"` rows), npm's five severities and "the other three warn" (`KNOWN` minus `REFUSED`), "three checks and the trace's own presence" (four entries in `unreachable_problems`), "three quote styles", "the fourteen consumer injections" (`assertEqual(len(CASES), 14)`) all agree with the code.
- **Base vocabulary left alone**: "the consumer proof" (`package_check.py:28` in the base), the `proof` field, `PROOFS`, and "The consumer lock is the proof" in the version probe note are the design's names for the row field and the version check, not claims that unreachability is proved.

## Limitations and handoff

- `gate guards-deep` was not run. No deep-only probe touches the changed lines.
- The mutations ran through `guard_probe.py --only`, not a full `gate guards` on the mutated file.
- Python 3.12, which CI pins, is not installed here.
- The remediation is still unstaged. The tree that gets staged needs its own verification receipt.
- Next owner: implementation, for D1, which is two comment edits. Then a fresh independent pass 5.
