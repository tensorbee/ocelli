# F-101 boundary review, pass 4

**Reviewed**: staged tree `118d12e9d25b931458df3c2b89d704e26d830852`
**Pinned first**: `b71910494b2474338073d0f3ed122077700f3b14`
**Author / reviewer**: Implementer recorded as Codex in `.claude/reviews/S16-codex-completion-evidence.md`. Reviewer is DeepSeek V4.1 Flash. I did not author the implementation, the design plan or the prior reviews.
**Result**: 1 defect, 2 smells, 5 nitpicks
**Date**: 2026-10-26

## Previous findings

Pass 3 reported 0 defects, 0 smells and 0 nitpicks in its scoped repairs against tree `f6057868244a71679ebfd95a17e440084699aa76`. That receipt does not cover this tree. The pass 1 and pass 2 findings D1 to D7 are closed in the inspected paths. The stack extension and the CPU presentation path are new work after pass 3, so this pass reviews them as new material rather than as reopened findings.

The pinned tree carried stale ring-producer prose in `packages/core/src/ring.ts:14` and `:17`, in `packages/core/src/ring.test.ts:32-37` and in `docs/lld/errors.md:418-423` and `:510-512`. The reviewed tree corrects all of it. `ring.ts` now names `crates/ocelli-viewport/src/ring.rs`, `ring.test.ts` names the producer test that pins the layout and `errors.md` no longer claims the producer does not exist. I verified the correction and did not count it as a finding.

## Acceptance challenged

**Reduced-frame presentation.** The boundary contract says a reduced interactive draw publishes a device-sized presentation buffer by integer cell coverage. I derived a case the current fixtures do not express. A 4x1 device with a 1x1 drawn prefix and a nonuniform source must produce four copies of the prefix sample. `record_presentation` zeroes the tail of `cpu_rgba` but not `cpu_present`, so a replication that stops at the drawn width leaves the previous frame's bytes in the device tail. The native suite draws a 1x1 device with a 1x1 drawn extent, which is the full-frame path. The manual `cpu-canvas.mjs` harness does assert the full 2x1 reduced pixels, but its fixture uses a full frame of `[10,20,30,255, 40,50,60,255]` and a reduced sample of `40,50,60`, so the stale second pixel equals the replicated value and the mutation is invisible by construction. The browser benchmark's interactive subject measures CPU time and does not assert the presented pixels. The mutation `while x < device[0]` to `while x < drawn[0]` therefore passes the native suite and the browser suite. This rejects the claim that the reduced path is covered.

**Stack wire layout.** The operator-approved extension says `writeStack` writes each key as little-endian u64 series, u32 instance and u32 frame. I derived a case with distinct instance and frame values. The boundary fixture uses `instance: 1` for both keys and `frame: 7` and `3`, and it checks stack length and selection index rather than the installed key fields. Swapping the instance and frame offsets in `writeStack` leaves the boundary suite green. This rejects the claim that the stack wire layout is covered.

**Empty-stack refusal.** `docs/lld/public-api.md:92-93` says an empty sequence is refused before replacing the viewport stack. I removed the `len == 0` arm from `configure_stack` and the native suite stayed green. The claim is true of the code and false of the evidence, because no test passes a zero length. This rejects the claim that malformed stack input is covered by the named regression.

**Claims that did discriminate.** Removing the epoch equality in `commit_selected_frame` turned `ocelli-wasm` red. Swapping the centre and width offsets in `commands.rs` turned `ocelli-viewport` red. Changing `EVENT_BYTES` from 48 to 56 turned the ring test red. Replacing `device[0]` with the literal `1` in `replicate_cells` turned the native suite red, which shows the full-frame path is covered even though the reduced path is not.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` at pin | index before the pass | executed now | `b71910494b2474338073d0f3ed122077700f3b14` matched the supplied tree |
| `git write-tree` when the product review closed | index after concurrent writes, before this report was staged | executed now | `118d12e9d25b931458df3c2b89d704e26d830852`, the reviewed tree |
| `bin/ocelli.sh test ocelli-viewport` | `crates/ocelli-viewport` | executed now | exit 0, 47 passed |
| `bin/ocelli.sh test ocelli-wasm` | `crates/ocelli-wasm` | executed now | exit 0, 30 passed, 0 failed |
| `bin/ocelli.sh gate unsafe` | staged tree | executed now | exit 0, no new unsafe site |
| `bin/ocelli.sh gate bindgen` | staged tree | executed now | exit 0, `wasm-bindgen` only in `ocelli-wasm` |
| `bin/ocelli.sh gate bench` | staged tree | executed now | exit 0 |
| `npm run test` | default vitest config | executed now | exit 0, 104 passed in 10 files |
| `npm run typecheck` | package | executed now | exit 0 |
| `npm run lint` | package | executed now | exit 0 |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` then `npx vitest run --config vitest.boundary.config.ts` | built dev module | executed now | exit 0, 6 passed |
| `OCELLI_BENCH_BROWSER=1 node --test tools/bench/tests/run_test.mjs` | real Chromium worker instrument | executed now | exit 0, 15 passed, 0 skipped |
| `node packages/core/cpu-canvas.mjs` | manual Chromium harness | executed now | exit 0 |
| `bin/ocelli.sh wasm` | release wasm build and size gate | executed now | exit 0 |
| Mutation, swap centre and width state offsets | `crates/ocelli-viewport/src/commands.rs` | executed now | `ocelli-viewport` exit 101, red, reverted, `git diff` empty |
| Mutation, `EVENT_BYTES` 48 to 56 | `crates/ocelli-viewport/src/ring.rs` | executed now | ring test exit 101, red, reverted, `git diff` empty |
| Mutation, remove epoch equality | `crates/ocelli-wasm/src/session.rs` | executed now | `ocelli-wasm` exit 101, red, reverted, `git diff` empty |
| Mutation, swap `replicate_cells` arguments | `crates/ocelli-wasm/src/session.rs` | executed now | native 30 passed, green, reverted, `git diff` empty |
| Mutation, `while x < device[0]` to `while x < drawn[0]` | `crates/ocelli-wasm/src/session.rs:936` | executed now | native 30 passed and browser 15 passed, green, reverted, `git diff` empty |
| Mutation, `while x < device[0]` to `while x < 1` | `crates/ocelli-wasm/src/session.rs:936` | executed now | native exit 101, red, reverted, `git diff` empty |
| Mutation, swap `writeStack` instance and frame offsets | `packages/core/src/bulk.ts:139-140` | executed now | boundary suite 6 passed, green, reverted, `git diff` empty |
| Mutation, remove `configure_stack` `len == 0` | `crates/ocelli-wasm/src/session.rs:232` | executed now | native 30 passed, green, reverted, `git diff` empty |
| `npx vitest run packages/core/src/boundary.wasm.test.ts` | excluded file under the default config | executed now | exit 1, "No test files found" |
| Prior receipt, native session, boundary, browser, TS, unsafe, tier A, DPR and skill checks | `.claude/reviews/S16-codex-completion-evidence.md` | prior receipt, implementation evidence | native 30, boundary 6 in dev, release and SIMD variants, browser 15, TS 104, unsafe 4, tier A 1, DPR 1.25 1, oracle and clippy, 21 skill adapters |

## Defects

### D1, reduced-frame replication is not discriminated

**Where**: `crates/ocelli-wasm/src/session.rs:820-830` (`record_presentation`) and `:926-953` (`replicate_cells`), with the loop bound `while x < device[0]` at `:936`.

**What**: The presentation path copies the drawn prefix across the device-sized buffer by integer cell coverage. No test discriminates the reduced path. The native suite draws a 1x1 device with a 1x1 drawn extent, which is the full-frame path. The manual `cpu-canvas.mjs` harness asserts the full 2x1 reduced pixels, but its fixture uses a full frame of `[10,20,30,255, 40,50,60,255]` and a reduced sample of `40,50,60`, so the stale second pixel equals the replicated value and the mutation is invisible by construction. The browser benchmark's interactive subject measures CPU time and does not assert the presented pixels. Swapping the `replicate_cells` arguments leaves the native suite green. Replacing `device[0]` with `drawn[0]` leaves the native 30 and the browser 15 green. Replacing it with the literal `1` goes red, which shows only the full-frame path is covered.

**Why it is wrong**: `record_presentation` zeroes the tail of `cpu_rgba` but not `cpu_present`, so a short replication leaves the previous frame's pixels in the presented buffer. F-X003's design says the retained `ImageData` stays the bound device size and the canvas blits the device-sized buffer, with replication onto the canvas rather than a second resampling. The boundary's `record_presentation` is the step that must fill that buffer. The microscope rule 2 says a test that passes both ways is a defect because it will be counted as coverage forever.

**Evidence**: Mutation `while x < device[0]` to `while x < drawn[0]` at `session.rs:936`, then `bin/ocelli.sh test ocelli-wasm` exit 0 with 30 passed and `OCELLI_BENCH_BROWSER=1 node --test tools/bench/tests/run_test.mjs` exit 0 with 15 passed. Mutation `replicate_cells(drawn, device, ...)` to `replicate_cells(device, drawn, ...)` also stayed green on the native suite. Reverted after each run, `git diff` empty, baseline re-run 30 passed. The literal `1` mutation went red with exit 101.

**Scope and regression**: Present in the reviewed tree `118d12e9` and in the pinned tree `b7191049`. Affected consumers are the CPU canvas blit, the real worker benchmark's `session.cpu_interactive` and `session.cpu_idle` subjects, and any reduced interactive draw on tier C. The smallest meaningful regression is a 4x1 device with a 1x1 drawn prefix and two distinct source cells, asserting all four presented pixels. A 2x1 fixture is not enough because the stale second pixel can equal the replicated value. The CPU raster arithmetic itself is F-X003's and is not re-reviewed here.

## Smells

### S1, `writeStack` wire layout is not discriminated

**Where**: `packages/core/src/bulk.ts:132-150`, instance at `:139`, frame at `:140`.

**What**: `writeStack` writes series at `0..8`, instance at `8..12` and frame at `12..16`, little-endian. The boundary suite exercises `writeStack` but checks stack length and selection index, not the installed key fields. Swapping the instance and frame offsets leaves the boundary suite 6 green. `bulk.test.ts` has no `writeStack` case.

**Why it is wrong**: The wire layout is the contract between the shell and `configure_stack`. A swapped instance and frame would install a different stack order and a different selected frame, with no test failure. The code is correct now, so this is a smell rather than a defect. The design plan's stack extension names the layout and requires malformed-input regressions.

**Evidence**: Mutation of `view.setUint32(i * 16 + 8, ...)` and `:12`, then `npx vitest run --config vitest.boundary.config.ts` exit 0 with 6 passed. Reverted, `git diff` empty.

**Scope and regression**: Present in `118d12e9` and `b7191049`. A direct `writeStack` test against a fake or built module with distinct instance and frame values is the smallest regression. The current boundary fixture uses `instance: 1` for both keys and `frame: 7` and `3`, so it cannot distinguish the fields.

### S2, `configure_stack` empty-sequence refusal is not discriminated

**Where**: `crates/ocelli-wasm/src/session.rs:232` (`if len == 0`), with the public claim at `docs/lld/public-api.md:92-93`.

**What**: `configure_stack(ticket, 0)` returns `SessionError::Length` and preserves the installed sequence. No test passes `len == 0`. Removing the `len == 0` arm leaves the native suite 30 green. The public-api doc says an empty sequence is refused, but the claim is not backed by a discriminating test.

**Why it is wrong**: The operator-approved stack extension says validation finishes before replacing the sequence and the shell installs nonempty ordered keys. An empty stack is malformed input. The guard is correct now, so this is a smell rather than a defect. The microscope rule 4 identifies an unexercised guard as the class that survives a green suite.

**Evidence**: Removed `len == 0` from the refusal condition, then `bin/ocelli.sh test ocelli-wasm` exit 0 with 30 passed. Reverted, `git diff` empty.

**Scope and regression**: Present in `118d12e9` and `b7191049`. Add `configure_stack(ticket, 0)` to `malformed_stack_and_foreign_tickets_preserve_the_installed_sequence` and assert `Err(SessionError::Length)` plus the preserved key. The existing test covers `len == 17`, stale and foreign tickets, but not zero.

## Nitpicks

### N1, the excluded-suite command in `vitest.config.ts` does not run the suite

**Where**: `vitest.config.ts:12`.

**What**: The comment says `npx vitest run packages/core/src/boundary.wasm.test.ts`. That command uses the default config, which excludes the file, and exits 1 with "No test files found" because `passWithNoTests` is false. The working command is `npx vitest run --config vitest.boundary.config.ts`.

**Why it matters**: A reader following the comment gets a red run and no boundary evidence. The boundary config and the test header carry the correct command, so this is a documentation nitpick rather than a false green.

**Evidence**: `npx vitest run packages/core/src/boundary.wasm.test.ts` exit 1, "No test files found". The config command exit 0 with 6 passed.

### N2, `typescript-packaging.md` still predicts a third wasm-memory allowance for `ring.ts`

**Where**: `docs/lld/typescript-packaging.md:86-87`.

**What**: The sentence says `ring.ts` will need the wasm-memory allowance when F-101 gives it a real ring to drain. F-101 chose a copy-out drain, so `ring.ts` never builds a view over linear memory. The reviewed `ring.ts` header says the buffer is already copied out.

**Why it matters**: The prediction is now false and contradicts the landed design. It does not change behaviour, so it is a documentation nitpick. The sentence should be deleted or rewritten to record the copy-out decision.

**Evidence**: `packages/core/src/ring.ts:14-17` says `drainRing` reads a copied buffer and never a view over wasm linear memory. The allowance list is unchanged.

### N3, the export inventory in `build-targets.md` is incomplete

**Where**: `docs/lld/build-targets.md:89-90`.

**What**: The section lists F-101 exports `alloc`, `apply_commands`, `commit_frame`, `copy_state`, `drain_events` and `ring_capacity`. The reviewed module also exports `bulk_ticket`, `release_bulk`, `configure_stack`, `commit_selected_frame`, `bindCpuCanvas`, `publishCpuCanvas`, `publishCpuColor`, `publishCpuInteraction`, `resizeCpuCanvas`, `copy_presented` and `bitmap_matches`.

**Why it matters**: The list is a partial inventory, not an exhaustive claim, so this is a nitpick. A reader using it as a checklist would miss the stack and canvas exports.

**Evidence**: The generated export names are in `packages/core/src/boundary.wasm.test.ts:337-345` and the session exports in `crates/ocelli-wasm/src/session.rs`.

### N4, `PRESENTED_RUNNERS` carries an unused second element

**Where**: `tools/bench/tests/run_test.mjs:59-66`.

**What**: The array pairs each subject id with a runner name, but the test destructures only `[id]`. The second element is unused.

**Why it matters**: It is dead data that looks meaningful. Either use it to assert the runner id or remove it. This is a nitpick.

**Evidence**: `for (const [id] of PRESENTED_RUNNERS)` at `:69`.

### N5, `StackViewport.error` (flag bit 2) has no setter

**Where**: `crates/ocelli-viewport/src/commands.rs:127`, `:173` and `:262-264`.

**What**: The `error` field is always false, and no public path sets it. The flag bit 2 is therefore never exercised.

**Why it matters**: F-103 owns the broader event catalogue and the error flag. F-101 does not claim it. This is a nitpick and a handoff, not an F-101 defect.

**Evidence**: `error: false` at construction, no assignment elsewhere. The state test asserts bit 2 is 0 after a refused commit, which is the default, not a set path.

## Verified clean

- The command header is 8 bytes per HLD 17.1, not 12. `packCommands` and the `commands.rs` parser agree on kind, viewport, count and padding. A one-command and a several-command batch decode identically.
- The event header is 16 bytes, the stride is 48 bytes and the sequence is a little-endian u64. The ring test goes red on a 56-byte stride.
- `ViewportState` is 96 bytes with camera at 0, centre at 64, width at 68, slice at 72, count at 76, flags at 80 and padding at 84. Swapping centre and width goes red.
- Ticket and ownership are correct. Allocation refusal writes nothing, stale and foreign tickets refuse, and release is ticket-scoped.
- Stack epoch and selection are correct. `configure_stack` returns a nonzero epoch, replacement resets selection, a stale epoch refuses, legacy `commit_frame` refuses while a stack is active, scroll invalidates the old source and sets loading, and no neighbour is drawn.
- The CPU canvas path exposes no pixel getter. `bitmap_matches` compares the presented copy and `copy_presented` copies out. The boundary suite checks the export names and the banned getters.
- The benchmark instrument passes `gate bench` and the real browser suite. The interaction runners no longer return the native diagnostic, and the host attestation and CPU collector refuse invalid counters.
- `gate unsafe` and `gate bindgen` exit 0. There is no `unsafe` outside `crates/ocelli-wasm/src/ring.rs` and `crates/ocelli-core/src/cast.rs`, and `wasm-bindgen` is confined to `ocelli-wasm`.
- The pinned tree's stale ring-producer prose is corrected in the reviewed tree. `public-api.md` names the stack layout correctly. The 8-byte header is consistent with `commands.rs` and HLD 17.1.
- No new trait, generic or `Box<dyn>` with one implementer was added. The `Decoder` and `SeriesSource` exceptions are unchanged.
- No patient data, DICOM corpus file or build artefact was staged.

## Limitations and handoff

The pass pinned `b71910494b2474338073d0f3ed122077700f3b14` first and reviewed `118d12e9d25b931458df3c2b89d704e26d830852`. The F-101 product logic is unchanged between the two trees. The delta is prose and documentation, including a doc comment in `packages/core/src/bulk.ts`, the `packages/core/src/ring.ts` and `packages/core/src/ring.test.ts` prose, `docs/lld/README.md`, `benchmarks.md`, `build-targets.md`, `cache.md`, `comparator.md`, `dicomweb.md`, `errors.md`, `feature-availability.md`, `gpu-ownership.md`, `guards.md`, `oracle.md`, `public-api.md`, `render-graph.md`, `typescript-packaging.md`, `viewport.md`, the sprint documents, the workflow and skill files, `scripts/verify_ledger.py` and a new F-X023 review. I reviewed the F-101-relevant prose in `ring.ts`, `ring.test.ts`, `errors.md`, `public-api.md`, `build-targets.md` and `typescript-packaging.md`. I did not review the remaining docs, sprint, workflow or ledger changes.

The pass did not run `gate --sprint`, the oracle, GPU tests, `#[ignore]` tests or a release-size measurement beyond `bin/ocelli.sh wasm`. Those remain for the sprint gate.

The CPU raster arithmetic is F-X003's and is not re-reviewed here. The boundary crossing is F-101's. The 2x1 fixture in `packages/core/cpu-canvas.mjs` is a manual harness and not in any gate. The D1 regression should be added to the native session suite and, if practical, to the real worker suite.

`an_over_long_panic_message_is_truncated_in_the_record` failed once during a mutation run with left 157 and right 512. It passed on three baseline re-runs and is likely flaky and unrelated to F-101. No gate was disabled.

The result is 1 defect and 2 smells, so F-101 stays in progress. Remediation needs a fresh independent pass after the new tree is verified. No commit, push, tag or verification receipt was created. No product file was edited and no file outside this review was written.

This report is a new staged addition. At report time `git write-tree` is `89032fbcf1bcf3c808f2cd4bc3411d22bad969e9`, which is not the reviewed tree `118d12e9d25b931458df3c2b89d704e26d830852`. The reviewed tree's receipt must not be attached to the union that contains this report.
