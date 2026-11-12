# F-245 user documentation and More matrix review, pass 1

**Reviewed**: `examples/folder-viewer/src/more.html` DICOMweb sources row, `examples/folder-viewer/src/use.html` section 5.6, `README.md` CSP section and Start the local series viewer, against checkpoint fa3a7019.

**Author / reviewer**: GLM authored the four edits and one correction. Claude coordinated and independently reviewed every claim against the reviewed code and the browser suite evidence.

**Result**: 0 defects, 0 smells, 1 nitpick after one correction.

## Acceptance challenged

AC7 rejects relabelled legacy pixels and overstated matrix rows. The More row moves from Not yet to the existing `part` state with text that names what the browser suite proves, study, series and instance search plus one validated instance fetch, says remote image display is not available yet, and keeps the existing delivery cell. No completion is claimed and no story ID enters product copy. The README and Use text say the local study, marks, documents and checks stay as they are, which the suite asserts at refusal, held pending and validated stages.

## Corrected claim

GLM first wrote that without both values the viewer runs local only. A single value stops startup, as `viewer_dicomweb_config_from_os` and its tests require. The sentence now distinguishes neither value, which runs local only, from one or an invalid value, which stops startup with a fixed message.

## Claims checked

The CSP note matches `F-245-serve-context-pass-3.md`, the configured origin applies only to the image page. The IPv6 refusal matches the user-approved amendment and the Chromium observation. The DicomwebProvider method list matches the exported class.

## Findings

**N1** `more.html`. The viewer cell joins three clauses with commas in one sentence. It is accurate and the prose gate passes, so it is noted only.

## Still open

The README Start the example viewer rewrite waits for the launcher and package entry owned by the F-268 shared slice.
