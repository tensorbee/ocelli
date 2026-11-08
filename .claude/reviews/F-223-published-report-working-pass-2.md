# F-223 report and workload order, independent repair review pass 2

Author: Codex glm_codec_merge. Reviewer: Codex root, who authored neither
repair nor its permanent controls. Reviewed complete six-path source
`3e60f34e9e65f582ea262cf2dedd2a5b4ee37a39` against
`2d55a0cfc7f3572aa7f7a9ca171dccde0a79c1d8`.
Result: 0 defects, 0 smells, 0 nitpicks in this bounded repair.

The independent artifact review reported one protocol defect and one report
smell. All 44 statistic cells and 31 rendered rows were arithmetically correct,
but that did not supply protocol or presentation acceptance. The prior
numerically complete run and generated pair remain preserved by digest.

## Starting side

The runtime selects a starting side once per declared metric/workload visit,
then shares it across cold and warm. Advancing to the next workload flips
once, with sequence retained across metric boundaries. An unavailable
workload reserves its position without launching observations. An unavailable
condition consumes no additional flip. CPU still reuses memory sessions.
All existing inner operations, endpoints, per-side N and predicates remain.
The meaningful runtime changes are the two loop scopes, the once-per-workload
assignment and its raw receipt. Reindentation does not change the inner flow.

The actual measureAll control substitutes browser transports only and checks
the invoked observation order, raw starting-side receipts, both conditions,
unavailable workload behavior, a later metric and cleanup. It uses the real
canonical N=9. Restoring only the old scheduling body makes the warm starting
side fail. This rejects the original error through its real caller, rather
than checking the isolated trialOrder helper alone.

## Memory method

The existing 250 ms minimum pause moves unchanged to one shared constant,
imported by the unchanged sampler body and used by the report renderer.
Primary memory rows now identify sampled concurrent footprint as a lower
bound. The separate upper-bound label remains, and the prose distinguishes
the minimum pause from actual raw sample timestamps. Both renderers have a
permanent text control. No JSON schema or numeric statistic changed.
The old generator fails the missing-method-description control.

## Actual evidence and limits

Author receipts inspected: 79 focused controls pass, comprising 17 controller
and 62 pure tests. The first attempt failed because the author used undeclared
fixture workload names and assumed a different existing label. Those setup
failures remain retained. Both old-behavior controls fail semantically and
the restored source passes. Seven named gates pass, with 27 Python and 315
Node benchmark tests, 309 passes and six explicit browser opt-in skips.
Census remains 1506 claimed and zero uncovered, with no budget/runbook change.

Root performed source and receipt review only, without repeating native,
browser or timing work. The supplemental colour branch returns before
measureAll and has separate alternating loops, so this default-order defect
is not inferred to invalidate those observations. A fresh default equipped
matrix is required. No feature or sprint acceptance follows from this review.
