# F-236 handoff

**Branch**: `work/f-236-xai`
**Base**: `282b9e8206ed33b33d78406da8a50c61ade80c82`
**Head**: `2541d9a77fd55d3d9e45fabb81b75c948b5ddec5`
**Files touched**: `.claude/plans/F-236-design.md .claude/reviews/F-236-working-pass-1.md .claude/reviews/F-236-working-pass-2.md .claude/reviews/F-236-working-pass-3.md .claude/reviews/F-236-working-pass-4.md docs/lld/public-api.md docs/lld/typescript-packaging.md packages/core/src/boundary.wasm.test.ts packages/core/src/bulk.test.ts packages/core/src/bulk.ts`
**Review**: `.claude/reviews/F-236-working-pass-4.md`
**Verify tree**: `d9134e54c6d21d22f2b67b739e8e97d453eab6b6`

The Head is the preparation commit, whose tree is the Verify tree, recorded under the feature profile with floor (28 gates) and corpus green. This handoff is committed after it, alone, because a file cannot name the commit that holds it. Pass 4 reviewed the code at `f559ed8d`, and the preparation commit after it changes only the two LLD files and the plan's evidence section.

Owed receipts, all recorded in `.claude/plans/F-236-design.md` under "Acceptance evidence at preparation":

- AC8. `bin/ocelli.sh gate wasm` on `git archive` copies of base and tip, each with its own `CARGO_TARGET_DIR`. `pkg/ocelli_wasm_bg.wasm` is 2,202,907 bytes on both and `pkg-decode/ocelli_wasm_bg.wasm` is 16,455 bytes on both, each with an identical md5, so the delta is 0. Both runs ALL GREEN, budgets unedited, `--accept-size` not run.
- AC9. `packages/core/dist/bulk.js` is 15,592 bytes on base and 15,672 on tip, +80. The doc comment grows by 216 bytes and the `committed` literal shrinks by 136, so the growth is comment only.
- AC6. Base `bulk.ts` under the tip tests: unit 6 red of 35, boundary 2 red of 19 (AC5 and AC7). Tip `bulk.ts` restored in the same tree: 35 of 35 and 19 of 19.

Left for the lead at integration: the `CHANGELOG.md` entry, AS_BUILT, the tracker, the backlog row and the `docs/lld/README.md` index row, whose contributor column for both LLD files does not yet name F-236.
