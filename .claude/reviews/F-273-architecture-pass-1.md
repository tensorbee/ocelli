# F-273 architecture review, pass 1

**Reviewed**: private GLM architecture leaf `8ea9b23fb370e6ac5452c214caafbb60f9aa211cf831b70c576c040c61029fec`, its own review and explicit controller qualifications, current canonical draft at c347be62652f5cbef6c9761b1b343b71e9ce86b3 and the installed pinned provider source.
**Author / reviewer**: GLM leaf and own microscope, independent Codex root review.
**Result**: 3 defects, 0 smells, 2 nonblocking nitpicks. No route or implementation is approved. A corrected, bounded private synthetic feasibility experiment is authorized below.

## D1, caller float-rendering flag is not the claimed control

In the installed dicom-image-loader createImage.js, createImage assigns options.allowFloatRendering from canRenderFloatTextures before decode. The caller value is overwritten. The leaf must not present setting that field as forcing a storage or upload policy. Actual capability and resulting image/texture state must be observed. This is a source-confirmed proposal defect, not a new runtime finding about the installed package.

## D2, namespace and registration alias are different exports

Root read wadouri/index.js, loadImage.js and register.js. The public namespace exports legacy loadImage and loadImageFromNaturalizedMetadata separately. The register module's named alias does not change that namespace identity. The selected existing naturalized provider must be delegated explicitly. A switch to the legacy loader would change dataset and cache behavior beyond the claimed scalar representation. The proposal's naturalized-delegate choice is useful, but its alias sentence is false.

## D3, later Float32 rewrap cannot restore discarded fractional values

Worker postProcessDecodedPixels admits the target through exact constructor equality against getPixelDataTypeFromMinMax on scaled extrema. This is not a numeric-capacity check. With default prescaling and stored [0,1,2], slope 0.5 and intercept 0, the extrema are integral 0 and 1. The selected constructor is Uint8Array, so the requested Float32 target is rejected there. The fallback integer array is then scaled in place. Main createImage subsequently constructing Float32 from that result cannot restore an earlier fractional interior. The leaf's unconditional worker-Float32 claim is false. Root inspected the actual worker, classifier and scaleArray. This sequence is source-derived and must still be exercised through the actual provider before any runtime bug attribution.

## Independent expectations and proposed experiment

Root checked DICOM PS3.3 2026d [C.11.1 rescale](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.html) and [C.11.2.1.2.1 LINEAR](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.2.html). Independent exact-rational expectations are preserved in /private/tmp/ocelli-s46-codex/f273-root-design-review/independent-gold.json. Stored [0,1,2], slope 1/2, intercept 0 give modality [0,1/2,1]. At center 5/4 and width 2 the display results are exactly [0,255/4,765/4], whose nearest-u8 project quantization is [0,64,191]. The signed-minimum pair -32768/-32767 at center -32767 and width 2 gives [0,255]. These expected vectors are not loader/GPU observations.

A private probe may compare stock, public Float32 target with default prescale, and the source-selected candidate using the actual public naturalized delegate with a Float32 target and prescale explicitly disabled. The last candidate changes prescale ownership and must be labeled as such, preserving the original slope/intercept for the actual renderer. It is not approved as a representation-only stock replacement. Omit the ineffective caller float flag. Observe actual stored/scaled arrays, preScale state, metadata and actual rendered output before accepting any route. Do not patch node_modules, normalize pixels after rendering, modify native arithmetic or change tolerances to obtain agreement.

Observe both WebGL upload type and sized internal format. Root read the actual VTK texture implementation: FLOAT is a data type, not proof of R32F storage, and half-float admission also depends on range, extensions and cached state. Include fresh and retained same-extent small-range to signed-extrema transitions. No all-Float32 or general 32-bit correctness claim follows from a Float32 JavaScript array. Reference CPU fallback must be identified separately from the intended GPU path.

All fixtures must be wholly synthetic and use the actual provider and render consumer, not a scalar-only simulation. Reuse the existing comparison capture/geometry/cleanup machinery where practical. The prototype may run only in private/disposable source, under a finite owned process deadline with exact cleanup and immutable original-source/package hashes. It supplies feasibility observations, not implementation, timings, F-223 acceptance or publication. No clinical inputs or observations enter GLM prompts. Keep stock contexts and the canonical comparison default unchanged.

## Nitpicks and remaining design

The FLOAT/internal-format wording needs precision. The duplicated classifications in the raw own report are not eleven independently established failures. Its raw 6D/2S/3N remain preserved alongside the controller qualification. A full normal design template, exact write set, immutable variant/cache identity, common fresh/retained/memory consumers, honest report schema and equipped-host acquisition remain required after feasibility. The current bounded leaf never claimed to be a complete plan, so its limited scope is not itself another defect.

The design remains draft. No claim/start, canonical implementation, corpus/clinical run, baseline or threshold change, merge or publication is authorized here.
