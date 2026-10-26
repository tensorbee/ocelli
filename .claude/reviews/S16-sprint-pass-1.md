# S16 sprint review, pass 1

**Reviewed**: staged tree `200af119107b377792475d2316fc0750c034f528`, confirmed by `git write-tree`. HEAD is `2d5d101f0a021da49ecd7c3c41318f18f521ca2b` with tree `3b97147d10b2e335fb721b1d717d87707182500f`. At the reviewed tree the index is fully staged, with no unstaged or untracked files. The union is 190 files changed, 22,574 insertions and 493 deletions against HEAD.
**Author / reviewer**: Implementation by the S16 workers and Codex, recorded in `.claude/scratch/S16-run.json` and `.claude/reviews/S16-codex-completion-evidence.md`. The reviewer is this agent, independent of that implementation. The feature passes were written by other participants. This pass reviews the whole-sprint union, not a single feature branch.
**Result**: 3 defects, 0 smells, 1 nitpick. The pass does not clear S16.

## Sprint scope

| Story | Disposition in the reviewed tree | Feature-pass record |
|-------|----------------------------------|---------------------|
| F-048 | done | Seven passes ending 0 defects, 0 smells, 0 nitpicks. AS_BUILT entry present. |
| F-101 | done | Seven passes ending 0 defects, 0 smells, 3 nitpicks. AS_BUILT entry present. |
| F-X003 | carried | Thirteen passes ending 0 defects, 0 smells, 2 nitpicks. AC5 coverage verdict is red and AC6 has no representative host. No AS_BUILT entry, which is consistent with a carried story. |
| F-X022 | done | One pass at 0 defects, 0 smells, 0 nitpicks. AS_BUILT entry present. |
| F-X023 | done | Four passes ending 0 defects, 0 smells, 0 nitpicks. AS_BUILT entry present. |

The union contains all five stories. F-X003 is included because its implementation is in the reviewed tree even though the story is carried. The carry is consistent across `docs/sprints/CURRENT_SPRINT.md`, `docs/sprints/BACKLOG.md` (`in-progress`) and `.claude/scratch/S16-run.json` (`carried`).

## Acceptance challenged

The first cross-story challenge is the Presented event extent. `publishCpuInteraction` on the 2x1 RGB fixture publishes one Kind 6 event. `packages/core/cpu-canvas.mjs:1043-1045` asserts the event width is 1 and height is 1 while the canvas width is 2 and height is 1. This is the F-X003 drawn extent. `docs/lld/viewport.md:85` says the payload is device width and device height, which would be 2 and 1. The two cannot both describe the same event. This rejects the LLD sentence and confirms the code follows the F-X003 design at lines 332-334.

The second challenge is the SIGMOID reference curve. PS3.3 C.11.2.1.3.1 with width 0.5 at centre 40 gives 5, 30, 128, 225 and 250 at modality 39.5, 39.75, 40, 40.25 and 40.5. Width 1.5 gives 53, 87, 128, 168 and 202. The F-X012 record at `docs/lld/oracle.md:358-363` records the pinned reference's observed codes as 5, 30, 127, 225 and 250, which fit width 0.5 within one code. The S16 diagnosis at `docs/lld/oracle.md:1233-1238` says the same pinned reference follows width 1.5 within one code. This rejects the diagnosis sentence.

The third challenge is the tier C answer. `docs/lld/public-api.md:52` says tier C drawing is unavailable. `docs/lld/viewport.md:112` says `CpuRaster` draws tier C. `docs/lld/feature-availability.md:32` says `CpuRaster` provides tier C stack drawing. `packages/core/src/boundary.wasm.test.ts:377-385` asserts the glue exports `bindCpuCanvas`, `publishCpuCanvas` and `resizeCpuCanvas`. `packages/core/src/viewport.ts:41` pins `TIER_C_DRAWING = "unavailable"`. The public API answer and the delivered CPU path cannot both be true as unqualified statements.

The carried F-X003 acceptance was also challenged. The tier A against C comparison is 67 matched, 0 pixel failures and 17 unmeasured, and the coverage verdict is `coverage-loss`. The LLD records that honestly at `docs/lld/oracle.md:1219-1231` and says AC5 remains open. The candidate checks in `.claude/reviews/S16-codex-completion-evidence.md` are not a profile pass. They are consistent with producer exit 2 and coverage loss, not with a green census.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` | staged index | executed now | `200af119107b377792475d2316fc0750c034f528` |
| `git status --porcelain` | staged index | executed now | all changes staged, no unstaged or untracked paths at the reviewed tree |
| `git diff --stat HEAD` | HEAD to staged tree | executed now | 190 files, 22,574 insertions, 493 deletions |
| `bin/ocelli.sh gate backlog` | staged tree | executed now | exit 0, 180 planned F-IDs, sprint assignment and estimate agree |
| `bin/ocelli.sh gate prose` | staged tree | executed now | exit 0, voice rules clean over 472 files |
| `bin/ocelli.sh gate deviations` | staged tree | executed now | exit 0, 25 deviations declared, citations resolve, D-01 matches Cargo.toml |
| `bin/ocelli.sh gate skills` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate unsafe` | staged tree | executed now | exit 0, 155 files checked, 2 permitted, vendored audit surface unchanged |
| `bin/ocelli.sh gate bindgen` | staged tree | executed now | exit 0, wasm-bindgen confined to ocelli-wasm |
| `bin/ocelli.sh gate content` | staged tree | executed now | exit 0, no patient data or build artefacts staged |
| `bin/ocelli.sh gate provenance` | staged tree | executed now | exit 0, clean over 1070 files |
| `bin/ocelli.sh gate ci` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate guards` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate guards-deep` | staged tree | executed now | exit 0, 401 refusal probes drove 35 guards red, 56 accept probes green, 54 controls green, 0 known defects open |
| `bin/ocelli.sh gate bench` | staged tree | executed now | exit 0, 127 tests, 125 pass, 0 fail, 2 skipped |
| `python3 scripts/guard_census.py` | staged tree | executed now | exit 0, 892 refusals in 73 files, all claimed, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | staged tree | executed now | exit 0 |
| `python3 scripts/sync_agent_skills.py --check` | staged tree | executed now | exit 0, 42 Codex and 19 OpenCode adapters match 21 canonical sources |
| `python3 scripts/tests/test_sprint_workflow.py` | staged tree | executed now | 16 tests pass |
| `python3 scripts/sprint_workflow.py close-preflight` | staged tree | executed now | FAIL, no sprint-scope review recorded, latest sprint-profile verification tree `f4d963f1adad` is stale for `200af119107b`, working tree is not clean, staged tree is not HEAD |
| `python3 scripts/verify_ledger.py assert` | staged tree | executed now | FAIL, no verification recorded for `200af119107b` |
| `packages/core/cpu-canvas.mjs:1043-1045` | 2x1 RGB fixture, interaction step | read now | interaction event width 1 and height 1, canvas width 2 and height 1 |
| `docs/lld/oracle.md:358-363` against `:1233-1238` | pinned reference, SIGMOID ramp | executed now, arithmetic | width 0.5 gives 5, 30, 128, 225, 250, width 1.5 gives 53, 87, 128, 168, 202 |
| `docs/lld/public-api.md:52` against `docs/lld/viewport.md:112` and `docs/lld/feature-availability.md:32` | tier C capability | read now | unavailable against draws tier C |
| `packages/core/src/boundary.wasm.test.ts:377-385` | wasm glue exports | read now | `bindCpuCanvas`, `publishCpuCanvas` and `resizeCpuCanvas` present |

## Defects

### D1, Presented event payload is documented as the device extent while the code publishes the drawn extent

**Severity**: Defect

**Where**: `docs/lld/viewport.md:85`, `crates/ocelli-viewport/src/ring.rs:73-80`, `crates/ocelli-wasm/src/session.rs:857-864`, `docs/sprints/AS_BUILT.md:3634`, `.claude/plans/F-101-design.md:122-123`. The behaviour is at `crates/ocelli-wasm/src/session.rs:517`, `:568` and `:576-577`, and at `packages/core/cpu-canvas.mjs:1043-1045`.

**What**: The prose says the Kind 6 payload width and height are the device width and device height. The F-X003 repair says they are the drawn extent, which is the reduced grid during interaction and the full device on settle. `record_presentation(outcome.extent)` and `publish_cpu_frame(staged.extent)` pass the drawn extent. On the 2x1 RGB fixture, the interaction event is 1x1 while the canvas is 2x1.

**Why it is wrong**: The F-X003 design at lines 332-334 is the authoritative later decision. A consumer that follows `docs/lld/viewport.md` would read a 1x1 interaction event as a 2x1 device event, then scale or position the reduced bitmap with the wrong extent. The wire has one meaning and the LLD states the other.

**Evidence**: `packages/core/cpu-canvas.mjs:1043` asserts `assertPresented(settledRgb.interaction, reducedRgba, 2, 1, 1, 8)` and lines 1044-1045 assert the canvas is 2x1. The `assertPresented` helper at lines 684-694 reads `step.events[0].width` and `step.events[0].height`. `session.rs:576-577` forwards `extent[0]` and `extent[1]` to `present`, and `present` writes them at `ring.rs:87-88`.

**Scope and regression**: Consumers of `drainRing` in `packages/core`, the `render.first_frame` benchmark endpoint, and the F-101 AS_BUILT record. The stale sentence is present only in this tree, introduced by the F-X003 repair without the LLD update. Smallest meaningful regression: change the LLD, the two Rust doc comments and the AS_BUILT entry to say drawn extent, and add a test that pins the interaction event width and height against a device extent that differs. Uncertainty: F-X003 is carried, so the fix belongs to its remaining work, but the stale sentence is in the reviewed tree now.

### D2, The public API records tier C drawing as unavailable while the same tree records `CpuRaster` drawing tier C

**Severity**: Defect

**Where**: `docs/lld/public-api.md:52-53`, `packages/core/src/viewport.ts:31-41`, `packages/core/src/viewport.test.ts:61-67`, `docs/lld/viewport.md:112`, `docs/lld/feature-availability.md:32`, `packages/core/src/boundary.wasm.test.ts:377-385`.

**What**: `public-api.md` says "Tier C drawing is `unavailable`" and the JS constant `TIER_C_DRAWING` is `"unavailable"`, with a test that pins it. `viewport.md` says "`CpuRaster` draws tier C" and `feature-availability.md` says `CpuRaster` provides "Tier C stack drawing". The boundary test asserts the glue exports `bindCpuCanvas`, `publishCpuCanvas` and `resizeCpuCanvas`.

**Why it is wrong**: The public API is the published capability answer. A consumer that reads `TIER_C_DRAWING` concludes tier C drawing is unavailable, while the same package's boundary test asserts the CPU canvas exports exist. Two LLD files in one tree give opposite answers to the same question. The F-X003 design required `public-api` to be updated with the tier C change at lines 167-172, and it was not.

**Evidence**: `grep` over `docs/lld/public-api.md:52`, `docs/lld/viewport.md:112` and `docs/lld/feature-availability.md:32`. `packages/core/src/boundary.wasm.test.ts:377-385` contains the export names. `packages/core/src/viewport.test.ts:61-67` asserts `TIER_C_DRAWING` is `"unavailable"`.

**Scope and regression**: Public API consumers, the `packages/core` test suite, and F-X004 which depends on F-X003. The constant and its test are present in this tree. Smallest meaningful regression: decide whether the public stack viewport's answer is scoped to its own surface, in which case the sentence and the comment must say so, or whether the CPU canvas path is part of the public API, in which case the constant and the test must change. Add F-X003 to the public-api F-ID list in either case.

### D3, The oracle LLD records the pinned reference's SIGMOID curve as both width 0.5 and width 1.5

**Severity**: Defect

**Where**: `docs/lld/oracle.md:358-363` against `docs/lld/oracle.md:1233-1238`.

**What**: The F-X012 record says the 5.8.2 reference's observed codes at modality 39.5, 39.75, 40, 40.25 and 40.5 are 5, 30, 127, 225 and 250, and that they track the PS3.3 curve. Those codes fit width 0.5. The S16 diagnosis paragraph says "Ocelli follows width 0.5 within one output code. The pinned reference follows a width 1.5 curve within one output code." Both sentences describe the same pinned reference and the same synthetic ramp.

**Why it is wrong**: A width 1.5 curve at the same five points gives 53, 87, 128, 168 and 202, not 5, 30, 127, 225 and 250. The two records cannot both be true. The diagnosis is the basis for the A/C parity attribution, so a reader cannot tell which curve the reference followed.

**Evidence**: A Python evaluation of `255 / (1 + exp(-4 * (x - 40) / w))` for w = 0.5 and w = 1.5 gives the values above. `tools/oracle/diagnose-sigmoid.mjs:88-96` computes both `expectedDisplay` at width 0.5 and `width1_5Display` at width 1.5 over stored 151 to 170. The older record at lines 358-363 records the reference's codes.

**Scope and regression**: The oracle and comparator LLDs, `tools/oracle/diagnose-sigmoid.mjs`, and the F-X003 parity record. The S16 diagnosis was disclosed as a limitation in F-X003 passes 11 to 13, but the contradiction is still in the reviewed tree. Smallest meaningful regression: run the diagnosis against the pinned reference, record the reference's curve once, and delete or correct the sentence that does not match. Do not change DICOM arithmetic or the parity policy to make the two records agree.

## Smells

None.

## Nitpicks

### N1, F-X003 index rows are missing where the design required them

`docs/lld/public-api.md:3` and `docs/lld/README.md:37` list F-100 and F-101 but not F-X003. `docs/lld/build-targets.md:3` omits F-X003 while lines 91-92 document the CPU canvas exports. `docs/lld/typescript-packaging.md:3` omits F-X003. The F-X003 design at lines 167-172 required mirrored index rows in public-api, TypeScript packaging and build-targets. This is a provenance omission, not a wrong behaviour, so it does not block on its own.

## Verified clean

The command channel and the event ring agree between Rust and TypeScript. The command ids are 1 to 11, the envelope is `[u16 kind][u16 viewport_id][u32 payload_len]`, the limit is 64 commands and 4096 bytes, and the payload sizes match. The ring header is 16 bytes, the stride is 48 bytes, the capacity is 256 events, and the state copy is 96 bytes with camera at 0, centre at 64, width at 68, slice at 72, count at 76, flags at 80 and padding at 84. The Presented event generation, width, height and flags offsets match between `crates/ocelli-viewport/src/ring.rs` and the harness reader in `packages/core/cpu-canvas.mjs`, which reads them at `at + 16`, `at + 20`, `at + 24` and `at + 28`.

The unsafe allowlist is clean over 155 files with 2 permitted sites, and no new site is outside `crates/ocelli-wasm/src/ring.rs` or `crates/ocelli-core/src/cast.rs`. `wasm-bindgen` is confined to `ocelli-wasm`. No patient data or build artefact is staged, and the provenance gate is clean over 1070 files. The deviations gate declares 25 deviations, resolves every citation and confirms D-01 against `Cargo.toml`.

The CI gate is green. The unsafe step is strengthened to `bin/ocelli.sh gate unsafe`. The new boundary test is excluded from the default vitest config and is run by the `/verify` recipe, so the default suite does not silently skip it. The bench subject registry and the probe budget changes are additive, and the probe counts only increase.

The deep guard harness is green with 401 refusal probes, 56 accept probes, 54 controls and 0 known defects open. The census claims 892 refusals in 73 files with 0 watched by nothing. The skill adapters are in sync at 42 Codex and 19 OpenCode files against 21 canonical sources. The sprint workflow unit suite passes all 16 tests.

The five feature-pass records are consistent with `.claude/scratch/S16-run.json`. F-048 ends at pass 7 with 0 defects and 0 smells. F-101 ends at pass 7 with 0 defects, 0 smells and 3 nitpicks. F-X022 ends at pass 1 with 0 defects and 0 smells. F-X023 ends at pass 4 with 0 defects and 0 smells. F-X003 ends at pass 13 with 0 defects, 0 smells and 2 nitpicks. The carried F-X003 status is consistent across the sprint file, the backlog and the run state, and the absence of an F-X003 tracker row and AS_BUILT entry is consistent with a carried story.

The tier A against C candidate checks are not a profile pass. `S16-codex-verification.json` records tier-c exit 2, tier-a exit 2 and tier-a-c-gate exit 1, with `candidateChecksAreProfilePass: false`. That is consistent with producer exit 2 preserving unmeasured rows and with the coverage-loss verdict, not with a green census. The LLD records AC5 as open.

The reviewed code paths were read against the designs. `crates/ocelli-viewport/src/commands.rs` parses the whole batch before it writes. `crates/ocelli-render/src/cpu.rs` and `colour.rs` use the existing LUT chain and `ColorTransform` rather than a second copy. `crates/ocelli-wasm/src/session.rs` keeps one outstanding bulk allocation and refuses a stale, foreign or duplicate ticket. `packages/core/src/bulk.ts` keeps the ticket ownership and does not hand back a view over wasm memory. `packages/core/src/commands.ts` and `ring.ts` copy the bytes out. No pixel buffer is exposed and there is no per-event callback.

## Limitations and handoff

This report is the only file added after the reviewed tree. Writing it changes the working tree. The consolidated sprint gate runs after staging this report and is not covered by this pass. The reviewed tree hash and the gate results above describe `200af119107b`, not the tree that includes this report.

No oracle, GPU or browser run was permitted or performed. The byte-identical tier A against C claim and the SIGMOID diagnosis remain prior receipts. Their ignored outputs under `tools/oracle/compare-out/codex-tier-a-c`, `tools/oracle/compare-out/tier-a-c-rounding` and `tools/oracle/compare-out/tier-c-rounding-gate` are absent from the tree. `gate --sprint` was not run. Only the permitted gates were run.

`close-preflight` fails for the expected pre-gate reasons. It records no sprint-scope review, the latest sprint-profile verification tree `f4d963f1adad` is stale for `200af119107b`, the working tree is not clean, and the staged tree is not the HEAD tree. `verify_ledger.py assert` fails because no verification is recorded for `200af119107b`. These are not findings. They are the state that the sprint gate and the commit are meant to change.

F-X003 is carried for the reasons recorded in `docs/sprints/CURRENT_SPRINT.md:170-174`. AC5 is red with 67 matched views, 0 pixel failures and 17 unmeasured views under a `coverage-loss` verdict. AC6 has no representative GPU-less incumbent host and no accepted cost budget. The SIGMOID contradiction in D3 is also unresolved. The carry is honest and is not itself a finding.

The 2 nitpicks left by the F-X003 feature passes and the 3 nitpicks left by the F-101 passes remain carried. They are not re-reported here because this pass did not find a new trigger for them.

The review did not exercise GPU, browser, oracle or real wasm runtime paths. It is a static and gate-based review of the staged tree. A clean result here would mean the union has no contradiction in the reviewed surfaces, not that the sprint is verified, published or ready to close.

The next owner should remediate D1, D2 and D3, stage the changes, re-run the permitted gates, and obtain a fresh independent sprint pass on the new tree. The consolidated sprint gate and the verification record must follow that staging and must not reuse this tree's receipt.
