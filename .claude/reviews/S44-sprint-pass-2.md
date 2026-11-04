# S44 review, pass 2

**Reviewed**: staged tree `0ed6f19ea10c81e2732934e8f7333bd894a155ec`. That is HEAD `833e2c8` plus the pass 1 remediation.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 2 smells, 2 nitpicks

## Acceptance challenged

- Pass 1 D1 is closed. The panel spy replaces the TID read-back with 3 and expects that line to read 3 while the stored line stays 12.5.
- Pass 1 S2 is closed. Each named field has a second input or a refusal. The reviewer derived the new numbers from the core functions.
- Pass 1 S1 is only partly closed. Quadrant is separated. Magnify scale and crop count still match on both clicks.
- The eight host mutations still have a test that goes red. The reviewer did not re-run them.

## Evidence

The reviewer read the pass 1 file, the remediation diff, the example reports and panels, and the core functions those reports call. The reviewer ran nothing. The host receipts on this tree were `gate --sprint` ALL GREEN for 31 gates, and compare gate at 102 views, 71 pass, 0 fail, 31 unmeasured, 0 absent, verdict pass.

## Defects

None.

## Smells

### S1, magnify scale and crop count still share a value on both clicks

**Where**: `manipulation-panel.test.tsx` expects 1 and 1, then 2 and 2.
**What**: The magnify line can render the crop count, and the crop line can render the magnify scale, and both clicks stay green.
**Why it is wrong**: Pass 1 asked those page lines to be separable. The AS_BUILT sentence that says scale 2 and two planes separate them is false for this pair.
**Evidence**: Read on tree `0ed6f19e`. The host mutation of the quadrant line does not cover this pair.

### S2, a huge crop count builds every plane before the refusal

**Where**: `copiedPlanes` in `manipulation-report.ts` pushes `count` planes, then `cropPlanes` refuses more than six.
**What**: The page passes `Number` of a free-text box. `1e9` is an integer, so the loop runs before the core refusal.
**Why it is wrong**: The tab hangs or runs out of memory instead of showing a refused crop. The test count of 7 cannot see it.
**Evidence**: Read on tree `0ed6f19e`. The reviewer did not run it.

## Nitpicks

1. An empty rotation box and an empty crop box become 0. An empty magnify box stays null on purpose.
2. Two AS_BUILT sentences credit the read-only review with adding the vertical assertion and the three inputs. The host did both after the review.

Pass 1 nitpicks about the design sentence and the panel test calling `manipulationReport` stay as they were.

## Verified clean

- The remediation diff touches no Rust file, no `unsafe` and no `wasm-bindgen`.
- The CHANGELOG sentence and the public-api defaults are true on this tree.
- The F-201 panel spy, the key image spy, the touch spy, the two-plane crop case and the quadrant swap are present.
- The page still draws no volume, MPR, slab or fusion.
- M19, F-202 to F-205 and publication are unchanged.

## Limitations and handoff

The reviewer ran no test and did not open the live page. The host should make the second click use different magnify and crop values, correct the ledger sentence, and stop the plane loop at seven so the core still refuses. A new pass reviews that tree.
