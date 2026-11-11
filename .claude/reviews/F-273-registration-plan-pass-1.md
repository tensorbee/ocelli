# F-273 registration plan review, pass 1

**Reviewed**: Approved plan 15bd714f3186f24b8570928965020486ebbdad1a6e02b0623b975baefcdbeb8b and GLM amendment proposal b553f2a91b6d107886148892261626445e4815184f6affea8893702e3ac2dfa2. Final approval status bookkeeping is recorded separately in f273-registration-plan-root-adoption.json.

**Author / reviewer**: GLM authored the narrow clause and source-binding paragraph. Worker Codex discovered the integration mismatch. Root Codex independently traced the actual pinned producer, dispatch and consumer, reviewed the complete amendment and changed only its approval status wording during adoption.

**Result**: 0 defects, 0 smells, 0 nitpicks in the amendment. The earlier wrong registration scheme is corrected.

## Acceptance challenged

A public wrapper must intercept the images the benchmark actually displays. A registration mock is insufficient if its scheme never receives those image IDs. Existing stock behavior and other loader registrations must remain unchanged.

## Evidence and verified clean

Root read the pinned fileManager.js, wadouri/register.js and Core loaders/imageLoader.js, and traced tools/oracle/page/bench.mjs from fileManager.add through rows and imageIds to viewport.setStack. The producer returns dicomfile IDs and Core dispatches by the prefix before the colon. The earlier successful private public-scale prototype also registers dicomfile. The public delegate remains wadouri.loadImageFromNaturalizedMetadata. The prior plan line 164 targeted wadouri and would miss the actual file consumer.

The amendment changes only that registration key and appends the source trace plus an actual built-page wrong-scheme sensitivity control. It does not edit installed packages, broaden the admitted domain, change tolerances or add a public helper. The canonical file is independently verified as the entire original plan with one public clause replaced and the proposed paragraph appended. Only proposal/approval wording changes during final adoption. Original operator material remains byte-identical and was not transmitted to GLM.

Exact source hashes and observations are in /private/tmp/ocelli-s46-codex/f273-registration-scheme-root-source-check.json. Exact full-file reconstruction and adoption hashes are in /private/tmp/ocelli-s46-codex/f273-registration-plan-root-adoption.json. This review executed source inspection and byte comparisons, not the proposed runtime control.

## Limitations and handoff

The actual reference loader is not implemented by this amendment. GLM must register the wrapper once on the reference-only dicomfile path and prove a wrong-scheme mutant produces no successful committed facts or accepted metrics through the actual built-page consumer. Synthetic numerical, lifecycle and equipped acceptance remain owed. The real-study hold remains active. Approval permits the corrected isolated synthetic implementation and supplies no runtime or full F-273 acceptance.
