# F-101 review, pass 5

**Reviewed**: staged tree `1d7a0fce852ec1a61ccb01f1c9acd8eafe57be4d`. The pass pinned `4e50c8f5dac90205a75056b664e9ca8e2d04775b` first, then the supplied tree moved to `d07f566f6423c60499605a546803326383fb44fa`, `edc78a490226e1eb251551e8ab91f1f0e9489ecd` and `1d7a0fce852ec1a61ccb01f1c9acd8eafe57be4d`. The F-101 product material is byte-identical across `d07f566f`, `edc78a49` and `1d7a0fce`. The unstaged `.claude/reviews/F-X023-instruction-pass-4.md` (`AM` in `git status`) is outside F-101 and is reported separately below.

**Author / reviewer**: Implementer recorded as Codex in `.claude/reviews/S16-codex-completion-evidence.md`. Reviewer is DeepSeek V4.1 Flash. I did not author the implementation, the design plan, the remediation or the prior reviews. The remediation was authored by another agent. Independence is per changed material. I reviewed the remediation as an independent pass.

**Result**: 1 defect, 0 smells, 1 nitpick

**Date**: 2026-10-26

## Previous findings

D1 (reduced-frame replication), S1 (`writeStack` wire layout) and S2 (`configure_stack` empty-sequence refusal) are closed. Each prior trigger now goes red on this tree. N1 (the `vitest.config.ts` excluded-suite command), N2 (the `typescript-packaging.md` third-allowance prediction), N3 (the F-101 export list) and N4 (`PRESENTED_RUNNERS`) are closed. N5 (`StackViewport.error` has no setter) is a handoff to F-103 and was not remediated. It remains a nitpick and is not an F-101 defect.

The delta also carries material that the remediation summary did not name: the boundary suite test `keeps a copied state and ring readable across a later memory growth` and the `drain_events` binding in `packages/core/src/boundary.wasm.test.ts`. I reviewed it as additional F-101 material. It is a correct test, with the nitpick below.

## Acceptance challenged

**Reduced-frame presentation.** The test binds a 4x1 device, commits a four-cell RGB frame with distinct values `10,20,30`, `40,50,60`, `70,80,90` and `100,110,120`, draws it at the full device extent, then begins an interaction and draws a 1x1 mono frame from stored samples `10,20,30,40`. The presented buffer is asserted to be 16 bytes and each 4-byte cell to equal the drawn prefix. The seed is genuinely nonuniform: the test asserts cell 0 differs from cell 1 and cell 2 differs from cell 3, and the seeded presented buffer is 16 bytes. The `while x < device[0]` to `while x < drawn[0]` mutation leaves cells 1 to 3 as the nonuniform seeded values and goes red. The `replicate_cells(device, drawn, ...)` argument swap writes only cell 0 and goes red. A `replicate_cells` mutation that writes opaque black to every cell goes red on the seed nonuniformity, so the test as a whole cannot be satisfied by an all-black presented buffer. The drawn prefix happens to be opaque black because the identity chain's window is centred at 40 with width 1, and the 1x1 draw samples a stored value below that window. A reduced-path-only opaque-black fill would still satisfy the assertion, but that is a contrived implementation rather than a plausible one.

**Stack wire layout.** The new `writeStack` test installs two keys with distinct series, instance and frame values and asserts the exact 32 little-endian bytes and the returned epoch. Swapping the instance and frame offsets in `bulk.ts` moves `9` to byte 12 and `4` to byte 8 and goes red. The test's fake sink reads the bytes after the write and before `configure_stack` returns, so it cannot pass against a wrong layout.

**Empty-stack refusal.** `malformed_stack_and_foreign_tickets_preserve_the_installed_sequence` now allocates a zero-length slot and calls `configure_stack(zero_ticket, 0)` on a fresh slot, asserting `Err(SessionError::Length)`, then asserts the installed epoch and the displayed key are unchanged. Removing the `len == 0` arm from `configure_stack` returns `Ok(2)` instead of `Err(Length)` and goes red on that assertion.

**Copy-out across memory growth.** The new boundary test seeds a frame, copies the 96-byte state and the ring out, grows wasm memory by writing a 1 MiB frame, and asserts the memory grew, the copied state is still 96 bytes with unchanged content, and the copied ring is unchanged. A view over wasm memory would detach on growth and `Array.from` would throw, so the content equality is the discriminator. The buffer identity assertions are weaker, see N1.

**Claims that did discriminate.** The loop-bound mutation, the argument swap, the `writeStack` offset swap and the removed `len == 0` arm all went red. The opaque-black mutation also went red. The baseline was re-run green after every revert.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` at pin | index before the pass | executed now | `4e50c8f5dac90205a75056b664e9ca8e2d04775b` matched the supplied tree |
| `git write-tree` at review | index after the final move | executed now | `1d7a0fce852ec1a61ccb01f1c9acd8eafe57be4d` matched the supplied tree |
| `bin/ocelli.sh test ocelli-wasm --lib` | `crates/ocelli-wasm` | executed now | exit 0, 31 passed, 0 failed |
| `bin/ocelli.sh test ocelli-viewport` | `crates/ocelli-viewport` | executed now | exit 0, 47 passed across the unit and integration suites |
| `npx vitest run packages/core/src/bulk.test.ts packages/core/src/ring.test.ts packages/core/src/commands.test.ts` | TS core | executed now | exit 0, 37 passed in 3 files |
| `npm run lint` | package | executed now | exit 0 |
| `npm run typecheck` | package | executed now | exit 0 |
| `node --test tools/bench/tests/run_test.mjs` | bench node suite | executed now | exit 0, 14 passed, 1 skipped because no browser was requested |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | F-101 dev module | executed now | exit 0 |
| `npx vitest run --config vitest.boundary.config.ts` | the built dev module | executed now | exit 0, 7 passed |
| `bin/ocelli.sh clippy ocelli-wasm` | crate | executed now | exit 0 |
| `bin/ocelli.sh gate unsafe` | staged tree | executed now | exit 0, no unsafe outside the allow-list, 155 files checked, 2 permitted |
| `bin/ocelli.sh gate bindgen` | staged tree | executed now | exit 0, `wasm-bindgen` confined to `ocelli-wasm` |
| `bin/ocelli.sh gate bench` | staged tree | executed now | exit 0, 12 subjects and 5 recorded baselines |
| Mutation, `while x < device[0]` to `while x < drawn[0]` | `crates/ocelli-wasm/src/session.rs`, `replicate_cells` | executed now | `ocelli-wasm` exit 101, red on `a_reduced_presentation_replicates_the_drawn_prefix_across_the_device`, reverted, `git diff` empty |
| Mutation, swap `writeStack` instance and frame offsets | `packages/core/src/bulk.ts:139-140` | executed now | `bulk.test.ts` exit 1, red, reverted, `git diff` empty |
| Mutation, remove the `len == 0` arm | `crates/ocelli-wasm/src/session.rs:232` | executed now | `ocelli-wasm` exit 101, red with `left: Ok(2)` against `right: Err(Length)`, reverted, `git diff` empty |
| Mutation, swap `replicate_cells` arguments | `crates/ocelli-wasm/src/session.rs:830` | executed now | `ocelli-wasm` exit 101, red, reverted, `git diff` empty |
| Mutation, write opaque black in `replicate_cells` | `crates/ocelli-wasm/src/session.rs:946` | executed now | `ocelli-wasm` exit 101, red on the seed nonuniformity and on the committed-RGB test, reverted, `git diff` empty |
| Baseline re-run after the reverts | `ocelli-wasm` and `bulk.test.ts` | executed now | exit 0, 31 passed and 21 passed |
| `git diff --quiet` after each revert | `session.rs` and `bulk.ts` | executed now | clean |
| Prior receipt, pass 4 | tree `118d12e9d25b931458df3c2b89d704e26d830852` | prior receipt, not this tree | 1 defect, 2 smells, 5 nitpicks |

## Defects

### D1, `build-targets.md` still says the module exports six functions

**Where**: `docs/lld/build-targets.md:298` ("The module exports six functions of ours"), against the F-101 paragraph at `docs/lld/build-targets.md:88-93`.

**What**: The doc says the module exports six functions. The generated glue `crates/ocelli-wasm/target/boundary-wasm/ocelli_wasm.js` exports 21 functions. The F-101 paragraph in the same doc says F-101 adds 15 exports. The module therefore exports 21, not six. The sentence is false and contradicts the F-101 paragraph in the same file.

**Why it is wrong**: The export inventory is part of the F-101 deliverable. A reader using this doc as a checklist would miss the 15 F-101 exports, including `bulk_ticket`, `release_bulk`, `configure_stack`, `commit_selected_frame` and the five CPU canvas exports. The microscope's rule 3 says every factual sentence is checkable. The severity table says a claim in prose that is false is a defect. The remediation made the F-101 paragraph complete but left the global count stale, so the document now states two different export counts.

**Evidence**: `grep -c "^export function" crates/ocelli-wasm/target/boundary-wasm/ocelli_wasm.js` reports 21. The names are `alloc`, `apply_commands`, `attachOffscreenCanvas`, `bindCpuCanvas`, `bulk_ticket`, `commit_frame`, `commit_selected_frame`, `configure_stack`, `copy_state`, `drain_events`, `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr`, `publishCpuCanvas`, `publishCpuColor`, `publishCpuInteraction`, `release_bulk`, `resizeCpuCanvas`, `resizeOffscreenCanvas` and `ring_capacity`. The F-101 paragraph lists 15 of these as F-101 additions. `docs/lld/build-targets.md:298` says six.

**Scope and regression**: Present in the reviewed tree `1d7a0fce` and in `118d12e9`. The sentence predates F-101 and was already false after the F-101 exports landed. The smallest fix is to change it to "The module exports twenty-one functions of ours. The panic probe table below names six of them", or to qualify the table as a subset. The affected consumers are readers of the LLD and anyone using the export section as an inventory. The smallest meaningful regression is a doc consistency check that compares the stated count with the glue's `export function` count. Uncertainty: the sentence may be intended as an introduction to the six-row panic probe table, but its wording is a global claim and the F-101 paragraph now makes the contradiction explicit.

## Smells

None.

## Nitpicks

### N1, the new boundary test's buffer identity assertion does not discriminate a view

**Where**: `packages/core/src/boundary.wasm.test.ts:212` and `:215`, with the comment at `:209-211`.

**What**: The test asserts `state.buffer` and `ring.buffer` are not `wasm.memory.buffer` after the growth. After growth, `wasm.memory.buffer` is a new `ArrayBuffer`. A view returned by `copy_state` or `drain_events` would hold the old, detached buffer, which is also not the new `wasm.memory.buffer`. So the identity assertion passes for both a copy and a view. The discriminating assertion is `expect(Array.from(state)).toEqual(stateBefore)` at `:214` and the ring equivalent at `:217`, which throws on a detached view. The test is still correct because of the content equality, but the identity assertion and the comment overstate what that line proves.

**Why it matters**: A reader could mistake the identity assertion for the copy-out guard. The content equality is the guard. This is a nitpick and does not change the verdict.

**Evidence**: The test passes. I did not run a view-returning mutation. The reasoning is from the detach semantics of a grown wasm memory.

## Verified clean

- The three prior findings D1, S1 and S2 are closed with mutations that now go red. The smallest meaningful regression for D1 is the 4x1 device with a 1x1 drawn prefix and four distinct seeded cells, which the new test supplies.
- The new reduced-presentation test is a correct test. The seed is nonuniform, the presented buffer is asserted to be 16 bytes, and each cell is compared with the drawn prefix. The loop-bound, argument-swap and opaque-black mutations all go red. The seeded frame is not all black, and the seed uses the same replication function, so an all-black presented buffer fails on the seed assertion.
- The new `writeStack` test asserts the exact 32 bytes and the epoch, and the instance and frame swap goes red. The test's fake sink reads the bytes after the write.
- The zero-length `configure_stack` case is now exercised, and the removed arm goes red. The test also asserts the installed epoch and the displayed key are preserved.
- The `PRESENTED_RUNNERS` array is a flat list of ids. The test imports each runner and asserts `loaded.id` equals the id and `loaded.run` is a function. The test passes with 14 passed and 1 skipped in the no-browser run.
- `vitest.config.ts` now names `npx vitest run --config vitest.boundary.config.ts` and states that the default config excludes the file. The named command exits 0 with 7 passed. The comment is true.
- `docs/lld/typescript-packaging.md` records the copy-out decision. `packages/core/src/ring.ts` reads a copied buffer and builds a `DataView` over that copy, never over wasm linear memory. The prose is true.
- `docs/lld/build-targets.md` F-101 paragraph is true for the 15 exports. The generated glue matches the list. There is no `ring_ptr` and no pixel getter, and the boundary suite checks the banned getters.
- The new boundary test `keeps a copied state and ring readable across a later memory growth` is a correct test. The copied state is 96 bytes, the ring is at least 16 bytes, and the content equality after growth catches a detached view. The buffer identity assertions are weak, see N1.
- `docs/lld/README.md` adds F-048 to the `render-graph.md` row. `render-graph.md:3` lists F-048 as a contributor, so the README row matches the doc's own header. The row is consistent.
- `docs/sprints/CURRENT_SPRINT.md` changes the second count command from `'^| F-'` to `'^| F-[0-9X]'`. The old command counts 6 rows including the `| F-ID |` header, the new command counts 5 story rows. The change is correct.
- No gate was disabled or weakened, no tolerance changed and no baseline widened. `vitest.config.ts` keeps `passWithNoTests: false`. The bench gate passes with the recorded baselines unchanged.
- `gate unsafe` and `gate bindgen` exit 0. There is no `unsafe` outside `crates/ocelli-wasm/src/ring.rs` and `crates/ocelli-core/src/cast.rs`, and `wasm-bindgen` is confined to `ocelli-wasm`.
- No patient data, DICOM corpus file or build artefact was staged. The only binary in the tree is the gitignored `target/` output from the boundary build.
- The F-101 product material is byte-identical across `d07f566f`, `edc78a49` and `1d7a0fce`. The later deltas are F-X023 metadata and sprint ledger changes.
- N5 remains a handoff to F-103. The `error` flag bit is still never set, and F-101 does not claim it.

## Limitations and handoff

The pass reviewed the F-101 material on `1d7a0fce`. The later deltas after `d07f566f` are outside F-101. I did not review the F-X023 review files, the F-X023 completion metadata or the sprint ledger changes except for the `docs/lld/README.md` F-048 row and the `CURRENT_SPRINT.md` count command.

The supplied statement that the worktree is clean did not hold at the time I checked. `git status --porcelain` showed `AM .claude/reviews/F-X023-instruction-pass-4.md`, a staged addition with an unstaged modification. This is outside F-101 and is not part of the reviewed tree `1d7a0fce`, which is the index. A later reader should treat the unstaged F-X023 path separately.

The pass did not run `gate --sprint`, the oracle, GPU tests, `#[ignore]` tests or a release-size measurement. Those remain for the sprint gate. The CPU raster arithmetic is F-X003's and is not re-reviewed here. The boundary crossing and the copy-out are F-101's.

The result is 1 defect, so F-101 stays in progress. The remediation must update the `build-targets.md` export count, then a fresh independent pass must review the new tree. The N1 nitpick does not block.

No commit, push, tag or verification receipt was created. No product file was edited and no file outside this review was written. The mutations were reverted from copies made before editing, not with `git checkout`, and `git diff` was empty after each revert.

This report is a new staged addition. At report time `git write-tree` is the tree that contains this report, which is not the reviewed tree `1d7a0fce852ec1a61ccb01f1c9acd8eafe57be4d`. The reviewed tree's receipt must not be attached to the union that contains this report.
