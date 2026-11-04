# Current sprint, S44

**Milestone**: M19, series open in the core and the viewer.
**Branch**: `sprint/s44`
**Opened**: 2026-11-04
**Goal**: The example viewer gains layout, manipulation and measurement controls for the behaviours the core already has.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-199 | E38.9 | Viewer layout grid, series panel and drag, full screen, prior studies, linked scroll, synced window and zoom, reference lines, 3D cursor, MPR, slab, volume rendering, fusion | TS | 4w | pending |
| F-200 | E38.10 | Viewer manipulation and overlays: presets and file windows, one to one, rotate, flip, markers, magnifier, histogram, colour maps, four corners, scale bar, overlays and shutters, banner and warnings | TS | 4w | pending |
| F-201 | E38.11 | Viewer measurement, annotation list, undo, TID 1500 save, key images, export, metadata browser with compare and copy, shortcuts, touch, preferences | TS | 4w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority.

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S44 / {print $2, $9}'
```

## What this sprint is

S43 is closed at tag `s43`. This sprint is the third of the three series-open
sprints inserted before the brick work. The brick story F-119 is S45. It is
not opened here. Closing this sprint completes M19. It does not publish.

The showcase is `examples/viewer-react`, which `npm run dev` serves. It
must display every behaviour this programme adds. A core function with no
control on that page is not done. `examples/folder-viewer` stays the
local folder page and is not a second product.

## What is carried in

- **D-22 still blocks publication.** Nothing in this sprint publishes. The
  version stays 0.1.0.
- **The LUT chain already exists.** `ocelli-pixel::LutChain` is the only
  pixel formula. Viewer controls call it. They do not add a second one.
- **`coreAvailable()` is true only after `loadCore` succeeds.** A clean import
  stays false. The example worker is the caller.
- **F-198 does not decode a posted WADO-RS envelope.** The worker publishes
  one stored sample and holds the envelope. Decode, the cache and a render
  of that envelope are unscheduled F-202. This sprint does not build F-202.
- **The S42 and S43 page sections stay.** Frame open, series assembly,
  overlays, interpolation, calibration, key object, secondary capture, tilt
  and the wasm core panel are already on the page.

## The defect class this sprint is exposed to

A control that does a different thing from the one it names. Linked scroll
on the wrong series, a window preset that changes zoom, a measurement that
ignores the named calibration source, and a prior study shown as the current
study are the same class. A control for a core function this sprint does not
have reports unavailable. It does not invent a result.

## What done means

Each story's design names the example-page controls and the core functions
they call. A backlog row is not marked done while it claims a control the
page does not have. Layout, manipulation and measurement use the functions
S42 and S43 already shipped. F-202 stays unscheduled.
