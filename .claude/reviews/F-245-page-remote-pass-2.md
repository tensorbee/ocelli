# F-245 page remote slice review, pass 2

**Reviewed**: unstaged worker files in /private/tmp/ocelli-s47-f245-glm, the same hashes before and after the pass.
- `examples/folder-viewer/src/page.html` sha256 dbf54b6dbf88dd8b4aa4fb28d50335340255bf8be3341fc52a829b2c8d80d38d, diffed against `claude-page-before-review-repair.html`.
- `examples/folder-viewer/tests/browser/dicomweb.mjs` sha256 645a96cc7b2db766bb930f05fe59111e901b523953e90860aae783f4c6e6a0e4, diffed against `claude-browser-suite-before-repair.mjs`.
- `examples/folder-viewer/tests/preview.rs` sha256 52a974d44a5dcc7d506174570608a5eaccd116ff6ca4eb16132dd4507b7d8415, needle line 820 only.

**Author / reviewer**: GLM authored the slice. Codex coordinated the repairs, including the restoration of the general `.segment` rule. Reviewer Claude (root independent reviewer, subagent), who wrote none of the repairs. No worker file was edited. Mutants ran on copies under /private/tmp/claude-501/f245r.

**Result**: 0 defects, 0 smells, 3 nitpicks.

## Acceptance challenged

- Pass 1 D1 to D4 and S1, S2, N2 were each replayed against their original trigger.
- The local viewport was measured with the panel open at 1, 20 and 60 rows, and with the inspector collapsed.
- Segment radii were measured in the default local-only launch and the configured launch, against the original pre-slice baseline.
- AC3 page mutants were rerun through the repaired suite.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `CARGO_BUILD_JOBS=2 cargo test -p ocelli-folder-viewer` | worker tree, hashes above | executed now | every result `ok`, 382 passed, 0 failed |
| `node examples/folder-viewer/tests/browser/dicomweb.mjs` | release binary built 21:58:53, after the page mtime of 21:58:40 | executed now | exit 0, `ok` |
| `p2-probe.mjs` static computed style and layout | `claude-page-before-remote-a.html` against the current page, 1280x800 | executed now | exit 0, see Verified clean |
| `suite2.mjs` copy of the repaired suite, page mutants through `page.route`, CSP removed, Chromium local network checks disabled | real viewer, synthetic upstream | executed now | control `same` ok (102 page and 8 upstream requests). `relabel` killed. `ingrid` (`position: static`) killed on all three wrap checks. `checks` (`resetChecks()`) ok, survives. `cache` (`dropCache()`) ok, survives |
| `claude-browser-guard-mutants.json`, `claude-browser-flow-run-v7.log` | coordinator, same hashes | prior receipt | grid-flow mutant exit 1, malformed B exit 1 at the length check, restored `ok` |

## Defects

None.

## Smells

None.

## Nitpicks

- N1, `page.html:110-113`: 64-character UIDs have no break opportunity, so the 358 px panel scrolls sideways (scrollWidth 490). Setting `overflow-wrap: anywhere` on the row buttons would keep them inside the panel.
- N2, `page.html:110`: `#remote` at z-index 8 sits above `#modal` at 7, so with the panel open the headers sheet opens beneath it.
- N3, carried from pass 1 N1: `preview.rs:815-826` still checks only that strings are present. The browser suite covers the behaviour.

## Verified clean

- **D1 closed.** `#wrap` is 848x609 with the panel hidden and stays 848x609 at 1, 20 and 60 rows. The panel is 360 px wide at the top right and stops growing at 660 px, then scrolls inside itself (scrollHeight 1489). The body scroll height stays 800. At 360 and 320 px viewports the panel stays 12 px inside each edge. The panel now covers part of the right side while open, and only when the user opens it. The real-viewer wrap guard kills `position: static`.
- **D2 closed.** Default launch: Synthetic `6px 0 0 6px`, Folder `0 6px 6px 0`, the same as the original baseline. Configured launch: Folder `0`, DICOMweb `0 6px 6px 0`, with border-left 0. The expanded state draws an inset 1 px outline. The size row is identical to the baseline (S `6px 0 0 6px`, M, L and XL `0 6px 6px 0`).
- **D3 closed.** The header now names the SOP Instance UID contradiction and says registration is pending.
- **D4 closed.** Buffer A is accepted and buffer B is refused. The suite now requires B to be the same length as A and to differ only inside the two SOP Instance UID values. That leaves identity as the only difference available to cause the refusal. The upstream GET for B is asserted.
- **S1 closed.** There is one construction site (`page.html:1453`). The promise is memoised synchronously before any await, so concurrent callers share one provider. A rejected or null promise stays cached, which matches the `loadLibrary` caching.
- **S2 closed.** There is no `.active` toggle on the button, and `markSource` is untouched. `aria-expanded` follows `#remote.hidden`, and `aria-controls="remote"` is set.
- **N2 closed.** Provider creation is inside `try` and falls back to the library sentence.
- Isolation, `textContent` rendering, the code-only error copy and the race reasoning from pass 1 are unchanged by the repair diff.

## Limitations and handoff

- **AC3 marks, SR documents, F225 checks and cache remain uncovered.** The `resetChecks()` and `dropCache()` mutants still pass the suite. These belong to the next slice.
- The AC3 pending stage is not sampled. AC6 tier independence, the AC7 More matrix row and docs, and `viewer-browser` registration in `bin/ocelli.sh` (owned by another story's shared slice) are still open.
- The page mutants ran without CSP and with local network checks disabled. The control mutant passed under the same conditions.
- This is a clean bounded pass on this slice. It is not acceptance of F-245.
