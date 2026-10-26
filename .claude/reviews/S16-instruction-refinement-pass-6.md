# S16 instruction refinement, pass 6

**Reviewed text**: the unstaged pass-9 additions in `.claude/commands/design.md` and `.claude/skills/dicom-tooling/SKILL.md`
**Exact diff**: `git diff --numstat` still shows the accumulated unstaged refinement. `.claude/commands/design.md` is 58 insertions and 1 deletion. `.claude/skills/dicom-tooling/SKILL.md` is 51 insertions. This pass reads only the two new sentences in the design comparison paragraph and the new dicom-tooling provenance paragraph.
**Author of those paragraphs**: Codex, recorded in F-X003 producer pass 9
**Reviewer**: implementing xAI model
**Result**: 0 defects, 0 smells, 0 nitpicks
**Date**: 2026-10-19

This is a bounded reading of the new instruction text. It is not a product receipt and it does not approve AC5. Passes 2, 3, 4 and 5 keep the scopes already written in their own files. This pass does not extend them.

## What this pass reviewed

`design.md` now classifies each consumed comparison field as a command, observed state, or an internal coefficient. An observed transform may already contain a reported flip or conversion. That included operation is not replayed as a second command.

`dicom-tooling` now requires the reference provenance of a field before it is composed. A camera readback and its flip flag can be one already-applied operation. They are not necessarily two commands. The applied basis is derived completely. An internal raster coefficient stays distinct from the reference field's meaning. A fixture built from an assumed command interpretation is not proof of the readback contract.

## What this pass did not review

The rest of the unstaged design, microscope, verify, implement-feature, workflow and dicom-tooling additions stay under the earlier passes. Pass 5 covers the pass-7 paragraphs only. The trichotomy for unsupported camera state, the nonconstant synthetic input, and the red-comparison grouping were already in that pass.

Generated adapters name the canonical file and a SHA-256. They do not copy these bodies. The design adapter records `50d95a8f1f9a0ec563b264bc7e0f3266235665f035cfa13ead55cba13a7b4c18`. The dicom-tooling adapter records `a53e3d0bfa041775c8518b2510dc3b5a6a301d6595d7298b6c038c8ba4facf5a`. Both match the current canonical files. This pass did not regenerate adapters and did not stage the canonical files or the adapters.

## Finding

No defect. The reviewed sentences have no em dash and no prose semicolon. They do not add a gate, a tolerance, a threshold, a baseline or a budget. They do not mark a story done. They do not tell a producer to export pixels or to treat a synthetic frame as corpus coverage.

The design sentence is conditional. "May already include" does not say a flip flag is never a command. The dicom sentence says the pair is not necessarily two commands, so a later plan still traces the producer that wrote the field. Together they forbid the pass-8 composition, which treated an observed flag as a second reflection, and they also forbid treating a fixture that assumes that command reading as proof.

The paragraphs are instructions for later plans and handoffs. They do not prove that the applicable-corpus comparison passes. That claim belongs to the product evidence.
