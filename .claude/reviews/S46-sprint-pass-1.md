# S46 review, pass 1

**Reviewed**: sprint/s46 at a48a9172, tree 87b959607d75, the tree the sprint profile passed on before this pass.
**Author / reviewer**: implementation by the sprint hosts and GLM. Review by Claude in three read-only slices, viewer interactions, Rust crates and the pixel path, and delivery records with user-facing claims, composed by the Claude coordinator, which also found D3 while writing a regression test.
**Result**: 5 defects, 9 smells, 7 nitpicks

## Acceptance challenged

- **Claim.** F-227 says an image is never drawn at two windows, and F-213's corner names the window that is drawn.
- **Combination.** F-208's pump, F-213's presets, F-209's window link and F-227's series window, with a request still in flight.
- **Result.** Three cross-story races broke the claim. D1, D2 and D3 below each now have a real-browser control that fails on the original page and passes on the repair.

## Defects

**D1** `examples/folder-viewer/src/page.html` pumpMain. `moved()` ignored the window cache key, so a preset, link, invert or drag change on the same image while a request was in flight painted the old window's reply under the new label. Control `observeHeldLoad` in `tests/browser/window.mjs`: the original page shows the Bone label over WINDOW1's 128 and 0, and the repair shows Bone's 83 and 0 (PS3.3 C.11.2.1.2.1 gold in the suite).

**D2** `page.html` showIndex painted tiles before the F-227 series-window wait, so tile mode drew each image at its own window first. The first repair exposed a regression, because the key check after the wait always fired once the series header changed, leaving tile mode blank. The final repair checks token, index and image after the wait and recaptures the key. Control `observeTilesBeforeSeriesWindow` in `tests/browser/auto.mjs` fails on the original page with several windows, fails on the first repair with blank tiles, and passes on the final page with one automatic window.

**D3** `page.html` chooseWindow committed whichever trial fetch resolved, so a superseded preset could win. Found by the coordinator. A per-viewport choice counter now guards the commit. Control `observeOutOfOrderPresets` ends on Lung on the original page and on Bone after the repair.

**D4** `CLAUDE.md` said the oracle still had no Ocelli renderer to compare against, contradicted by the F-X021 native candidate. Corrected in three places.

**D5** `docs/lld/README.md` index rows omitted contributors their files name, F-238 on tier resolution and F-225 on the public API. Corrected, and the oracle row now covers the candidate side.

## Smells

**S1** Window link copies the file-window index rather than the numbers, weaker than cornerstone3D 5.8.2's numeric VOI sync. A documented F-209 design choice, so changing it is a design decision. Carried as a follow-up for S47 planning under the cornerstone-floor rule.

**S2** A changed file's pooled series window stays in force. Dropping it would recompute from a file F-224 now refuses, so the correct behaviour is not settled. Follow-up.

**S3** `crates/ocelli-volume/src/levels.rs` averages stored values across slices that may carry different rescales. Latent until F-120 consumes the pyramid. Follow-up, to refuse or record before F-120.

**S4** `crates/ocelli-codec/src/jpegxl.rs` accepted RGB encodings it does not report, linear or gamma transfer, non-sRGB primaries or a non-D65 white point, and drew them as device RGB. Repaired: RGB now requires D65, sRGB primaries and the sRGB transfer, grey keeps its behaviour, with five unit cases. Codec tests 241 pass, clippy 0.

**S5** `examples/folder-viewer/src/preview.rs` per-image auto window allocates a full frame outside the counted resident term. Needs a measurement at maximum frame size. Follow-up.

**S6** CURRENT_SPRINT's F-223 carry reason was unclear and partly stale. Rewritten in plain words.

**S7** CURRENT_SPRINT said F-227 AC18 was still to run. Corrected, the sprint profile with oracle passed.

**S8** `docs/benchmarks/cornerstone-comparison.md` publishes ratios without saying it is the synthetic controlled observation of 2026-11-09. It is generated output, so the fix belongs to its generator, which F-273 owns in S47 together with strict publication.

**S9** `more.html` called linked series Implemented while the index and section said Partial. Corrected to Partial.

## Nitpicks

N1 a linked region window keeps its region label, N2 the region tool is silent on colour, N3 the palette ICC arm repeats a lookup, N4 ContentId trusts the caller's sample shape, N5 duplicated bullets in the generated comparison page, N6 the oracle LLD row description, N7 a count in prose in the F-238 entry. N6 is corrected with D5. The rest are carried.

## Preflight findings

29 completed stories had no review record in run state although their review files exist. An independent audit named each final pass, all clean, and they are recorded. F-239's bounded series are recorded at its latest library pass with that qualification. 25 S46 worker worktrees remain. Each HEAD is pinned under `refs/archive/s46/` with its tracked diff and untracked files archived. Their removal was refused by the session's permission classifier and is left to the operator.

## Evidence

Remediation receipts are under the private evidence root `claude-s46-review-remediation`. A new sprint profile and comparison run on the remediated tree is required before pass 2.
