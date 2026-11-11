# F-240 source basis and availability, pass 2

Reviewed 2026-11-09. Result: 0 defects, 0 smells, 0 nitpicks in the bounded root-authored adoption delta. Exact patch SHA-256 is `44bd807c707912fdca64a417d7f22faffcb8adcc52c5ded73874d971be54e9cf`, preserved at `/private/tmp/ocelli-s46-codex/f240-current-doc-adoption.patch`.

## Scope and authorship

The reviewer authored neither the two proposals nor root's adopted documentation and comment edits. The reviewer previously reviewed the proposals and authored the current AC12 re-observation driver and some measurement notes. This pass does not independently reaccept that instrument or its observations. It checks the actual adoption diff, explanatory code comments and index/header consistency. The provisional completion metadata, readout repair and other canonical changes are outside this pass.

The patch has eleven paths: the F-240 plan, cpu.rs, the LLD index and eight LLD files. A read-only postimage check proved all eleven actual canonical blobs match the patch's expected after identities. Twelve updated index rows exactly match their current contributor-header sets, with 2026-11-09 dates. These are twelve index/header correspondences, not a claim that twelve header files changed in this patch. Evidence is `/private/tmp/ocelli-s46-codex/f240-current-doc-independent-review/checks.json`, SHA-256 `c3c8ec0a3aaed74a0a00ccd6cdd63c1c0f72c2827bfae8238248ddd960e05261`.

## Accepted delta

The CpuRaster availability row correctly names the native CPU matrix/TRC ICC subset. `RasterColor::supported` accepts the resolved transform without an unconditional ICC rejection. The shared mapper applies that transform. GPU `ColourLut::icc` still returns unavailable and `RenderGraph::new` still refuses Tier C. The row preserves that graph refusal and the outstanding representative-host qualification. It does not claim full ICC or GPU colour management.

The four changed cpu.rs documentation lines now describe an explicitly unavailable input or descriptor. They remove the incorrect universal ICC refusal without replacing it with the rejected model suggestion that all outside-subset profiles are refused. D-30's named device-RGB fallback remains intact. Both old and new source have 1,247 lines. Only documentation lines 50, 112, 359 and 360 differ, with before/after SHA-256 values matching root's comment-only proof. No executable Rust token changes. This does not by itself establish a rebuilt binary identity.

The plan now lists render-graph.md in its actual LLD impact. The dated AC12 reconciliation explicitly adopts the already reviewed ICC.1:2022-05 and official ICC sRGB interpretation note. It preserves the original source history and states that IEC publisher text was not obtained or read. It retains the current six-profile external comparison and investigated clipping flags, without changing the matrix, fixture values, clipping policy or tolerance. This is explicit lead technical reconciliation of the literal source wording, not a claim that direct IEC review occurred or that the operator gave a new approval.

The index adds existing contributions rather than deleting previous F-IDs. The oracle header is unwrapped without losing an ID. F-240 is added to the corpus contribution list. Other LLD edits in this patch are date changes. The unrelated living bodies remain unchanged, apart from the reviewed availability sentence.

## Checks and limits

Read-only identity and consistency assertions exited 0. They verified eleven actual postimages, twelve index/header sets, current dates, the exact four comment lines and unchanged CPU source line count. No compiler, test, browser, GPU, corpus, profile, commit or publication ran in this pass. The reviewer wrote only this review record and private evidence. Final metadata-tree verification remains root-owned.
