# F-269 design microscope, pass 1

Reviewer: Codex, independent of the GLM author. Design only.

Source SHA256: `81f6a6351ba4fa3ddfbe3d239021f1682907e8238f0484090ea1a5c05d16c046`, private branch work/f-269-glm at base `16c782d17f673b1af14fb09e906e3f29f16fdb02`. GLM authored the refinement and its first microscope. Actual final prose checks passed. Its incorrect self-reported action count and earlier censored source-reading attempt remain qualified, not discarded or called completion.

Result: 3 defects, 0 smells, 1 nit. Status remains draft.

D1. The new normative threshold paragraph omits the MAXVAL >= 128 condition and the separate low-MAXVAL formulas. It also abbreviates T2 and T3 rather than transcribing their distinct offsets and NEAR coefficients. The preserved official T.87 text, lines 2219 to 2244, explicitly separates Figures C.4 and C.5. At MAXVAL 127 and NEAR 0, independently evaluating Figure C.5 gives thresholds 2, 3 and 10. Misapplying Figure C.4 gives 2, 3 and 4. Transcribe both branches, integer floors and the exact CLAMP rule. This is a source and arithmetic inspection control, not an executed codec test.

Primary source: ITU-T T.87 (06/1998), official PDF SHA256 `4ca6f9816fb024b954fc811c8c6821814b45beb1690dff32ed1e8aef609a8778`, preserved in `/private/tmp/ocelli-s46-codex/f269-current-clause-inventory/`. The primary-source receipt identifies the official edition and download and confirms no archive implementation source was opened.

D2. The exact four-path write set and dependency section exclude docs/hld/DEVIATIONS.md, but the LLD section conditionally requires a D-20 count correction. Declare that conditional path now and update its truthful description in the same comment and pin commit when the patched-file inventory actually changes. Do not invent an implemented plan-status value or change the underlying deviation policy.

D3. The consumer map attributes JPEG-LS encode to crates/ocelli-codec. Its source provides decode, while tests/jpegls.rs:1422 calls the vendored encoder directly to build a regression fixture. Map that actual consumer and the vendor encoder tests rather than inventing a public Ocelli encoding API.

N1. Keep the historical damaged-stream decoder sweep distinct from encoder inspection and any new universal proof. The encoder's modulo-reduction premise can be recorded with its actual initial and reset conditions, but the decoder measurement is not an encoder measurement.

The caller-specific decoder refusal and encoder modulo-reduction distinction materially improves the inherited plan. Comment 4's local-gradient wording and the existing immutable inventory and patched-hash mechanism remain settled. No new behavior, arithmetic, tolerance, gate, profile or reference default is authorized. GLM must repair the bounded source claims, run its own review and actual prose checks, then return the exact draft for independent approval before touching vendor comments or pins.
