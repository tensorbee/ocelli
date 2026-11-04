# Current sprint, S43

**Milestone**: M19, series open in the core and the viewer.
**Branch**: `sprint/s43`
**Opened**: 2026-11-04
**Goal**: Calibration provenance and the extra SUV methods, Key Object Selection and Secondary Capture, a volume from a gantry-tilted stack, and the example viewer loading one wasm worker that publishes one stored sample, reports the resolved tier, and posts one WADO-RS envelope.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-195 | E38.5 | Calibration provenance: spacing, imager spacing with ERMF, ultrasound regions, then manual. SUV by lean body mass and body surface area, length ratio, volume from contours | Rust | 4w | done |
| F-196 | E38.6 | Key Object Selection read and write (TID 2010) and Secondary Capture write | TS | 4w | done |
| F-197 | E38.7 | Volume from a gantry-tilted stack | Rust | 3w | done |
| F-198 | E38.8 | Example viewer loads one wasm worker, publishes one stored sample, reports the resolved tier, and posts one WADO-RS envelope | TS | 4w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority.

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S43 / {print $2, $9}'
```

## What this sprint is

S42 is closed at tag `s42`. This sprint is the second of the three series-open
sprints inserted before the brick work. The brick story F-119 is S45. It is
not opened here.

The showcase is `examples/viewer-react`, which `npm run dev` serves. It
must display every behaviour this programme adds. A core function with no
control on that page is not done. `examples/folder-viewer` stays the
local folder page and is not a second product.

## What is carried in

- **D-22 still blocks publication.** Nothing in this sprint publishes.
- **The LUT chain already exists.** `ocelli-pixel::LutChain` is the only
  pixel formula. Calibration and SUV read geometry and tags. They do not
  map samples.
- **`coreAvailable()` is true only after `loadCore` succeeds.** A clean import
  stays false. The example worker is the caller. The page still shows the
  S42 sections beside that gate.
- **A gantry tilt is measured from geometry.** F-020 and F-049 already
  refuse a sheared stack. F-197 decides whether a tilted stack can become
  a volume. It does not read the gantry tilt tag as the shear.

## The defect class this sprint is exposed to

A quietly wrong measurement. Spacing taken from the wrong source, an SUV
computed from the wrong body-size inputs, and a tilted stack assembled as
an untilted box are the same class. The hand fixtures cite the DICOM
sections for pixel spacing, imager pixel spacing, ultrasound regions, SUV,
and image-plane geometry. A tolerance is not widened to make one of them
pass.

## What done means

Calibration names its source in the order spacing, imager spacing with the
estimated radiographic magnification factor, ultrasound regions, then a
manual value. SUV by lean body mass and by body surface area are separate
from the SUV already shipped. Length ratio and volume from contours cite
their inputs. Key Object Selection reads and writes TID 2010. Secondary
Capture writes a display image. A gantry-tilted stack either builds the
volume the geometry supports or refuses with that geometry's reason. The
example viewer loads one wasm worker through `@ocelli/core`. That worker
publishes one stored sample with `publishCpuCanvas` and reports the tier
`resolve` kept. A form posts one WADO-RS instance envelope into the worker
bulk slot. The envelope is not decoded and the cache is not used.
`coreAvailable()` is true only after `loadCore` resolves true.
