# S16 repair evidence, pass 3

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree this repair answers**: `f6057868244a71679ebfd95a17e440084699aa76`
**Not a receipt**: `6b562be784f4365e7781869b5c511e9e0a1162a8`, `1f525c04`, `7de677af36b9122e60d2da4700c96b730f422e0a`, `8d0a94a34d4b35cd38e26c5056311eab05937231` and `f6057868244a71679ebfd95a17e440084699aa76` do not cover the repaired tree. No sprint-profile receipt was recorded for this repair.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is `git write-tree` after this file is staged. It is reported with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

F-048 pass 3 and F-101 pass 3 reported no new defects in their scoped repairs. Those fixes were left as they were. This pass changes the CPU canvas resize path.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-X003 D8, identical resize keeps the settled frame | evidenced | Permanent worker and the pass 3 probe, below | implementing agent | Codex review of the staged tree |
| F-X003 D8, rejected zero extent preserves the canvas | evidenced | Same worker and the same probe | implementing agent | Codex review of the staged tree |
| F-X003 D5 to D7 worker controls | evidenced | Same `cpu-canvas.mjs` run | implementing agent | Codex review of the staged tree |
| F-101 public adapter repairs | evidenced on the rebuilt module | Boundary suite, 5 tests | implementing agent | Leave the interaction instrument unimplemented |
| F-048 camera LLD | evidenced as documentation on the previous tree | Pass 3 found no new defect | implementing agent | Rendered DPR 1.25, tier B and browser GPU stay unperformed |
| F-X003 AC5 | implementation missing | The corpus harness has no Ocelli candidate frames | implementing agent, after review | Keep AC5 open |
| F-X003 AC6 | external observation unavailable | No collector and no representative host | a future representative host | Keep AC6 open |
| Browser colour and reduced-resolution canvas integration | implementation missing | The worker binding calls `draw_committed_mono` only | implementing agent, after review | Give those paths their own disposition |
| Registered interaction subjects | evidenced as a refusal | Runners throw `instrument is unavailable` and record no sample | implementing agent | Leave the instrument unimplemented |
| Release wasm size | evidenced as a measurement, budget unchanged | `2,043,442` bytes, ceiling `2,099,014` | implementing agent | Do not re-baseline from this repair |
| Instruction refinements | reviewed separately, 0 defects | `.claude/reviews/S16-instruction-refinement-pass-2.md` | Codex wrote them | Leave the canonical files and adapters unstaged |

## Resize commit point

`resizeCpuCanvas` checks that a canvas is bound, then asks the session to accept or refuse the size. The bitmap is unchanged when that call returns an error. A zero CSS width is `cpu raster extent is outside the pre-sized canvas`. The canvas width, the visible pixels, the retained publication and the settled flags stay where they were.

When the accepted device extent already matches the bitmap and the retained image, the binding does not assign canvas width or height. That assignment clears displayed pixels even when the numbers are unchanged. The repeated call therefore stays a real no-op. Refining stays clear. No kind 6 event is enqueued. The retained `ImageData` is not replaced.

When the device extent changes, the raster invalidates settlement first. The binding then assigns the new bitmap size and replaces the retained image. The next publication draws the current frame.

## RED, then GREEN, same inputs

Before the source change, `node /private/tmp/ocelli-pass3-resize.mjs` exited 1 against the dev module built from `f6057868`. Both workers had already shown opaque white `[255, 255, 255, 255]`.

Identical `resizeCpuCanvas(1, 1, 1)` returned success and the pixel became `[0, 0, 0, 0]`. Flags stayed 0. The following publish stayed transparent and emitted no event.

`resizeCpuCanvas(0, 1, 1)` returned `cpu raster extent is outside the pre-sized canvas` after the canvas width was already 0. The following publish left width 0 and an empty pixel read.

After the source change the dev module was rebuilt:

```bash
wasm-pack build crates/ocelli-wasm --dev --target web \
  --out-dir target/boundary-wasm
```

The same probe exited 0. `failed preservation checks` was 0. Both modes kept width 1, height 1 and `[255, 255, 255, 255]` after the resize and after the following publish. The rejected call still returned `cpu raster extent is outside the pre-sized canvas`. Chromium was headless with `--disable-gpu`. This is a tier C canvas observation.

## Permanent worker

`node packages/core/cpu-canvas.mjs` exited 0 on that rebuilt module. The script does not alias `CanvasRenderingContext2D`. `htmlContext` was `"undefined"`.

From the settled white frame, generation 1, kind 6:

- `resizeCpuCanvas(1, 1, 1)` returned an empty error. Pixels stayed `[255, 255, 255, 255]`. Extent stayed 1 by 1. Events were empty. The refining bit was clear. The following publish was idle and stayed white.
- `resizeCpuCanvas(0, 1, 1)` returned `cpu raster extent is outside the pre-sized canvas`. Pixels, extent, events and the refining bit were unchanged. The following publish was idle and stayed white.
- The next invert still published generation 2. The failed blit, the retry at generation 3, and the newer frame at generation 5 behaved as in pass 2.

The allocation worker, with constructor proxies installed before the glue import, stayed at one `ImageData` and zero `Uint8ClampedArray` constructions across the identical resize and the rejected resize. `resizeCpuCanvas(2, 1, 1)` raised `ImageData` constructions from 1 to 2. The following publish did not add another. The final read was `[0, 0, 0, 255]` on a 2 by 1 canvas because invert stayed on. A probe `new Uint8ClampedArray(4)` after that window incremented the clamped counter.

`npx vitest run --config vitest.boundary.config.ts` passed 5 tests on the same dev module. `bin/ocelli.sh cargo clippy -p ocelli-wasm --target wasm32-unknown-unknown --lib --no-deps -- -D warnings` exited 0.

## Release wasm size

`bin/ocelli.sh gate wasm` exited 0 on 2026-10-19. The size line was:

```text
wasm size 2,043,442 bytes, baseline 1,999,061, ceiling 2,099,014
```

The pin check printed `OK: wasm-bindgen, wgpu pinned exactly, wasm size within budget, package licences match`. `ci/wasm-size-budget.json` is unchanged. The pass 2 measurement of the previous tree was 2,042,672 bytes. The recorded F-045 module was 2,012,345 bytes. The parent tree was not rebuilt alone in this turn, so neither difference is an attribution. The budget was not moved. The release output is gitignored `crates/ocelli-wasm/pkg`. The worker proof used `target/boundary-wasm`.

## Still open

AC5 stays open. The synthetic `compare_view` call is not the applicable-corpus producer. `bin/ocelli.sh gate oracle` was not run.

AC6 stays open. No CPU-time or wall-time collector is configured, and no representative GPU-less incumbent host is available. No vCPU fraction and no changed-frame rate were recorded.

Browser colour and interactive reduced resolution are not wired through this canvas binding. The registered interaction subjects still refuse with `instrument is unavailable`. Rendered DPR 1.25, tier B and a browser GPU canvas were not run in this pass.

## Instruction refinements

The canonical refinements and the regenerated adapters stay unstaged. Their review is `.claude/reviews/S16-instruction-refinement-pass-2.md`, with 0 defects. The audit trigger for the next actual `AS_BUILT` entry stays in the gitignored progress notes. That entry is not written now.

## Handback

Please give the staged index a fresh independent Codex review before completion. Do not mark F-048, F-101 or F-X003 done. Do not commit, push, close S16, start S17, or record a verification receipt from this evidence alone.
