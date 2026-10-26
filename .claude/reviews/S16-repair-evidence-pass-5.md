# S16 repair evidence, pass 5

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree this repair answers**: `d8ae7aa9c810e3aa1e619e67a96ca710a9df4502`
**Not a receipt**: `6b562be784f4365e7781869b5c511e9e0a1162a8`, `1f525c04`, `7de677af36b9122e60d2da4700c96b730f422e0a`, `8d0a94a34d4b35cd38e26c5056311eab05937231`, `f6057868244a71679ebfd95a17e440084699aa76`, `9ae40681bae54a69a21705d98a40a6337adce74d` and `d8ae7aa9c810e3aa1e619e67a96ca710a9df4502` do not cover the repaired tree. No sprint-profile receipt was recorded for this repair.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is `git write-tree` after this file is staged. It is reported with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

Pass 5 found D9 on `d8ae7aa9c810e3aa1e619e67a96ca710a9df4502`. Ordinary RGB publication on that tree stays. This pass repairs mode preservation. F-048 and F-101 were not reworked. D8 resize stays closed.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-X003 D9 mode preservation | repaired in this tree | Permanent mode worker, four scenarios | implementing agent | Codex review of the staged tree |
| F-X003 browser colour and reduced publication | retained | Existing colour worker, same run | implementing agent | Leave those controls in place |
| F-X003 D5 to D8 monochrome, blit and resize controls | retained | Same `cpu-canvas.mjs` run | implementing agent | Leave those controls in place |
| F-X003 AC5 | implementation missing | `tools/oracle/src/candidate.rs` is still one synthetic `mono16` frame | implementing agent | Keep AC5 open |
| Interaction instruments | implementation missing | `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` were not given a collector | implementing agent | Leave the instruments unimplemented and record no sample |
| F-X003 AC6 | external observation unavailable | No CPU-time or wall-time collector, and no representative GPU-less host | a future representative host | Keep AC6 open |
| F-101 public adapter repairs | unchanged from the reviewed tree | Boundary suite, 5 tests, on the rebuilt dev module | implementing agent | Leave the interaction instrument unimplemented |
| F-048 camera LLD | unchanged from the reviewed tree | No edit in this pass | implementing agent | Rendered DPR 1.25, tier B and browser GPU stay unperformed |
| Release wasm size | not measured in this pass | Pass 5 supplied no new measurement. The budget file is unchanged | implementing agent | Do not re-baseline from this repair |
| New design and microscope paragraphs | reviewed separately, 0 defects | `.claude/reviews/S16-instruction-refinement-pass-3.md`, left unstaged | implementing agent | Leave the canonical files and adapters unstaged |

## Repair

The approved plan stays `approved`. The new section is `Repair, 2026-10-19, mode preserved on refusal`.

Source length is accepted before a settled idle return. `ColorTransform::accept_source` is the length and 4:2:2 alignment rule `map_into` already used. The monochrome raster uses one stored sample per source pixel. The colour raster uses the resolved transform. A mismatch is `RasterError::Length` and writes nothing. A stale generation is still `RasterError::Stale`. A matching length still returns idle when that settled generation is current. The length check does not allocate colour scratch.

`select_supported_color(1)` accepts the code and does not remember RGB. `draw_committed_mono` and `draw_committed_rgb` remember their shade only after the draw returns Ok, including a valid idle of a compatible payload. A refusal leaves the previous shade in place. Codes other than 1 still return `cpu raster colour space is unavailable` before any draw.

`publishCpuInteraction` checks the remembered payload before `begin_cpu_interaction`. A length refusal does not bump the generation and does not set refining. The following valid interaction uses the preserved route. The permanent worker has no mode-reset call.

Palette, YBR and ICC stay on the native `raster_color_into` fixtures. No palette LUT, colour threshold or perceptual score was added. The resize section is unchanged.

## RED

Pass 5 ran `node /private/tmp/ocelli-pass5-colour.mjs` against the unrepaired dev module. The review records exit 1. This turn did not rebuild that old module and did not re-run the probe, so the log below is the retained RED record. `/private/tmp/ocelli-pass5-colour.log` reads:

```text
{"mode":"odd","firstError":"","interactionError":"","reducedMatches":true,"settledMatches":true}
{"mode":"mode","first":{"error":"","pixels":[255,255,255,255],"flags":0,"events":1},"incompatible":{"error":"","pixels":[255,255,255,255],"flags":0,"events":0},"interaction":{"error":"cpu raster source length does not match the grid","pixels":[255,255,255,255],"flags":8,"events":0}}
{"mode":"refusal","first":{"error":"","pixels":[255,255,255,255],"flags":0,"events":1},"incompatible":{"error":"cpu raster source length does not match the grid","pixels":[255,255,255,255],"flags":9,"events":0},"interaction":{"error":"cpu raster source length does not match the grid","pixels":[255,255,255,255],"flags":9,"events":0}}
failed preservation checks 2
```

The odd 7 by 5 control passed. The settled mono colour call returned success with no event and left white in place. The following interaction failed the length check, kept white, set refining and emitted no event. After SetInvert, the colour call returned the length error and the following interaction returned the same error. The invert was not drawn. Both inputs are synthetic.

## GREEN

The permanent worker now launches `/mode-worker.js` four times, each in a fresh wasm instance. Each rejection is followed by `publishCpuInteraction` and a full settle on the preserved route. There is no mode-reset helper.

| Scenario | Rejection | Interaction | Settle |
| --- | --- | --- | --- |
| settled mono | length error, white, flags 0, no event | generation 2, flags 8, white, event 1 by 1 | generation 2, flags 0, white |
| invalidated mono | length error, white, flags 9, no event | generation 3, flags 9, black `[0,0,0,255]` | generation 3, flags 1, black |
| settled RGB | length error, full RGBA, flags 0, no event, canvas 2 by 1 | generation 2, flags 8, reduced RGBA, event 1 by 1, canvas 2 by 1 | generation 2, flags 0, full RGBA |
| invalidated RGB | length error, full RGBA, flags 9, no event | generation 3, flags 9, reduced RGBA, canvas 2 by 1 | generation 3, flags 1, full RGBA |

Full RGBA is `10, 20, 30, 255` then `40, 50, 60, 255`. Reduced RGBA is `40, 50, 60, 255` on both device pixels. White is stored 40 through the identity monochrome window. Black is that frame after SetInvert. These literals were already fixed by the native fixture and were not copied from a new canvas read. The length string is `cpu raster source length does not match the grid`. `htmlContext` was `"undefined"`.

The same process kept the previous controls. Mono still published white at generation 1, kept it across identical and rejected resizes, inverted to black at generation 2, retried the failed blit at generation 3, and published the newer frame at generation 5. The allocation worker stayed at one `ImageData` until the real 2 by 1 resize, which rose from 1 to 2. The colour worker still settled generation 1, interacted at generation 2, failed a blit with no event, retried and settled at generation 5, idled, and kept the resize and unavailable-code controls.

## Checks

```bash
bin/ocelli.sh cargo fmt -p ocelli-pixel -p ocelli-render -p ocelli-wasm
bin/ocelli.sh test ocelli-wasm --lib
```

Exit 0. 28 passed, including `committed_rgb_replicates_the_reduced_sample_and_settles_latest`, `a_rejected_colour_request_keeps_the_monochrome_route` and `an_incompatible_interaction_does_not_bump_the_generation`. The RGB test rejects a monochrome request after settlement and again after SetInvert, then interacts and settles on the RGB route. The monochrome test does the opposite from both a settled frame and an invalidated frame. A later mono commit while RGB is remembered refuses interaction without moving generation 2, then the monochrome settle draws opaque white.

```bash
bin/ocelli.sh test ocelli-render --test cpu
```

Exit 0. 14 passed, including `settled_idle_rejects_a_source_that_does_not_match_the_layout`. A settled monochrome frame refuses a one-sample RGB call with `RasterError::Length` and still idles on the matching monochrome source. A settled RGB frame refuses the three-sample buffer as monochrome and still idles on the matching RGB source. The refused buffers stay at their filler bytes.

```bash
bin/ocelli.sh test ocelli-pixel --test color --test palette
```

Exit 0. `color` 14 passed. `palette` 7 passed. `map_into` still refuses a length mismatch before writing.

```bash
bin/ocelli.sh clippy ocelli-pixel
bin/ocelli.sh clippy ocelli-render
bin/ocelli.sh clippy ocelli-wasm
bin/ocelli.sh cargo clippy -p ocelli-wasm --target wasm32-unknown-unknown --lib --no-deps -- -D warnings
```

Each exited 0. The wasm32 run finished the dev profile in 0.10s with no diagnostic.

```bash
wasm-pack build crates/ocelli-wasm --dev --target web \
  --out-dir target/boundary-wasm
```

Exit 0. Done in 3.28s. The package is `crates/ocelli-wasm/target/boundary-wasm`. It does not replace gitignored `pkg`. This is a dev build. No release-size measurement was taken.

```bash
node packages/core/cpu-canvas.mjs
```

Exit 0 in headless Chromium with `--disable-gpu` and `--force-device-scale-factor=1`. The log is `/tmp/ocelli-d9-worker.log`. The assertions in the script passed, which is the GREEN record above.

```bash
npx vitest run --config vitest.boundary.config.ts
```

Exit 0. 5 passed. The log is `/tmp/ocelli-d9-boundary.log`.

```bash
python3 scripts/prose_check.py --staged
python3 scripts/unsafe_allowlist_check.py
python3 scripts/sync_agent_skills.py --check
```

Prose exited 0: `OK: voice rules clean over 58 file(s)`. The unsafe working-tree check exited 0: 152 files checked, 2 permitted, vendored audit surface unchanged (`openjph-core-0.1.0` 104, `ritk-codecs-0.6.0` 0). The `--staged` unsafe form reports those two vendored packages absent from the index. They are outside this repair. The working-tree check is the one that reads them. Skill sync exited 0: `OK: 21 adapters match their canonical sources`. The adapters were not regenerated in this turn.

## Still open

AC5 stays open. The applicable-corpus candidate producer is missing. The synthetic `compare_view` call is one `mono16` frame. `bin/ocelli.sh gate oracle` was not run. This is missing implementation.

The interaction instruments stay open as a separate gap. `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` were not modified. No sample was recorded under those subject ids. This is missing implementation.

AC6 stays open as a third gap. No CPU-time or wall-time collector is configured, and no representative GPU-less incumbent host is available. No vCPU fraction and no changed-frame rate were recorded. This is an external observation, not a refused sample.

Rendered DPR 1.25, tier B and a browser GPU canvas were not run in this pass. No new release-size number is claimed.

## Instruction paragraphs

The new cache-validity paragraphs in canonical `design.md` and `microscope.md` were reviewed by the implementing agent. The review is `.claude/reviews/S16-instruction-refinement-pass-3.md`. It reports 0 defects on those two paragraphs only. It stays unstaged with the instruction sources. Pass 2 does not cover these paragraphs. The D9 guidance trigger for the next actual `AS_BUILT` entry stays in the gitignored progress notes. That entry is not written now.

## Handback

Please give the staged index a fresh independent Codex review before completion. Do not mark F-048, F-101 or F-X003 done. Do not commit, push, close S16, start S17, or record a verification receipt from this evidence alone.
