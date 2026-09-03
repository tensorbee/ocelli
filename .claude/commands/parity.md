---
description: Report progress against the cornerstone3D parity surface, from the checklist rather than from a feeling.
---

# /parity [--epic E12] [--surface "Tool classes"]

Phase 1's definition of done is feature parity with cornerstone3D v5.8.9, and
`docs/hld/B-parity-surface.md` is the enumeration. This command reports against
it.

## The surface

| Surface | Count |
|---------|-------|
| Viewport types | 12 |
| Tool classes | ~63, being 26 annotation, 12 segmentation, 25 manipulation and utility |
| Blend modes | 5 |
| VOI LUT functions | 3 |
| Transfer syntaxes | ~13, two of which are the open gates A1 and A2 |
| Segmentation representations | 3 |
| Core events | 50, plus 53 tool events |
| Adapters | 4, being SR TID 1500, SEG, RTSTRUCT and parametric map |

## How to report

1. Read the parity checklist. Each row has a `Covered by` column naming an
   **epic ref** (E7.1, E12.3).
2. Resolve each epic ref to its F-ID and status through
   `docs/sprints/allocation.json`.
3. Report per surface: covered by a `done` story, covered by a planned story,
   or **not covered by any story**.

That third category is the reason this command exists. A parity row with no
covering story is a gap in the plan, and it is invisible from the backlog
because the backlog does not know the checklist exists.

## Rules

- **A story being `done` is not evidence the surface works.** It is evidence
  someone intended it to. Where the oracle covers a surface, report the corpus
  result alongside the status, and say when it does not.
- **Report `DEFER` rows as deferred, not as covered.** The checklist marks WSI
  and ECG viewports DEFER pending gate A5, and rolling them into a percentage
  makes a product decision look like progress.
- **Do not report a single completion percentage without the denominator and
  the date.** Parity against a moving reference is a claim about two things.
