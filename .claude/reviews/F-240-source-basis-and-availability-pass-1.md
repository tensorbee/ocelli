# F-240 documentation reconciliation, bounded participant review

Reviewed on 2026-11-09. Result: 0 defects, 0 smells in the two private proposals. This is independent review of the coordinator's proposed availability-row correction and AC12 source-basis record. It is not whole-feature acceptance or adoption. Root owns the dated plan reconciliation and final verification.

## Scope and authorship

The exact inputs are `feature-availability-f240.patch` and `F-240-AC12-source-basis-proposal.md` in `/private/tmp/ocelli-s46-codex/f239-f240-closure-inventory`. The reviewer authored neither proposal nor the F-240 production ICC implementation. The reviewer previously authored the current AC12 re-observation driver and some measurement notes, and reviewed the ICC viewer implementation. This review does not independently reaccept that author's instrument or its observations. The current CMM facts retain their existing root and non-author evidence review.

## Availability row

The one-row patch corrects an obsolete unconditional ICC refusal. `RasterColor::supported` accepts an immutable `ColorTransform`, checking palette representability without rejecting an attached ICC transform. `CpuRaster::raster_color_into` and `colour_bytes_into` use that accepted transform. The latter calls the shared colour mapper, which preserves the ICC transform's Stage 4 behavior. This supports the narrowly worded native CPU matrix/TRC subset claim.

The patch does not claim a GPU ICC path or a Tier C render graph. `ColourLut::icc` still always returns `Unavailable`. `RenderGraph::new` still returns `TierCUnavailable` for `Tier::Cpu`. Those are separate entry points, and their preserved refusal text is accurate. The unchanged representative-host interactive and idle qualification is not silently converted to acceptance.

Some inherited `cpu.rs` comments still label `RasterColor::Unsupported` as ICC. They describe the explicit unsupported descriptor rather than an executable check rejecting every ICC-bearing transform. Their stale breadth does not disprove the corrected availability row. It should not be quoted as current support policy. This bounded proposal review does not approve those inherited comments or expand the requested write set.

## AC12 source basis

The original AC12 and operator Q5 require an independent transcription check and an external host-tool comparison. Their literal source text names IEC 61966-2-1. The historical pass-1 review expressly says that publisher text was not obtained. Therefore direct IEC reading cannot honestly be marked satisfied by inference.

The same review actually checked ICC.1:2022-05 and the ICC sRGB interpretation note. The subsequent approved D1 and D2 explicitly choose that note's clipping and printed D50 matrix. The proposed append accurately states those facts, retains the independent comparison limb and expressly disclaims direct IEC review. It changes no formula, matrix, fixture, clipping rule or numerical threshold.

The remaining decision is a narrow technical evidence-source reconciliation, adopted explicitly by the lead now. It must be recorded as reconciliation of the literal wording, not as a claim that the old direct-IEC clause had literally been performed or that the operator gave a new approval. The proposal already reserves normal review and parent adoption. No further material runtime or policy choice is identified, and no inspected workflow rule requires a new operator question for this documented technical reconciliation. Q5's independent-check purpose remains intact.

For source confirmation, this reviewer reopened the official [ICC sRGB interpretation note](https://www.color.org/chardata/rgb/sRGB.pdf) and [ICC.1:2022](https://www.color.org/specification/ICC.1-2022-05.pdf). The note provides the transfer function, clipping instruction and adapted matrix. ICC.1 F.3 supplies the three-channel matrix/TRC model. This is confirmation of the actually cited source basis, not independent access to IEC publisher text or a new derivation of every fixture. The historical review identifies its note as April 2015. No new claim is made that today's extracted PDF prints that date.

## Limits

No source, index, status, canonical document, model output, runtime or hardware observation changed. Neither proposal supplies missing acceptance for any other F-240 row or permits a tolerance change. At adoption, retain the original source disclosure and review chronology, update normal contributor/date housekeeping, and run the normal affected verification. The independent bounded GLM source-risk cross-check is separate and remains advisory.
