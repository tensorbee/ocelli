# F-100 review, pass 2

**Reviewed**: working tree on `sprint/s15`, uncommitted F-100 files
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the padding field drops an underscore

`docs/lld/public-api.md:29` transcribes HLD section 17.4 and names the last field `pad`. The struct in `docs/hld/14-the-boundary-in-code.md:95` names it `_pad`. Every other field name in that fence matches. The type is still three `u32`s, and `VIEWPORT_STATE_FIELDS` does not publish the padding, so the bytes a reader checks are unchanged.

## Verified clean

Pass 1's two defects, three smells and two nitpicks were checked against the files and the commands below. The summary of the remediation was not treated as evidence.

D1. `packages/core/src/index.test.ts` lists `CAMERA_COMPONENTS`, `COLORMAPS`, `INVERT`, `STACK_COMMANDS`, `TIER_AVAILABILITY`, `TIER_C_DRAWING`, `VIEWPORT_STATE_FIELDS`, `VOI_FUNCTIONS`, `WINDOW_PRESETS` and `viewportId`. `npx vitest run packages/core/src/viewport.test.ts packages/core/src/index.test.ts` exited 0. Both files passed, 10 tests. The publication assertion compares `Object.keys` of the index with that hand-written list, so a missing or extra runtime export is red.

D2. `docs/lld/public-api.md` no longer says F-045, F-046 or F-047 write the readback. A search of that file for those three ids finds nothing. The file says F-101 implements the wire and that nothing in this sprint copies the struct into JavaScript. `crates/ocelli-viewport/src/lib.rs` is still the scaffold. No Rust file defines `ViewportState`. The only `voiCenter` hits under `packages/` are the new type and its test.

S1. `StackViewport` has `camera`, `voiCenter`, `voiWidth` and `voiFunction`. `VOI_FUNCTIONS` is `linear`, `linear_exact`, `sigmoid`, which is the defined-term order LINEAR, LINEAR_EXACT, SIGMOID in PS3.3 C.11.2. Those strings are the `VoiFunction` variants `Linear`, `LinearExact` and `Sigmoid` in `crates/ocelli-pixel/src/lut.rs`. `Camera` is a 16-number tuple and `CAMERA_COMPONENTS` is 16.

S2. `TIER_AVAILABILITY` is `available`, `degraded`, `unavailable`. `TIER_C_DRAWING` is `unavailable`. `docs/lld/feature-availability.md` names the states Available, Degraded and Unavailable. The published strings can say all three. `docs/lld/public-api.md` says the plan word `full` is published as `available`, that tier C drawing is `unavailable`, and that this story declares no drawing fallback. The comment on `TIER_AVAILABILITY` says the same refusal. Nothing in the new prose claims the shipped union is still `full | unavailable`, and nothing claims a drawing fallback exists. F-X003 is still the pending CPU raster row in `docs/sprints/BACKLOG.md`.

S3. `ViewportId` is `number` intersected with a unique-symbol brand. `WindowPreset`, `Colormap` and `Invert` are exported from `viewport.ts` and from `index.ts`. `.claude/plans/F-046-design.md` says those three TypeScript names are declared once, in F-100, and that F-046 adds no TypeScript file. The values are `soft_tissue`, `lung`, `bone`, then `grey` and `stops`, then `identity` and `inverse`. That matches the F-046 preset table, F-044's `PresetId::Grey` and `PresetId::Stops`, and the Identity and Inverse presentation shapes. F-044 is done. `ColourLut::preset` builds those two tables and `write_colour_lut` uploads them.

N1 from pass 1. `viewport.test.ts` locks the colormap, preset, function and invert arrays. Changing the first `WINDOW_PRESETS` entry from `soft_tissue` to `abdomen` made `npx vitest run packages/core/src/viewport.test.ts` exit 1. The failing assertion was `packages/core/src/viewport.test.ts:69`. Expected was `["soft_tissue", "lung", "bone"]`. Received was `["abdomen", "lung", "bone"]`. The line was restored to `["soft_tissue", "lung", "bone"]`.

N2 from pass 1. The comment above `PUBLISHED` now names `Camera`, `Colormap`, `Invert`, `Scene`, `StackCommand`, `StackViewport`, `TierAvailability`, `ToolBinding`, `ViewportId`, `VoiFunction` and `WindowPreset`. Five older type-only exports were already absent from that comment before this edit: `DicomwebResponseKind`, `DicomwebResponseSink`, `DicomwebClientOptions`, `DicomwebErrorCode` and `QidoSearchOptions`. Nothing asserts the comment. Not re-filed.

The brand guard is real under `tsc`. Vitest strips types. Removing the intersection so `ViewportId` was plain `number` made `npm run typecheck` exit 2:

```text
packages/core/src/viewport.test.ts(76,11): error TS2322: Type 'true' is not assignable to type 'false'.
```

That assignment is `NotAssignable<number, ViewportId>`. Restoring the old structural `{ readonly id: number }` made `npm run typecheck` exit 2 on the other check and on the mint:

```text
packages/core/src/viewport.test.ts(75,11): error TS2322: Type 'true' is not assignable to type 'false'.
packages/core/src/viewport.ts(84,10): error TS2352: Conversion of type 'number' to type 'ViewportId' may be a mistake because neither type sufficiently overlaps with the other. If this was intentional, convert the expression to 'unknown' first.
```

Line 84 was the mutated file, after the declaration shrank. The restored mint is `packages/core/src/viewport.ts:86`. The brand was put back. `npm run typecheck` then exited 0.

The single `as ViewportId` is justified. It brands a number. It does not narrow a numeric range. The unique symbol cannot be attached any other way, the function is the only mint, and the runtime value stays the number. `viewportId(4)` is `4`. The assertion type-checks because the brand is an intersection with `number`. The structural mutation above shows the same assertion is rejected when the type stops overlapping `number`.

`viewport.ts` has no import, no `wasm.memory`, no `wasm-bindgen` and no `Uint8Array`. `coreAvailable` is still `return false` at `packages/core/src/index.ts:119`. The diff does not touch that function. `git status --short` and `git diff --stat` show `docs/lld/README.md`, `docs/lld/typescript-packaging.md`, `packages/core/src/index.test.ts` and `packages/core/src/index.ts`, plus the untracked public API, the two viewport files, the node shim and pass 1. No Rust crate and no `packages/react` path is in that set. The tracked diff is 4 files, 49 insertions and 6 deletions.

`node-test-modules.d.ts` is the same three-call shim pass 1 accepted. It did not grow.

The command table still matches F-045, F-046 and F-047. Fit returns pan to zero and zoom to one and keeps rotation and flip. Pan is a canvas translation. Zoom and both flips are about the image centre. Rotate is clockwise radians on a y-down canvas. `SetInvert` does not XOR with photometric interpretation. `Scroll` is stored order. Flag bits 0, 1 and 2 match the HLD comment. The event is kind `u32`, viewport `u32`, seq `u64`, payload 32 bytes and stride 48.

The `scripts/prose_check.py` scanner, run on `docs/lld/public-api.md`, `docs/lld/typescript-packaging.md` and `docs/lld/README.md`, reported no em-dash and no prose semicolon. The only semicolons in `public-api.md` are inside the section 17.4 fence.

The preset line and the brand were restored. `git status --short` after that restore matches the tree this pass started from.
