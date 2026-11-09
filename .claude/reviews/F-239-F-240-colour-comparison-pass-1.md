# F-239 and F-240 controlled colour comparison, pass 1

Author: Codex comparison worker. Independent reviewer: Codex root.

Source: staged tree `3ab66e07798f921742526576edee2882d7c5fcda`, against
`3ecb12399395e2f13e0baf691c9db8d70fe83cd3`. This review covers the complete
eight-path supplemental module and generator change. The main F-223 runtime
and its integration are separate review and acceptance work.

## Findings

Defects: 0. Smells: 0 within this bounded source review.

The closed five-case protocol retains the unchanged informative CT calibration,
uses generated inputs with complete file identities, alternates both sides for
nine cold and warm trials, retains actors for nine explicit window changes and
requires three all-process memory sessions per side. ICC16 names its actual
512 MiB native budget. Other cases retain 256 MiB. No threshold or published
five-workload record is changed. ICC semantics are explicitly different and
colour is unscored.

The root read the complete runtime, contract tests and generator delta.
The row writer stores PS3.3 native YBR_FULL_422 pairs as Y1,Y2,Cb,Cr.
The two-entry ICC derivative updates its tag lengths and recomputes the
ICC.1 profile ID, independently matching the other implementer's profile.
Streaming avoids allocating the entire 384 MiB RGB16 pixel payload.
No tracked DICOM or corpus-manifest row is added.

Lifecycle challenge: the root inspected the consumer combination where a
window operation refuses and one actor closes synchronously with an error.
Both actor closes are attempted, and the primary draw failure is preserved.
The permanent controls also reject an invented memory peak, missing process
receipts, substituted source identities and a noninformative calibration.
The root reran all eleven Node contract cases, direct exit 0, in
`colour-comparison-root-node-review.json` under the local S46 evidence folder.
The author's generator, census and eight semantic mutation receipts remain
separate author evidence.

## Open acceptance

This is source approval for integration, not a claim of comparative performance,
pixel parity, successful large-fixture execution or a final feature profile.
The actual F-223 actors, CLI branch, measurement source and publication refusal
must be reviewed after composition. Equipped runs and final staged-tree
verification remain open. No colour score or new tolerance is authorized.
