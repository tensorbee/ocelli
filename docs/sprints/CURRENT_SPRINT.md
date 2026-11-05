# Current sprint, S45

**Milestone**: M20, local folder viewer.
**Branch**: `sprint/s45`
**Opened**: 2026-11-04
**Goal**: The local folder viewer opens a synthetic study or a folder, and the page shows the image tools, the marks, and how to call the library.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-206 | E38.16 | Folder viewer synthetic study, spacing, edge letters and preset window | Rust | 5w | done |
| F-207 | E38.17 | Folder viewer layout, gestures, marks and image controls | TS | 8w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority.

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S45 / {print $2, $9}'
```

## What this sprint is

S44 is closed at tag `s44`. That close completed M19 and published nothing.
This sprint is the local folder viewer at `examples/folder-viewer`. It is
further Phase 1 and it is outside the imported Phase 1 total. F-119 brick
decomposition is S46. It is not opened here. Closing this sprint does not
publish.

`cargo run -p ocelli-folder-viewer` prints one loopback URL. The React
harness that `npm run dev` serves is a different page.

## What is carried in

- **D-22 still blocks publication.** The version stays 0.1.0.
- **One LUT chain.** Preset centre and width stay in `ocelli-viewport`. The page sends a preset name.
- **Marks stay in the browser until reload.** They are not DICOM SR, GSPS, or a saved measurement.
- **F-202 to F-205 stay unscheduled.** This sprint does not build them and does not print a stand-in result.
- **The existing window drag axes stay.** Centre follows vertical movement. Width follows horizontal movement.
- **Overlay, shutter, and slice blending stay off this image.** A later change would be a new pixel step.

## The defect class this sprint is exposed to

A quietly wrong grey byte. A preset that copies the window numbers into the page. A length that swaps row spacing with column spacing. A rectangle fill that tints the grey under it. A trackpad swipe that steps one slice per event. A pinch that zooms the browser page instead of the image.

## What done means

F-206 serves a synthetic study with no patient name, reports spacing and edge letters for the shown frame, and reruns the existing LUT for a named preset. A VOI LUT Sequence refuses that preset. The tests cite PS3.3 C.11 and go red when the intercept changes.

F-207 is the page. The chrome is the pure gray ramp and the system sans stack. Pinch zooms at the pointer. A wheel without Ctrl changes slice from accumulated travel. Freeform remains. Rectangle and ellipse are drawings with a colour. Text size and colour change after the note exists. Any mark can be removed. `/more` and `/use` are static. The README matches the page.
