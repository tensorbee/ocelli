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

## Pass 3

**Result**: 1 defect, 0 smells, 0 nitpicks

The close-time changelog audit found that `Unreleased` retained the pre-S04
corpus counts, named the wrong volume as the declared refusal, still described
the HTJ2K decoder route as unpriced, and omitted the sprint's metadata, render
hash, tier fallback and workflow changes. The close preflight requires every
user-visible change to be covered, so the branch was not ready to merge even
though its code and mechanical gates were green.

The remediation corrects the stale claims from current oracle evidence and
adds concise coverage of the S04 changes without claiming that either carried
story shipped.

## Pass 4

**Result**: 0 defects, 0 smells, 0 nitpicks

The complete S04 diff and its `Unreleased` record now agree. The changelog
separates implemented work from the unregistered HTJ2K route and the two
explicit carry-forwards. No remaining correctness defect or structural smell
was found.
