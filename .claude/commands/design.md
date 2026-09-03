---
description: Write the design plan for one F-ID against the normative HLD, and get it approved before any code is written.
---

# /design F-XXX [--draft]

Produce `.claude/plans/F-XXX-design.md`. `--draft` records open questions
without pausing, for the batch design round in `/run-sprint` step 2.

## Rule one, transcribe the normative source before designing

**Open the HLD sections this story implements and transcribe what they
actually say into the plan, before writing a single design sentence.**

This is the single highest-value step in the whole workflow. A story built
from its plan alone re-derives decisions the specification already made, and
each re-derivation is a place the review loop has to catch a divergence. A
story transcribed from its normative source first has almost nothing left to
diverge on.

`docs/hld/` Part II is prescriptive. Where it gives a formula, a layout or a
signature, **that is the implementation**. The plan's job is not to invent an
approach, it is to find the specified one and name what the specification does
not cover.

Concretely, for this project:

- A story touching the LUT chain transcribes the three VOI formulas from
  `15-lut-chain.md` section 18.2 **character by character**, including which
  comparison is `<=` and which is `<`, and the four rows of the 18.3 fixture
  table with their exact expected values.
- A story touching coordinates transcribes the `Pt<S>` and `Transform<A, B>`
  signatures from `13-core-types.md`, including the note that `derive(Clone,
  Copy)` adds an `S: Clone` bound the marker types do not satisfy.
- A story touching the boundary transcribes the wire format and the 48-byte
  event stride from `14-the-boundary-in-code.md`.
- A story touching rendering names the tier it needs and what it degrades to.

If the section you need does not exist, say so in `## Open questions`. Do not
fill the gap with a plausible design and present it as specified.

## Steps

1. Read the story row in `docs/sprints/BACKLOG.md`: F-ID, epic ref, sprint,
   layer, estimate, dependencies. Read its `notes` in `allocation.json`, which
   sometimes carries an architecture-hook flag or a source-policy warning.
2. Read every HLD section the story touches. Transcribe per rule one.
3. Read `docs/hld/DEVIATIONS.md`. If the plan needs a new deviation, it gets a
   `D-NN` row there in the same change, with a reason. A plan citing an
   undeclared `D-NN` is refused by `scripts/deviation_check.py`.
4. Check the four crate-boundary questions:
   - Does this put `wasm-bindgen` anywhere but `ocelli-wasm`? If so, redesign.
   - Does it move pixels across the boundary? If so, redesign.
   - Does it allocate in the render loop? If so, say where the pre-sized
     buffer comes from.
   - Does it need `unsafe`? If so and it is not one of the two permitted
     files, redesign or raise a deviation.
5. Check the tier question. Every rendering or compute feature declares the
   tier it needs and what it does on each tier it does not get. **There are
   three tiers**, A (WebGPU), B (WebGL2 downlevel) and **C (CPU, deviation
   D-07)**. A feature that cannot run on the resolved tier **reports
   unavailable and never silently produces a different answer.**

   "Not applicable" is a legitimate CPU answer for plenty of stories, and a
   plan that says so has answered the question. A plan that omits the row has
   not, and the two are indistinguishable later, which is why the row is
   required rather than encouraged.
6. Read the parity checklist rows this story covers, in
   `docs/hld/B-parity-surface.md`, via the `Covered by` column keyed on the
   epic ref.
7. Write the plan. Ask questions. Set `**Status**: approved` only after they
   are answered, or `draft` under `--draft`.

## Plan template

```markdown
# F-XXX, {title}

**Status**: draft | approved
**Epic ref**: E{n}.{m}
**Sprint**: S{NN}
**Estimate**: {n}w

## Normative source, transcribed

{The exact text, formulas, signatures and tables from docs/hld/. Cite the
file and section for each. This section is quotation, not paraphrase.}

## What the specification does not cover

{Every decision this plan makes that the HLD does not make for it. If this
section is empty, say so explicitly, because an empty section and a section
nobody wrote look identical.}

## Approach

{The design, in terms of the transcribed source.}

## Boundary and tier

- wasm-bindgen: {not touched | ocelli-wasm only}
- Pixels across the boundary: no
- Render-loop allocation: {none | pre-sized at <where>}
- unsafe: {none | permitted file <which>, and why}
- Tier A (WebGPU): {full | degraded, how | unavailable | n/a}
- Tier B (WebGL2): {full | degraded, how | unavailable | n/a}
- Tier C (CPU): {full | degraded, how | unavailable | n/a}

{All three rows are required. "n/a" is a valid answer and an omitted row is
not, because an omission and a deliberate "no CPU path" read identically six
months later. See docs/hld/DEVIATIONS.md D-07.}

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|

{`fixture` is MANDATORY for any pixel or geometry arithmetic, with the DICOM
section each hand-computed value cites. HLD 27.2 R3.}

## Parity surface covered

{Rows from docs/hld/B-parity-surface.md, or "none".}

## Deviations

{D-NN rows, or "none". A new one is added to docs/hld/DEVIATIONS.md here.}

## LLD impact

{The docs/lld/*.md files /complete-feature step 9 will update.}

## Open questions

{Blocking decisions. Cleared before Status becomes approved.}
```

## What makes a plan bad

- It paraphrases a formula instead of quoting it.
- It says "follow the HLD" without saying which section or what it says.
- It proposes a trait, a generic or a `Box<dyn>` with one implementer, outside
  the two declared extension points.
- Its test table says `unit` for a function doing pixel arithmetic.
- It gives one tier row instead of three, or writes "fallback: TBD".
- It answers a question the HLD already answered, differently.
- It has no `## What the specification does not cover` section, which almost
  always means the author did not look.
