# S02 sprint review, pass 1

**Scope**: the four lane-B stories seen together, F-002, F-007, F-008 and
F-003, and specifically the interactions no per-story review could see. F-010
is still in its worker and is reviewed after integration.
**Result**: 2 defects, 0 smells, 0 nitpicks. Both remediated.

## Why a sprint pass finds different things

Each story's own review looked at its diff. Four of them wrote
`bin/ocelli.sh`, four wrote `.github/workflows/ci.yml`, three wrote
`docs/lld/build-targets.md` and all four wrote the ledgers. Those files are
correct per story and can still be wrong as a set.

## Defects

### S1. Nothing made the CI floor's central claim true

`bin/ocelli.sh gate --floor` documents itself as "the gates CI runs", and
`.github/workflows/ci.yml` is what CI actually runs. **Nothing compared them.**

This sprint added three gates to the floor, `native` in F-007, `device` in
F-008 and `packages` in F-003. Each needed a hand-written step in the workflow
and each got one, so the tree is correct today. That is the point: the
correctness was one person remembering, three times, and a missed one would
have produced a gate that is green locally, absent in CI, and a floor whose own
description is quietly false. That is the same shape as a skipped gate reading
as a passed one, which this project refuses everywhere else.

**Remediation.** `scripts/ci_floor_check.py` and a `ci` gate. It asserts that
every floor gate is either invoked by name in the workflow or has its
underlying command run there, since `ci.yml` legitimately does both. Proved red
by adding a floor gate with no CI step.

**What it deliberately does not check** is that the CI step is EQUIVALENT to
the gate. A step running `cargo clippy` without `-D warnings` would satisfy it.
Closing that would mean CI calling `bin/ocelli.sh gate --floor` as a single
step, which would collapse the job matrix that gives CI its useful per-area
failure names. Recorded in the script rather than left as a silent limit.

### S2. `.gitignore` could not stop the corpus symlink a parallel wave creates

`.gitignore` carried `corpus/data/`, with a trailing slash, which matches only
a directory. A parallel sprint worktree gets its corpus as a **symlink** to the
canonical one, and that pattern does not match a symlink, so it showed as
untracked in the F-010 worktree and a `git add -A` would have committed a path
pointing at one machine's filesystem.

The pre-commit hook would not have caught it. `staged_content_check.py` refuses
a staged DICOM by magic bytes, and a symlink to a directory is neither.

Measured in this sprint, in the live F-010 worktree.

**Remediation.** The trailing slash is gone, with the reason recorded at the
entry. The worker worktree also got the path in `.git/info/exclude`
immediately, because the tracked fix does not reach a branch based before it.

## What was checked across the set and found clean

- **`bin/ocelli.sh`**: the `GATES` array and the `run_gate` arms agree, every
  new arm is chained on `&&` so a first command's failure cannot be masked, and
  the four stories' edits sit in disjoint regions.
- **Every floor gate is reachable from CI**, now mechanically.
- **`docs/lld/build-targets.md`** was written by F-002, extended by F-007 and
  corrected by F-008, and reads as one current-state document rather than three
  appended histories. Its `F-IDs that contributed` line names all three.
- **The ledgers are additive, not overwritten.** Six AS_BUILT entries, six
  tracker rows, twelve CHANGELOG bullets, and `CURRENT_SPRINT.md` shows four
  `done` and F-010 `pending`. Recorded completion dates match the commit dates.
- **The sprint profile is genuinely strict now.** `s01_pre_oracle` requires the
  active sprint to be S01, it is S02, so `gate oracle` fails rather than skips
  in the canonical worktree. That is correct and is satisfied when F-010
  integrates.
- **The wasm size budget is unaffected by wgpu.** `ocelli-wasm` does not depend
  on `ocelli-render`, and the module still measures 14,104 bytes after F-008.
- **Deviations**: D-10, D-11 and D-12 all have rows, every citation resolves,
  and `deviation_check.py` is green.
