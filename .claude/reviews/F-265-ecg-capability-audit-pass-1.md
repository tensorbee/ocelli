# F-265, installed ECG capability and scheduling gap audit

## Scope and source identities

Read-only capability audit against Ocelli showcase plan tree c6874996 and
its product base 4a3c0132, plus the installed permissive cornerstone packages
at version 5.8.2. Actual package paths and 15 full source digests are recorded
in /private/tmp/ocelli-s46-codex/ecg-audit/source-identity.json.
The core, dicom-image-loader and metadata packages are MIT licensed.

The reviewer did not author Ocelli ECG product code, because none exists in
the inspected tree. The same participant subsequently authored the new
planning draft at the lead's instruction. That draft needs another reviewer.
This is capability inspection and bounded utility execution, not a browser,
clinical, pixel-parity or feature-completion review.

## Finding: a real implementation gap, not only a missing menu item

The pinned reference implements ECG rather than merely declaring its enum:

- viewportTypeToViewportClass registers the legacy ECG and generic ECG_NEXT
  viewports, including the legacy adapter selection.
- GenericViewport/ECG/ECGViewport loads display sets and establishes a
  time/value range and channel visibility, line, amplitude and grid settings.
- DefaultECGDataProvider loads actual waveform metadata and sample arrays.
  The default resolver selects CanvasECGRenderPath.
- CanvasECGRenderPath computes a visible sample window and draws channel
  traces, baselines, grid and labels, applying its owned canvas transform.
- ECGUtilities draws a path through the selected channel's actual samples.
  This is executable rendering code, not a type declaration or placeholder.
- The loader's ECGHelpers supports signed 16-bit channel deinterleaving and
  raw/inline/bulk retrieval helpers. The metadata package also registers a
  naturalized-instance ECG provider with raw, InlineBinary and BulkDataURI
  handling. Both currently select the first multiplex group.

The old standalone loader helper is not referenced by its default WADO
metadata provider. That narrow observation alone does not prove missing ECG
loading infrastructure: further inspection found the metadata package's
registered instance provider. The audit therefore claims no lack of all
provider wiring, and also no turnkey configured DICOMweb ECG success without
running that full consumer. The raw helper subset does not prove support for
other sample representations, all multiplex groups or all calibration rules.
The existing metadata calibration provider uses fixed amplitude scaling, so
its existence is not evidence of every DICOM per-channel calibration rule.

Ocelli searches across crates, packages, examples, backlog and allocation
find only KOS WAVEFORM references and parity-surface mentions. TID2010's
correct classification of an ECG reference is not sample reading or drawing.
No waveform reader, signal viewport or allocated ECG implementation story was
found. F-202 implements image worker decode, the other planned viewport
stories implement image/volume modes, and F-262 is an inventory gate.
Counting ECG in that gate cannot implement the missing provider.

## Bounded executed evidence

The installed ECGHelpers deinterleaver and metadata package instance builder
both receive the same own two-channel, four-sample signed 16 signal. Their
actual channel outputs match literal vectors. The real drawECGTraces utility
receives one such channel and a recording Canvas2D context. Its move/line
commands match independently written coordinates for that simple scale.
The test invokes the actual installed code, with no source modification,
network fetch, browser or GPU. It proves these functions execute the claimed
work, not that a real reference viewport has been visually accepted.

Direct receipt ecg-audit-installed-metadata-and-traces returned 0. The earlier
loader/trace-only receipt also returned 0. Literal inputs, recorded commands
and commands are preserved under the local ecg-audit evidence directory.
Several initial discovery paths were absent or hidden by default node_modules
ignore rules. They were corrected before reading the actual installed files.
No failed discovery was treated as evidence that a capability was absent.

## Authorized scheduled disposition

The lead authorized F-265/E38.60 in S48 at a provisional 6 engineer-weeks,
depending on F-245, F-202 and F-017. It owns new bounded waveform parsing and
the real signal workflow in the same showcase viewer. Initial signed 16,
single-group ECG scope, finite limits and explicit unsupported-format refusal
must be pinned in detailed DICOM design before coding. No API or numerical
formula is invented by this intake. Time/amplitude calibration, point readout,
source cancellation, actual traces and no-GPU observation belong to its
acceptance. It adds no diagnosis, filtering or live monitoring.

F-262 gains F-265 as a prerequisite. S48 becomes 8 stories and 36 estimated
engineer-weeks including the preserved MPEG-2 story, while M21 becomes 25
stories and 104 weeks. These are provisional engineering sums, not team
capacity or a delivery-date claim. Other 24 draft scopes and their existing
allocations are unchanged except F-262's prerequisite. The pending S46-close
backlog drafts must start at F-266 rather than collide with F-265.

The scheduled draft and bookkeeping are concrete planning output. They do not
claim this missing implementation or its future runtime evidence is complete.
