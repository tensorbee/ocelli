# S46 review, pass 2

**Reviewed**: the staged pass 1 remediation on sprint/s46 a48a9172, staged tree 54cfb3c07bfc.
**Author / reviewer**: remediation by GLM with coordinator adaptations. Review by Claude, a read-only subagent that ran nothing, composed by the Claude coordinator.
**Result**: 2 defects, 3 smells, 4 nitpicks

## Acceptance challenged

- **Claim.** Pass 1's three viewer repairs keep F-227's one-window rule and the last choice wins, in every path.
- **Combination not in the controls.** A scroll during the series-window wait in tile mode, and a manual window change after a slow preset.
- **Result.** Both break the claim, below.

## Defects

**D1** `examples/folder-viewer/src/page.html` showIndex and pumpMain. In tile mode on a series that stores no window, an ArrowDown during the series-window wait leaves the tiles blank. showIndex skips paintTiles while the series window is wanted, the post-wait check continues because the index moved, and the next iteration no longer wants the series window, so paintTiles is never reached.

**D2** `page.html` chooseWindow's choice counter fences only later presets. The W/L drag commit, resetView and copyWindow set the window directly without bumping it, so a slow preset trial still overwrites a later manual, reset or linked window and clears the draft.

## Smells

**S1** `crates/ocelli-codec/src/jpegxl.rs`. The RGB encoding rule is correct and the corpus RGB rows are very likely sRGB, but the unit cases call the helper directly. Removing its call in validate_header leaves them green. An integration case through the decoder is needed.

**S2** The new window controls detect on fixed waits, and the out-of-order control does not assert which request it held. Correct behaviour passes reliably, the defect could pass unseen on a slow host.

**S3** Pass 1's carried smells and nitpicks exist only in the pass 1 review. They must be filed before close, and pass 1 S3, averaging stored values across differing rescales in the volume pyramid, as a precondition of F-120.

## Nitpicks

N1 the start-of-loop and in-block key captures are overwritten by the recapture before the fetch, so the comment overstates them. N2 the new auto.mjs function sits under the previous function's JSDoc. N3 unwrapped long lines in CLAUDE.md. N4 contributor order differs between public-api.md and the LLD index.

## Verified clean

No endless loop, because the key changes only on finite state changes. Tile mode without a pending series window paints immediately. Colour images keep the stable colour key. The tile control fails on the original page. The docs claims match the tree. eslint.config.js only adds a global.
