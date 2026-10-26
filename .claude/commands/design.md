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
3. Read `docs/hld/DEVIATIONS.md`. If the plan needs a new deviation, a draft
   records it under `## Open questions`. The `D-NN` row is written there with
   a reason in the same change that sets the plan to approved, so a draft
   never writes the register. A plan citing an undeclared `D-NN` is refused by
   `scripts/deviation_check.py`.
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
6. Inspect `docs/hld/B-parity-surface.md` and identify the actual rows by
   capability name and behavior, cross-checked against the story's epic ref.
   Do not assume a Covered by column exists. Name exactly which observations
   this story can establish and which remain unavailable.
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

## Problem and reproducer

{Concrete triggering input and command, expected observation, current behavior,
source identity and why this is a story. A new feature states what is absent.}

## Scope, API and compatibility

{In scope, explicit non-goals, concrete public signatures, wire/data layouts
where applicable, defaults, refusal policy and compatibility decisions.}

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|

{Include unchanged consumers affected by changed providers. Record the owner
of units, rounding, selection, conversion and state transitions.}

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| {applicable public route} | {concrete input} | {observable change} | {error and fields unchanged} |

{For stateful changes, include competing mutation routes and the relevant
cancel, retry, stale-result or recovery transition. State which invariants
span consumers. For stateless work, say why this section is not applicable.}

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| {applicable allocation, frame or external resource} | {concrete owner} | {ticket or generation, distinct from address} | {accepted commit, successful blit or equivalent} | {release, retry and state retained} |

{For fallible stateful work, identify every side effect before the commit
point, including browser or device setters. State what an identical repeated
request does. Equal arguments do not prove an external operation is a no-op.
Test failure after staging and before publication without losing the current
visible result. Pure stateless work may mark this section not applicable.}

{For cached or settled output, list every input that determines validity,
including format, layout and rendering mode where applicable. Specify which
validation precedes an idle/cache return. A refused request must not replace
the last usable mode or configuration. Include a valid draw, incompatible
request, then valid retry, both before and after settlement.}

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | {input and output or refusal} | {named case} | {specific plausible mistake} | {implementation / integrated / external} |

{Every acceptance claim has a row. Missing external evidence remains an open
criterion. A PR's implemented subset cannot stand in for the full issue.
For cross-target rows, name the production caller, built artifact/profile,
permanent test selector and observation. Include negative, recovery and
duplicate/no-op controls where the operation changes owned state.}

## Approach

{The smallest implementation in terms of the transcribed source. Name concrete
files/functions, existing providers reused and rejected alternatives.}

## Dependencies, write ownership and resources

{Declared dependencies AND source-level dependencies, safe order, files that
cannot have concurrent writers, ledgers, shaders, GPU/browser exclusivity.}

## Verification and evidence reuse

{Exact focused checks, final required profile and configuration. Identify
input/tool/source identities, selected tests and external observers. Name what
invalidates evidence, including final completion metadata. Baseline/output
changes need a stated policy. Performance needs workload, release build and
acceptance threshold. A missing observation does not become a passed row.}

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

{D-NN rows for approved deviations, or "none". A draft names a needed
deviation under Open questions. The row is added to docs/hld/DEVIATIONS.md
when the plan is approved.}

## LLD impact

{The docs/lld/*.md files /complete-feature step 9 will update.}

## Open questions

{For each material decision, name affected ACs, options, decision owner and
the consequence of deferral. Preserve the operator's answer and where it was
given. Cleared before Status becomes approved. External measurements belong
in the evidence inventory, with an owner and acquisition prerequisite.}
```

## Observable acceptance and composition

Extend the template for each story with numbered acceptance criteria, each
mapped to a test or external observation and a responsible execution stage.
Name the concrete trigger, expected result, current failure, scope, non-goals,
public signature and compatibility choice. Map every consumer to its shared
provider and caller-specific defaults, units, rounding and refusal policy.

Use a small risk-based matrix: empty/one/several items, ordinary/boundary/
unsupported inputs, fresh/cached/evicted/recovered state, and affected native,
wasm, CPU, WGSL and actual output consumers. Choose applicable combinations.
Say which plausible wrong implementation each important test rejects. Retain
semantic old-source RED and repaired-source GREEN results. A missing API
compilation failure does not establish the intended semantic failure.

Name dependencies, shared files and exclusive resources, the implementation
order, rejected alternatives, documentation impact and exact focused/final
gates. Performance claims need workload, release configuration and threshold.
Do not assume an unimplemented consumer or external oracle exists. Distinguish
internal consistency from independent correctness and external parity.

Use `/prepare-sprint` for a design-only batch. Preserve draft status while
material decisions remain. Consolidate questions and reuse decisions already
made by the operator. Approval does not establish implementation completion.

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

## Readiness before approval

Approve only when the normative source is transcribed, concrete API/context
choices are settled, every acceptance row has an observation and execution
stage, the write set/dependencies are feasible, and material questions are
resolved. Existing user decisions are reused. Unknown facts are inspected.
Routine choices are stated and made locally. Consolidate only decisions whose
answers materially change scope, behavior, compatibility or evidence.

Examples of insufficient evidence: a round-trip whose two halves share the
same wrong spacing, a sequence refusal tested only through named presets, a
GPU test that returns before opening an adapter, and a close check over a
scratch inventory that omitted allocated work. Designs must name the
independent observation that rejects the relevant failure.

## Evidence feasibility before implementation

For an external acceptance row, name the actual host, adapter, browser,
candidate entry point, reference pin and measurement procedure as applicable.
Separate a settled decision from a missing observation. A fully specified
measurement can remain unperformed during design, but cannot pass acceptance.
If the host, comparison scope or threshold is undecided, keep the plan draft.
Reuse an operator's stated lack of equipment rather than asking again or
substituting a convenient host. Record what would unblock the observation.

For a comparison or measurement row, describe the complete evidence path in
that row or its linked procedure: input selection, production entry point,
candidate output or observed completion event, comparator or collector, and
reported result. Name the missing code or configuration separately from the
external equipment. A reference renderer, a synthetic comparator call or a
refusal-only runner does not supply the missing production stage. Give an
equipped-host invocation and its required inputs when that route is built.
For timing, define start/end events, clock or CPU-time source, units, workload
and sample validity. Do not manufacture a threshold where acceptance still
requires an unavailable incumbent measurement.

Separate requested comparison inputs from the state the candidate actually
applies. Name how each candidate parameter and geometry field is obtained.
Classify consumed fields as commands, observed state or internal coefficients.
An observed transform may already include a reported flip or conversion, so
do not replay that operation as a second command. Reference metadata may
specify the workload, but copying it cannot prove candidate execution.
Unsupported camera or presentation state must be
applied, independently checked as equivalent, or explicitly refused. Include
a nonconstant synthetic input whose output changes under one parameter change.

For a guard, map authoritative input, derived state and the command that must
refuse. Construct negative cases with otherwise valid receipts and identities,
so an unrelated stale-tree failure cannot masquerade as inventory enforcement.
Give a positive control and name the intended failure reason. These details
belong in the acceptance rows, not a second test inventory.

For a language or target boundary, compare the generated export declarations
with the real caller's argument shape, result and ownership handling. Name a
test against the built target, including allocation refusal and allocation
address reuse when lifetime identity matters. A native counter token does not
establish the lifetime semantics of a wasm address. Preserve an explicit
adapter when the public metadata object and numeric export differ.

For cached or settled rendering, start at a successfully settled frame and
follow command changes, new input and resize through invalidation, generation,
draw and the observable event. Name the concrete owner of each transition.
Keep source dimensions independent of target dimensions and DPR. Declare
scratch capacity against the dimension that actually bounds the work. An
allocation claim covers the session and entry point, including cloning,
not just the raster's own buffer-reallocation counter.

For CPU-cost measurements, define the process inventory, cumulative counter,
wall-clock interval and snapshot uncertainty before implementing the runner.
Include browser/worker overhead explicitly. A changed process set or invalid
counter refuses the observation. Separate instrument diagnostics from the
representative physical-host workload, incumbent comparison and noise floor.
An executable collector does not by itself satisfy host acceptance.
