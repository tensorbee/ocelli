# F-273 Reference scalar normalization for equipped comparison

**Status**: approved
**Epic ref**: E38.64
**Sprint**: S47
**Estimate**: 3w provisional, not measured, not capacity-approved

**Declared dependency**: F-216 done provider only. F-223 is carried with AC18 and AC19 open. F-223 is not a structural dependency in either direction.

**Modality note**: this is an explicit modality-rescale ownership change. It is not an affine-only display or VOI correction.

---

## Ground truth carried from F-223

Approved rule records pixel refusal as a recorded outcome, not passing AC18, with a file-divergence story, and AC18 and AC19 stay open. The actual operator-root refused-pixels verdict is max 1, signed mean bias informative 0.3279290752351097, informative fraction 0.233642578125, low information false, passes false, with no timings and no publication. The actual refused-pixels report is retained privately, not discarded. The earlier measured controlled 201 record stays separate and is not actual study acceptance.

Own retained synthetic evidence remains recorded, interior stock-restored bias 0.3191428998631308 with 83243 reference exact mismatches and zero native gold mismatches, private uniform-diagnostic interior bias 0.00004600644857054131 with 26216 reference exact mismatches and not adopted, adjacent signed minima -32768 and -32767 at C=-32767 and W2 requiring 0 and 255 where stock and uniform-corrected both refuse with 131072 reference mismatches, max 255, zero native gold mismatches, not low information. Native independent gold is unchanged. Historical prepared producer stays b889 tree 752a, not 1d execution.

## Normative source transcribed

Source: `docs/hld/15-lut-chain.md`, section 18, normative. A deviation is raised in a design plan, not improvised.

### 18.1 Modality LUT

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

The HLD adds: if a Modality LUT Sequence is present it wins over slope and intercept. PET SUV is a separate path and needs the radiopharmaceutical sequence. It is not folded in here.

### 18.2 VOI LUT, the three functions

The HLD states the differences between LINEAR and LINEAR_EXACT are a half and a one and that these are the single most commonly mis-ported detail in DICOM viewers. Copy these exactly.

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

### 18.3 The fixture that proves it

Soft-tissue CT window, centre 40, width 400, output range 0 to 255. The HLD states it is hand-computed and must be in the test suite before the shader is written. The numeric columns are quoted as transcribed, including the erroneous LINEAR_EXACT value in row 1, which D-13 corrects on formula authority.

| Input (HU) | LINEAR | LINEAR_EXACT | Why this row |
|----|----|----|----|
| −160 | 0.000 | 1.594 | LINEAR boundary is c'−w'/2 = −160 exactly. The comparison is <=, so this clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| −60 | 63.910 | 63.750 | Mid-lower quarter. Catches sign and slope errors |

**Numeric correction under D-13, stated explicitly.** The transcribed `LINEAR_EXACT(-160) = 1.594` is retained above as a source transcription, not as a usable expectation. D-13 establishes that §18.2 is the formula and §18.3 is a worked value, and where they disagree the formula is the specification. §18.2 clamps at `x <= c - w/2`, which is `40 - 200 = -160`, and `-160 <= -160` holds, so the value is `ymin`. The formula body evaluates to `((-160 - 40) / 400 + 0.5) * 255 = 0.000` there in any case, so no boundary convention produces 1.594, which is the value at `x = -157.5`. not all table decimal entries are exact, since LINEAR(40)=127.819548... rounds to 127.820 at three places whereas the table value 127.819 is truncated, while clamp255 remains exact, and formula authority and D-13 LINEAR_EXACT(-160)=0.000 are retained. The 0.32 headline figure is unaffected. The correct fixture value on formula authority is `LINEAR_EXACT(-160) = 0.000`. This correction is not applied quietly.

### Other normative rules invoked

- 18 stage order: Modality, VOI, Presentation, Palette/ICC, applied strictly in order. Implement once, in ocelli-pixel, and let the shader read the parameters. A second copy of this logic must not appear anywhere.
- `23-performance-rules.md`: measure before optimizing. No performance claim is asserted here without measurement.
- `docs/hld/11-decision-log.md` Decision D14: measured divergence, as recorded.
- `DEVIATIONS.md` D-07: a CPU tier may be resolved. CPU reports are stated honestly as unavailable only when the route is actually unsupported, not extrapolated.

---

## What the specification does not cover

- The HLD specifies the arithmetic chain. It does not specify a public utility that normalizes an image's stored values to Float32 through slope and intercept before VOI, nor where such a utility sits.
- The HLD does not name `utilities.scaleArray` or a pre-core public entry point for that normalization.
- The HLD does not specify how a reference variant is declared, bound, refused, or how it is separated from the stock oracle default.
- The HLD does not cover the comparison harness: no reference-variant option exists in `tools/bench/run.mjs` parseArgs today, and the closed record, schema, renderSummary and report in `tools/bench/src/comparison.mjs` do not carry a reference variant field.
- The HLD does not cover shared view plus Core cache lifetime, failure after staging, or repeated, cache-hit, inflight, eviction and retained generation ownership.
- The HLD makes no general 32-bit claim. The public Float32-style target is a candidate requiring source, API, precision, rescale and lifecycle design.
- Generalization, PT, Modality LUT sequence, color, presentation and capability decisions are governed by the approved contracts below.

---

## Problem and bounded evidence

Two distinct defects motivated this route. First, fractional prescale precision loss: the default-prescale targetFloat32 path lost fractional precision in Modality LUT values. Second, a historical signed-minimum scalar/upload failure affecting CT MONO2 signed16 data. The intermediate route, targetFloat32 plus preScale:false, preserved raw values but did not apply modality rescale in actual GPU observation. The fourth route combines representation and scaling ownership: the raw naturalized Float32 delegate retains source data, and scaling is applied explicitly and once.

Typed facts: positive_procedure_exit=0, positive_wall_seconds=5.510, positive_groups=12, fourth_captures=8, fourth_full_RGBA_gold_matches=8, missing_utility_procedure_exit=0, missing_utility_gold_predicate_exit=1, double_utility_procedure_exit=0, double_utility_gold_predicate_exit=1, restored_procedure_exit=0, restored_groups=3, restored_gold_predicate_exit=0, restored_wall_seconds=2.198, unchanged_source_provider_checks=38. These are SwiftShader WebGL2 synthetic diagnostic observations covering ONLY four fixed CT MONO2 signed16 fixtures. Earlier three modes' exact pixels are unchanged. No general CT, clinical, or equipped acceptance is claimed.

## Admission and transform ownership

Approved proposed bounded admission: when both raw rescale tags are absent, identity is permitted ONLY if the actual winning provider resolves exactly slope 1 and intercept 0. When both tags are present, admission requires finite valid resolved values that match the raw values and a nonzero slope. Partial presence, zero slope, malformed or nonfinite values, or any disagreement between raw and provider refuses before candidate success. This is this route's conservative supported-domain policy, not a universal DICOM rejection rule. Explicit identity uses public scaleArray once as well, so once-only utility controls apply uniformly to identity and non-identity cases alike. Any required selected-input refusal blocks AC24, and never falls back or substitutes. The remaining domain matrix names CT/MR/CR/DX/MG, MONO1/MONO2 single-sample supported stored descriptors signed12, signed16 and unsigned16 with allocation of only 16 bits, HighBit=BitsStored-1 and PixelRepresentation matching the descriptor. Raw presence of any Modality LUT Sequence or foreground (FG) refuses. Classic multi-frame without FG uses the top-level transform. PT, RTDOSE, color, palette, windowless, and non-LINEAR functions refuse, as does the CPU reference path. Synthetic DX carries explicit m1/b0. The proposed broader domain is not already proven and remains subject to verification.

Root-reviewed bounded policy requires the appropriate VR and VM plus complete numeric syntax for raw RescaleSlope (0028,1053) and RescaleIntercept (0028,1052). A finite `parseFloat` prefix or first-value-only acceptance is not validity. Both tags are DS, VM 1. The full value lexical form and multiplicity are validated before provider comparison. Refusal cases: empty value, malformed lexical suffix, backslash or multiple values, nonfinite parsed value, zero slope, and a partial pair where only one tag is present. Supported implicit-VR input is preserved: no explicit VR marker is demanded where none exists in the encoding. The known tag VR plus the actual syntax and encoding context govern. An explicit wrong VR refuses. When both tags are absent, identity applies only when the winning provider itself resolves 1/0. When both tags are present with valid finite values, they match the provider. This is conservative candidate admission, not a claim of universal DICOM invalidity. Add exactly named BR-TX browser cases for malformed lexical and VM refusal and for valid explicit-VR and implicit-VR transform. Fixture writer byte tests stay separate from browser cases. No actual patient input is used.

### Admission scope and preconditions

Admission is evaluated against the original raw DataSet reachable via `public dataSetCacheManager.get(public wadouri.parseImageId(imageId).url)`. Because parseImageId strips `frame=` from the returned url, inspection uses that url and not the raw imageId. Missing raw source refuses. Tag presence in `DataSet.elements` is authoritative. Any source key presence, including empty or malformed values, counts as presence. Projected module absence is not treated as absence proof, since getLUTs may return undefined for present but empty sequences.

Proposed supported domain, all conditions required:

- Affine non-PT modality explicitly named CT, MR, CR, DX or MG.
- Verified MONOCHROME1 or MONOCHROME2 photometric interpretation, single sample per pixel, allocation of exactly 16 bits with stored descriptor signed12, signed16 or unsigned16, HighBit=BitsStored-1 and PixelRepresentation matching the descriptor, one sample, MONO1/2. 8-bit allocation is refused before candidate success until separately reviewed. This is route policy, not a universal DICOM invalidity claim.
- Finite shared Float32 pixel data.
- Finite transform metadata, with an explicit stated policy for identity and absent defaults consistent with the actual provider.
- No Modality LUT Sequence, no VOI LUT Sequence, no shared or per-frame Functional Group in the raw elements.
- No stored explicit zero slope, no nonfinite, partial or malformed transform metadata.

### Refusals, ordered

Refusal is a distinct outcome, never replacement selection, never silent stock fallback, never a passing label. A refused required selected input blocks full AC24 acceptance and leaves F223 AC18/AC19 open.

1. Source Modality LUT Sequence (x00283000) refuses before any affine mutation. This includes a synthetic discriminator carrying both a LUT and rescale tags with different hand-derived outputs, confirming LUT precedence does not silently become scaling.
2. Source VOI LUT Sequence (x00283010) refuses. No source-proven public GPU LUT application route is established in this proposal.
3. Shared (x52009229) or per-frame (x52009230) Functional Groups refuse. Classic multiframe without FG may proceed using top-level transform, public frame binding and each frame's actual decoded object.
4. PT and RTDOSE refuse before scaling. SUV factors derive from acquisition metadata not present in the affine path, and absence of suvbw is not equivalence proof. RTDOSE doseGridScaling has no public scaleArray route.
5. Color and palette inputs refuse as outside the scalar route.
6. Stored explicit zero slope refuses before the public `slope || 1` resolver silently substitutes 1.
7. Nonfinite, partial or malformed transform metadata refuses.
8. Windowless inputs refuse for this first route. Public `toWindowLevel` exists, but current createImage computes its fallback before the new scaling stage, so assuming fallback correctness is false and no recomputation policy is designed here.

### Supported and refused matrix

| Branch | Outcome | Actual public operation |
|---|---|---|
| CT, MR, CR, DX, MG MONOCHROME2 signed16 or signed12, affine transform, LINEAR window | Supported (proposed, unproven) | metaData provider wrapper wins at priority 0, then `core.utilities.getScalingParameters(imageId)` and `core.utilities.scaleArray(array, parameters)` mutate the shared Float32 array in place |
| Same descriptors, MONOCHROME1 | Supported (proposed) | Same scaleArray path, MONOCHROME1 inversion remains an existing GPU consumer render operation separate from CPU-side modality scaling |
| Unsigned16 stored, positive, negative, fractional, identity or default slope and intercept | Supported (proposed) | Same scaleArray path with parameters exactly as resolved by the actual provider, slope carries modality-units per stored-unit, intercept carries modality-units, never labelled stored-units |
| Source Modality LUT Sequence present, including LUT plus rescale discriminator | Refused | No mutation, refusal recorded before affine stage |
| Source VOI LUT Sequence or FG present | Refused | No mutation |
| PT suvbw available, RTDOSE, color, palette | Refused | No mutation |
| Explicit zero slope, nonfinite, partial or malformed transform | Refused | No mutation before the public resolver |
| Windowless input or VOI function other than LINEAR at benchmark presentation | Refused | benchmark.readWindow accepts LINEAR only, other functions refuse rather than promising all3 |

The previously implemented positive experiment covers only four hash-bound CT MONOCHROME2 signed16 fixtures through Float32 representation plus public pre-Core scaling. It does not approve the CT rows above and does not approve general CT.

### Required independent controls

The following controls require independent fixtures and an actual required-workload precheck after design approval, before any admission is treated as compatible:

- Signed minimum and signed full extrema, plus signed12 boundary descriptors.
- Unsigned16 full-range extrema.
- MONOCHROME1 inversion verified on the actual GPU consumer path, not a CPU reference. The CPU reference fallback remains unproved and is a named unsupported outcome in the selected variant, not stock fallback. Ocelli native CPU architecture and tiers are unchanged.
- Positive, negative, fractional, identity, absent-default and explicit zero slopes, and partial transform metadata, each as separate fixtures.
- Raw present-but-empty Modality LUT Sequence and FG fixtures confirming refusal despite projected module absence.
- The LUT-versus-rescale precedence discriminator with hand-derived differing outputs.
- Synthetic DX written through existing mono16Part10 with explicit slope 1 and intercept 0, never described as absent-rescale, plus selected pseudo DX and the actual pseudo-full selection run through the real equipped protocol.

No new arithmetic, tolerance, N, budget or publication authority is introduced by this section. Real unknown cases remain unknown until covered by the required precheck.

## Scope and interfaces

Scope covers one closed stock|reference build variant for the oracle bench page, one reference image loader wrapper installed at the existing provider composition point, one immutable external per-run artifact manifest, and one extended comparison publication schema. Domain policy (admission, transforms) lives in the separate Admission and transform ownership section and is not repeated here. The design is approved for isolated synthetic implementation after canonical assembly.

CURRENT facts retained: `tools/oracle/build-page.mjs` exports async `buildPage({dist=PAGE_DIST}={})` and returns dist. It builds app/volume, bench plus worker and copied codec assets. No variant CLI exists in that module. `tools/bench/run.mjs` parseArgs and `tools/bench/src/comparison_run.mjs` `runComparison(options,services)` are the actual comparison CLI and provider. Bench page `bench.mjs` `ready()` performs coreInit, `dicomImageLoader.init({wasmBasePath})`, registration of a priority 0 wrapper around public `wadouri.metaData.metaDataProvider` that only normalizes VOI function shape, then new `RenderingEngine`/`enableElement`, then input enabled. `runOpen` loads raw base headers through public `dataSetCacheManager.load` before grouping.

PLANNED interfaces, one option each, no alternatives:

- `buildPage({dist, variant})` with `variant` a closed `'stock' | 'reference'` string, default `'stock'`, omitted entirely meaning stock. Return value remains the dist string. The variant selects the bench entry asset only, baked as an immutable constant inside the bench entry. No production rebind setter, no runtime variant switching, no current CLI claiming variant behavior.
The F273 reference helper module exports exactly these three named bindings and nothing else.

1. `installReferenceLoader({ core, dicomImageLoader })` returns `void`. It owns a nested `wrapped(imageId, options)` closure. The actual public `core` and `dicomImageLoader` namespaces are in scope of that closure. Installation registers the nested function exactly once via `core.imageLoader.registerImageLoader('wadouri', wrapped)`. There is no separate exported `wrapWadoUriLoader`, no unused `loadFn`, and no pending, claimed, or params bags.
2. Each request inside `wrapped` captures the actual public `dicomImageLoader.wadouri.loadImageFromNaturalizedMetadata(imageId, mergedOptions)`. It preserves the delegate `loadObject` fields by spread and wraps the returned promise only. The request sets `targetBuffer: { type: 'Float32Array' }` and `preScale: { enabled: false }`.
3. There is no existing public all-purpose validation utility, and the plan must not claim one. The planned local nested wrapper performs domain, alias, and full finite checks. The actual public `core.utilities.scaleArray` mutates the buffer exactly once and does only scaling. Public `getMinMax` then computes statistics. The local wrapper sets four internal statistic fields plus both image-level and frame-level enabled and scaled flags. A WeakSet of claims and a WeakMap of success records, plus immutable installed route state, are owned by the module. No claim is stored in a WeakMap. There is no strong imageId cache. Installed route state as a permitted structure replaces any statement that the module holds "WeakSet/WeakMap and nothing else." Successful facts are written to the WeakMap keyed on the yielded image object after all validation and scaling steps complete and before the wrapper promise resolves. The public delegate and the registration surface remain exactly as already defined. No additional helper export is added.

4. `describeReferenceRuntime()` returns the actual installed route facts, with keys and types exactly as defined in the installed-route structure of table C. It is read-only description of the installed route. It does not inspect or claim success.
5. `getSuccessfulFacts(image)` returns the exact-object WeakMap facts for that image object, with keys and types exactly as defined in the current-object success structure of table C, or `undefined` when absent.

Module state is a WeakSet of claims, a WeakMap of success records, and an immutable installed route state, and nothing else. There is no strong imageId cache. This aligns with describeReferenceRuntime. Exact return keys for all helpers are the keys of table C. No new cache, framework, or API beyond these explicitly planned exports. The stock bench produces its own closed route and current facts. It does not import or install the reference helper.

- Comparison CLI: `tools/bench/run.mjs` parseArgs accepts `--variant <stock|reference>` ONLY for `--compare-cornerstone` WITHOUT `--colour-case`. Omission resolves `stock` for scalar. Explicit `--variant` in subject-only or colour calls, including explicit `stock`, is a fixed early reject before build, input, fs work, context, and timing, and unknown labels refuse. Existing no-variant subject and stock colour calls are unchanged. `runComparison(options,services)` re-checks the same combination rule at `executeComparison` entry so programmatic bypass cannot mix modes, then threads one immutable variant object through `launchSide`, `pixelPrecheck`, `measureAll`, each fresh trial, the retained warm window actor, and each memory/CPU session. No rebind operation, a label change produces a distinct build and context. Every new page context installs after the actual metadata wrapper and before enable/input/loads/prefetch.

No decodeBytes API, no stage bag. Public imports are existing packages only.

## Binding and lifecycle

Installation order in `bench.mjs` `ready()` is fixed. Reference override installs exactly after the existing priority 0 VOI-normalizing metadata wrapper and before `RenderingEngine` construction, `enableElement`, input enabled and any load or prefetch. The stock branch makes no override and the stock default loader is unchanged. Default cache stays 3 GiB, default worker count unchanged, stock app/volume assets unchanged. Pending, cache, inflight and eviction are owned by Core before the promise resolves. No custom strong image cache.

Per-load lifecycle:

1. Delegate request. The wrapper calls actual public `loadImageFromNaturalizedMetadata(imageId, mergedOptions)` and preserves the returned loadObject by spread, wrapping only `promise`. Delegate-request-false is recorded separately.
2. Claim happens AFTER the delegate resolves the actual image object, adding it to the module WeakSet. Claimed failure stays claimed, a retry requires a new object and a new claim.
3. Validation sequence on the claimed object, in order: validate domain and alias, scan finite before, call public `core.utilities.scaleArray` once in place, scan finite after, call public `getMinMax` for the four internal stats, then set effective enabled and scaled flags on BOTH `image.imageFrame` and `image` (the actual field is `image.imageFrame`, not `image.frame`). Original slope, intercept, metadata and explicit file VOI are unchanged. Effective flags and request-false provenance are separate facts.
4. Required alias: `image.imageFrame.pixelData`, `image.getPixelData()` and `image.voxelManager.getScalarData()` are one Float32 alias of the same buffer.
5. Success is recorded in the module WeakMap keyed by the actual yielded decoded image object, after successful validation, scaling, and flags, and before the wrapped promise resolves. Later observation reads the actual current object via `state.viewport.getCornerstoneImage()` and then looks up that exact current object. A module WeakSet claim is not success. Prefetch B completing cannot mark displayed A successful. A missing, wrong, or unvalidated reference object refuses before timing. Failure leaves the claim but no success record. No commit is inferred from promise resolution.

Per-context installed route, per-load pending, per-object claim and validated promise resolution, and actual renderer commit are four distinct facts.

Successful facts bind the CURRENT DISPLAYED image, never the latest completed or prefetched image. The existing `bench.mjs:200` uses `state.viewport.getCornerstoneImage()?.imageId`, and the actual public `getCornerstoneImage()` returns the image. Plan metadata outside clocks may call it directly, then perform an exact object WeakMap success lookup by that image. If the reference is missing, wrong or an unvalidated object, the route refuses before timings and never emits a successful route claim. Installed label and claim membership alone are insufficient. The prefetch out of order test must prove current image A does not inherit later image B facts. Synthetic tests may inspect actual scalars and GL upload. Production timed metadata cannot. Real route storage state is `'unobserved'` unless actual safe observation proves an enum value, never inferred from Float32 scalar type. `bench.mjs` `metadata()` keeps core_version, renderer, cache_bytes, workers, hardware_concurrency and no frame values. `__bench.snapshot` reports generation, index, commitSequence, blank. `__bench.capture` resizes and renders, so it is used only for pixel precheck and synthetic fixtures. Every timed actor reads nonrendering facts outside clocks. No capture inside clocks. Node owns actual HEAD/index/working identity, source and provider and package-lock hashes and versions, native binary hashes, built asset bindings, the external manifest and input content identity. Browser owns installed variant and route facts, real object dtype and shape, applied flags. No patient paths, UIDs, window values, min or max, samples, raw errors or uncontrolled strings in provenance.

## C. Exact closed final schema (single authority `tools/bench/src/comparison.mjs`)

Existing validated metric, machine, browser, and version fields stay existing and unchanged. No duplication of their schema. `publishedRecord` already validates `run.input_identity` as a 64 lower-hex SHA256 string at lines 1280 to 1282 and copies it unchanged. Any new provenance copy must equal that existing `input_identity`. No new input-content algorithm is invented. The current `comparison_run.mjs` already exports `publishComparison(result, destination)` and remains the writer/current publisher. Writer and current publisher call strict new-format validation. Explicit read-only legacy renderer path exists in `renderReport(run)` and `renderSummary(published)` but does not permit old records to be newly published and does not let missing new fields silently legacy.

New-format records for the new scalar comparison path open with `format: 'f273-comparison-v1'`. This format and its provenance apply ONLY to the new scalar comparison path including `stock`, never to colour records, ordinary subject records, or the existing colour FORMAT `ocelli-controlled-colour-comparison-1`, which remains unchanged. Provenance is mandatory for both `stock` and `reference` variants in this path. Node hashes provenance. Separately, browser-produced actual current facts are joined and validated by Node. No values are placed on the image except the needed internal stats and flags. No pixel, statistic, slope, or window samples appear in real provenance. Only Node hashes stay off the image. Browser facts are not claimed as Node-owned.

### Node-owned provenance (inside `run_provenance`)

| Key | Type | Null | Refusal rule |
|---|---|---|---|
| `variant` | string enum `stock \| reference` | never | unknown value refuses |
| `source_head` | string, 40 lower-hex Git | never | not 40 lower hex refuses |
| `index_tree` | string, 40 lower-hex Git | never | not 40 lower hex refuses |
| `working_sha256` | string, 64 lower-hex SHA256 aggregate of tracked working/source tree | never | not 64 lower hex refuses |
| `provider_source_sha256` | string, 64 lower-hex SHA256 | never | mismatch with deployed provider refuses |
| `package_lock_sha256` | string, 64 lower-hex SHA256 of `package-lock.json` with exact pinned versions | never | not 64 lower hex refuses |
| `viewer_binary_sha256` | string, 64 lower-hex SHA256 of native viewer binary | never | not 64 lower hex refuses |
| `comparator_binary_sha256` | string, 64 lower-hex SHA256 of native comparator binary | never | not 64 lower hex refuses |
| `assets` | array of `{ name, sha256 }`, name from the closed relative set derived from actual build output inventory, sha256 is 64 lower-hex | never empty | name outside set, or 40-hex asset bytes, refuses |
| `manifest_sha256` | string, 64 lower-hex SHA256 of completed `dist/variant-manifest.json` bytes, stored outside the manifest | never | not 64 lower hex refuses |
| `input_identity` | string, 64 lower-hex SHA256, equal to existing `run.input_identity` | never | inequality with `input_identity` refuses |

The exact manifest content contains `variant`, `route_version`, and asset bindings only, with no own digest. There is no inline page hash circularity and no shared manifest.

Two structures exactly. No rows, columns, or size appear on installed-route facts, which precede any image existing. Node `run_provenance` remains Node-owned. Browser facts are browser-produced, then Node-validated and Node-joined. They are not entirely Node-generated.

Installed route facts:

| Field | Type / rule | Reference | Stock |
|---|---|---|---|
| route_version | closed enum `'stock-5.8.2' \| 'f273-reference-v1'` | `'f273-reference-v1'` | `'stock-5.8.2'` |
| installed | bool | `true` | `false` |
| requested_target | `'Float32Array'` reference, `null` stock | `'Float32Array'` | `null` |
| requested_pre_scale | `false` reference, `null` stock | `false` | `null` |

Current-object facts, produced only after a real committed first image:

| Field | Type / rule | Reference | Stock |
|---|---|---|---|
| dtype | closed enum `'Uint8Array' \| 'Int8Array' \| 'Uint8ClampedArray' \| 'Int16Array' \| 'Uint16Array' \| 'Int32Array' \| 'Uint32Array' \| 'Float32Array' \| 'Float64Array'` | `'Float32Array'` | actual type |
| rows | positive integer | actual | actual |
| columns | positive integer | actual | actual |
| byte_size | observed numeric pixel VIEW.byteLength, positive integer, NEVER backingbuffer.byteLength | observes actual F32 view.byteLength and validates 4 * rows * columns | observes actual view.byteLength and validates view.length * BYTES_PER_ELEMENT with view.length = rows * columns * components |
| components | positive integer | exactly 1 | actual public image/voxel numberOfComponents in (1, 3, 4) with shape consistency, field PRESENT in both variants |
| applied_count | integer, 1 reference, 0 stock | `1` | `0` |
| success | bool, `true` reference, `false` stock | `true` | `false` |
| image_flags | `{enabled: boolean\|null, scaled: boolean\|null}` | both `true` | `null` where corresponding preScale record/field absent |
| frame_flags | `{enabled: boolean\|null, scaled: boolean\|null}` | both `true` | `null` where corresponding preScale record/field absent |
| storage | literal `'unobserved'` on a real run | `'unobserved'` | `'unobserved'` |

Stock image and frame flag values are the actual observed values, `null` when the corresponding preScale record or field is absent. Never fabricate a flag. Current lookup reads `state.viewport.getCornerstoneImage()`. The reference path then performs the exact yielded-object WeakMap success lookup. The stock path gets actual current image flags, type, and shape without importing the helper. Prefetch stage B cannot certify displayed stage A.

Phase rule: `ready` has no committed image. At `ready`, validate installed route and build metadata, and current must be `null`. After a real committed first image, current-object facts are mandatory outside clocks for frame-bearing actors, alongside retained, window, and memory checks. No dummy decode is inserted into `page_ready` timing, and a successful object is never inferred from `ready`. Node validates phase `('ready' | 'committed')` plus exact current nullability. Missing, wrong, or unvalidated current in the committed phase refuses. Phase binds to the existing actual snapshot commit and index, not caller assertion alone. Node precheck validates the actual route before `measureAll`. The same immutable build and context install before every load. All metadata is gathered outside clocks and never captured inside timings. Core cache ownership is unchanged.

Node schema: the existing `input_identity` 64-hex field is reused exactly. Package versions remain the existing closed `run.versions` keys `chromium`, `playwright`, `@cornerstonejs/core`, `@cornerstonejs/dicom-image-loader`. Provider and package-lock hashes bind the actual pins. No new version bag and no new output API. Manifest asset bytes are each a 64-hex SHA256, excluding the manifest's own digest, hashed by Node outside the file. The new writer plus publisher are a declared strict format with explicit read-only legacy support, not an automatic fallback. The Node provenance table is not repeated here. This corrects only the browser-table gap.

**Observation placement and record join.** PLANNED attachments, not existing exports. Scalar state moves out of page metadata to a new getter `__bench.scalarReferenceState()` returning exactly the planned `{phase, installed_route, current}` contract. `__bench.metadata()` keeps exactly its existing shared shape (`core_version`, `renderer`, `cache_bytes`, `workers`, `hardware_concurrency`) with no `reference_state` field. A separate private Node helper `readScalarReferenceState(actor)` calls this getter ONLY on the cornerstone actor in scalar comparison, using the actual `actor.context` field, where an explicitly defined optional `context.scalarReferenceVariant` (`stock|reference`) is set ONLY for non-colour comparison BEFORE its precheck and is absent in colour and subject mode. Shared helpers do NOT auto-invoke the scalar observer on default stock. The scalar precheck and measureAll caller conditionally invokes the accessor at the placements below, including before precheck actor close. The colour branch, colour precheck, colour trials, colour windows, and colour memory never invoke it, and native actors never invoke it, so the new getter is never called there. After existing openFolder returns, both the original open and first_image endpoints are already captured, so gather committed current facts only then, with no new wait, load, or inspection inserted between those endpoints. After window and scroll endpoint completion outside existing clocks, read actual current facts and match the actual committed snapshot. For memory and CPU, observe facts only after both sampler.stop and final cpuSnapshot, before actor.close, outside entire collection windows. Ready has actual installed and build facts and current null, with no dummy decode or added load inside page_ready. The binding phase ready|committed plus the actual snapshot commit index prevents any null certifying framework. Real precheck remains mandatory before measureAll. No measurement, endpoints, or actor work changes. Serialization for the new scalar comparison path only: the getter returns the closed `{phase, installed_route, current}` object whose substructures are exactly the already authored browser facts tables. Raw per-actor route observations use one proposed run.route_observations array whose closed row keys bind the existing side, metric, workload, condition, and trial_index together with that reference_state. The side, metric, workload, and condition enums reuse the existing comparison.mjs constants and the declared trial index. No patient identifiers. Node validates complete actor membership using the actual unchanged trial and session inventory, and the current publisher copies only validated value-free facts via the same comparison.mjs authority. Observation is not a new metric or sample, and adds no extra CPU or memory session and no source or API framework. Existing public renderer and publication function signatures remain unchanged except the already planned strict format branch. No second Python validator and no global image cache. If a sentence would force a new actor wait or measurement change, refuse it rather than inventing it.
## Exact write and registered test map

Writers, in full:

- `tools/oracle/page/reference_loader.mjs` (PLANNED): new helper as specified in Scope.
- `tools/oracle/build-page.mjs`: variant option, bench entry selection, external manifest write after asset hashing.
- `tools/oracle/page/bench.mjs`: build-selected bench constant read, reference install at the fixed order point.
- `tools/bench/run.mjs`: parseArgs closed variant option, omitted stock.
- `tools/bench/src/comparison_run.mjs`: immutable variant threading through precheck, fresh trials, warm window actor, memory and CPU sessions, stale or wrong-registration refusal before timing based on real object ownership facts, never label alone.
- `tools/bench/src/comparison.mjs`: format discriminator, required provenance, legacy read-only path, publisher enforcement.

All files are EXISTING. Both `tools/bench/package.json` scripts.test and `bin/ocelli.sh bench arm` already register all six files. No new file or registration.

- `tools/bench/tests/viewer_work_test.mjs` (synthetic fixture/schema evidence only, no GPU, cache or pixel proof): generated fixture byte controls, decoded hand-vector expectations, preserved mono16Part10 default bytes.
- `tools/bench/tests/comparison_test.mjs` (one closed record, legacy-read and new-publish): schema, discriminators, stock record shape, refusal record shape.
- `tools/bench/tests/comparison_run_test.mjs` (controlled orchestration, not a GPU proof): selected variant to precheck/fresh/warm/memory actors, refusal persistence and no timing continuation, manifest and wrong-route failure, identity and provenance records, AC9, AC12, AC13, AC15, AC16, AC21, AC24 gates.
- `tools/bench/tests/comparison_page_test.mjs` (real built-page browser executor, `OCELLI_COMPARISON_BROWSER=1 node --test tools/bench/tests/comparison_page_test.mjs`), named cases:
  - BR-DS: signed16-extrema, signed16-interior, signed12, unsigned16, 8bit-refusal. Covers AC1, AC2 and the 8-bit refusal policy.
  - BR-TX: mono1-invert, tx-positive, tx-negative, tx-fractional, tx-nonbinary, tx-identity, tx-default, tx-zero-refusal, tx-partial-refusal, tx-nonfinite-refusal. Covers AC3, AC4, AC14.
  - BR-RAW: raw-modality-lut, raw-voi-lut, fg-shared-refusal, fg-perframe-refusal, both-lut-rescale-before-affine, classicmf-top-transform. Covers AC6, AC7, AC4 refusal rules on loader.
  - BR-REF: pt-refusal, dose-refusal, color-refusal, windowless-refusal, non-linear-refusal. Covers AC5, AC8.
  - BR-CTX: options-targetBuffer, options-preScale, loadObject-fields-preserved, wrapped-promise, weakmap-not-weakset, scale-exactly-once, image-and-frame-flags, original-mbvoi, request-false. Covers AC10, AC17.
  - BR-MUT: mutant-missing-RED, mutant-double-RED, restored-GREEN. Covers AC22.
  - BR-ROUTE: display-A-prefetch-B, cornerstone-witness, frame-transition-no-reopen, window-pixels, resize-pixels, cache-hit, cache-inflight, cache-eviction, cache-new-object, failed-object-claim, new-object-retry, last-valid-A-during-B-refusal. Covers AC11, AC18, AC19, AC20.
  - BR-STOCK: stock-no-reference-registration, stock-image-facts, stock-pixels-app-volume-unchanged. Covers AC23.
  - All capture occurs outside any timed clocks.


## Caller compatibility and scoped observation write map

Add to the existing comparison_run write map, scoped to the new scalar comparison path only, after colour outputs which remain written as today under their existing format:

- Scalar comparison path only: `comparison.json` and `comparison.txt` gain the strict `f273-comparison-v1` branch with run.route_observations and Node-hashed provenance. Colour outputs (`colour-comparison.json`/`.md`/`.sha256`) and subject outputs keep their existing formats and are untouched.

- `tools/bench/tests/run_test.mjs` `comparison --variant without colour-case accepted`, `subject --variant rejected`, `colour-case --variant rejected including explicit stock`, `unknown variant label rejected`, all asserting fixed early error before any build or fs work. Extend existing 321-335 combination coverage only.
- `tools/bench/tests/comparison_run_test.mjs` `executeComparison rejects mixed programmatic variant combinations`, `scalar path reads scalarReferenceState only on cornerstone actor`, `colour path never invokes scalar accessor`.

- run_test.mjs owns parseArgs combination tests: variant onlyscalarcompare, any explicit variant rejecting subject/colour before build/input, unknown rejection, and omitted existing calls unchanged. Existing registrations for all six files are retained.
- comparison_run_test.mjs owns executeComparison programmatic mixed-mode early reject, and the common actual orchestration over launchSide, openFolder, window, and memory with only scalar Cornerstone getter calls, no scalarCallNative and no colour. No 18-trial fakeTools schema assertion lives here.
- comparison_colour_test.mjs already has fakeTools, measureColourComparison, and its fixed 18 trials (9 cold, 9 warm, 9 windows, 3 memory) per side. It is extended with the exact named controlled case 'stock colour shared helpers never request scalar state', a scalar-accessor tripwire, and unchanged colour FORMAT ocelli-controlled-colour-comparison-1 schema, counts, and cleanup. This is controlled orchestration, not GPU proof.
- comparison_page_test.mjs owns actual small stock RGB built-page preservation under OCELLI_COMPARISON_BROWSER=1: original __bench.metadata with exactly five keys (core_version, renderer, cache_bytes, workers, hardware_concurrency), unchanged pixels, window, and retained behavior, and a scalar-accessor tripwire that is not called. It reuses existing writeColourFrameFixtures(dir, 2) with no source values and no patient data.
- Node comparison_test is shape only. viewer_work_test is fixture-writer control only.

Named tests above are the exact test names, not vague unchanged registrations.

### Colour calibration refusal remains separate

Current executeComparison runs shared pixelPrecheck before the colour input branch. If the pixel predicate fails or information is low, the flow sets record.state to 'refused_pixels', calls safeOutputs(record, denySet), then renderReport(record), writes the ignored comparison.json and comparison.txt, and returns the refusal. The success colour branch later uses safeColourOutputs and result.record, writing colour-comparison.json, colour-comparison.md, and colour-comparison.sha256. Colour compatibility therefore includes this earlier CT calibration refusal reporting, not just the final colour record.

Writer policy: the scalar writer enforces the new format and provenance in scalar mode before generic safeOutputs, with no silent legacy downgrade. An active colour calibration refusal retains its existing refusal envelope, schema, deny checks, and cleanup, with no F273 scalar facts, format, or new validator injected. Shared safeOutputs and renderReport cannot unconditionally enforce the F273 schema on colour callers. This is a caller-specific current colour contract, not permission to downgrade a new scalar record. Existing scalar publication stays strict, and colour publish already refuses.

Add the exact controlled comparison_run_test case 'colour calibration refusal persists existing report without scalar observer': a failing calibration predicate, a tripwire on the scalar getter and validator, no colour measure or timing continuation, an actual retained comparison.json and comparison.txt refusal, and owned cleanup. No patient inputs and no runtime here.


## Generic stock observation and mixed-folder preservation

- Stock current `byte_size` comes from the ACTUAL `image.getPixelData()` numeric VIEW `.byteLength`, never `view.buffer.byteLength` and never scalar `rows * columns`. Dtype comes from the actual supported numeric typed array kind. Components come from the actual public image and voxel contract with a consistency check.
- For stock checks: actual `view.length == rows * columns * components` and actual `view.byteLength == view.length * typedArray.BYTES_PER_ELEMENT`. Rows, columns, components must be positive integers. Components must be 1, 3, or 4 per the actual contract. The observed components must agree with `image.numberOfComponents` and with the available public `voxelManager.numberOfComponents` and data shape.
- The stock check must NOT require a reference WeakMap, F32 shared alias, or scaled flags. Root requests no invented stock refusal for valid existing numeric kinds.
- This is a metadata observation only, not new stock decode policy. Finite pixel full scan and scaling remain reference-only as before. An invalid or inconsistent actual object fails proof, with no silent fallback.
- Stock actual flags remain nullable as before, `applied_count` is 0, `success` false means no F273 application, and `route_installed` is false. The new common `components` field may be the same closed current-facts structure: reference exactly 1, stock actual 1, 3, or 4. The `rows * columns` scalar formula remains reference-only. Reference `byte_size` observes the actual F32 view and validates `4 * rows * columns` with no weakening.

Public getter operations observed by the regression:

1. `image.getPixelData()` returns the actual numeric view, so dtype is the concrete typed array constructor name and `byte_size` is the actual view `.byteLength`, never the backing buffer `.byteLength`.
2. `image.numberOfComponents` and public `voxelManager.numberOfComponents` are both read, and the actual view `.length` must equal `rows * columns * components` with components in 1, 3, 4.
3. `voxelManager.getScalarData()` is used only to confirm shape consistency with the same actual view, never to synthesize a normalized copy.

Test map, all named in `comparison_page_test` as `'stock scalar comparison preserves RGB-first mixed pseudo-full'`:

- Wholly synthetic folder with a stock RGB frame first, then a qualifying mono CT calibration frame, under a normal standard comparison context with the variant omitted, meaning no `--colour-case`.
- Real public loader and CoreGPU paths only. No clinical read and no actual run now.
- Old metadata unchanged in shape. New getter captures actual dtype, actual view `.byteLength`, and components.
- Full RGBA gold is preserved, along with retained and window behavior.
- The reference variant on the same unsupported RGB selection must refuse before timing and block AC24, with no substitution or fallback.
- Existing `writeColourFrameFixtures(dir, 2)` and mono 16-bit Part 10 fixture providers are preserved, and the stock default is preserved.
- The Node `comparison_test` closed shape permits all nine enum kinds and 3-component stock, and reference components other than 1 refuse.
- A tiny synthetic view and subview control verifies that actual `view.byteLength` is not larger than the backing buffer. This is a control only, NOT GPU proof.
- Existing registrations are unchanged.

Guard process: `scripts/guards/catalogue.py` existing `bench.comparison` and `bench.comparison-runtime` probes plus refusal sites in the new helper, budget recorded in `ci/guard-probe-budget.json`, regenerated `docs/runbooks/guard-verification.md` with only measured delta and existing gates, no count reset. Shared guard provider edits are serialized with other owners.

## LLD and user documentation

LLD updates exactly: `docs/lld/benchmarks.md` (variant threading, schema authority, refusal rules), `docs/lld/oracle.md` (buildPage variant, manifest write, install order), `docs/lld/README.md` index, contribution and Last updated. User documentation: `README.md` gains exact compare usage including the variant option, the omitted-stock default, refusal behavior on stale or wrong-registration builds, and the operator publication boundary (operators publish from the external manifest and published record, never from captured geometry). No use.html or more.html feature surface is required by this instrument story. Unchanged: `tools/bench/comparison.json` N, endpoints, selections, bands, native rendering and arithmetic, and the default stock route.

## Acceptance inventory

Browser case groups (all in the actual executor `tools/bench/tests/comparison_page_test.mjs`, gated `OCELLI_COMPARISON_BROWSER=1`):

- **BR-DS**: signed16 extrema/interiors, signed12, unsigned16, 8-bit-allocation refusal. Generated synthetic bytes, actual public naturalized delegate plus public `scaleArray`, real Core GPU render, full captured RGBA versus independent DICOM formula and hand gold.
- **BR-TX**: MONO1 inversion, positive/negative/fractional/nonbinary/identity/default transforms, zero/partial/nonfinite refusal. Same delegate and render path, independent gold per case.
- **BR-RAW**: raw present-empty Modality LUT, raw present-empty VOI LUT, shared and per-frame FG refusal, both-LUT-and-rescale refusing BEFORE affine mutation with independently different transform outcomes, classic multiframe top-level transform with real frame index transition.
- **BR-REF**: PT, dose, color, windowless, non-LINEAR, 8-bit refusals.
- **BR-CTX**: targetBuffer/preScale options, preserved loadObject fields, wrapped promise, WeakSet claim versus successful WeakMap fact, exactly once public scaling, both image/image.imageFrame flags, original m/b/VOI and separate request-false.
- **BR-MUT**: missing and double public utility semantic mutants through the exact delegate/render path failing the independent gold predicate, restored route passing.
- **BR-ROUTE**: displayed A versus prefetched B, current `getCornerstoneImage()` witness, retained same-viewport frame/index transitions with no reopen for second image, window and resize correct pixels, cache hit/inflight/eviction/new object owned by Core, failed-object claim and new-object retry, AC11 last-valid-frame observation on A during B refusal through the existing selected route.
- **BR-STOCK**: no reference registration, actual stock image facts, unchanged stock pixels/app/volume assets.

| AC | Required behavior | Actual evidence or permanent test | Wrong implementation | Stage and owner |
|---|---|---|---|---|
| AC1 | Signed16 extrema and interiors on real GPU pixels | BR-DS signed16 cases, full RGBA vs independent gold | Clamped or wrapped extremes | Synthetic browser, bench owner |
| AC2 | Signed12 and unsigned16 pixels | BR-DS signed12, unsigned16 | Bit-depth mishandling | Synthetic browser, bench owner |
| AC3 | MONO1 GPU inversion | BR-TX mono1 case | CPU-side inversion only | Synthetic browser, bench owner |
| AC4 | Positive/negative/fractional/nonbinary/identity/default rules, zero/partial/nonfinite refusal | BR-TX full set, BR-RAW raw-LUT cases | Silent fallback on refusal | Synthetic browser, bench owner |
| AC5 | LINEAR supported, LINEAR_EXACT/SIGMOID refusal | BR-REF, comparison_run_test.mjs refusal actors | Accepting unsupported functions | Both, bench owner |
| AC6 | ClassicMF top-level transform, FG refuses | BR-RAW classicmf and fg cases | FG silently applied or ignored | Synthetic browser, bench owner |
| AC7 | Raw Modality/VOI LUT precedence, both-LUT-and-rescale refuses BEFORE affine | BR-RAW raw-lut, both-lut-rescale | Affine applied despite LUT | Synthetic browser, bench owner |
| AC8 | PT/dose/color refusal | BR-REF | Unproved fallback | Synthetic browser, bench owner |
| AC9 | Required DX/pseudo-full route must actually pass the admitted route and full unchanged protocol. Refusal aborts and blocks AC24, not success | both actual equipped evidence and controlled comparison_run_test.mjs orchestration | Substitution on refusal, refusal treated as success | Equipped, bench owner |
| AC10 | Immutable build/context, no invented setter | BR-CTX | Wrong loader registration | Synthetic browser, bench owner |
| AC11 | Last valid A visible while unsupported B refuses on same route, abort before timings | BR-ROUTE last-valid-A observation of existing consumer | New recovery API invented | Synthetic browser, bench owner |
| AC12 | Metadata/snapshot outside clocks, capture-only precheck | comparison_run_test.mjs timing scopes | Metadata inside clocks | Equipped, bench owner |
| AC13 | Value-free metadata, real hashes, actual browser object facts | comparison_run_test.mjs identity record | Invented hashes | Both, bench owner |
| AC14 | Synthetic-only samples and stats | BR-TX instrumentation scope | Value provenance leaks | Synthetic browser, bench owner |
| AC15 | Strict new-record provenance, legacy read-only | comparison_run_test.mjs provenance gates | Silent downgrade | Both, bench owner |
| AC16 | Ephemeral precheck cleanup and deny scan | comparison_run_test.mjs precheck actor | Persistent scan residue | Equipped, bench owner |
| AC17 | WeakSet claim after yield, scale once, finite scans, 4 stats, both flags, original m/b/VOI, request false, successful WeakMap facts written only after all validation | BR-CTX, BR-MUT | Rescale mutation, early claim, premature displayed commit | Synthetic browser, bench owner |
| AC18 | Retained frame transitions, window and resize correct pixels, no capture in clocks | BR-ROUTE window/resize cases | Geometry on window change | Synthetic browser, bench owner |
| AC19 | Cache hit/inflight/eviction/new object owned by Core | BR-ROUTE cache cases | Custom cache layer | Synthetic browser, bench owner |
| AC20 | Failed object stays claimed, new-object retry | BR-ROUTE retry cases | Hidden retry | Synthetic browser, bench owner |
| AC21 | Hardware identity, synthetic GL evidence, CPU reference unsupported, no Tier B conflation | comparison_run_test.mjs identity record | Conflated tier claims | Both, bench owner |
| AC22 | Missing/double mutants RED, restored GREEN, exits 0/1/1/0 not acceptance | BR-MUT | Treating procedure as proof | Synthetic browser, bench owner |
| AC23 | Stock default, app, volume, pixels preserved, native arithmetic, N/endpoints, publication hold | BR-STOCK, comparison_test.mjs stock record, comparison_page_test.mjs stock RGB built page | Source or pixel mutation, colour-path scalar call, native-actor scalar call | Both, bench owner |
| AC24 | Equipped 5 workloads, 7 metrics, selected-reference, complete precheck before timings, refusal not accepted | comparison_run_test.mjs plus actual protocol | Timing after refusal | Equipped, bench owner |
| AC25 | Operator-held publication, staged verification, no F223 AC18/19 closure | Delivery checklist | Self-closure | Delivery, operator |

## Benchmark browser-log privacy boundary

### Scope

This policy governs design only, for the private benchmark harness. It covers console calls made in the main realm of owned benchmark pages in `comparison_run.mjs` flows, replacing named value-emitting page console entrypoints before any loaded module, delegate, input, or prefetch runs. It claims protection only for source-bound known value calls through the listed methods. Worker contexts, Node processes, and native processes are not covered, and no universal all-realm privacy claim is made. All clinical runs remain held, with no PHI exposure claimed.

### Installed operation

A single private benchmark init function is installed via the existing init point, `browser.newPage -> addInitScript(observer...) -> actor.navigate -> page.goto`, by replacing the page context during `actor.newPage` in `comparison_run.mjs`, before `goto` in every fresh or recreated owned benchmark page, main realm only. The installed script runs before any page module or delegate executes, and it replaces the following exact set of page console callables with argument-discarding no-ops:

`log`, `info`, `debug`, `warn`, `error`, `trace`, `dir`, `dirxml`, `table`, `assert`, `group`, `groupCollapsed`.

Each replacement callable discards its arguments unconditionally. Replaced calls must not invoke the native original, must not emit any event, and must not inspect, stringify, serialize, or retain any argument. No log buffering and no whitelist or marker matching is permitted, including for trusted-looking strings. Console calls reaching these entrypoints do not reach the browser emission sink. No raw console argument capture occurs in real contexts. The `page.on` handler is not modified and is not a protection mechanism. There is no public logger API and no restore-to-native while a page may still process inputs. Page or context close owns disposal of the replaced callables, with no retained state or payload, and the installation is repeated on each new `actor.newPage`.

### Protected and unprotected diagnostics

The authoritative new errors must remain fixed and closed through Node's existing safe stage boundary. This does not certify every existing DOM string or every bench exception as static. These are not console passthrough. No semantic console GREEN or RED production channel is created. The native URL handshake, stdio, and process sampler are unchanged. There is no bundle drop-console, no `node_modules` patch, and no change to assets, pixels, decode, arithmetic, N, or bands. Console facilities outside the listed set carry no privacy guarantee.

### Known source binding

`createImage.js` at SHA `98bf77e77479250119319022d28ffb8156b56f8112012a7542bb20900ce20607` contains a `PALETTE COLOR` `console.log` passing `imageId`, a `pixelData` slice, and the first ten LUT entries before the delegate yields. This known call is protected by the installed no-op set while stock palette display is preserved. F273 post-delegate palette admission refusal is too late and stays unsupported before timings, so it cannot pass AC24. Protection extends to stock, reference, scalar, colour calibration, mixed palette, and retained pages, and it takes effect from the first navigation. Synthetic-only implementation follows root review, and real input runs stay held until protection is implemented and actual sensitivity controls are verified, with no new root input authorization request.

### Test separation

Tests use exact names. `comparison_run_test` proves controlled orchestration only: init registration before first `goto`, `actor.newPage` recreation, native-side, and colour-owned page coverage, with no browser proof. `comparison_page_test` runs the real built page with `OCELLI_COMPARISON_BROWSER=1`, using a wholly synthetic palette under Part 10 with the public stock loader. It checks the actual known baseline console call as semantic RED, protection as semantic GREEN with no value-bearing event and no retained payload, a very early inline call, new, recreated, and retained image failure, fixed DOM and stage error availability, and unchanged stock full RGBA and asset bytes. Test markers, pixels, and LUT values use only own synthetic data, with no production canary filter, and production records carry only closed status and counts. The baseline and mutant remove the actual no-op installation and must restore the known raw console call, not merely a source pattern. Existing registrations on both lists are unchanged.
### Installation failure holds input

Add a policy paragraph to leaf F273: input holding on failed installation verification. Init registration must succeed before any navigation. In each new page document the init function may establish one benchmark-private closed readiness getter that reports success only when every named console method is in fact the installed argument-discarding no-op. No marker derived from intent or unconditional true is valid. Verification reads only owned function identities and installation state on the protected surface and never inspects foreign console arguments or values. No public logger or pass-through is created. If the marker is missing or false, if any named method was not replaced or is nonreplaceable, if the script fails, or if verification fails, the owned actor aborts before any file input or delegate invocation. The abort emits one fixed static stage error through existing non-console channels and performs normal owned cleanup. No retry, no fallback, no capture of raw foreign exceptions. actor.newPage registers init before goto, and actor.navigate captures the existing page_ready endpoint. Verify effective readiness immediately after that original endpoint is recorded and before openFolder, setInputFiles, or any real input. Do not insert a dummy image, a wait between the original open and first-image endpoints, a new timing endpoint, or a changed N. Keep setup work inside the memory and CPU windows that already cover startup, with no collector-boundary movement and no zero-overhead claim. The guard applies fresh, recreated, and retained pages before each new folder input. No restore to native while inputs or prefetch can run. Source-bound known modules do not reassign protected methods, and no malicious-script guarantee is offered. Negative cases: a nonreplaceable console.log and a missing or false readiness witness must both refuse before setInputFiles or any delegate call, with zero input or decode witnesses and actual cleanup, not timeout alone. Existing palette missing-install semantic RED, effective-protected GREEN, and stock pixels remain owed. No clinical inputs. Realm limitations hold, and all real input runs stay held pending reviewed implementation plus actual controls.

## Implementation, verification and delivery

F216 is done only as structural dependency. F223 is carried forward, not a dependency. S47 estimate E38.64 / 3 weeks is provisional, not measured and not capacity approved. Meaningful tests precede implementation, with small source-controlled route, fixture and schema actors, own microscope and root review, then the actual equipped protocol. The canonical operator GroundTruth remains unchanged and withheld from all model packets. The already authorized private input root is used via env OCELLI_PSEUDO_DIR, no permission request and no path printed. Publication remains operator held, with no push, merge, tag, or closure authority.

Acquisition contract, commands to perform only after reviewed implementation and release of the real-input hold, not during isolated synthetic implementation:

```
PATH=/Users/atulsharma/.local/share/fnm/node-versions/v24.13.0/installation/bin:$PATH
export PATH
out=$(mktemp -d /private/tmp/ocelli-f273-equipped.XXXXXX)
bin/ocelli.sh bench --compare-cornerstone --pseudo "$OCELLI_PSEUDO_DIR" --variant reference --out "$out"
```

Handle the actual command exit. A closed precheck refusal JSON is persisted, and no timing continues after failure. Own server, browser, and temp cleanup is performed. Record actual host resource conditions and source before and after, plus Node, browser, pkg lock, provider, native binary, built assets, and manifest identity. No fully idle claim. The same route precheck runs on every chosen series before measureAll. Synthetic independent gold and the real pair predicate are unchanged. No clinical data reaches GLM. Protocol authority is tools/bench/comparison.json unchanged, not a copied invented definition: sides ocelli and cornerstone, workloads synthetic CT 512² stored 50, synthetic DX 3000x2900 stored 50, pseudo-full ALL files, largest qualifying 512² MONO2 windowed CT, and largest DX, with the current digest tie-break unchanged. Seven metrics: page_ready 9 cold only, open 9 cold warm (synthetic CT, pseudo-full), first_image 9 cold warm (CT/DX synthetic plus pseudo), scroll 5 warm (CT synthetic plus pseudo) 3s or 150 keys then 300ms settle requiring at least 120 keys and no blanks, window_change 9 warm four CT/DX series centres 40..48 width 400 LINEAR, peak_memory 3 cold session, session_cpu 3 cold session. Session covers pseudo-full open and first, CT scroll, and 9 CT and DX windows. Original clock endpoints, first commit plus rAF, per-workload start side carried cold warm then flipped next workload, memory 250ms minimum from actual timestamps sampled concurrently with lower and sum peak upper, CPU stable inventories, and current quartiles and spread are unchanged. No baseline accept, tolerance, N, or endpoint change. The exact existing runner defines clocks and statistics rather than a new implementation.

Focused pure tests: node --test tools/bench/tests/comparison_test.mjs tools/bench/tests/comparison_run_test.mjs tools/bench/tests/comparison_page_test.mjs tools/bench/tests/viewer_work_test.mjs. Actual browser gated integration: OCELLI_COMPARISON_BROWSER=1 node --test tools/bench/tests/comparison_page_test.mjs, a PLANNED extension using that file's actual browser gating pattern, not a currently present test claim. Stage before gate, invalidate old receipts, then run normal /verify --profile feature floor plus corpus on the exact final tree with a named generator after review, no sprint profile and no new gate. Read the actual workflow before prepare and commit.

Milestones: confirm route and fixtures against the immutable stock build, author pure tests first, implement the selected-reference route, pass synthetic gates, then run the equipped protocol after reviewed implementation and release of the real-input hold. Open decisions: none delegated to the model.

Open material risk: required actual selected inputs may fall outside the conservative domain. If so, this plan's full AC24 cannot pass, and work stops to reopen a reviewed design without changing selection or fallback. Reference hardware float precision, cache, and memory all need actual production route observations. The CPU reference is explicitly unsupported by policy. Windowless, PT, and FG support are not a secret future claim.
