# F-266 guard-consumer amendment review, pass 2

Reviewer: Codex, independent of GLM. Reviewed the draft pass 2 amendment after the bounded native author correction. Plan SHA256: `8ba8d11399c5a7b570967a1608f18ebb2f6d4e1cf3a1f1b81d51c3e3acccce8f`. No code adoption follows.

Result: 1 defect, 0 smells, 1 nitpick. Earlier scope-extension and exact percent-mapping findings are closed.

D1. The all-emitted-file promise still contradicts the API and binary treatment. The text map excludes non-UTF-8 content, says binaries receive name and size checks only, and then says a decode failure is refused. Name and size checks cannot detect the forbidden byte sequences, while refusing valid binary WASM breaks the healthy bundle. A source map is normally JSON and must not be categorically excluded as binary. Inspect all emitted file bytes for the known checkout-root and /@fs patterns. Decode only where the loader's textual import semantics require it, without rejecting a valid binary for being binary. The final API and actual callers must carry the content required for that check.

N1. Specify the key relationship between paths relative to dist and the existing worker asset basenames. A map keyed as assets/core-worker-a1.js cannot be read through core-worker-a1.js without a declared conversion. The author may choose either consistent representation, with all current callers and controls updated together.

The extension is now exactly catalogue.py, the existing probe budget and generated guard runbook, with existing checker unit tests already owned. Probe registration and meaningful valid-chunk absolute-fetch controls are included. Unrelated ratchets and policy remain unchanged. A direct GLM paragraph correction is appropriate. Recheck the resulting full appendix before approval. Implementation remains blocked on design approval.
