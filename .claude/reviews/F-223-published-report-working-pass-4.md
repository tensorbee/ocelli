# F-223 post-remediation full comparison, independent working review pass 4

Reviewer: Codex. I authored no F223 runtime or report-generator code. Root ran the comparison. My separate colour/native instruments are excluded from this approval. Review used frozen source/metadata and independent Python arithmetic without importing the production summarizer, hardware jobs, model inference or candidate/index edits. Earlier code-repair and measured-run reviews remain unchanged.

Result: **0 defects, 0 smells, 0 nits** within the new default F223 evidence and generated-report scope. The ordinary published run directly exits 0 in 232.441650042 seconds, from 2026-11-09T01:06:37.827748Z through 01:10:30.270060Z. No supplemental colour mode or injected services is selected.

## Protocol and arithmetic

The independent checker exits 0 with no errors, reconstructing all 44 statistic cells and 31 Markdown metric rows. It verifies the seven declared metrics, complete workload/condition/side coverage, declared unavailable cells with N 0 and reasons, required per-side N 9 for ready/open/first/window, N 5 for scroll and N 3 for memory/CPU. Actual trial values equal saved vectors. Every measured block has the required count and unique summary. Medians, linearly interpolated quartiles, minima/maxima, separation decisions, three-significant-figure ratios, rendered rounding and MiB conversions agree without a new tolerance.

AC7 uses one start side per declared metric/workload visit, carried across metrics and unavailable visits, with both cold and warm conditions sharing it. All saved starts and alternating per-trial sequences match the independently reconstructed protocol. First-image starts are cornerstone, Ocelli, cornerstone, Ocelli for synthetic CT, synthetic DX, pseudo CT and pseudo DX, respectively, in both conditions. Page-ready starts Ocelli, open starts cornerstone then Ocelli, scroll starts cornerstone then Ocelli, window starts cornerstone/Ocelli/cornerstone/Ocelli, and memory starts cornerstone. CPU derives from the actual memory sessions rather than introducing another loop. All twenty scroll observations meet key-count/zero-blank predicates and their FPS equals observed frames divided by elapsed time.

The six memory sessions contain 7/6/6/6/6/6 rounds in actual trial order. Successive recorded timestamps differ by 403.714667 to 434.687625 ms, above the unchanged 250 ms minimum. The publication explicitly identifies sampled concurrent footprint as a lower bound and sum of per-process peaks as an upper bound, with the minimum pause distinguished from raw timestamps. I recomputed both figures from every saved process map. Each lower bound is no greater than its upper bound. Product/infrastructure inventories are disjoint. CPU before/after PID sets match, differences are nonnegative and sums equal all six recorded CPU values and their summaries. This validates saved observations and arithmetic, not an independent kernel sampler.

## Identity and input qualification

Measurement HEAD: d34d9baff2477fae9f98e904dae643a4edcbb5f5.
Measurement index: 0fad23dfbd206135c1dfd51d1123e04df2b79bf4, worktree matching index.
Native binary: 85b4baaf65f5fe139dc9cc4f8e93a8bc1b5b400607103989f23175fc8fce082a.
Comparator: ce91b6547a5408da6beb7cacf094c73bd101050342be5190b49dedc9a5f866cc.
Comparator page: d9cfa522c0a6c934e79c4d34ad2bd40bf5e234a20a951a26b95d0ad8e694c1ca.
Input identity: d87461fd59a9dc875b6b5ffe96f20ff7640059f8cd69c04c89adc97a7ff5aa58.

Actual recorded versions are Chromium 151.0.7922.34, Playwright 1.62.1 and both cornerstone packages 5.8.2. Node is absent from the raw closed version schema. The direct launcher explicitly prepends the v24.13.0 installation to PATH, so Node 24 is attributed to launcher selection, not an invented in-run self-report or retrospective preflight.

The launcher selects the controlled fixture directory in its --pseudo argument. The metric keys retain pseudo-full/CT/DX names, but the operator identifies this as controlled synthetic input. Counts are 1693/1 synthetic CT/DX and 201/200/1 in those pseudo slots, with the same retained input digest as the earlier controlled run. This is a reproducible controlled-input timing matrix, not a claim of clinical diversity or general clinical pixel parity. The pixel precondition passes. No DICOM payload was opened by this review.

## Publication, source guards and cleanup

Raw SHA256: 30121e5b223951c6aeb635376b6373ac1079fdda3a4ed153ed51342fa10a806b.
Published JSON SHA256: 1baf304a1abd271188c2edec004f1a3692f372170073aa70cffd47993b1b33c3.
Published Markdown SHA256: 42b8ca2087cb7055082b753906ce5def4e11bb88745e2c6b827bc36257c17cf1.

The staged publication tree a601d30620b727abda7225b3b215dea76e31d358 differs from measured 0fad only at the generated pair. Frozen bytes equal the current tracked pair, and every published subtree equals its raw counterpart. The closed published subset intentionally omits source/trial/condition metadata, retained separately in this snapshot.

Source captures HEAD/index/worktree agreement before work and recomputes it after observations before saving. There is no separately serialized end-source object, so none is asserted. The real command rejects injected or reused-build publication, closes owned actors/static server/temporary input, removes the output lock and rejects cleanup or interruption before tracked replacement. Direct exit 0 and publication pass those guards. The actual run output lock is independently absent. No whole-machine process census, full input-content rehash or live native-binary rehash was run during the next exclusive timing batch.

Host conditions record load 3.412/3.040/3.375, not an idle-machine claim. No baseline, tolerance, status or source change is proposed. This review approves only the stated default measured-run/report scope. Colour/ICC supplements, five-subject movement bands, manual acceptance and sprint closure remain separately evidenced.
