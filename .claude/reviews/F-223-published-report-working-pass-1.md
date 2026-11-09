# F-223 published report and arithmetic, independent review pass 1

Reviewer: Codex. I authored no F223 runtime or report-generator code. My F239/F240 native colour benchmark instruments and AC27 setup are outside this independent approval. Source reviewed is frozen tree 2d55a0cfc7f3572aa7f7a9ca171dccde0a79c1d8, with published JSON/Markdown snapshotted alongside this report. Root executed the full run. This review executes only small, pure arithmetic over saved metadata, with no browser, native build, corpus pixel read, timing run or candidate/index edit.

Result: **1 defect, 1 smell, 0 nits.** The existing run is numerically complete and internally consistent. It does not satisfy AC7's starting-side protocol. Root has accepted the finding, assigned the repair and required a fresh full run. The original observations remain retained, not relabelled as accepted.

## D1, starting side flips per condition instead of per workload

Authority: F223 plan decision 16 at line 99 and AC7 at line 286 require a chosen starting side to flip between workloads. Source comparison_run.mjs lines 579 to 641 iterates workload and cold/warm condition together and toggles firstSide after each measured condition. This is a protocol defect, not a wording exception. The first-image raw record makes it concrete:

| Workload | Cold first side | Warm first side |
| --- | --- | --- |
| synthetic-ct | cornerstone | ocelli |
| synthetic-dx | cornerstone | ocelli |
| pseudo-ct | cornerstone | ocelli |
| pseudo-dx | cornerstone | ocelli |

Every individual cell alternates its two sides correctly and has the required N. But completing two conditions toggles twice, so the next workload starts from the same side. Correct orchestration must choose one start side per workload, reuse it for its cold/warm conditions, and flip for the next workload. Root requires a real-orchestration RED/GREEN, preserving the old record and rerunning the full matrix after repair. This review does not rewrite any observation or claim that arithmetic consistency cures AC7.

## S1, primary memory row is not identified as the sampled lower bound

Authority: plan decision 8 at line 91 defines the maximum sampled concurrent process-footprint sum as a lower bound at the observed sample times, and the sum of per-process kernel peaks as an upper bound. The published Markdown labels the upper row, but calls the primary section simply peak memory and labels its first row only session/cold. It never explains that primary figure as the sampled concurrent footprint/lower bound, or states the minimum sampling interval. A reader cannot recover the method from the printed bound labels alone.

The arithmetic is correct. The generator should disclose the two methods, using the existing FOOTPRINT_PERIOD=250 ms constant rather than inventing another duration. Minimum spacing is not an exact 250ms cadence: the raw record retains actual sample_times_ms, and all six sessions have intervals at least 250ms. Root has accepted this as a separate smell and assigned generator-text repair. The tracked files are left unchanged here.

## Independent arithmetic result

arithmetic.py independently reconstructs summaries without importing summarise, separated or the production renderer. It checks the entire seven-metric workload/condition matrix from comparison.json, both sides, exact required counts and unavailable cells with reasons. Forty-four statistical cells, including two memory upper bounds and both CPU summaries, match exactly. All 31 printed metric rows match their required values, units, quartiles, separation decision and ratios. Duration and CPU rounding, MiB conversion and three-significant-figure ratios agree with the stated report rules. No new numeric tolerance is used.

For every measured timing cell, N is 9 for ready/open/first/window, 5 for scroll and 3 for memory/CPU. Quartiles use linear interpolation between sorted order statistics. No fewer-than-N cell is published as a median. The two sides' current measured IQRs separate where a numeric ratio is printed. Touching intervals remain non-separated in the source's strict comparison, independently of these particular observed values.

For all six memory sessions I recomputed max(sum footprint per round) and sum(max kernel peak per PID) directly from the saved maps. Each lower result is no greater than its upper result, and infrastructure PID maps are disjoint from product maps. The six before/after CPU inventories have identical PID sets, no backward counter and exact summed deltas matching their recorded CPU seconds. The resulting three-session summaries agree with the published values. This verifies recorded arithmetic and inventory consistency, not the kernel collector or the process-inventory mechanism independently.

## Source, inputs and publication identity

The direct full-run receipt exits 0 after 249.4611s. It names the ordinary comparison mode with --publish and the controlled pseudo root. Raw comparison.json SHA-256 is 02b1869b1fd69781f8baff6e21b99351ce1520ff2dac5cca07f09d78b4984ba9. It binds:

- staged source 2d55a0cfc7f3572aa7f7a9ca171dccde0a79c1d8, worktree matching index
- native binary 0b8b34dc38d8da0ed427927fc19c7b2f3d85316543b012d2bc2f15a3d2054c7a
- comparator ce91b6547a5408da6beb7cacf094c73bd101050342be5190b49dedc9a5f866cc
- comparator-page bundle d9cfa522c0a6c934e79c4d34ad2bd40bf5e234a20a951a26b95d0ad8e694c1ca
- input identity d87461fd59a9dc875b6b5ffe96f20ff7640059f8cd69c04c89adc97a7ff5aa58

The published input digest, machine, versions, defaults, workload counts, all metric subtrees and pixel verdict equal the corresponding raw subtrees exactly. Workloads are 1693/1 synthetic CT/DX and 201/200/1 controlled pseudo-full/CT/DX files. The raw native binary hash is coherent with source.binary_sha256. The declared source identity is the sourceIdentity guard's actual start/end record, not inferred from a filename. During root's performance hold I did not rehash live binaries or reread all input files, so the source/binary/input binding is verified through these saved identities and runtime source checks, not a new independent content census.

The published closed schema intentionally retains the input digest and omits raw source/binary/trial/condition fields. That matches the approved closed publication subset, with the raw handoff retained by digest. It is not an accidental loss of an allowed source field. The pixel precondition passes and its numbers match the published section. A passing pixel check is not proof of timing protocol conformity.

## Separate controlled colour mode

comparison_run.mjs returns from its colourInput branch at lines744 to 754 before it reaches measureAll at line758. measureColourComparison calls collectPreparedColourComparison in comparison_colour.mjs, which owns separate trial, window-step and memory loops and reverses side order by its own trial index. It has no reference to measureAll or that function's firstSide state. Therefore the default matrix AC7 defect is not inherited by that supplement through the callgraph. This is a bounded separation check, not an independent approval of my colour instruments, a new audit of all supplemental protocol details or a reason to invalidate already completed colour observations by inference.

## Review execution and limits

The pure arithmetic inspection exited0 and wrote arithmetic.json with no discrepancies. An initial metadata exploration assumed every raw trial object had samples and stopped with KeyError at the first detailed scroll object. The corrected audit distinguishes detailed observations from summary cells. That inspection mistake was not a product or benchmark failure and is not counted as a passing command.

No report, source, gate, tolerance or baseline was edited. Root owns the AC7 repair, S1 clarification, regenerated pair, actual rerun and final acceptance. Existing F216 absolute-budget and historical movement-band observations remain separate from this F223 comparison, whose rAF endpoints differ by approved design.
