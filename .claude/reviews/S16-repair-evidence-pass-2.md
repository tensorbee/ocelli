# S16 repair evidence, pass 2

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree these repairs answer**: `8d0a94a34d4b35cd38e26c5056311eab05937231`
**Not a receipt**: `6b562be784f4365e7781869b5c511e9e0a1162a8`, `1f525c04`, `7de677af36b9122e60d2da4700c96b730f422e0a` and `8d0a94a34d4b35cd38e26c5056311eab05937231` do not cover the repaired tree. No sprint-profile receipt was recorded for this repair.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is `git write-tree` after this file is staged. It is reported with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-101 D4, stale allocation identity through the public adapters | evidenced | Boundary test on the dev module, below | implementing agent | Codex review of the staged tree |
| F-101 D5, metadata rejection and recovery on one live module | evidenced | Same boundary file | implementing agent | Codex review of the staged tree |
| F-101 D6, transferred frame with an explicit target | evidenced | Same boundary file, plus the unit handoff identity check | implementing agent | Codex review of the staged tree |
| F-101 D7, registered interaction subjects | evidenced as a refusal | Runners throw `instrument is unavailable` and record no sample | implementing agent | Leave the instrument unimplemented until the real endpoints exist |
| F-X003 D5, worker OffscreenCanvas presentation | evidenced | `node packages/core/cpu-canvas.mjs` on the dev module | implementing agent | Codex review of the staged tree |
| F-X003 D6, failed blit stays retryable | evidenced | Same worker script, one injected `putImageData` failure | implementing agent | Codex review of the staged tree |
| F-X003 D7, publication storage retained across draws | evidenced | Same script's allocation worker | implementing agent | The native clone counter is a separate regression |
| F-048 D2, GPU camera uniform record | evidenced as documentation | `docs/lld/render-graph.md` and `docs/lld/viewport.md` | implementing agent | Rendered DPR 1.25, tier B and browser GPU stay unperformed |
| F-048 native GPU CSS regressions | evidenced on the previous unsandboxed run | Pass 2 already re-ran both ignored tests and saw tier A | implementing agent | This continuation did not open an adapter |
| F-X003 AC5, one settled frame per applicable corpus row | implementation missing | The corpus harness has no Ocelli candidate frames | implementing agent, after review | Keep AC5 open |
| F-X003 AC6, representative-host cost | external observation unavailable | Host runners refuse. No collector and no incumbent host | a future representative host | Keep AC6 open |
| Browser colour and reduced-resolution canvas integration | implementation missing | The worker binding calls `draw_committed_mono` only | implementing agent, after review | Give those paths their own disposition |
| Release wasm size | evidenced as a measurement, budget unchanged | `2,042,672` bytes, ceiling `2,099,014` | implementing agent | Do not re-baseline from this repair |
| Instruction refinements | reviewed separately, 0 defects | `.claude/reviews/S16-instruction-refinement-pass-1.md` | Codex wrote them | Leave the nine files unstaged |

## F-101 public adapters

`sessionBulkSink` and `sessionCommandSink` keep the ticket captured at allocation. `commit_frame(address)` and `apply_commands(address)` return 0 and do not consume the slot. `commitAllocated`, `applyAllocated` and `releaseAllocated` present that ticket. A matching ticket drops the slot. A foreign ticket leaves it. Ticket `0` is never live.

`writeFrame` validates `FrameCommitMeta` before allocation when the sink has `commitAllocated`. A later write or commit exception calls `releaseAllocated` with the ticket captured for that allocation. `acceptTransferredFrame` takes the existing handoff metadata and a separate `FrameCommitMeta` target. The committed object is `{ handoff, viewport, columns, rows, generation }`. The byte length does not supply the grid.

Build used for the boundary file:

```bash
wasm-pack build crates/ocelli-wasm --dev --target web \
  --out-dir target/boundary-wasm
npx vitest run --config vitest.boundary.config.ts
```

On 2026-10-19 that file passed 5 tests. The dev module is `crates/ocelli-wasm/target/boundary-wasm`. Init in the boundary file is `glue.default({ module_or_path: bytes })`. The wasm session viewport is 1.

`keeps a reused address from authorizing the newest public ticket` allocates 4 bytes, sees `commit_frame(address)` return 0, commits with the saved ticket, allocates 4 bytes again at the same address, and shows the stale ticket leaving `[4, 3, 2, 1]` in place. The current ticket returns 1. The command sink does the same with a Zoom command on viewport 1.

`recovers from bad metadata and a write failure on one live module` rejects `viewport: "bad"` before a slot exists, leaves a held slot in place for ticket `0` and for a foreign ticket, releases the held ticket, then accepts a valid `writeFrame`. It then accepts `acceptTransferredFrame` of a 4-byte buffer with `{ series: 1, instance: 1, frame: 0, bytes: 4 }` and the same target. A `submitCommands` write that throws leaves `bulk_ticket()` at 0. The next valid submit on that same sink returns 1.

`npx vitest run` passed 103 tests in 10 files. `npx tsc --build` exited 0. Eslint on the touched TypeScript files exited 0. `npm run test` does not collect the boundary file. `coreAvailable()` still returns false.

`bin/ocelli.sh test ocelli-wasm` passed 25 tests, including `release_bulk_drops_only_the_current_ticket` and `a_deferred_presentation_stays_retryable_until_it_is_accepted`.

Native `bin/ocelli.sh clippy ocelli-wasm` and `bin/ocelli.sh clippy ocelli-render` exited 0. `bin/ocelli.sh cargo clippy -p ocelli-wasm --target wasm32-unknown-unknown --lib --no-deps -- -D warnings` exited 0. That wasm32 lint is the library only. A pre-existing `needless_return` in `crates/ocelli-render/src/tier_b.rs` appears when the wasm32 dependency tree is linted, and this repair did not edit that file.

## Worker publication

`packages/core/cpu-canvas.mjs` serves the same dev module to a Chromium module worker. The worker creates its own `OffscreenCanvas`. The script does not alias `CanvasRenderingContext2D`. The context feature is `OffscreenCanvasRenderingContext2d`. `publishCpuCanvas` blits with the retained `ImageData` and enqueues kind 6 only after `putImageData` returns.

Command and result on 2026-10-19, exit 0:

```bash
node packages/core/cpu-canvas.mjs
```

Chromium was headless with `--disable-gpu` and `--force-device-scale-factor=1`. Playwright 1.62.1 came from `tools/oracle/node_modules/playwright`. This run launched the browser in the default environment. It was not a Mach-port denial. It is a tier C canvas observation. It is not a GPU rendering observation and not a representative-host run.

The worker calls the named glue exports after `glue.default()`. Bulk bytes are written through `glue.__wasm.memory`. `copy_state` and `drain_events` are the JS wrappers, which return a `Uint8Array`.

Observed happy report, abbreviated to the asserted fields:

- `htmlContext` is `"undefined"`.
- First publish pixels `[255, 255, 255, 255]`, error empty, one kind 6 event, generation 1, width 1, height 1, refining bit clear. Identity LINEAR centre 40 width 1 maps stored 40 to display 255. That grey byte is the existing `apply_window` result. It is not a new threshold. The oracle producer uses width 400 and is a different frame.
- Idle publish adds no event and leaves the white pixel.
- `SetInvert(true)` then publish is `[0, 0, 0, 255]`, one kind 6 event, generation 2, invert bit set, refining bit clear.
- The `putImageData` patch is installed after those three publishes. One throw reports `canvas blit failed`, one put, no event, refining bit 8, and the previous black pixel left on the canvas. `JsValue::as_string` does not read `Error.message`, so the injected text is not the product string. The put counter shows the injection ran.
- The retry publishes one kind 6 event, generation 3, white pixel, refining bit clear, one put.
- A later inverse command whose blit throws publishes nothing. The following identity command publishes generation 5, which is the generation 3 settlement plus 2, with a white pixel and the refining bit clear. A stale retry of the failed inverse would have been generation 4.

Allocation worker, counted with proxies installed before the glue import:

- After bind, `Uint8ClampedArray` constructions 0 and `ImageData` constructions 1.
- After ordinary publish, invert publish and idle publish, the same counts.
- After `resizeCpuCanvas(2, 1, 1)`, `ImageData` constructions 2.
- The following publish keeps that count.
- A probe `new Uint8ClampedArray(4)` after the window increments the clamped counter by 1.
- Final `getImageData` is `[0, 0, 0, 255]` on a 2 by 1 canvas. Invert stays on through that resize publish, so the pixel is black.

The native test `draw_and_idle_do_not_clone_lut_buffers` also passed. It does not cover the browser publication objects.

`bin/ocelli.sh test ocelli-render --test cpu` passed 13 tests. The browser binding still calls `draw_committed_mono` only. Supported colour-to-canvas and interactive reduced-resolution integration are not in this worker path.

## Benchmarks

`render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` throw from `unavailableInstrument` and do not spawn `presented_frame`. `node --test tools/bench/tests/run_test.mjs` passed 13 tests. The window check shows a delay before the start, and a delay after the end, leave `end - start` unchanged, and a longer interior changes it. The resident-frame check is true when the keys differ and false when they match.

The native example remains a named diagnostic. This continuation did not run it and did not record a sample under those three subject ids. `ci/bench-baseline.json` was not created or edited.

`session.cpu_interactive`, `session.cpu_idle` and `cine.frame_change_rate` still execute the diagnostic workload and then refuse. The refusal names both gaps: no CPU-time or wall-time collector is configured, and no representative GPU-less incumbent host is available. No vCPU fraction and no changed-frame rate were recorded. AC6 stays open. AC5 stays open. The one synthetic `compare_view` call is not the applicable-corpus producer. `bin/ocelli.sh gate oracle` was not run.

## Camera uniform record

`docs/lld/render-graph.md` and `docs/lld/viewport.md` now describe the 80-byte `DeviceCamera` at group 0 binding 2. The first 64 bytes are the CSS matrix. The next four floats are scale, with `.x` the device pixel ratio. Registration, `set_view` and `rebind` own that buffer. Recovery drops the GPU buffer and keeps the CSS matrix and the ratio. `ViewportState` stays 96 bytes. Its camera field stays the 64-byte CSS matrix. State readback was not expanded.

This continuation did not re-run the ignored GPU tests. The pass 2 review already recorded both opening tier A outside the sandbox. A uniform-word check at DPR 1.25 is not a rendered 1.25 frame. Tier B and a browser GPU canvas were not run. A missing adapter is an environment result and is not a product pass.

## Release wasm size

`bin/ocelli.sh gate wasm` exited 0 on 2026-10-19. The size line was:

```text
wasm size 2,042,672 bytes, baseline 1,999,061, ceiling 2,099,014
```

The pin check printed `OK: wasm-bindgen, wgpu pinned exactly, wasm size within budget, package licences match`. `ci/wasm-size-budget.json` is unchanged. The recorded F-045 release module was 2,012,345 bytes. This build is 30,327 bytes above that recorded figure. The pass 1 measurement of the previous tree was 2,039,152 bytes. The parent tree was not rebuilt alone in this turn, so neither delta is an attribution. The budget was not moved. The release output is gitignored `crates/ocelli-wasm/pkg`. The worker proof used the dev `target/boundary-wasm` module and did not replace `pkg`.

## Instruction refinements

The nine unstaged instruction files stay unstaged. Their review is `.claude/reviews/S16-instruction-refinement-pass-1.md`, with 0 defects. `python3 scripts/sync_agent_skills.py --check` printed `OK: 21 adapters match their canonical sources`. The audit trigger for the next actual `AS_BUILT` entry stays in the gitignored progress notes. That entry is not written now.

`python3 scripts/prose_check.py` was clean over 429 tracked files before this file existed, and clean over 434 tracked files after the pass 2 reviews and this file were staged. `python3 scripts/unsafe_allowlist_check.py` was clean over 152 files, with 2 permitted, on both runs.

## Handback

Please give the staged index a fresh independent Codex review before completion. Do not mark F-048, F-101 or F-X003 done. Do not commit, push, close S16, start S17, or record a verification receipt from this evidence alone.
