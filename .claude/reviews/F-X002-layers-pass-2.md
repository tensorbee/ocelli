# F-X002 review, pass 2

**Reviewed**: working tree on sprint/s14 against 879f80f
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

## Smells

## Nitpicks

### N1, the SwiftShader fixture is still shorter than the recorded renderer string

**Where**: `crates/ocelli-render/src/caps.rs:425` and `crates/ocelli-render/src/caps.rs:2528-2530`

**What**: Unchanged from pass 1. The negative fixture is `ANGLE (Google, SwiftShader Device)`. The layer 3 record also contains `LLVM 10.0.0`. A needle of `llvm` stayed green in pass 1, and that shorter needle is a substring of the recorded renderer string. The shipped needle is still `llvmpipe`, which the recorded string does not contain. This does not block.

## Verified clean

Pass 1 defect D1 is fixed. `scripts/tests/test_lavapipe_layer.py` now calls `lavapipe_layer.main` through `run_main`. `test_main_exit_codes_are_what_the_workflow_branches_on` expects a missing file to return 10 and print nothing, a drop record to return 0 and print `layer 2 dropped: no ICD on ubuntu-latest` plus the newline from `print`, and skip, pass, an empty reason, a whitespace-only reason, `[]`, and non-JSON to return 1 and print nothing. `main` in `scripts/lavapipe_layer.py` still returns 10 for the probe branch, 0 after printing the drop line, and 1 otherwise. The workflow at `.github/workflows/lavapipe.yml:42-48` still runs the probe only when that process exits 10, and treats exit 0 as the drop.

`python3 -m unittest scripts/tests/test_lavapipe_layer.py` exited 0 with 6 tests before any mutation. Replacing `return 10` with `return 0` made `test_main_exit_codes_are_what_the_workflow_branches_on` fail with `AssertionError: 0 != 10` at `test_lavapipe_layer.py:65`. That command exited 1. The other five tests stayed green on that mutation, which is the gap pass 1 named, and the new test is what went red. Restoring `return 10` made the same unittest exit 0 with 6 tests. The file hash returned to `61c0a041b989e3d66ffac1adcbd2aecec6d4ab87`, the hash taken before the edit.

Accepting status `skip` beside `dropped` made two tests fail, and the unittest command exited 1. `test_main_exit_codes_are_what_the_workflow_branches_on` reported `AssertionError: 0 != 1` for the skip document. `test_skip_and_pass_are_refused` reported `dropped` against `refused`. Restoring `status != "dropped"` made the unittest exit 0 with 6 tests. The source hash matched the pre-edit hash again. `git diff HEAD --stat` is still 38 lines in `caps.rs`, 2 in `docs/lld/README.md`, and 43 in `docs/lld/tier-resolution.md`. No mutation remains in the product files.

`ci/lavapipe-layer.json` is absent. The write set is the pass 1 set plus `.claude/reviews/F-X002-layers-pass-1.md` and this file. `docs/hld/DEVIATIONS.md`, `.github/workflows/ci.yml`, and `tools/tier-b/run.mjs` are not in the diff.

The lavapipe example still returns 2 when `request_adapter` fails, 0 when `fallback_is_lavapipe` matches, and 1 otherwise. The predicate needle is still `llvmpipe` and `lavapipe`. Layer 3 still exits 0 only when the renderer string contains `SwiftShader` and the tier is `C`. The LLD sentences for the local probe exit 2 and the SwiftShader exit 0, tier `C`, `AdapterType`, and the recorded renderer string are unchanged. Those runs from pass 1 still support the sentences. Chrome was not started again.
