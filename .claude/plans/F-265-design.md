# F-265, Bounded ECG waveform reading and display in the single showcase viewer

**Status**: draft
**Epic ref**: E38.60
**Sprint**: S48
**Estimate**: 6w

The operator requires real parity workflows in one viewer. The lead scheduled
this newly identified gap on 2026-11-08. This is a provisional engineering
estimate, not observed capacity or a calendar promise. The normal detailed
design and approval round must finish before implementation.

## Normative source and design work still required

HLD Appendix B names ECG among the viewport families. The existing DICOM
metadata model and three-channel worker boundary remain authoritative.
DICOM PS3.3 C.10.9 defines the Waveform Module. Before coding, the detailed
design must transcribe its relevant channel, sample, multiplex, timing,
calibration and units rules, the selected ECG IOD tables and PS3.5 waveform
sample encoding. It must cite the editions read and write independently
computed vectors with exact expected values. This intake approves no new
formula, numerical tolerance, public signature or boundary exception.

The installed cornerstone3D 5.8.2 implementation is an additional capability
reference, not a substitute for the DICOM standard. Its source observations
and their limits are recorded in the companion ECG audit.

## Problem and source facts

Ocelli has WAVEFORM references in Key Object Selection documents, but no
waveform sample reader, ECG render provider or same-viewer ECG workflow at
the inspected source. Reading or writing that reference does not read or
draw the signal. No existing allocation row implements this capability.
F-262 inventories features and checks coverage. It cannot replace this
implementation story.

The pinned reference has both legacy and generic ECG viewports, an actual
canvas signal render path, channel selection and time/value controls.
Its raw sample helpers implement signed 16-bit multiplexed data and use the
first multiplex group. The registered metadata provider can obtain a
naturalized waveform instance and retrieve raw, inline or bulk data. The
audit executed the installed deinterleaver and trace utility on own synthetic
samples, without a browser. It did not establish full configured DICOMweb
loading, calibrated clinical output or general waveform-format parity.

## Scope, providers and compatibility

Implement a bounded read/display subset for synthetic 12-lead, general and
ambulatory ECG Waveform Storage with a valid single multiplex group and
signed 16-bit samples. The detailed design must pin the supported SOP Class
UIDs and attribute/transfer-syntax requirements before code starts. Other
storage classes, unsupported encodings and multiple groups are refused by
name until separately designed. They must never become a plausible image
stack or a silently truncated first group.

Reuse F-017's metadata model and its underlying DICOM reader, F-245's local
and DICOMweb source selection, and F-202's owned worker and surface lifetime.
The waveform reader and rendering provider are new work. No existing
waveform API is asserted to exist. The detailed design selects the concrete
modules, typed sample representation and commands while preserving the
current boundary and allocation rules.

The same canonical viewer opens the waveform, draws real channel traces,
shows channel labels and declared time/amplitude units, and exposes channel
visibility, time pan/zoom, amplitude scaling and reset. Point readout names
the selected channel, sample time and calibrated value. Image window/level,
image inversion and patient-space image rulers are not silently applied to
waveform samples. This story adds no rhythm interpretation, diagnosis,
filtering, reconstruction, audio playback, acquisition or live monitoring.

## Consumer and provider map

| Consumer | Provider | Required result |
|----------|----------|-----------------|
| Local or DICOMweb selected ECG object | Existing source/metadata entry and new bounded waveform reader | Current object identity and validated sample metadata |
| Same-viewer signal viewport | New owned signal render provider using the approved worker boundary | Actual traces, labels and controlled time/value presentation |
| Channel/time/value controls | One typed waveform and calibration provider | Visible result and readout from the same selected samples |
| More matrix and F-262 inventory | Real browser workflow and source-pinned reference observation | Planned or usable status matching evidence |

## State, ownership and refusal policy

The selected source and object generation own every pending load and trace.
A source, channel or time-range change discards older results before visible
commit. A refused load preserves the previous usable view and does not
partially replace its data or labels. Cancellation and repeated release free
buffers and surface state exactly once. Tests inject a failure after staging
and before publication, then retry with the current identity.

The detailed design must specify finite byte, channel and sample limits,
checked length arithmetic, positive finite sampling frequency, calibration
validation and truncated/extra sample policy before allocation. No zero fill,
invalid units or guessed calibration may turn malformed data into a valid
trace. Renderer scratch is prepared outside the draw loop. Metadata values
remain private to their intended local UI, never copied into errors or logs.

## Numbered acceptance inventory

| ID | Trigger and expected behaviour | Required observation | Wrong implementation rejected | Stage |
|----|--------------------------------|----------------------|-------------------------------|-------|
| AC1 | Open an own synthetic supported ECG object from local data and configured DICOMweb in the same canonical viewer | Actual fetch/selection identity and visible multi-channel traces | Enum, label-only panel, KOS reference or separate demo counted as display | integrated browser |
| AC2 | Decode literal signed, interleaved channel vectors and apply the approved sampling, channel calibration and units rules | Hand-derived PS3.3/PS3.5 vectors through the actual provider and point readout | Channels swapped, sign lost, ignored calibration or guessed sample interval | implementation and integrated |
| AC3 | Show labels and declared time/amplitude axes matching the selected signal | Independently expected sample coordinates and units through the actual canvas | An image stack, arbitrary file order or unlabelled device codes used as a waveform | integrated browser |
| AC4 | Change channel visibility, time range, amplitude scale and reset | Real controls change the committed traces and preserve unrelated viewports | Controls disconnected from rendering or accumulated transforms | integrated browser |
| AC5 | Reject unsupported sample forms, multiple groups, impossible counts, nonfinite calibration and truncated data before allocation/publication | Permanent named negative and recovery cases from the detailed design | Silent first-group selection, zero-filled tail, panic or unbounded allocation | implementation and integrated |
| AC6 | Change source or selection during load, then release, reopen and retry after refusal | Adversarial browser lifecycle case with current source/object identity | Old trace, old labels, late readout or duplicated surface survives selection | integrated browser |
| AC7 | Preserve the existing image, video, annotation, document and conformance workflows | Current standing suites on the combined source | Waveform route changes an image's presentation or ownership | integration |
| AC8 | Display supported signals on every resolved A/B/C image tier through the declared signal path | Actual owned 2D surface and forced no-GPU case | Basic ECG silently unavailable merely because imaging GPU support is absent | integrated browser |
| AC9 | Qualify cornerstone parity and unsupported formats truthfully | Source-pinned comparison and actual browser observations before measured parity claim | SS16 helper or generic viewport existence inflated into all-waveform support | integration and external |
| AC10 | Register a meaningful load/draw/resource instrument before claiming waveform performance | Named instrument, release source/binary identity, raw timing and process evidence | An image window benchmark relabelled as waveform timing | external after instrument exists |
| AC11 | Expose the working action and limits in More and the standing inventory | F-262 includes this implemented same-viewer workflow and fails when it is removed | Inventory row substitutes for the new reader/render implementation | integration |

## Dependencies and S48 sequencing

Dependencies: F-245, F-202 and F-017. Each must be done before this story starts,
as `/start-feature` requires. A reviewed partial checkpoint does not satisfy
that prerequisite. F-262 depends on F-265 and cannot close while this workflow
is missing. No other draft story gains waveform implementation ownership.

Use focused reader, numerical and lifecycle controls while implementing.
The required feature profile belongs to the final staged integration tree.
Any source or metadata change invalidates earlier evidence. Browser and GPU
work uses the coordinated resource slot, and the sprint profile runs once
when every story is completed or carried under the normal workflow.

## Boundary and tier

- wasm-bindgen remains only in ocelli-wasm if the detailed design needs an export.
- No new unsafe site or unapproved sample/pixel boundary is introduced.
- The worker owns the actual signal surface and pending generation.
- A/B/C use the declared owned 2D signal path, independently of image GPU capability.
- The same numerical waveform provider supplies traces and point readout.
- Draw-loop allocation and per-event boundary calls remain forbidden.

## Open questions before implementation

Complete the selected IOD/module transcription, exact supported encodings,
finite resource limits, calibration policy, typed module/API map, command and
surface ownership, literal fixture values and actual browser selectors.
Independently review those decisions before coding. This is scheduled new
implementation work, not an existing feature hidden behind a UI selector.


## Reviewed draft refinement

Additive to the F-265 draft only. Status, sprint, estimate and dependencies stay unchanged. No implementation, acceptance, new API, policy, math, tolerance or encoding is selected.

Same-viewer consumer controls:

1. Open an own synthetic supported ECG object from local data and from a genuinely configured remote source. Real multi-channel traces appear through the new bounded reader. A KOS reference, enum or label panel is not display.
2. Decode literal signed 16-bit interleaved vectors through the actual provider and point readout. Challenge channel order, sign, stored and allocated bits, byte order and several distinct channel shapes.
3. Confirm labels and declared time and amplitude axes come from the same selected samples. Raw sample, time, channel skew and offset, and coded amplitude units stay separate claims.
4. Change channel visibility, time pan and zoom, amplitude scale, reset and resize. Only the current identity commits, unrelated viewports keep their state, and release and reopen leave no duplicated surface or stale readout.
5. Present unsupported forms, multiple groups, impossible counts, nonfinite calibration and truncated data. Refusal happens before allocation or publication, with no silent first-group choice and no zero-filled tail, and the prior usable view survives.
6. Observe the declared owned no-GPU 2D path on each tier, separating supported behaviour from missing-host evidence. No image window, inversion or ruler is applied to samples.
7. Check logs and errors expose no samples, patient attributes, labels, identifiers or paths, and rerun standing image, marks, colour/ICC, video, document and conformance workflows. More names no second showcase.

Unresolved lead questions: exact calibration policy, padding and truncated or extra byte treatment, typed ownership boundary, presentation and units fallback including physical display scaling, and permanent observers for every approved acceptance row.
