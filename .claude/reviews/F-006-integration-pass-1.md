# F-006 review, integration pass 1

**Reviewed**: the squashed merge of `work/f-006-claude` at `4b94cd4`.
**Reviewer**: the integrator, independent of the implementing agent.
**Result**: 0 defects, 0 smells, 1 nitpick corrected in place.

## Defects

None.

## Smells

None.

## Nitpicks, corrected at integration

**A false attribution in `docs/lld/benchmarks.md`.** It said the F-096 comments
"were corrected in S02" and that every instance now names F-101. They were
corrected in S03 by `d74ad3a`, earlier in this same run, and two sites
deliberately keep the old number because they quote what an earlier comment
said. Corrected in the integration commit. The story changed neither preserved
quote, which I checked.

## Verified clean

**The honesty of the instrument, which is the whole story.** Eleven subjects
are declared and exactly one, `wasm.cold_start`, has a subject today. The other
ten each name a blocking F-ID and report `unavailable`. No proxy workload, no
timed stub, and no invented number.

**The one real number states its provenance and its tolerance is derived.**
2.3 ms, median of fifteen kept iterations against the actual release artefact.
The 25 per cent tolerance is reasoned from an observed spread of 2.2 to 2.5 ms
across fifteen runs and from `performance.now()` being quantised to 0.1 ms,
which is itself over 4 per cent of a figure this small. It says in terms that
this is the second half of gate A4 and **is not an answer to it**, because the
module has no wgpu and no Naga in it against A4's 3 to 8 MB estimate.

**The guard refuses the thing it exists for, re-proved by me.** I injected a
recorded number for `decode.frame`, whose story F-023 is `pending`, into
`ci/bench-baseline.json`. `gate bench` exit **1**: "records a number for
decode.frame ... and its subject story F-023 is 'pending' rather than done. This
is the entry an invented number would come to rest in, and it is refused."
Control after restoring the file, exit **0**, and `git diff` clean.

**The integration wrinkle I predicted did not bite.** Both wave-two worktrees
were cut before I moved F-004's backlog row to `done`, so `subject_story: F-004`
resolves differently either side of the merge. Checked against the canonical
tree: `gate bench` passes, 43 tests. The guard refuses a runner for a story that
is NOT done and does not demand one for a story that is, so both readings are
safe.

**Wiring.** `bench` is in `GATES` with a `run_gate` arm and a matching
`ci.yml` step, and `gate ci` passes over 24 floor gates. `gate --floor` is ALL
GREEN over 24, up from 23.

**The section 26 paraphrase is restored.** `AGENTS.md` now reads "Measure with
the benchmark harness before optimising anything" and names the harness and the
registry. That rule was unenforceable until this story existed, and the
paraphrase hid the dependency.

**What the story says it does not catch, and says so.** A `subject_story`
naming a real but wrong F-ID passes, because the guard checks existence and not
intent. Recorded as a non-firing mutation rather than left for a reader to
discover, which is the right way to report the limit of a guard.

## Carried to F-X009

`scripts/ci_floor_check.py` line 77 tests `f"gate {gate}" in workflow` as a
plain substring over the whole file, so **a YAML comment naming a gate
satisfies it with the step deleted**. Found by this story, confirmed by me, not
fixed here. It is a census entry and a probe for F-X009, and the probe fails
today.
