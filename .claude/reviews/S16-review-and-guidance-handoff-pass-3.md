# S16 review and instruction handoff, pass 3

**Reviewed product index**: `f6057868244a71679ebfd95a17e440084699aa76`
**Reviewer**: Codex, 2026-10-19

The reported pass-2 repairs were replayed and inspected. F-048's camera LLD
and F-101's scoped ownership, metadata, transfer and instrument corrections
have no new findings in their pass-3 reports. The real worker now blits,
publishes, retries a failed blit and retains publication objects as tested.
The following resize boundary still fails and prevents acceptance of F-X003.

## Next implementing-agent prompt

Read `.claude/reviews/F-X003-raster-pass-3.md` and its D8. Preserve the
repairs that pass. Fix the public CPU canvas resize as one coordinated state
transition. Validate before destructive external changes. A same-configuration
resize must not leave the visible frame cleared while the raster stays settled.
A rejected zero extent must preserve the previous canvas and publication state.
If the chosen successful policy recreates the bitmap, invalidate and publish
the current frame coherently instead of treating it as idle.

Add both reproductions to the permanent unmodified worker test. Begin with an
opaque white settled frame. Check RGBA including alpha, canvas size, refining,
generation and events after equal-argument resize and rejected resize. Keep
the valid changed-size, failed-blit retry, newer-generation and retained-object
controls. Retain semantic RED/GREEN against the same inputs.

Read the F-048 and F-101 pass-3 reports for the closed findings and bounded
scope. Do not repeat unchanged fixes. Keep those stories in progress while
their remaining acceptance is open.

Review the new unstaged canonical instruction refinements separately from
the product fix. Their scope is listed below. The earlier instruction review
does not cover these new edits. Regenerate adapters from canonical sources,
then check them. Preserve unrelated changes and prior findings.

Return a concrete staged product tree for independent review, with exact
tests and remaining acceptance dispositions. Do not mark done, commit, push,
close S16, start S17 or reuse an older receipt. The normal final integrated
verification still follows the reviewed final union, not this handoff alone.

## Instruction refinements and their trigger

These are process improvements, not a second feature tracker or a new set of
mechanically enforced gates:

- Design now specifies ownership, lifetime identity, the observable commit
  point and failure/duplicate behavior in the existing plan template. The
  failed-blit and resize defects show why native state alone is insufficient.
- Implementation handoffs require the actual artifact build/test chain and
  public-path coverage, including repeated requests and rejected configuration.
- Microscope repair passes separate closed findings from new ones and inspect
  the external sink after failure and recovery.
- Verify names the existing dev-wasm build, explicit boundary suite and CPU
  worker command for affected changes. These tests are outside default npm
  discovery. The profile remains required, and standalone evidence does not
  replace its gate list or a release-size measurement.
- Complete-feature explicitly checks every numbered acceptance row before
  completion. A bounded clean repair review cannot waive missing instruments,
  corpus comparison or representative-host evidence.
- Dicom-tooling requires visible RGBA/alpha and extent alongside generation,
  refining and event observations for frame-preserving operations.
- WORKFLOW points at the explicit target requirements and acceptance boundary.

Canonical paths are `.claude/WORKFLOW.md`, `.claude/commands/design.md`,
`implement-feature.md`, `microscope.md`, `verify.md`, `complete-feature.md`
under that command directory, and `.claude/skills/dicom-tooling/SKILL.md`.
Generated adapters are derived, never hand-edited. These refinements remain
unstaged and require another reviewer. Note the trigger in the next actual
AS_BUILT entry, not a fabricated completion entry now.

## Still open

AC5's applicable-corpus producer/comparison, AC6's collector and representative
host, browser colour and reduced-resolution integration, registered interaction
instruments and the declared GPU observations keep their existing dispositions.
No measurement, tolerance, baseline or budget has been invented or changed.
