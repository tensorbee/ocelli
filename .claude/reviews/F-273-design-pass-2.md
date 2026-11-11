# F-273 design review, pass 2

**Reviewed**: private PROPOSED-F-273-design-v36-before-own.md, SHA256 cf428650785f57bc3b6ed96d25a9dfc59b69565ae6be3a8ec2ece1c9b6791df0, all 266 lines. Canonical plan and withheld operator GroundTruth are unchanged.
**Author / reviewer**: GLM assembled prose, Codex coordinator supplied explicit admission rulings and source facts, independent Codex root review of the GLM proposal. GLM completed its own nonclean review in normal-plan-own-v37 before this review. Root is not an independent reviewer of its own admission rulings.
**Result**: 4 defects, 0 smells, 0 nits. Implementation is not approved.

## Acceptance challenged

AC10, AC13, AC17 to AC20 and AC24: a selected build label or an image that finished prefetch must not certify the displayed image. Trace the real load delegate, its promise, the actual current viewport image and nonrendering provenance through the existing bench consumer. AC1 to AC8 and AC22 need runtime and independent pixel controls, not fixture bytes or record-schema tests alone. No new clinical observation or browser execution is claimed by this design pass.

## Evidence

| Material | Review action now | Observation |
| --- | --- | --- |
| Exact frozen proposal | Read all lines, compared the actual own report and prior pass | Domain and equipped procedure improve, four blocking contract groups remain |
| tools/oracle/page/bench.mjs | Checked current-image caller and metadata/ready/snapshot locations | Existing viewport.getCornerstoneImage is available, installed-route facts alone do not bind a displayed object |
| tools/bench/src/comparison_run.mjs launchSide | Read current actor and fresh-page lifetime | Each browser.newPage owns a fresh context, per-page installation must precede load |
| tools/bench/src/comparison.mjs publishedRecord | Read existing closed validation consumer | New provenance requires exact schema and real producer, not a requested label |
| tools/bench/tests/comparison_page_test.mjs | Checked existing opt-in sites | Actual opt-in is OCELLI_COMPARISON_BROWSER, not OCELLI_BENCH_BROWSER |
| Prior synthetic render and mutant receipts | Historical evidence only, retained from prior review | Four pixel predicate exits are positive0, missing1, double1, restored0, all procedure exits0 |

## D1, selected loader and object contract remains internally wrong

**Where**: proposal lines155 to157 and168 to172.
**Trigger**: implementing the specified install and promise wrapper literally.
**Expected**: register the selected wrapper once, then invoke the actual public naturalized load delegate per request, preserving its loadObject fields and validating the actual yielded image. Set effective flags on image and image.imageFrame.
**Actual**: line168 describes invoking registerImageLoader as the load-object operation. It does not name the actual naturalized public delegate in the interface contract. Line171 sets image.frame, while line172 correctly expects image.imageFrame. The exposed params, pending, claimed and context inputs lack defined ownership although claims are module owned.
**Contract and repair**: WORKFLOW design interfaces and HLD24 local reasoning. Name the actual public delegate and exact registration versus request operation. Remove undefined injected state. Define the minimum exact helper signatures and module-owned weak state. Correct image.imageFrame before approval. Add actual browser controls for the requested options, preserved delegate fields, flags and once-only scaling. A pure mocked image is not runtime evidence.

## D2, provenance still cannot certify the current displayed producer

**Where**: proposal lines157,175 to191,201 and233.
**Trigger**: two images load or prefetch out of order, or a stock build emits a new-format record.
**Expected**: Node hash facts bind the exact build, and browser facts bind the actual displayed object, separately from installed-route facts. Stock has an explicit producer for its required false installation fact. The external manifest digest has a noncircular definition.
**Actual**: describeReferenceRuntime has no current object input or observation path and returns installation facts only. WeakSet claim membership is neither successful scaling nor displayed-object identity. No exact successful-object facts interface connects the current viewport image to the record. Stock provenance production is unnamed. The external manifest contains a separately computed manifest hash but the digest payload and hash location are still unspecified.
**Contract and repair**: microscope candidate-producer tracing and WORKFLOW acceptance boundaries. Define weak object-keyed successful facts separately from claim membership, observable through the actual current viewport image outside timing. Public cache lookup is acceptable if its exact key and current-image binding are established, and is not defective merely because another caller is simpler. Define exact closed value-free record keys/types, stock producer and missing/wrong-object refusal. Pick one noncircular manifest construction, either an exact unhashed envelope payload with external digest or a fully written manifest file whose digest is recorded outside it, not a circular field. Node owns source/asset hashes. Do not add hashes or pixel samples to real image objects or timed provenance. Test current display versus a later prefetched object.

## D3, permanent evidence map still substitutes fixture and schema tests for actual runtime behavior

**Where**: proposal lines204 to209,221 to244 and262.
**Trigger**: execute the listed pure tests or the listed browser command, then claim the arithmetic, GPU, ownership or cache ACs.
**Expected**: fixture writers prove fixture bytes. Record-schema tests prove shape. Actual built-page browser controls prove decoding, scaling, current-object facts, aliasing, GPU pixels, cache/eviction and retained behavior. The browser command must enable the real suite.
**Actual**: AC3 names viewer_work_test gold as GPU proof, AC4/6/7 name comparison_test for transform and sequence behavior, and AC17/19/20/22 name viewer_work_test for loader, cache and actual utility mutants. Those files currently have different consumers. The vague proposed test extensions do not explicitly give these ACs an actual browser executor. AC5 has no exact permanent case. Line262 uses the wrong environment variable, so the actual opt-in cases skip. AC18 also ambiguously says no geometry, capture inside clocks, contrary to the explicit no-capture timing rule.
**Contract and repair**: HLD27.2 specification-derived tests, microscope executed membership, workflow cross-target verification. Give each runtime AC an exact case in the existing actual browser file and OCELLI_COMPARISON_BROWSER=1 command. Keep pure writer/schema/orchestration cases separate. Define rendered gold and missing/double utility mutation cases through the real public loader route. For AC11, specify a real refusal observation and protocol abort with no timings, rather than a conditional invented recovery API. State capture stays outside clocks. Preserve existing registration, no automatic discovery claim or fake Core import as runtime substitute.

## D4, supported descriptor domain contradicts itself

**Where**: proposal lines88,97 and120 to122.
**Trigger**: an 8-bit scalar image meets the advertised allocation-width condition.
**Expected**: an unambiguous supported descriptor set and corresponding independent fixtures.
**Actual**: 8 or16 bits allocated are allowed, but the only named stored descriptors are signed12, signed16 and unsigned16. No valid 8-bit container can satisfy those descriptors. The prose and matrix do not agree on whether8-bit input is admitted.
**Repair**: restrict this initial proposed domain to16-bit allocation and explicitly refuse8-bit until a separately reviewed extension, or author and review exact8-bit descriptor rules and fixtures. Do not silently widen the domain. Required selected-input refusal still blocks full AC24.

## Prior findings and own report disposition

Pass1 numeric and decision-namespace mistakes are repaired. The raw sequence-presence policy now distinguishes Modality LUT precedence from affine scaling. The source-specific provider path, required-input failure, existing seven metrics and five workloads, and operator-held publication are substantially clearer. The explicit identity/default ruling is limited to this route, and is not a general DICOM rejection policy.

The completed own report has useful supported findings, but its proposed acceptance of a refused required workload as an alternative AC24 success is rejected. Required complete equipped success remains owed. Its mutant-exit inconsistency is not a reproduced error: the draft's missing1, double1 and restored0 are correct, and adding the positive0 is optional clarification. The38 unchanged source/provider checks are historical evidence, not a new performance gate or an AC that must keep that number. A synthetic actual browser is valid runtime evidence even when not an equipped timing host. Pure Node is not a replacement for that browser evidence.

## Limitations and handoff

No implementation, updated canonical plan, source checkpoint or new equipped run is approved here. SwiftShader diagnostic results remain bounded to the four existing fixed synthetic fixtures. F223 AC18 and AC19 stay open. Repair the same private proposal in bounded source-complete fragments, then GLM completes a findings-only own review and root reviews the new exact proposal. Preserve all rejected or incomplete outputs. Do not repeat another broad rewrite or change arithmetic, budgets, N, selection or publication authority.
