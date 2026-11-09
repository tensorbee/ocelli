# F-273 Reference scalar normalization for equipped comparison

**Status**: draft
**Epic ref**: E38.64
**Sprint**: S47
**Estimate**: 3w provisional, not measured, not capacity-approved

**Declared dependency**: F-216 done provider only. No F-223 structural dependency in either direction. F-223 AC18 and AC19 stay open and F-223 is carried, not completed.

## Intent

F-273 proposes to implement and validate a reviewed scalar and upload route that preserves full-range signed distinctions, then use that same declared route for the actual pixel precheck and for the full unchanged equipped timing protocol. This is not design-only or refusal-doc-only. Affine-only correction is not adoptable, because adjacent signed minima still fail stock and diagnostic routes. No route is approved. The public Float32-style target is only a candidate requiring source, API, precision, rescale and lifecycle design, with no general 32-bit claim. A package patch alternative needs provenance and pins. Stock oracle default scope is unchanged unless separately approved. No native arithmetic change solely to match reference. No tolerance, baseline, N or endpoint waiver.

## Ground truth carried from F-223

Approved rule records pixel refusal as a recorded outcome, not passing AC18, with a file-divergence story, and AC18 and AC19 stay open. The actual operator-root refused-pixels verdict is max 1, signed mean bias informative 0.3279290752351097, informative fraction 0.233642578125, low information false, passes false, with no timings and no publication. The actual refused-pixels report is retained privately, not discarded. The earlier measured controlled 201 record stays separate and is not actual study acceptance.

Own retained synthetic evidence remains recorded, interior stock-restored bias 0.3191428998631308 with 83243 reference exact mismatches and zero native gold mismatches, private uniform-diagnostic interior bias 0.00004600644857054131 with 26216 reference exact mismatches and not adopted, adjacent signed minima -32768 and -32767 at C=-32767 and W2 requiring 0 and 255 where stock and uniform-corrected both refuse with 131072 reference mismatches, max 255, zero native gold mismatches, not low information. Native independent gold is unchanged. Historical prepared producer stays b889 tree 752a, not 1d execution.

## Source map

- tools/bench/src/comparison_run.mjs, the real precheck with fresh and retained timed actors and frame, window and memory sessions.
- tools/oracle/page/bench.mjs.
- Pinned public loader scalar-target and VTK upload and image-mapper providers for inspection only.

## Draft acceptance controls, all provisional pending route approval

1. Signed interior, signed minima -32768 and -32767 at C=-32767 and W2, and positive-negative discriminator pass with signed 12, unsigned 16, rescale and MONO1 against independent gold.
2. Exact scalar and upload identity verified for full RGBA and extent.
3. Retained actor plus frame and window sessions exercise the same declared route end to end.
4. Declared variant closed, unknown label refused and default stock preservation confirmed.
5. Fixture, binary, comparator, built worker and provider before and after bindings captured.
6. Only after a route is approved through normal design approval, the real full equipped protocol runs unchanged N, endpoints and thresholds, with the actual same-route precheck first and actual timings on that same route, no affine-only adoption, holding privacy and cleanup. The current refused-pixels report stays private with no publication now. A future approved protocol including F-223 AC19 gated publication follows the unchanged privacy, cleanup and publication rules and any actually needed authorization, with no blanket permanent ban and no implied permission.

Route choice is an open lead question. No synthetic pass closes F-223 AC18 or AC19. No code, runtime, canonical status or acceptance approval is claimed here.

## Verified references only

- DICOM PS3.3 C.11.2: https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.2.html
- OpenGL ES 3 equation 2.2: https://registry.khronos.org/OpenGL/specs/es/3.0/es_spec_3.0.withchanges.pdf

Plan remains draft.
