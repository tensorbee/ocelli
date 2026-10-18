# F-100 review, pass 1

**Reviewed**: working tree on sprint/s15
**Result**: 2 defects, 3 smells, 2 nitpicks

## Defects

### D1, the publication list rejects the new values

**Where**: `packages/core/src/index.ts` re-exports `CAMERA_COMPONENTS`, `STACK_COMMANDS`, `TIER_C_DRAWING` and `VIEWPORT_STATE_FIELDS`. `packages/core/src/index.test.ts` keeps the old `PUBLISHED` list.

**What**: `publishes exactly the runtime surface named above` failed under `npx vitest run packages/core/src/index.test.ts`. The received keys add those four names. The test is hand written so an added runtime export goes red on purpose. `viewport.test.ts` never imports the package index, so it stays green while the index drifts.

**Judgment**: Wrong behaviour. The story publishes four values and leaves the check that records publication failing.

### D2, the LLD says unlanded stories already write the readback

**Where**: `docs/lld/public-api.md`, the paragraph under the section 17.4 fence.

**What**: The file says F-045 writes the sixteen camera floats, F-046 writes centre, width and the invert bit, and F-047 writes the slice index and the slice count. All three rows are pending in `docs/sprints/BACKLOG.md`. `crates/ocelli-viewport/src/lib.rs` is still the scaffold. `docs/lld/README.md` requires an LLD to describe what the code does today.

**Judgment**: False claim. The header, the 48-byte event and the `ViewportState` layout match HLD section 17.4. The writer sentences do not. The following sentence, that nothing in this sprint copies the struct into JavaScript, is true and does not repair the three claims above it.

## Smells

### S1, the documented state cannot hold a window

**Where**: `packages/core/src/viewport.ts` `StackViewport`, `SetWindow` in `docs/lld/public-api.md`, and `VIEWPORT_STATE_FIELDS`.

**What**: The command table says `SetWindow` asks for centre, width and one of the three VOI functions. `StackViewport` stores a preset name, a colormap and an invert flag. It has no centre, no width and no function. The wire list names `voiCenter` and `voiWidth`, and no exported type has those fields. `WindowPresetName` is only `soft_tissue`, `lung` and `bone`, which F-046 fixes as `Linear`. A `LINEAR_EXACT` window has no name in this module. The design says `StackViewport` fields are the view inputs and the presentation inputs. The sixteen camera numbers are a bare count plus the string `camera`, not a field of that state.

**Judgment**: F-101 has no published type to copy centre and width into, and no published name for the three VOI functions. It will invent both.

### S2, drawing availability is not the three-state contract

**Where**: `TierAvailability` in `packages/core/src/viewport.ts`. `docs/lld/feature-availability.md` assigns F-100 the public API for shell presentation and names the states `Available`, `Degraded` and `Unavailable`.

**What**: The published union is `full` or `unavailable`. `full` is the plan-annotation word, not `Available`. `Degraded` cannot be said. Tier C drawing is honestly `unavailable` today, and F-X003 is still the CPU raster. When that raster is a fallback rather than the primary path, this type can only call it `full` or change under F-101.

**Judgment**: The approved F-100 plan asked for this two-value union. It still cannot carry the standing contract without a later break.

### S3, two name collisions are already scheduled

**Where**: `ViewportId` in `packages/core/src/viewport.ts`. F-046's plan says the TypeScript names `WindowPreset`, `Colormap` and `Invert` are declared once, in F-100, and that F-046 adds no TypeScript file.

**What**: `ViewportId` is `{ readonly id: number }`. The comment says an index is not interchangeable with another id space. Any other `{ id: number }` is assignable to it, including a slice index wrapped the same way. The exported preset and colormap types are `WindowPresetName` and `ColormapName`. Invert is a boolean field. None of the three F-046 identifiers exist.

**Judgment**: The next id space will mix, and F-046 will either miss its names or add a second set. The plan says a second set of names is a defect.

## Nitpicks

### N1, colormap and preset strings are unlocked

`viewport.test.ts` locks the eleven commands, the six readback names, `CAMERA_COMPONENTS` and `TIER_C_DRAWING`. It never mentions `grey`, `stops`, `soft_tissue`, `lung` or `bone`. Those unions can drift from F-044, F-046 and `docs/lld/public-api.md` without a red test.

### N2, the type-only aside in the index test is stale

The comment on `PUBLISHED` lists the type-only exports `tsc` is supposed to watch and stops before `ColormapName`, `Scene`, `StackCommand`, `StackViewport`, `TierAvailability`, `ToolBinding`, `ViewportId` and `WindowPresetName`. Nothing asserts that comment. It is now a short list of a longer surface.

## Verified clean

The eleven commands match the plan order and the F-045 and F-046 operations. Fit returns pan to zero and zoom to one and keeps rotation and flip. Pan is a canvas translation. Zoom and both flips are about the image centre. Rotate is clockwise radians on a y-down canvas. `SetInvert` does not XOR with photometric interpretation. `Scroll` is stored order. `grey` and `stops` are F-044's `PresetId`. The preset names match the F-046 table. The readback names are the HLD section 17.4 order without the padding. The event stride is 48 bytes. Flag bits 0, 1 and 2 match the HLD comment. `coreAvailable` still returns false. `viewport.ts` has no import, no `wasm.memory`, no `wasm-bindgen` and no `Uint8Array`. The new markdown has no em-dash. The only semicolons in it sit inside the section 17.4 code fence. No Rust crate and no `packages/react` file is in the diff.

`node-test-modules.d.ts` is acceptable. The package build includes `src/**/*`, so the test's `node:fs`, `node:path` and `node:url` imports must typecheck, and the package has no `@types/node`. The shim types the three calls the test makes. It does not declare `process`, `Buffer` or the rest of Node. `tsc` does not emit it, and `dist/index.d.ts` does not reference it. A full `@types/node` dependency would open a wider Node surface on a browser package whose runtime dependencies are none.
