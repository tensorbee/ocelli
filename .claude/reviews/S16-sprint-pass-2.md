# S16 sprint review, pass 2

**Reviewed**: staged tree `9033a65f1a2f654c477db84fb23eebd22bfa3453`, confirmed by `git write-tree`. HEAD is `2d5d101f0a021da49ecd7c3c41318f18f521ca2b`. The index is fully staged, with no unstaged or untracked files. The union is 191 files changed, 22,747 insertions and 500 deletions against HEAD. The remediation delta against the pass 1 tree `200af119107b377792475d2316fc0750c034f528` is 13 files, 199 insertions and 33 deletions, of which `.claude/reviews/S16-sprint-pass-1.md` accounts for 143 insertions.
**Author / reviewer**: Implementation by the S16 workers and Codex, recorded in `.claude/scratch/S16-run.json` and `.claude/reviews/S16-codex-completion-evidence.md`. The remediation was written by that implementation side. The reviewer is this agent, independent of both. The feature passes were written by other participants. This pass reviews the whole-sprint union and the remediation delta, not a single feature branch.
**Result**: 3 defects, 0 smells, 1 nitpick. The pass does not clear S16.

## Sprint scope

| Story | Disposition in the reviewed tree | Feature-pass record |
|-------|----------------------------------|---------------------|
| F-048 | done | Seven passes ending 0 defects, 0 smells, 0 nitpicks. AS_BUILT entry present. |
| F-101 | done | Seven passes ending 0 defects, 0 smells, 3 nitpicks. AS_BUILT entry present. |
| F-X003 | carried | Thirteen passes ending 0 defects, 0 smells, 2 nitpicks. AC5 coverage verdict is red and AC6 has no representative host. No AS_BUILT entry, which is consistent with a carried story. |
| F-X022 | done | One pass at 0 defects, 0 smells, 0 nitpicks. AS_BUILT entry present. |
| F-X023 | done | Four passes ending 0 defects, 0 smells, 0 nitpicks. AS_BUILT entry present. |

The disposition agrees across `docs/sprints/CURRENT_SPRINT.md`, `docs/sprints/BACKLOG.md` (`F-X003` is `in-progress`), `.claude/scratch/S16-run.json` (`carried`), `docs/sprints/SPRINT_TRACKER.md` (four S16 rows, no `F-X003` row) and `CHANGELOG.md` (four S16 bullets, no `F-X003` bullet). The absence of an `F-X003` tracker row, AS_BUILT entry and CHANGELOG bullet is the convention a carried story uses.

## Acceptance challenged

The first challenge is the pass 1 D1 remediation. The new Rust test at `crates/ocelli-wasm/src/session.rs:1580-1587` publishes a 1x1 drawn frame on a 4x1 device and asserts the Kind 6 width and height are 1 and 1 while `cpu_device` is `[4, 1]`. I mutated `publish_cpu_frame` to publish `self.cpu_device()` instead of `extent` and re-ran the single test. It went red at line 1586 with `left: 4` and `right: 1`, so the test discriminates the found defect. I restored the file from a copy made before the edit and `git diff` is empty. The restore hash is `ba6409160f6d2bf36d3b9f7e43c35e24fc082f89`, identical to the pre-edit hash. The height assertion is weaker than the width assertion because the device height is also 1 on a 4x1 device, which is N1.

The second challenge is the pass 1 D3 remediation. The recorded reference codes are 5, 30, 127, 225 and 250 at modality 39.5, 39.75, 40, 40.25 and 40.5. PS3.3 C.11.2.1.3.1 with width 0.5 gives 4.59, 30.40, 127.5, 224.60 and 250.41, a maximum error of 0.5 codes. Width 1.5 gives 53.20, 86.51, 127.5, 168.49 and 201.80, a maximum error of 56.5 codes. The recorded codes therefore fit width 0.5 and refute width 1.5. The traced reference range round-trip is `toLowHighRange(0.5, 40, "SIGMOID")` returning lower 39.75 and upper 39.25, then `toWindowLevel(39.75, 39.25)` returning window width 1.5 and centre 40. That round-trip does not agree with the recorded codes.

The third challenge is the pass 1 D2 remediation. `docs/lld/public-api.md:52-56` and `packages/core/src/viewport.ts:31-34` now scope the answer to the stack viewport's GPU drawing and say the CPU raster path draws tier C as a separate surface. `docs/lld/viewport.md:112-113` says `RenderGraph::new` returns `TierCUnavailable` and does not draw on CPU, and `CpuRaster` draws tier C. `docs/lld/feature-availability.md:32` says `CpuRaster` provides tier C stack drawing and `RenderGraph::new` still returns `TierCUnavailable`. The boundary test at `packages/core/src/boundary.wasm.test.ts:377-385` still asserts the glue exports `bindCpuCanvas`, `publishCpuCanvas` and `resizeCpuCanvas`. No consumer-facing claim now reads the GPU path as available on tier C or the CPU path as unavailable.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` | staged index | executed now | `9033a65f1a2f654c477db84fb23eebd22bfa3453` |
| `git status --porcelain` | staged index | executed now | 0 unstaged and 0 untracked paths |
| `git diff --shortstat HEAD` | HEAD to staged tree | executed now | 191 files, 22,747 insertions, 500 deletions |
| `git diff --shortstat 200af119107b 9033a65f1a2f` | remediation delta | executed now | 13 files, 199 insertions, 33 deletions |
| `bin/ocelli.sh test ocelli-wasm --lib` | staged tree | executed now | exit 0, 31 passed |
| `bin/ocelli.sh test ocelli-viewport` | staged tree | executed now | exit 0, 47 passed across 8 test binaries |
| `npx vitest run packages/core/src/viewport.test.ts packages/core/src/bulk.test.ts packages/core/src/ring.test.ts packages/core/src/commands.test.ts` | staged tree | executed now | exit 0, 4 files, 44 tests |
| `npm run test` | staged tree | executed now | exit 0, 10 files, 105 tests |
| `bin/ocelli.sh clippy ocelli-wasm ocelli-viewport` | staged tree | executed now | exit 0 |
| `npm run lint` | staged tree | executed now | exit 0 |
| `npm run typecheck` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate backlog` | staged tree | executed now | exit 0, 213 F-IDs, 180 planned, assignment and estimate agree |
| `bin/ocelli.sh gate prose` | staged tree | executed now | exit 0, 473 files |
| `bin/ocelli.sh gate skills` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate deviations` | staged tree | executed now | exit 0, 25 deviations, citations resolve, D-01 matches Cargo.toml |
| `bin/ocelli.sh gate ci` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate guards` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate unsafe` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate bindgen` | staged tree | executed now | exit 0, wasm-bindgen confined to ocelli-wasm |
| `bin/ocelli.sh gate content` | staged tree | executed now | exit 0, no patient data or build artefacts staged |
| `bin/ocelli.sh gate provenance` | staged tree | executed now | exit 0, 1071 files |
| `python3 scripts/sync_agent_skills.py --check` | staged tree | executed now | exit 0, 42 Codex and 19 OpenCode adapters match 21 canonical sources |
| `python3 -m unittest discover -s scripts/tests -p test_sprint_workflow.py` | staged tree | executed now | exit 0, 16 tests pass |
| `python3 scripts/guard_census.py` | staged tree | executed now | exit 0, 892 refusals in 73 files, all claimed, 0 watched by nothing |
| Mutation of `publish_cpu_frame` to the device extent, then `cargo test -p ocelli-wasm --lib a_reduced_presentation` | staged tree plus one local edit | executed now | exit 101, FAILED at `session.rs:1586`, `left: 4`, `right: 1`, then restored from a copy and `git diff` empty |
| SIGMOID arithmetic at the five recorded points | PS3.3 C.11.2.1.3.1, width 0.5 and 1.5 | executed now | width 0.5 maximum error 0.5 codes, width 1.5 maximum error 56.5 codes against 5, 30, 127, 225, 250 |
| `toLowHighRange` then `toWindowLevel` on the installed cornerstone3D 5.8.2 | `tools/oracle/node_modules` | executed now | lower 39.75, upper 39.25, then window width 1.5 and centre 40 |
| `tools/oracle/page/app.mjs:582` | installed page source | read now | the page calls `toLowHighRange` only, and no page file calls `toWindowLevel` |
| `docs/lld/oracle.md:1221` against `:1241` | S16 diagnosis paragraph | read now | the tier A/C run is byte-identical with zero pixel failures, and the diagnosis has no tier A arm |
| `docs/sprints/AS_BUILT.md:3506` | F-100 entry | read now | "Tier C drawing is `unavailable`" in a prior append-only entry, which `AS_BUILT.md:1-6` says records what was believed at the time |

## Defects

### D1, The corrected oracle paragraph still says the finding agrees with a round-trip that yields the withdrawn width

**Severity**: Defect

**Where**: `docs/lld/oracle.md:1237-1243`.

**What**: The paragraph says the pinned reference's observed codes are 5, 30, 127, 225 and 250, that they fit width 0.5, and that the earlier width 1.5 claim is withdrawn. It then says "This agrees with the traced reference range round-trip". That round-trip is `toLowHighRange` followed by `toWindowLevel`, and it returns window width 1.5 and centre 40. The paragraph therefore asserts agreement with the very width it has just withdrawn.

**Why it is wrong**: The recorded codes fit width 0.5 with a maximum error of 0.5 codes. The round-trip yields width 1.5, whose values are 53, 87, 128, 168 and 202 at the same five points. The two cannot agree. The sentence is also the only place the phrase "traced reference range round-trip" appears, so a reader cannot resolve it from this file.

**Evidence**: The arithmetic above, executed now. `toLowHighRange(0.5, 40, "SIGMOID")` returns `{"lower":39.75,"upper":39.25}` and `toWindowLevel(39.75, 39.25)` returns `{"windowWidth":1.5,"windowCenter":40}`, executed against the installed 5.8.2 utilities. `tools/oracle/page/app.mjs:582` calls `toLowHighRange` only.

**Scope and regression**: The oracle and comparator LLDs, the F-X003 parity attribution, and every reader who follows the diagnosis paragraph. The stale clause is inside the remediation's changed paragraph. Smallest meaningful regression: delete the "agrees with the traced reference range round-trip" clause, or replace it with a sentence that names the round-trip's width 1.5 as the helper result and says the rendered codes do not follow it. Do not change DICOM arithmetic or the parity policy.

### D2, Two staged artefacts still assert the reference's applied SIGMOID width is 1.5

**Severity**: Defect

**Where**: `.claude/reviews/S16-codex-completion-evidence.md:41` and `.claude/plans/F-X003-design.md:510-512`.

**What**: The evidence record says "The pinned reference follows width 1.5 within one code, matching the traced range conversion". The F-X003 design says "the reference page passes the declared width through cornerstone's `toLowHighRange` and `toWindowLevel`, which turns `0.5` into `1.5`, so that row compares two different applied widths". The corrected LLD now says the same reference's codes fit width 0.5 and withdraws the width 1.5 claim.

**Why it is wrong**: The recorded codes 5, 30, 127, 225 and 250 refute width 1.5 by up to 56.5 codes. The tree cannot say the reference's applied width is both 0.5 and 1.5. The design's second clause also asserts an applied width the recorded codes do not support, and the page source calls only `toLowHighRange`, not `toWindowLevel`, at `tools/oracle/page/app.mjs:582`.

**Evidence**: The same arithmetic and `toWindowLevel` execution as D1. `grep` over the tree finds the surviving width 1.5 claims at these two locations, plus the withdrawal at `docs/lld/oracle.md:1240`. The F-X012 record at `docs/lld/oracle.md:358-363` and `tools/oracle/reference-divergence.json:15` both record the codes as tracking the standard curve with 127 at the centre.

**Scope and regression**: The F-X003 design is the origin of the diagnosis the LLD now withdraws, and the Codex evidence record is the sprint's implementation handoff. A reader of either gets the opposite answer to the corrected LLD. Smallest meaningful regression: correct both sentences to the recorded width 0.5, or add a dated correction that names the withdrawn claim and points at the LLD. Preserve the rest of each record rather than deleting it.

### D3, The corrected paragraph calls the reference and candidate diagnosis the remaining A/C difference

**Severity**: Defect

**Where**: `docs/lld/oracle.md:1241`.

**What**: The sentence says "The remaining A/C difference is therefore not explained by a SIGMOID width mismatch". In this file "A/C" is the tier A against C run, which `docs/lld/oracle.md:1221` records as byte-identical on all 84 produced frames with 67 pixel passes and zero pixel failures. The diagnosis that the sentence follows renders the pinned reference and the CPU candidate, and it has no tier A arm.

**Why it is wrong**: There is no remaining tier A against C difference to explain, and the diagnosis is a reference against candidate comparison rather than an A against C comparison. The sentence therefore asserts a difference under a name the same section uses for a comparison that has none.

**Evidence**: `docs/lld/oracle.md:1221` for the byte-identical A/C run. `tools/oracle/diagnose-sigmoid.mjs:59-85` renders the reference and the `produce_stack` CPU candidate only. `grep` over `docs/lld/oracle.md` finds "A/C" at lines 1221 and 1241 only.

**Scope and regression**: The diagnosis paragraph and the F-X003 parity attribution. Smallest meaningful regression: name the comparison the diagnosis actually makes, which is the reference against the CPU candidate, or name the SIGMOID row directly.

## Smells

None.

## Nitpicks

### N1, The new Kind 6 regression pins the width axis but not the height axis

**Severity**: Nitpick

**Where**: `crates/ocelli-wasm/src/session.rs:1580-1587`.

**What**: The test publishes a 1x1 drawn frame on a 4x1 device and asserts the Kind 6 width and height are 1 and 1 while `cpu_device` is `[4, 1]`. The width assertion discriminates a device-extent regression, because the device width is 4. The height assertion does not, because the device height is also 1.

**Why it is wrong**: A future change that published the device height instead of the drawn height would keep this test green. The JS harness fixtures are all one row high as well, so no test in the tree distinguishes the drawn height from the device height.

**Evidence**: The mutation to the device extent went red at line 1586 on the width, not at line 1587 on the height. `grep` over `crates/ocelli-wasm/src/session.rs` shows every `bind_cpu` call has height 1. `packages/core/cpu-canvas.mjs` binds devices of height 1 only.

**Scope and regression**: The Kind 6 wire contract and the CPU canvas harness. This does not block because the code passes the drawn extent through both fields and the width assertion catches the found defect. Smallest meaningful improvement: use a device with height greater than one, such as 4x2 with a 2x1 drawn frame, or assert the height against a device height that differs.

## Verified clean

The pass 1 D1 defect is closed on the surfaces it named. `docs/lld/viewport.md:85-87`, `crates/ocelli-viewport/src/ring.rs:73-76`, `crates/ocelli-wasm/src/session.rs:857-864`, `docs/sprints/AS_BUILT.md:3634` and `.claude/plans/F-101-design.md:122-124` now say drawn width and drawn height, and say the drawn extent is the reduced grid during interaction and the full device on settle. The code at `session.rs:576-577` passes `extent[0]` and `extent[1]` to `present`, and `canvas.rs:926` and `:938` pass `staged.extent`. `grep` finds no remaining "device width" or "device height" claim outside the pass 1 report, which quotes the old wording as the defect. The new test discriminates the found defect as described in N1.

The pass 1 D2 defect is closed. `docs/lld/public-api.md:52-56` scopes the answer to the stack viewport's GPU drawing and says the CPU raster path draws tier C as a separate surface. `packages/core/src/viewport.ts:31-34` and `:41` say the same, and `packages/core/src/viewport.test.ts:61` names the test "records tier C GPU drawing as unavailable for the stack viewport". `TIER_C_DRAWING` stays `"unavailable"` because it is the stack viewport's GPU answer. `docs/lld/viewport.md:112-113`, `docs/lld/feature-availability.md:32` and `docs/lld/render-graph.md:174` agree. No consumer-facing claim reads the GPU path as available on tier C or the CPU path as unavailable. The unscoped sentences left in the tree are in historical artefacts. `docs/sprints/AS_BUILT.md:3506` is a prior entry in an append-only file whose header says it records what was believed at the time and never edits a prior entry. `.claude/plans/F-100-design.md:102` is the F-100 approved plan, which also says the CPU raster is F-X003. Both are dated receipts rather than current claims, and the F-101 entry in the same file records the CPU canvas exports.

The pass 1 N1 nitpick is closed. `docs/lld/public-api.md:3`, `docs/lld/build-targets.md:3` and `docs/lld/typescript-packaging.md:3` now name F-X003, and `docs/lld/README.md:29`, `:36` and `:37` carry it in the mirrored rows. The F-X003 design at lines 167-172 required those three files plus the rows that already carried it.

The remediation introduces no other new false claim. The new sentences in `docs/lld/public-api.md`, `packages/core/src/viewport.ts`, `crates/ocelli-viewport/src/ring.rs`, `crates/ocelli-wasm/src/session.rs`, `docs/lld/viewport.md`, `docs/sprints/AS_BUILT.md:3634` and `.claude/plans/F-101-design.md` were read end to end and are consistent with the code. The CHANGELOG's F-101 bullet says "a device-sized presentation buffer", which is the buffer, not the event, so it does not contradict the drawn-extent change.

The sprint-wide consistency the pass 1 report checked still holds. The status table, the backlog, the run state, the tracker, the AS_BUILT entries and the CHANGELOG agree on the five dispositions. The four completed stories have tracker rows, AS_BUILT entries and CHANGELOG bullets. F-X003 has none of the three and is `carried` in the run state and `in-progress` in the backlog. `docs/sprints/SPRINT_TRACKER.md` still reads `not measured` for every S16 row, and no capacity claim was invented.

No gate was disabled or weakened. The remediation delta touches no gate script, no tolerance file and no baseline. `tools/oracle/src/tolerance.rs` is not in the union's changed files. `ci/wasm-size-budget.json` is unchanged and the recorded size stays 1,999,061 bytes. `ci/guard-probe-budget.json` only adds entries and raises counts. The `ci` gate is green.

No `unsafe` exists outside `crates/ocelli-wasm/src/ring.rs` and `crates/ocelli-core/src/cast.rs`. `grep` over `crates/` finds no other site outside comments and lint attributes, and the `unsafe` gate is green. `wasm-bindgen` appears only in `ocelli-wasm`, and the `bindgen` gate is green. The `content` gate finds no patient data or build artefact staged.

The command channel and the event ring were verified in pass 1 and the remediation does not touch either. The command ids are 1 to 11, the envelope is `[u16 kind][u16 viewport_id][u32 payload_len]`, the limit is 64 commands and 4096 bytes, and the payload sizes match. The ring header is 16 bytes, the stride is 48 bytes, the capacity is 256 events, and the Presented payload offsets are generation at 0, width at 4, height at 8 and flags at 12. The Rust test reads them at 36 and 40, which is the first event's payload at offset 32 plus 4 and 8. The state copy is 96 bytes with camera at 0, centre at 64, width at 68, slice at 72, count at 76, flags at 80 and padding at 84.

The unsafe allowlist, the bindgen isolation, the provenance policy, the skill adapters, the sprint workflow unit suite and the guard census are unchanged and green. The census still claims 892 refusals in 73 files with 0 watched by nothing. The deep guard harness is a pass 1 receipt and was not re-run here.

## Limitations and handoff

This report is the only file added after the reviewed tree. Writing it changes the working tree. The consolidated sprint gate runs after staging this report and is not covered by this pass. The reviewed tree hash and the gate results above describe `9033a65f1a2f`, not the tree that includes this report. The run state still records pass 1 against `200af119107b`, and this pass must be recorded against `9033a65f1a2f` after the report is staged.

No oracle, GPU or browser run was permitted or performed. The recorded reference codes are a prior receipt. Their ignored outputs under `tools/oracle/compare-out` are absent from the tree. `gate --sprint` was not run.

One unconfirmed concern is separate from the findings and needs an oracle run to settle. A static evaluation of the installed reference's `createSigmoidRGBTransferFunction({lower: 39.75, upper: 39.25})` returns 53, 87, 128, 168 and 202 at the five points, which fits width 1.5. The recorded codes are 5, 30, 127, 225 and 250, which fit width 0.5. The `StackViewport` GPU path calls `createSigmoidRGBTransferFunction`, which calls `toWindowLevel`, so the static path and the recorded codes disagree. The pass 1 report and the F-X012 record both accepted the codes, and this pass follows them, but the next owner should re-run `node tools/oracle/diagnose-sigmoid.mjs` at the sprint gate and record the reference's codes once before changing the design or the evidence to width 0.5. If the transfer-function path is the rendered path, the withdrawn width 1.5 claim may be the correct one and the recorded codes may be wrong. Do not change DICOM arithmetic or the parity policy to settle it.

F-X003 is carried for the reasons in `docs/sprints/CURRENT_SPRINT.md:170-174`. AC5 is red with 67 matched views, 0 pixel failures and 17 unmeasured views under a `coverage-loss` verdict. AC6 has no representative GPU-less incumbent host and no accepted cost budget. The carry is honest and is not itself a finding.

The 2 nitpicks left by the F-X003 feature passes and the 3 nitpicks left by the F-101 passes remain carried. They are not re-reported here because this pass did not find a new trigger for them.

The next owner should remediate D1, D2 and D3, stage the changes, re-run the permitted gates and the diagnosis, and obtain a fresh independent sprint pass on the new tree. The consolidated sprint gate and the verification record must follow that staging and must not reuse this tree's receipt.
