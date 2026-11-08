# F-203 showcase schedule repair review, pass 2

Scheduling author: Codex f225_review. Original independent reviewer and
repair author: Codex glm_codec_merge. Repair reviewer: Codex root.
Reviewed corrected tree `c6874996d98bd6ef13a316f759eea7a3e67d8a5f`
against the preserved original `803a2536aae40cec92c221ebf924e9c919ca708e`.

## Result

Defects: 0. Smells: 0 in the repaired scheduling and intake scope.
All four original defects and the removed-cross-reference smell are resolved.
This approves scheduling and draft intake, not implementation, numerical
rules, feature completion or the final pre-implementation design.

Root independently read the F-245 and F-202 acceptance and sequencing delta,
the backlog and roadmap repairs and the repeated dependency-policy change.
A direct byte check proves the other 22 plan changes are exactly that policy
replacement, allocation is unchanged from the original reviewed schedule,
the complete historical S16 tail matches the baseline, and both unresolved
import warnings are restored verbatim. The full allocation and scope review
remains the independent pass 1 record with its original findings preserved.

F-245 completes source selection, actual QIDO/WADO fetch and current identity
while preserving native image workflows. F-202 starts after that dependency
is done and supplies actual decoded remote pixels in the same viewer.
Remote viewing remains explicitly partial until then. Both are scheduled
in S47 and the wave's end-to-end result is unchanged. Source review alone
cannot replace dependency completion in any of the 24 draft plans.

Twenty new IDs F-245 to F-264 and four reused IDs F-202 to F-205 have actual
S47 to S50 allocations, estimates, dependencies and named viewer acceptance.
Later work stays scheduled four sprints later. Existing S47/S48 exceptions
and current S46 completion claims are unchanged. Estimates remain provisional
relative engineering effort, not measured dates or capacity.

The complete corrected 29-path patch was checked for applicability against
both the canonical sprint and the composed source candidate, then applied
without a conflict. The following staged-tree guards still run before the
local checkpoint. No push, close, future implementation or new completion
claim is authorized by this review.
