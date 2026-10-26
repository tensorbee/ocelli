# S16 repair evidence, pass 4

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree this repair answers**: `9ae40681bae54a69a21705d98a40a6337adce74d`
**Not a receipt**: `6b562be784f4365e7781869b5c511e9e0a1162a8`, `1f525c04`, `7de677af36b9122e60d2da4700c96b730f422e0a`, `8d0a94a34d4b35cd38e26c5056311eab05937231`, `f6057868244a71679ebfd95a17e440084699aa76` and `9ae40681bae54a69a21705d98a40a6337adce74d` do not cover the repaired tree. No sprint-profile receipt was recorded for this repair.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is `git write-tree` after this file is staged. It is reported with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

Pass 4 closed the resize finding on `9ae40681bae54a69a21705d98a40a6337adce74d` with 0 defects. That finding stays closed. F-048 and F-101 were not reworked. This pass wires browser colour and reduced-resolution publication through the wasm worker.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-X003 browser colour publication | evidenced on the rebuilt dev module | `publishCpuColor` in the permanent colour worker | implementing agent | Codex review of the staged tree |
| F-X003 reduced interaction, then latest full settle | evidenced | Same worker, generations 2 and 5 | implementing agent | Codex review of the staged tree |
| F-X003 failed blit, retry, identical resize, rejected resize | evidenced | Same worker | implementing agent | Codex review of the staged tree |
| F-X003 D5 to D8 monochrome controls | evidenced | Same `cpu-canvas.mjs` run, mono worker and allocation worker | implementing agent | Leave those controls in place |
| F-X003 AC5 | implementation missing | `tools/oracle/src/candidate.rs` is still one synthetic `mono16` frame | implementing agent | Keep AC5 open |
| Interaction instruments | implementation missing | `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` were not given a collector | implementing agent | Leave the instruments unimplemented and record no sample |
| F-X003 AC6 | external observation unavailable | No CPU-time or wall-time collector, and no representative GPU-less host | a future representative host | Keep AC6 open |
| F-101 public adapter repairs | evidenced on the rebuilt module | Boundary suite, 5 tests | implementing agent | Leave the interaction instrument unimplemented |
| F-048 camera LLD | unchanged from the reviewed tree | Pass 4 found no new defect in that scope | implementing agent | Rendered DPR 1.25, tier B and browser GPU stay unperformed |
| Release wasm size | evidenced as a measurement, budget unchanged | `2,049,647` bytes, ceiling `2,099,014` | implementing agent | Do not re-baseline from this repair |
| Instruction refinements | reviewed separately, 0 defects | `.claude/reviews/S16-instruction-refinement-pass-2.md` | Codex wrote them | Leave the canonical files and adapters unstaged |

## Colour entry

The approved plan stays `approved`. The new section is `Repair, 2026-10-19, colour and reduced resolution`.

`publishCpuColor(1)` remembers interleaved native RGB and draws the committed samples at the full device extent for the current generation. `CpuRaster::raster_rgb_into` resolves that space once per call, before the pixel loop. `reserve_source` runs before the draw. Photometric code 0 and every other code return `cpu raster colour space is unavailable`. They do not draw, blit or enqueue kind 6.

`publishCpuCanvas` still forces the monochrome path. `publishCpuInteraction` uses the remembered shade at `max(1, device / 2)` on each axis. It begins a generation, draws `Interacting`, and publishes kind 6 only after `putImageData` returns. It does not accept the presentation, so refining stays set and the generation is not marked settled. A failed blit publishes nothing.

The retained `ImageData` stays the bound device size. Interaction does not assign canvas width or height. Kind 6 carries the drawn extent. A presentation buffer allocated at bind, and again only when the device byte count changes, receives integer cell replication of the drawn prefix. A full-device draw is the identity case. Bytes past the drawn prefix are zeroed so they are not a second sample. There is no pixel getter and no colour threshold. Palette, YBR and ICC stay on the native `raster_color_into` fixtures.

## Expected pixels

The fixture is stored interleaved RGB `10, 20, 30, 40, 50, 60`, source 2 by 1, device 2 by 1, identity view, dpr 1. The unfitted origin is half a CSS pixel. The raster adds that half back, so the image coordinate equals CSS. Full columns sample CSS 0.5 and 1.5, which are source columns 0 and 1. The one reduced centre samples CSS 1.0, which is source column 1. Exact channels in `0..=255` stay those bytes. Alpha is 255.

Full settle is therefore `10, 20, 30, 255` then `40, 50, 60, 255`. The reduced sample is `40, 50, 60, 255`. Cell replication paints both device pixels with that sample. These literals are written in `session.rs` and in `packages/core/cpu-canvas.mjs`. They were not copied from a canvas read.

Generation arithmetic for the colour worker, from the bump points:

1. The commit requests generation 1. Colour settle does not bump. The event is generation 1, extent 2 by 1, flags 0.
2. Interaction begins generation 2. The event is extent 1 by 1, flags 8. The canvas element stays 2 by 1.
3. The failed interaction begins generation 3 and publishes nothing. Flags stay 8. The displayed pixels stay the reduced colour.
4. SetInvert begins generation 4. Colour does not read the monochrome LUT.
5. The retry begins generation 5 and publishes extent 1 by 1 with flags 9. The pixels stay `40, 50, 60, 255` on both columns.
6. Settle uses generation 5 at extent 2 by 1. Accept clears refining and leaves the invert bit. Flags are 1. The pixels are the full pair above.
7. A second `publishCpuColor(1)` is idle. Identical `resizeCpuCanvas(2, 1, 1)` keeps the pixels, the 2 by 1 canvas and the retained image. `resizeCpuCanvas(0, 1, 1)` returns `cpu raster extent is outside the pre-sized canvas` and keeps that frame. `publishCpuColor(0)` returns the unavailable string and keeps that frame.

## Checks

Native session test, after `bin/ocelli.sh cargo fmt -p ocelli-wasm -p ocelli-render`:

```bash
bin/ocelli.sh test ocelli-wasm --lib
```

Exit 0. 26 passed, including `committed_rgb_replicates_the_reduced_sample_and_settles_latest`. The reduced draw buffer is `40, 50, 60, 255, 0, 0, 0, 0`. The presented buffer is `40, 50, 60, 255` twice. A settled call that still names generation 1 returns `RasterError::Stale` and leaves both buffers unchanged. The following settle of generation 2 restores the full pair. The colour scratch allocation stays 1 after the first reservation. The presentation buffer capacity stays the same across the interaction and the second settle.

```bash
bin/ocelli.sh test ocelli-render --test cpu
```

Exit 0. 13 passed. The existing monochrome, palette, YBR and ICC fixtures were not edited.

```bash
bin/ocelli.sh clippy ocelli-wasm
bin/ocelli.sh clippy ocelli-render
bin/ocelli.sh cargo clippy -p ocelli-wasm --target wasm32-unknown-unknown --lib --no-deps -- -D warnings
```

Each exited 0.

Dev module used by the worker:

```bash
wasm-pack build crates/ocelli-wasm --dev --target web \
  --out-dir target/boundary-wasm
```

Exit 0. The package is `crates/ocelli-wasm/target/boundary-wasm`. It does not replace gitignored `pkg`.

```bash
node packages/core/cpu-canvas.mjs
```

Exit 0 in headless Chromium with `--disable-gpu` and `--force-device-scale-factor=1`. `htmlContext` was `"undefined"` for the mono worker and the colour worker. The script does not alias `CanvasRenderingContext2D`.

The mono worker still published white at generation 1, kept that pixel across identical and rejected resizes, inverted to black at generation 2, retried the failed blit at generation 3, and published the newer frame at generation 5. The allocation worker stayed at one `ImageData` across identical and rejected resizes, then rose from 1 to 2 on the real 2 by 1 resize. The following publish added none. `counterSeesClamped` was true.

The colour worker, with constructor proxies installed before the glue import, matched the pixels and generations above. `publishCpuColor(1)`, interaction, the failed blit, the retry, the later settle, both resizes and `publishCpuColor(0)` each added 0 `ImageData` and 0 `Uint8ClampedArray` during the wasm call. Bind added one `ImageData`. Successful blits incremented the patched `putImageData` counter by 1. The idle settle, both resizes and the unavailable code incremented it by 0. The failed blit incremented it by 1 and left the displayed reduced pixels in place. These constructor counts do not prove the absence of every native allocation. The native counter covers the colour scratch and the presentation capacity.

```bash
npx vitest run --config vitest.boundary.config.ts
```

Exit 0. 5 passed.

## Release wasm size

`bin/ocelli.sh gate wasm` exited 0 on 2026-10-19. The size line was:

```text
wasm size 2,049,647 bytes, baseline 1,999,061, ceiling 2,099,014
```

The pin check printed `OK: wasm-bindgen, wgpu pinned exactly, wasm size within budget, package licences match`. `ci/wasm-size-budget.json` is unchanged. The resize-repair measurement of the previous staged tree was 2,043,442 bytes. The recorded F-045 module was 2,012,345 bytes. That previous tree was not rebuilt alone in this turn, so the difference is not an attribution. The budget was not moved. The release output is gitignored `crates/ocelli-wasm/pkg`.

## Still open

AC5 stays open. The applicable-corpus candidate producer is missing. The synthetic `compare_view` call is one `mono16` frame. `bin/ocelli.sh gate oracle` was not run. This is missing implementation.

The interaction instruments stay open as a separate gap. `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` were not modified. No sample was recorded under those subject ids. This is missing implementation.

AC6 stays open as a third gap. No CPU-time or wall-time collector is configured, and no representative GPU-less incumbent host is available. No vCPU fraction and no changed-frame rate were recorded. This is an external observation, not a refused sample.

Rendered DPR 1.25, tier B and a browser GPU canvas were not run in this pass.

## Instruction refinements

The canonical refinements and the regenerated adapters stay unstaged. Their review is `.claude/reviews/S16-instruction-refinement-pass-2.md`, with 0 defects. The audit trigger for the next actual `AS_BUILT` entry stays in the gitignored progress notes. That entry is not written now.

## Handback

Please give the staged index a fresh independent Codex review before completion. Do not mark F-048, F-101 or F-X003 done. Do not commit, push, close S16, start S17, or record a verification receipt from this evidence alone.
