---
description: Run one of the six Appendix A spike gates and record its answer. Each can end or reshape the programme.
---

# /spike {A1|A2|A3|A4|A5|A6}

Gates live in two places. `docs/hld/A-spike-gates.md` carries A1 to A6 from the
authored document. `docs/spikes/GATES.md` carries gates added afterwards, with
the same authority. Each is cheap to answer and each can stop or reshape the
work it gates. A1 to A6 belong in the first six weeks, during M1 and M2.

**A spike is not a story.** It has no F-ID, it produces an answer and a
recommendation, and its output may be a decision to change the plan.

| Gate | Question | If it fails |
|------|----------|-------------|
| **A1** | Does HTJ2K decode correctly in openjp2 under wasm32, bit-exact against OpenJPH? | You maintain C codec builds regardless, and one of the four arguments for the rewrite weakens |
| **A2** | What is the JPEG-LS answer: a CharLS bridge, self-compiled CharLS, or a young pure-Rust crate? | Changes the architecture, not a dependency line. **Decide before anything else** |
| **A3** | Does a wgpu volume ray-cast run on WebGPU and degrade acceptably on WebGL2 at your real volume sizes? | Sizes the least-compressible phase in the plan |
| **A4** | Do binary size and cold start land within budget? | Estimated 3 to 8 MB uncompressed before tuning, Naga dominating. **Unmeasured today** |
| **A5** | Are WSI and ECG viewports in scope? | A product decision that moves the Phase 1 total |
| **A6** | Is the source-provenance policy agreed in writing? | **Must precede any agent touching any repository** |
| **A7** | Is tier C worth building, and where does its value actually come from? | Drop F-X002 to F-X004 and revert deviation D-07. See `docs/spikes/GATES.md` |

## How to run one

1. State the question and what a pass and a fail each look like, **before**
   doing the work. A spike whose success criterion is written afterwards
   always passes.
2. Build the smallest thing that answers it. A spike is throwaway code and is
   not held to the gate set. Say so in the record.
3. **Measure. Do not reason.** A4 asks for a number. A1 and A2 ask for a
   bit-exact comparison against an independent decoder, which means actually
   decoding the same file two ways and comparing bytes, not reading a
   changelog that says a format is supported.
4. Record the answer in `docs/spikes/A{N}-<slug>.md`: question, method, raw
   result, interpretation, recommendation, and what changes in the plan.
5. If the answer changes the plan, say what changes and stop for the operator.
   A spike that reshapes a milestone is a decision, not an implementation.

## A2 and A6 come first

**A6 before any agent touches any repository.** It is not a technical question
and it costs an afternoon. `docs/SOURCE-POLICY.md` and HLD C.2.1 are the
answer, and the gate is whether they are agreed in writing.

**A2 before the codec work is designed.** The HLD is explicit that it changes
the architecture rather than a dependency line, because a JS-side bridge to a
CharLS build is a registered decoder in the registry design and a native
self-compiled CharLS is not available to the browser at all.
