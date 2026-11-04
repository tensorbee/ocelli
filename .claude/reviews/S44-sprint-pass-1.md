# S44 review, pass 1

**Reviewed**: staged tree `15a01c891f353a670c836f72dfce76979212aaab` on `sprint/s44`. That tree is HEAD `833e2c8` before this remediation.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 1 defect, 2 smells, 2 nitpicks

## Acceptance challenged

- On that tree the report spy pinned `readTid1500`, and the rendered TID line did not. `PAGE_MEASUREMENT` set both millimetre fields to 12.5.
- Full screen followed `document.fullscreenEnabled`. An immediate return left the word off.
- Preset, flip and colour map bytes were compared with `packCommands` for viewport 1.
- M19 stayed 11 stories and 43 weeks. F-202 to F-205 had an empty sprint. F-119 stayed in S45.

## Evidence

The reviewer read `/tmp/ocelli-s44-sprint.diff` and the example sources, then the exports in `packages/core`. The reviewer ran nothing. The host receipts on this tree were `gate --sprint` ALL GREEN for 31 gates, and compare gate at 102 views, 71 pass, 0 fail, 31 unmeasured, 0 absent, verdict pass.

## Defects

### D1, the page TID line matched the stored document

**Where**: `examples/viewer-react/src/measurement-panel.tsx` rendered `report.tid`, and `measurement-panel.test.tsx` expected 12.5.
**What**: Replacing that line with `report.stored` kept the panel test green. The report spy never rendered the panel.
**Why it is wrong**: The TID line has to be the number `readTid1500` returns. Copying the loaded document is a defect when the page test stays green.
**Evidence**: Read on tree `15a01c89`. The reviewer did not run the mutation.
**Scope and regression**: The measurement panel on that tree.

## Smells

### S1, three manipulation lines shared the value 1

**Where**: the quadrant, magnify scale and crop count lines in `manipulation-panel.tsx`, against one panel assertion of 1.
**Mechanism**: Rendering one of those fields in another of the lines stayed green. The report test pinned each field and did not pin which line the page read.
**Evidence**: Read on tree `15a01c89`.

### S2, several report fields matched one fixture constant

**Where**: `measurement-report.ts` undo, key image and touch. `manipulation-report.ts` centre, width, region width, quadrant, marker, scale pixels and crop count. `layout-report.ts` vertical.
**Mechanism**: Each field had one asserted input. A constant of that input passed every test on that tree.
**Evidence**: Read on tree `15a01c89`. The reviewer did not run the mutation.

## Nitpicks

1. `.claude/plans/F-200-design.md` said the public API already recorded `packCommands` and `describeError`. On the parent of this sprint those names were not in `docs/lld/public-api.md`. The F-200 paragraph added them. The clinical-standard sentence is the doc comment in `packages/core/src/viewport.ts`, not the LLD.
2. The manipulation panel test built some expected text by calling `manipulationReport`. The report test still pins those values.

## Verified clean

- The diff touched no crate. It added no `unsafe` and no `wasm-bindgen`.
- The named tolerances were unchanged.
- The page did not draw a volume, an MPR, a slab or a fusion, and it did not add a writer or parse DICOM tags.
- `App.tsx` added only the three panels.
- Accepted leftovers from F-199, F-200, F-201 and S43 were still present and were not reopened.

## Limitations and handoff

The reviewer ran no test and did not open the live page. PS3.3 was not opened. The host remediates D1, S1 and S2 on the next tree, then a new pass reviews that tree.
