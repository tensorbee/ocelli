# S16 instruction refinement review, pass 2

**Reviewed**: unstaged working-tree diff of the canonical instruction files and their generated adapters. This is not a commit, not a tree hash, and not the product repair index.
**Author**: Codex, present before this repair turn
**Reviewer**: implementing agent, reading that diff on 2026-10-19
**Result**: 0 defects, 0 smells, 0 nitpicks
**Check**: `python3 scripts/sync_agent_skills.py` rewrote 21 adapters. `python3 scripts/sync_agent_skills.py --check` printed `OK: 21 adapters match their canonical sources`.

Pass 1 reviewed an earlier unstaged subset and does not cover this text. This pass reviews the additional guidance. The product repair is a separate index.

## Files

Canonical sources:

- `.claude/WORKFLOW.md`
- `.claude/commands/design.md`
- `.claude/commands/implement-feature.md`
- `.claude/commands/microscope.md`
- `.claude/commands/verify.md`
- `.claude/commands/complete-feature.md`
- `.claude/skills/dicom-tooling/SKILL.md`

Generated adapters, hash line only:

- `.agents/skills/design/SKILL.md`
- `.agents/skills/implement-feature/SKILL.md`
- `.agents/skills/microscope/SKILL.md`
- `.agents/skills/dicom-tooling/SKILL.md`
- `.agents/skills/verify/SKILL.md`
- `.agents/skills/complete-feature/SKILL.md`

No generated adapter body was edited by hand. Each adapter still names its canonical file. The recorded SHA-256 matches that file.

## What the additions require

Design plans for fallible state name the owner, the lifetime identity, the observable commit point, and what a failure or a repeated call does. Equal arguments are not treated as a no-op until the external setter is accounted for. Acceptance rows name the production caller, the built artifact, and the permanent selector, including a negative case and a duplicate call.

Implementation handoffs record evidenced, failed, implementation missing, or external observation unavailable. They name the rebuild command and keep suites that default npm test does not collect. A bitmap setter is inspected after refusal and after recovery.

Microscope repair passes separate closed findings from new ones and read the external sink, not only the native counter.

`/verify` names the dev wasm build, `vitest.boundary.config.ts`, and `node packages/core/cpu-canvas.mjs` for the affected paths. Those commands stay outside `bin/ocelli.sh` gate membership. The profile remains required. A green profile does not replace a failed target check, a release-size measurement, corpus coverage, or a representative host.

`/complete-feature` refuses done metadata while a numbered acceptance row lacks its declared evidence, unless that row's scope was changed through the approved design process. A clean bounded repair review does not close an unperformed measurement, a missing instrument, or an unsupported consumer.

Dicom tooling asks frame-preserving observations to record RGBA including alpha, canvas extent, generation, refining, and event count around success, refusal, and an identical repeat.

WORKFLOW points at those target checks and at the acceptance boundary. A missing host does not stand in for an unimplemented runner, canvas binding, or candidate producer.

## Voice

The added paragraphs have no em dash and no prose semicolon. Plans stay approved. The text does not mark F-048, F-101, or F-X003 done. It does not add a gate, a threshold, a baseline, or a budget.

## Disposition

These files stay unstaged. They are not part of the product repair index. The audit trigger for the next actual `AS_BUILT` entry stays in the feature progress notes. That entry is not written in this turn.
