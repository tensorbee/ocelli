# F-245 design microscope, pass 1

Reviewer: Codex, independent of GLM semantic authorship and consolidated own review. Reviewed the complete private plan `f70698b545fc8885b7eea05bc597492085a4045a0c41c4f74afea3bd1aa6f203`, delta `e2f2828fa4e1076fdef2ea7cfd3784e23d5741d1ac92bd152e2f96b5db5aa722`, canonical draft and relevant current provider code. Source inspection captured at 47356703 remains byte-relevant after the documentation-only 265f247e and 1f2809c0 commits. No implementation is reviewed or approved here.

Result: 2 defects, 1 smell, 2 nitpicks. Keep the canonical plan draft. No implementation is authorized by this pass.

## Defects

D1. The approval artifact no longer contains required delivery sections and its exact write map cannot satisfy its own AC7. In particular, it omits `examples/folder-viewer/src/use.html` and `examples/folder-viewer/src/more.html` while changing the primary npm launch, publishing new APIs and requiring More's partial-source matrix update. The unnamed ESLint configuration is actually `eslint.config.js`. The proposed replacement also drops explicit Boundary and tier, Tests, Parity surface, Deviations, LLD impact and User-facing documentation impact sections. The design template requires all three tier rows, and complete-feature steps 9 and 10 read those impact sections. A general file list and all-tiers sentence do not provide that delivery contract.

Restore the sections using the settled semantic choices, with exact API, native/npm, partial remote status and guide destinations. Keep the existing API blocks and legacy behavior intact. Declare test categories and real consumers without inventing an arithmetic fixture or remote rendered proof. No new deviation is implied by formatting the artifact. Either name the actual conditional ESLint write or state no change with its reason.

D2. AC8 lists subjects and says release runs are needed, but does not bind the external performance path to an actual host, input selection, production source entries, observed completion and collector procedure. The design readiness rule explicitly requires that feasibility path before approval. Root checked the actual registry, whose existing five IDs are `viewer.open`, `viewer.first_image`, `viewer.window_step`, `viewer.scroll_rate` and `viewer.resident_memory`. Link the existing benchmark procedure, choose the available current ARM Mac and pinned Node/Playwright Chromium cohort, and specify literal before/after release source identities and the otherwise-idle acquisition prerequisite. Reconfirm actual environment at measurement time. Keep global registered bands and host comparability rules unchanged. This is an unperformed observation, not an acceptance waiver or a request for a new performance axis.

## Smell

S1. Published nested receipt ownership is not precise enough. `StagedMetadata` applies shallow Readonly while `identity` has mutable `InstanceIdentity`. Root type-checked the exact two proposed type declarations: assigning `receipt.identity.studyUid` has no diagnostic, while assigning outer `receipt.byteLength` correctly fails as readonly. This proves the declared type's behavior, not a current runtime alias bug. Evidence is `/private/tmp/ocelli-s46-codex/f245-root-design-review/readonly-schema-proof.json`.

A reasonable shallow freeze or shallow DTO copy could expose the internal identity object, then a caller could change the identity while the retained raw body remains unchanged. If expected and actual receipt checks share that nested object, the check can validate its own mutation. Choose and state deep immutable receipt ownership or detached deep snapshots with comparison against a private independent receipt. Cover mutation of the original input identity and nested returned DTO before and after asynchronous settlement and before transfer, including refusal followed by a valid take/refetch. The actual parsed UID/hash/body association must remain independent of exposed aliases. Preserve the single owned body and no-copy transfer contract. Do not label this an observed product defect, because the API does not exist yet.

## Checked design direction

The current source supports preserving legacy Promise<void> client methods while adding a separately selected bounded receiver path. Actual source confirms the shell string codes used by the proposed error map exist, and native SourceError and ParseError limit variants are correctly marked proposed. Numeric ring codes remain separate. The plain parser's unbounded deflate reads are a real source prerequisite, addressed by the proposed new explicit-limit route rather than a false total-heap claim. Full object and RSS peaks remain unmeasured.

Root checked the relevant [PS3.18 series response rules](https://dicom.nema.org/medical/dicom/current/output/chtml/part18/sect_10.6.3.3.2.html) and [instance response rules](https://dicom.nema.org/medical/dicom/current/output/chtml/part18/sect_10.6.3.3.3.html). A constrained query need not return all parent attributes. Requiring the actual leaf UID while separately retaining optional returned parents and captured hierarchy avoids refusing valid responses or pretending echoed scope was parsed. WADO's full actual dataset tuple check remains a different condition.

The persistent finite operation pool, canceled-but-unsettled slot ownership, latest-only metadata queue, immutable operation context, explicit exact-cap acceptance and pre-second-part refusal are coherent selected choices. Actual remote fetch/staging remains separate from displayed local identity, marks and conformance. No remote pixels or worker decode are claimed before F-202. Complete multipart representation and boundary remain attached to the staged owner for its future consumer.

The one native HTML application, compiler-artifact-based launcher, preserved public native entry points, flat module closure and context-owned main policy are source-grounded choices. Only the actually fetched origin enters connect-src. Legacy/default contexts stay local. No auth forwarding or native upstream proxy is introduced.

## Nitpicks and next step

The runtime series/instance scope-key bullet is duplicated. The term native decoder in the QIDO section could be parser to avoid suggesting pixel decode. Neither requires an API rewrite.

GLM's own consolidated 0D/0S result is preserved as an author claim, not independent approval. Root challenged the delivery consumers and nested DTO combination beyond that report. GLM owns a bounded plan amendment and focused own review. Root will review the corrected whole artifact. F-266 still owns the product bootstrap files until explicit writer release. No product source, new workflow, acceptance, canonical completion, integration or publication follows from this review.
