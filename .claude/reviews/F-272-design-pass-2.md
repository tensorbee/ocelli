# F-272 design microscope, pass 2

Reviewer: Codex, independent of the GLM author. Design only.

Source: base `1d627bd93f0c0e3b44231be9a0f3e76607a8eeb9`, draft SHA256 `b608fbc94dbf437a5d70d13d3736d500fdc0834100393a0932ecd7b45ff91c47`. The author's fresh review reports zero defects and zero smells. This review does not accept that count as independent approval.

Result: 2 defects, 0 smells, 1 nit. Status remains draft.

D1. The guard checks the child exit but does not establish that its literal filter selected any tests. A stale filter returns success with zero tests and an unchanged tree. Before execution, use the actual test executable's list mode with the same selection and require a nonempty inventory containing the named missing-reference and applicable-monochrome producer tests. Record selected and filtered counts. A nonmatching selection must refuse rather than supply a passing guard observation.

D2. Actual `python3 scripts/prose_check.py` over the worker source exited 1, naming eleven prose semicolon violations in the revised plan. The substantive design repair did not receive the required basic source check before its handoff. GLM must author the literal punctuation repairs and the actual check must pass.

N1. The rejected-implementation column and Rejected paragraph still reject final-digest-only write proof without distinguishing the deliberately bounded final-state guarantee. A final digest is valid evidence of final content equality, while it does not establish absence of transient writes. State that distinction consistently.

## Settled implementation details

The private temporary owner must obtain its directory through exclusive creation and retry name collisions without deleting an existing directory. PID and counter are name candidates, not filesystem ownership proof. Cleanup removes only the directory acquired by that invocation.

The real guard observes the actual protected source and output tree. Only negative controls use disposable fixture roots. No transient guarantee, writer hook, new gate or parent-global environment mutation is introduced.

The previous writer-hook, PID-only and forced-missing-reference findings are materially addressed in the revised design. Preserve those repairs. GLM should make bounded changes for this pass, run the actual prose check and its own review, then return the exact source for independent approval.
