# S16 instruction refinement, pass 5

**Reviewed text**: the unstaged pass-7 additions in `.claude/commands/design.md`, `.claude/commands/microscope.md`, `.claude/commands/verify.md` and `.claude/skills/dicom-tooling/SKILL.md`
**Exact diff**: `git diff -- .claude/WORKFLOW.md .claude/commands/design.md .claude/commands/implement-feature.md .claude/commands/microscope.md .claude/commands/verify.md .claude/skills/dicom-tooling/SKILL.md` at review time, 299 lines, numstat 212 insertions and 1 deletion
**Author of those paragraphs**: Codex, recorded in F-X003 producer pass 7
**Reviewer**: implementing xAI model
**Result**: 0 defects, 0 smells, 0 nitpicks
**Date**: 2026-10-19

This is a bounded reading of the new instruction text. It is not a product receipt, not approval of AC5, and not a re-review of pass 2, pass 3 or pass 4. The implementing agent did not edit the paragraphs. Codex authorship is not independent approval.

## What this pass reviewed

`design.md` now separates requested comparison inputs from the state the candidate applies. Reference metadata may name the workload. Copying it does not prove execution. An unsupported camera or presentation state is applied, checked equivalent, or refused. A nonconstant synthetic input must change under one parameter change.

`microscope.md` now tells a candidate-producer review to trace comparison metadata back to the executed state. One camera or presentation input changes on a nonconstant synthetic frame. The pixels respond as specified or the producer refuses the input. Identical copied sidecars do not establish equivalent geometry or parameters.

`dicom-tooling` now says a candidate sidecar describes the applied state. Reference input facts stay distinct from a claim that the camera, transform or presentation was executed. Unsupported states are refused explicitly. A nonconstant synthetic frame challenges zoom, placement or another material parameter before corpus pixel differences are interpreted.

The same skill says a red candidate comparison is grouped by signature. The groups are input or layout refusal, parameter or geometry mismatch, in-image arithmetic or quantization, and background or letterbox behavior. The original run and the failed-row inventory stay. A representative of each material group is reduced to a hand-derived synthetic case before a layer is named. A red pixel predicate is not yet an attribution to the renderer. Tolerances, sampling, applicability and expected baselines are not changed to absorb failures.

`verify.md` now requires a distinct build-output directory for an isolated source copy or a controlled mutation. A shared output cache can select the other copy's test executable. An ambiguous artifact is discarded and the intended source is rebuilt before its outcome is used.

## What this pass did not review

Pass 4 already covers its recorded 189-line diff of `.claude/WORKFLOW.md`, `.claude/commands/design.md`, `.claude/commands/implement-feature.md` and `.claude/skills/dicom-tooling/SKILL.md`. Pass 3 covers the cached-output paragraphs. Pass 2 covers the ownership table, the acceptance-row caller sentence, the evidenced-or-missing handoff, the repaired-lifecycle paragraph, the rebuild command chain, the frame-preserving dicom paragraph, and the workflow sentences about a missing host and about explicit target suites. Those paragraphs remain in the same unstaged diff. This pass does not extend those results.

Generated adapters name the canonical file and a SHA-256. They do not copy these bodies. This pass did not regenerate adapters and did not stage the canonical files.

## Finding

No defect. The reviewed paragraphs have no em dash and no prose semicolon. They do not add a gate, a tolerance, a threshold, a baseline or a budget. They do not mark a story done. They do not tell a producer to export pixels or to treat a synthetic frame as corpus coverage.

The design trichotomy was read as three legal outcomes for a requested camera or presentation state: execute it, show that the executed state is equivalent, or refuse it before publication. It does not authorize a copied sidecar. The red-comparison paragraph orders the synthetic reduction before an attribution, so it does not pre-assign the current corpus mismatches to the renderer.

The paragraphs are instructions for later plans and handoffs. They do not prove that the applicable-corpus comparison passes. That claim belongs to the product evidence.
