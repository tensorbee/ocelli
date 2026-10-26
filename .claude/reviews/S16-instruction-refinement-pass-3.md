# S16 instruction refinement, pass 3

**Reviewed text**: the unstaged cache-validity paragraphs in `.claude/commands/design.md` (lines 136 to 140) and `.claude/commands/microscope.md` (lines 140 to 143)
**Author of those paragraphs**: Codex, during F-X003 pass 5
**Reviewer**: implementing xAI model
**Result**: 0 defects, 0 smells, 0 nitpicks
**Date**: 2026-10-19

This is a bounded reading of two new template paragraphs. It is not a product receipt, not approval of the D9 repair, and not a re-review of the older unstaged instruction diff. Pass 2 does not cover these paragraphs. The implementing agent did not edit them.

## What was read

`design.md` tells a plan for cached or settled output to list every validity input, including format, layout and rendering mode where those apply. It requires the plan to say which validation runs before an idle or cache return. A refused request must not replace the last usable mode or configuration. The required sequence is a valid draw, an incompatible request, then a valid retry, both before and after settlement.

`microscope.md` tells the reviewer to compare an incompatible request before and after settlement. An idle shortcut must not turn a refused request into success or remember an unusable mode. Each refusal is followed by a valid operation through the public caller, so a hidden configuration change has a place to show.

The generated adapters do not copy either body. They name the canonical file and its SHA-256. `python3 scripts/sync_agent_skills.py --check` exited 0 and printed `OK: 21 adapters match their canonical sources`. Adapters were not regenerated in this turn.

## Finding

No defect. Neither paragraph contains an em dash or a prose semicolon. The two sentences that carry D9 are present: validation before the idle return, and a refusal that must not become the remembered mode. The required follow-up call is what exposes a shade changed before the payload was accepted. That covers a settled idle and a refusal after the settled frame has been invalidated. A separate word for invalidation is not required once every refusal has a valid public retry.

## What this pass did not review

The rest of the unstaged `design.md` and `microscope.md` hunks, and the unstaged workflow, verify, complete-feature and dicom-tooling text. Those belonged to the earlier refinement set. This pass does not extend pass 2's result and does not reopen it.

The paragraphs are instructions for later plans and reviews. They do not prove that the D9 code preserves mode. That claim is in the product evidence and still needs a fresh independent product review.
