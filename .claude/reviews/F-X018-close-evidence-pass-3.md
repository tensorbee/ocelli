# F-X018 close evidence review, pass 3

**Reviewed**: carry-forward remediation found by the S04 whole-sprint review
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The lifecycle has separate `completed` and `carried` states. A carried story
can satisfy close preflight only when the matching S04 carry-forward section in
`CURRENT_SPRINT.md` has a non-empty reason. The preflight skips the feature
review only for that explicit state, so carried work cannot claim an
implementation review or completion.

The parser unit tests distinguish the named sprint section, ignore later
headings and reject an empty reason. The refusal probe makes an unrecorded
carry-forward red. The accept control uses the tracked F-012 or F-X011 reason
and remains green. The canonical commands, generated adapters, workflow and LLD
describe the same state transition.
