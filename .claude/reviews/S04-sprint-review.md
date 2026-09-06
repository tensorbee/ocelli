# S04 sprint review

**Scope**: the whole S04 diff on `sprint/s04`, reviewed after all implementable
stories landed and the two operator-approved carry-forwards were recorded.

## Pass 1

**Result**: 1 defect, 0 smells, 0 nitpicks

The close command explicitly allowed a story to be carried forward with a
reason in `CURRENT_SPRINT.md`, but `close-preflight` accepted only
`completed`. F-012 cannot run before an Ocelli candidate renderer exists and
F-X011 requires a second physical machine. Both reasons were tracked, yet the
mechanical preflight made the documented close state impossible.

The remediation adds a distinct `carried` state. It accepts that state only
when the current sprint's carry-forward section records a non-empty reason, and
does not demand an implementation review for work that was not implemented.
An unrecorded carry-forward is a standing refusal probe and a recorded one is
an accept control.

## Pass 2

**Result**: 0 defects, 0 smells, 0 nitpicks

The remediated whole-sprint diff has no remaining correctness defect or
structural smell. The close state now agrees with the canonical close command,
and the distinction between completed and carried work is visible in both run
state and the tracked delivery record.
