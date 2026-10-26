# S16 instruction refinement, pass 4

**Reviewed text**: the unstaged additions in `.claude/WORKFLOW.md`, `.claude/commands/design.md`, `.claude/commands/implement-feature.md` and `.claude/skills/dicom-tooling/SKILL.md` that pass 2 and pass 3 do not cover
**Exact diff**: `git diff -- .claude/WORKFLOW.md .claude/commands/design.md .claude/commands/implement-feature.md .claude/skills/dicom-tooling/SKILL.md` at review time, 189 lines
**Author of those paragraphs**: Codex, recorded in F-X003 pass 6
**Reviewer**: implementing xAI model
**Result**: 0 defects, 0 smells, 0 nitpicks
**Date**: 2026-10-19

This is a bounded reading of the new instruction text. It is not a product receipt, not approval of AC5, and not a re-review of pass 2 or pass 3. The implementing agent did not edit the paragraphs. Codex authorship is not independent approval.

## What this pass reviewed

`WORKFLOW.md` now says an audit finding is connected, in the existing review, to its permanent regression and to the instruction that should prevent it. If that instruction already covers the finding, the missed execution step is named instead of adding a duplicate rule. New guidance needs independent review and adapter sync. An observed failure stays distinct from a preventive gap. Missing implementation needs a concrete next action.

`design.md` now requires a comparison or measurement row to name input selection, the production entry, the candidate output or completion event, the comparator or collector, and the reported result. Missing code or configuration is named apart from missing equipment. A reference render, one synthetic comparator call, or a refusal-only runner does not supply the production stage. Timing names the start and end, the clock or CPU-time source, the units, the workload and sample validity. A threshold is not manufactured while acceptance still needs an unavailable incumbent measurement.

The same unstaged `design.md` hunk also contains two paragraphs pass 2 did not quote and pass 3 did not read. A language or target boundary compares generated exports with the real caller, including allocation refusal and address reuse. A native counter is not a wasm lifetime. Cached or settled rendering starts from a successful frame and follows command, input and resize through invalidation, generation, draw and the event. Source size stays independent of target size and device pixel ratio. Scratch is sized from the dimension that bounds the work, and an allocation claim includes the session and entry point.

`implement-feature.md` now requires each missing implementation to name the owning function or module, the smallest next deliverable and its discriminating check. Collector configuration and the external host are named separately. A handoff picks the next dependency-ready step inside the authorized scope. A repair may stop at its declared boundary, and that boundary is not described as a technical blocker. Work that can be implemented is not handed back only as still open. Unapproved scope is not started to fill a gap.

`dicom-tooling` section 11 requires selected manifest rows to be traced through the actual Ocelli candidate producer into the existing comparator. Permitted identifiers, selected and compared counts, and each unsupported, failed or unrun disposition are recorded without patient data. A passing reference render or one synthetic comparator call does not exercise that producer. Input frame, presentation parameters, geometry and output extent match across the two sides. A synthetic positive control and a deliberately wrong candidate are required. Producer execution stays separate from comparator sensitivity and numerical acceptance. A missing GPU observation does not account for an unimplemented candidate path. Pixels stay off the public shell.

## What this pass did not review

Pass 3 already covers the cached-output paragraph in `design.md` and the matching microscope paragraph. Pass 2 already covers the ownership table, the acceptance-row caller sentence, the evidenced-or-missing handoff, the repaired-lifecycle paragraph, the rebuild command chain, the frame-preserving dicom paragraph, and the workflow sentences about a missing host and about explicit target suites. This pass does not extend those results.

Generated adapters name the canonical file and a SHA-256. They do not copy these bodies. This pass did not regenerate adapters and did not stage the canonical files.

## Finding

No defect. The reviewed paragraphs have no em dash and no prose semicolon. They do not add a gate, a tolerance, a threshold, a baseline or a budget. They do not mark a story done. They do not tell a producer to export pixels or to treat a synthetic frame as corpus coverage. The smallest-next-deliverable rule still forbids unapproved scope, so it does not authorize F-X021 or a new shell command.

The paragraphs are instructions for later plans and handoffs. They do not prove that the applicable-corpus producer exists. That claim belongs to the product evidence.
