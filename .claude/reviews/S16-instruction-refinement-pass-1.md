# S16 instruction refinement review, pass 1

**Reviewed**: unstaged working-tree diff of the nine files below. This is not a commit, not a tree hash, and not the product repair index.
**Author**: Codex, present before this repair turn
**Reviewer**: implementing agent, reading that diff on 2026-10-19
**Result**: 0 defects, 0 smells, 0 nitpicks
**Check**: `python3 scripts/sync_agent_skills.py --check` printed `OK: 21 adapters match their canonical sources`

This pass does not approve the product repair, mark a story done, or replace the fresh Codex review the product tree still needs.

## Files

Canonical sources:

- `.claude/WORKFLOW.md`
- `.claude/commands/design.md`
- `.claude/commands/implement-feature.md`
- `.claude/commands/microscope.md`
- `.claude/skills/dicom-tooling/SKILL.md`

Generated adapters, hash line only:

- `.agents/skills/design/SKILL.md`
- `.agents/skills/implement-feature/SKILL.md`
- `.agents/skills/microscope/SKILL.md`
- `.agents/skills/dicom-tooling/SKILL.md`

`git diff --stat` reports 58 insertions and 4 deletions. The four deletions are the previous source hashes. The adapter bodies still point at the canonical file. No generated adapter was edited by hand in this turn. The check above compared the working tree.

## What the additions require

Acceptance handoffs keep four dispositions available: evidenced, failed, implementation missing, and an external observation that is unavailable. Each row keeps an owner and a next step. A missing host does not stand in for an unimplemented runner, canvas binding, or candidate producer.

A wasm boundary test executes the generated module through the public shell caller and its adapter. Allocation refusal stops before a write. Release and same-address reallocation are tested on that module. A native counter is not that proof. The public metadata object stays on the caller when the export is numeric.

Settled rendering starts from a successful settle. A successful command, a new source, and a resize are followed through invalidation, generation, the next draw, and the event. Source rows and columns stay independent of the target extent and of CSS versus device scale. Scratch capacity follows the dimension that bounds the work. An allocation claim includes the session entry point, including a presentation clone.

A runner that always throws shows that refusal is handled. It does not show that an equipped host can be measured. A failed adapter open is an environment result. Device-to-CSS conversion is traced through the shader or the canvas consumer. Raster fixtures name source rows and columns, target extent, and scale separately, and include a supported source larger than the destination when downsampling is claimed. An unclamped display probe stays an arithmetic probe.

The additions do not invent a millisecond, a vCPU fraction, a colour threshold, or a closed acceptance row.

## Voice

The added paragraphs have no em dash and no prose semicolon. Plans stay approved. The text does not mark F-048, F-101, or F-X003 done.

## Disposition

These nine files stay unstaged. They are not part of the product repair index handed back with this pass. The audit trigger for the next actual `AS_BUILT` entry stays in the feature progress notes. That entry is not written in this turn.
