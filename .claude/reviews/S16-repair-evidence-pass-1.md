# S16 repair evidence, pass 1

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree that these repairs answer**: `7de677af36b9122e60d2da4700c96b730f422e0a`
**Not a receipt**: `6b562be784f4365e7781869b5c511e9e0a1162a8` and `1f525c04` do not cover this work. No sprint-profile receipt was recorded for `7de677af` and none is recorded here.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is `git write-tree` after this file is staged. It is reported with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-101 allocation refusal stops before a write | evidenced | Dev wasm boundary test below | implementing agent | Codex review of the staged tree |
| F-101 same-address reuse needs a new ticket | evidenced | Same boundary test. Heap `alloc(4)` reused the address and the stale ticket returned 0 | implementing agent | Codex review of the staged tree |
| F-101 public metadata object against the numeric export | evidenced | `writeFrame` and `submitCommands` through the real adapters on that module | implementing agent | Codex review of the staged tree |
| F-048 device-to-CSS on the GPU consumer, native tier A | evidenced | Ignored view tests opened tier A and passed | implementing agent | Codex review of the staged tree |
| F-048 tier B, browser canvas, oracle frames | external observation unavailable | Not run | a later host that opens tier B, a browser, or the oracle | Keep the rows open |
| F-X003 command, source and resize invalidation | evidenced | Permanent session and CPU regressions on this working tree in the repair turn | implementing agent | Codex review of the staged tree |
| F-X003 source colour larger than the target | evidenced | CPU colour regression | implementing agent | Codex review of the staged tree |
| F-X003 no presentation clone on the draw | evidenced | `LutDescriptor` clone counter around the session draw | implementing agent | Codex review of the staged tree |
| CPU canvas blit in a browser worker | external observation unavailable | Exports exist. Node has no `OffscreenCanvas` | a browser worker | Observe bind, blit, kind 6, growth and RAF |
| Bounded synthetic comparator call | evidenced | `candidate.rs` calls `compare_view` for one synthetic `mono16` frame | implementing agent | Codex review of the staged tree |
| F-X003 AC5, one settled frame per applicable corpus row | implementation missing | The corpus harness was not invoked and has no Ocelli candidate frames | implementing agent, after review | Do not treat the synthetic call as AC5 |
| F-X003 AC5 tier A side | external observation unavailable | No tier A candidate frames were produced | a host that opens an adapter for the corpus path | A missing adapter is not a pass |
| F-101 tier C command-to-kind-6 medians | evidenced as observations only | Release example lines quoted below | implementing agent | Do not write them into the baseline |
| F-101 tiers A and B for those three subjects | external observation unavailable | The example does not open a GPU | a later tier A or B instrument | An unavailable tier is not a sample |
| F-X003 interactive, idle and cine host figures | external observation unavailable | Workload ran, then the process printed `refused` and no value | a future representative CPU-baseline host | AC6 stays unmet |
| Representative-host acceptance | external observation unavailable | No incumbent baseline and no threshold were created | that same future host | Leave the acceptance open |
| Release wasm size | evidenced as a measurement, budget unchanged | `2,039,152` bytes, ceiling `2,099,014` | implementing agent | Do not re-baseline from this repair |
| Instruction refinements | reviewed separately, 0 defects | `.claude/reviews/S16-instruction-refinement-pass-1.md` | Codex wrote them. This agent reviewed the diff | Leave the nine files unstaged |

## F-101 boundary

Build:

```bash
wasm-pack build crates/ocelli-wasm --dev --target web \
  --out-dir target/boundary-wasm
npx vitest run --config vitest.boundary.config.ts
```

On 2026-10-19 that vitest file passed 3 tests. Init was `glue.default({ module_or_path: bytes })`. The wasm session viewport is 1.

The file checks address 0 before any write, an occupied slot, ticket `0`, the 64 MiB limit and the inclusive 64 MiB commit, empty `alloc(0)` address sharing with two tickets, heap `alloc(4)` reuse after release, a stale ticket leaving the new bytes, and the current ticket returning 1. `writeFrame` accepts a 4-byte metadata commit, grows the memory, then accepts another small write. `submitCommands` of Zoom returns 1. A second submit while a slot is held does not call `write`.

`npx tsc --build` exited 0 after `packages/core/src/node-test-modules.d.ts` grew the node imports this file uses. Eslint on the touched TypeScript files exited 0. `npm run test` does not collect this file. The dev module is not the release-size measurement and not a browser observation. `coreAvailable()` still returns false.

## F-048 GPU consumer

`bin/ocelli.sh test ocelli-render -- --ignored --test-threads=1 --nocapture css`

Both ignored tests printed `auto opened tier A` and passed:

- `device_fragments_are_divided_by_dpr_before_the_css_camera`
- `set_view_dirties_only_the_camera_and_keeps_the_css_matrix`

The pixel test requires DPR 2 to read `[128, 128, 128, 255, 128, 128, 128, 255]`, DPR 1 to read `[128, 128, 128, 255, 255, 255, 255, 255]`, and the translated border to read `[0, 0, 0, 255]`.

The earlier unsandboxed probe is `/private/tmp/ocelli-s16-gpu-dpr-probe-unsandboxed.log`. It opened tier A and failed. It expected the DPR 2 pair of 128 and observed 128 then 255.

DPR 1.25 is the uniform-word test `the_camera_uniform_carries_dpr_and_the_shader_divides_first` from the earlier `ocelli-render` suite. That test checks the uniform words and that the shader divides before the CSS multiply. It is not a rendered 1.25 frame. Tier B, a browser canvas and the oracle were not run.

## F-X003 raster and session

The repair turn's `bin/ocelli.sh test ocelli-render -- --test-threads=1` passed the CPU regressions, including a source larger than the target and a DPR change that keeps the device extent. The wasm suite passed the command invalidation, the refused-batch control, and the clone counter. This continuation did not edit those test bodies and did not repeat the full suites.

`tools/oracle/src/candidate.rs` rasters stored 40 through LINEAR centre 40 and width 400 and calls `compare_view`. The hand grey byte is 127. The repair-turn test `the_settled_linear_frame_is_compared_not_assumed` passed. Stored 40 was `Outcome::Pass`. Stored 0 was `Outcome::Fail`. The corpus candidate remains a directory. `bin/ocelli.sh gate oracle` was not run.

## CPU canvas

`bindCpuCanvas` and `publishCpuCanvas` are exported. The node boundary test asserts those names and asserts that `cpu_frame`, `stored_samples`, `pixels`, `getPixels` and `readPixels` are absent. `publish_cpu_canvas` blits with `putImageData` and publishes kind 6 only after that blit returns. A failed blit returns before publish. No browser exercised that blit.

## Benchmarks

Release build:

```bash
bin/ocelli.sh cargo build --release -p ocelli-wasm --example presented_frame
```

Direct stdout, each with exit 0:

```text
first {"value":0.001333,"kind":6,"tier":"C","iterations":11}
window {"value":0.0022500000000000003,"kind":6,"tier":"C","iterations":11}
scroll {"value":0.000834,"kind":6,"tier":"C","iterations":11}
interactive {"refused":"host","reason":"no representative GPU-less incumbent host. No vCPU fraction and no changed-frame rate were recorded."}
idle {"refused":"host","reason":"no representative GPU-less incumbent host. No vCPU fraction and no changed-frame rate were recorded."}
cine {"refused":"host","reason":"no representative GPU-less incumbent host. No vCPU fraction and no changed-frame rate were recorded."}
```

A later call of the six runners with `build: false` returned the window and scroll values above, returned `0.0012920000000000002` for `render.first_frame`, and threw `host measurement is unavailable` for the three host subjects. No value was returned for those three. `ci/bench-baseline.json` is unchanged. These milliseconds are not an acceptance threshold. Tiers A and B have no sample. AC6 stays unmet.

## Release wasm size

`bin/ocelli.sh gate wasm` exited 0 on 2026-10-19. The size line was:

```text
wasm size 2,039,152 bytes, baseline 1,999,061, ceiling 2,099,014
```

The pin check printed `OK: wasm-bindgen, wgpu pinned exactly, wasm size within budget, package licences match`. `ci/wasm-size-budget.json` is unchanged. The recorded F-045 release module was 2,012,345 bytes. This build is 26,807 bytes above that recorded figure. The parent tree was not rebuilt in this turn, so the delta is not an attribution. The budget was not moved.

## Instruction refinements

The nine unstaged instruction files were reviewed on their own in `.claude/reviews/S16-instruction-refinement-pass-1.md`. Result: 0 defects, 0 smells, 0 nitpicks. `python3 scripts/sync_agent_skills.py --check` printed `OK: 21 adapters match their canonical sources`. Those nine files stay unstaged. The audit trigger for the next actual `AS_BUILT` entry is in the gitignored progress notes. That entry is not written now.

## Handback

Please give the staged index a fresh independent Codex review before completion. Do not mark F-048, F-101 or F-X003 done. Do not commit, push, close S16, start S17, or record a verification receipt from this evidence alone.
