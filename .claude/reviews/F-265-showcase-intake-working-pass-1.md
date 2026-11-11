# F-265 showcase intake, independent review pass 1

Author: Codex glm_codec_merge. Independent reviewer: Codex root.
Source: `79d1ed8c5ad5a4f3a7a6ceaf90025528e1c74dee`, against
`c6874996d98bd6ef13a316f759eea7a3e67d8a5f`.

Defects: 0. Smells: 0 in planning scope. The audit has one optional repeated
wording nit, which does not change its factual observations.

Root read the entire new draft and capability audit, all affected existing
plan/backlog/roadmap lines and the actual installed MIT ECGViewport source.
The reference implements waveform presentation and has a metadata provider.
Neither its declared enum nor its raw signed-16 helper is inflated into a
claim of complete configured DICOMweb or calibrated clinical parity.
Ocelli's KOS waveform references do not provide sample reading or rendering.

The new reader and signal provider are explicitly new work in F-265. The
single-multiplex, signed-16 ECG subset, finite input limits, calibration,
units and unsupported-form refusal need detailed DICOM design before coding.
Actual fetch, traces, controls, calibrated readout, cancellation and a no-GPU
browser observation are required in the same viewer. No invented API,
numerical rule or new tolerance is accepted by this intake.

F-265 is allocated S48 after completed F-245, F-202 and F-017. F-262's standing
inventory depends on it. S48 becomes eight stories and 36 provisional weeks,
M21 becomes 25 stories and 104 weeks. These are relative engineering estimates,
not dates or observed capacity. Other allocations and original 24 draft scopes
are preserved except the added inventory dependency. Historical notes and
unresolved import warnings remain intact. Pending close drafts start at F-266.

The complete six-path addition applies cleanly to both the canonical sprint
and composed candidate. Its author checks are recorded in f265-handoff.json.
Root stages and runs the named planning guards again before the checkpoint.
This review approves scheduling and draft scope, not feature implementation,
measured performance, S46 completion or publication.
