# Current sprint, S44

**Milestone**: M19, series open in the core and the viewer.
**Branch**: `sprint/s44`
**Opened**: 2026-11-04
**Goal**: The example viewer gains layout, manipulation and measurement controls for the behaviours the core already has.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-199 | E38.9 | Example layout grid, stack drag, full screen, reference lines and reference cursor | TS | 4w | done |
| F-200 | E38.10 | Example presets, window drag, region window, rotate, flip, marker, magnifier, colour map, scale length, crop planes and error sentence | TS | 4w | done |
| F-201 | E38.11 | Example measurement document, annotation list, undo, TID 1500, key image mark, shortcut and touch | TS | 4w | done |

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
page does not have. The controls call functions already exported by
`@ocelli/core`. They do not start a second renderer. Prior studies, linked
scroll, synced window, MPR, slab, volume rendering, fusion, one to one,
histogram, four corners, a metadata browser and preferences are unscheduled
F-203, F-204 and F-205. F-202 stays unscheduled.
