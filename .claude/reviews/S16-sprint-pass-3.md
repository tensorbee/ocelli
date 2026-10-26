# S16 sprint review, pass 3

**Reviewed**: staged tree `319a693d1792fdff9670fb0f7eaeae4442e34049`, confirmed by `git write-tree`. HEAD is `2d5d101f0a021da49ecd7c3c41318f18f521ca2b`. The index is fully staged, with no unstaged or untracked files. The union is 192 files changed, 22,929 insertions and 500 deletions against HEAD. The remediation delta against the pass 2 tree `9033a65f1a2f654c477db84fb23eebd22bfa3453` is 5 files, 211 insertions and 29 deletions. Of that delta, `.claude/reviews/S16-sprint-pass-2.md` accounts for 158 insertions and the four remediation files account for 53 insertions and 29 deletions.
**Author / reviewer**: Implementation by the S16 workers and Codex, recorded in `.claude/scratch/S16-run.json` and `.claude/reviews/S16-codex-completion-evidence.md`. The remediation was written by that implementation side. The reviewer is this agent, independent of both. The feature passes were written by other participants. This pass reviews the whole-sprint union and the remediation delta, not a single feature branch.
**Result**: 0 defects, 0 smells, 0 nitpicks. The pass clears the whole-sprint review on the reviewed tree. S16 is not closed by this report.

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

The first challenge is the pass 2 N1 remediation. The Rust test at `crates/ocelli-wasm/src/session.rs:1523-1598` binds a 4x2 device, seeds a 4x2 RGB frame, draws a 1x1 reduced frame, asserts all eight device cells equal the drawn prefix, and asserts the Kind 6 width and height are 1 and 1 while `cpu_device` is `[4, 2]`. I mutated `publish_cpu_frame` to publish the device height while keeping the drawn width. The test went red at the height assertion with `left: 2` and `right: 1`. I restored the file from a copy made before the edit and `git diff` is empty. The restore hash is `d0a3b8dc4ed1420e754f9c1ed25bee9815951955690a2de458b1df17f50b710d`, identical to the pre-edit hash. The width assertion was already discriminating because the device width is 4. Both axes now discriminate.

The second challenge is the pass 2 D1 closure. The corrected paragraph at `docs/lld/oracle.md:1233-1249` no longer says the finding agrees with the traced reference range round-trip. It records the rendered codes as fitting width 0.5, names the traced `toLowHighRange` result as lower 39.75 and upper 39.25, says the installed `createSigmoidRGBTransferFunction` on that range statically yields codes that fit width 1.5, says the rendered codes and the transfer-function path disagree, and says the reference's applied width stays unresolved pending an oracle run. I executed the installed 5.8.2 utilities statically. `toLowHighRange(0.5, 40, "SIGMOID")` returns lower 39.75 and upper 39.25. `toWindowLevel(39.75, 39.25)` returns width 1.5 and centre 40. `createSigmoidRGBTransferFunction` on that range returns 53, 87, 128, 168 and 202 at the five points. PS3.3 C.11.2.1.3.1 with width 1.5 gives 53.20, 86.51, 127.50, 168.49 and 201.80, a maximum error of 0.5 codes. Width 0.5 against the recorded codes 5, 30, 127, 225 and 250 gives a maximum error of 0.5 codes, and width 1.5 against those codes gives 56.507 codes. The paragraph's claims are true.

The third challenge is the pass 2 D2 closure. `.claude/reviews/S16-codex-completion-evidence.md:38-46` and `.claude/plans/F-X003-design.md:509-518` now record the same unresolved state with a correction dated 2026-10-26. Neither asserts the reference's applied or rendered width is 1.5 without the unresolved qualification. The design's correction names the withdrawn sentence accurately. The evidence record's correction names the withdrawn sentence accurately.

The fourth challenge is the pass 2 D3 closure. The diagnosis paragraph now names the comparison `reference-versus-candidate`. `grep` over `docs/lld/oracle.md` finds `A/C` only at line 1221, which is the tier A against C run that is byte-identical on all 84 produced frames with 67 pixel passes, zero pixel failures and 17 unmeasured views. The diagnosis has no tier A arm.

The fifth challenge is the sprint-wide consistency. The five story dispositions agree across the status table, the backlog, the run state, the tracker, the AS_BUILT entries and the CHANGELOG. The four completed stories have tracker rows, AS_BUILT entries and CHANGELOG bullets. F-X003 has none of the three and is `carried` in the run state and `in-progress` in the backlog. `docs/sprints/SPRINT_TRACKER.md` still reads `not measured` for every S16 row, and no capacity claim was invented.

The sixth challenge is the no-new-false-claim check. The oracle paragraph is internally consistent. It records the rendered codes as fitting width 0.5 and the static transfer-function path as fitting width 1.5, says they disagree, and says the applied width is unresolved pending an oracle run. It does not assert either width as the reference's settled applied width. The two corrected artefacts say the same. The test's new comment says the seeded frame is nonuniform on both axes, which is true of the 4x2 fixture. The test's Kind 6 assertion now distinguishes the drawn height from the device height.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` | staged index | executed now | `319a693d1792fdff9670fb0f7eaeae4442e34049` |
| `git status --porcelain` | staged index | executed now | 0 unstaged and 0 untracked paths |
| `git diff --shortstat HEAD` | HEAD to staged tree | executed now | 192 files, 22,929 insertions, 500 deletions |
| `git diff --shortstat 9033a65f1a2f 319a693d1792f` | remediation delta | executed now | 5 files, 211 insertions, 29 deletions |
| `bin/ocelli.sh test ocelli-wasm --lib` | staged tree | executed now | exit 0, 31 passed |
| `bin/ocelli.sh test ocelli-viewport` | staged tree | executed now | exit 0, 47 passed across 8 test binaries |
| `npx vitest run packages/core/src/viewport.test.ts packages/core/src/bulk.test.ts packages/core/src/ring.test.ts packages/core/src/commands.test.ts` | staged tree | executed now | exit 0, 4 files, 44 tests |
| `bin/ocelli.sh clippy ocelli-wasm ocelli-viewport` | staged tree | executed now | exit 0 |
| `npm run lint` | staged tree | executed now | exit 0 |
| `npm run typecheck` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate backlog` | staged tree | executed now | exit 0, 213 F-IDs, 180 planned, assignment and estimate agree |
| `bin/ocelli.sh gate prose` | staged tree | executed now | exit 0, 474 files |
| `bin/ocelli.sh gate skills` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate deviations` | staged tree | executed now | exit 0, 25 deviations, citations resolve, D-01 matches Cargo.toml |
| `bin/ocelli.sh gate ci` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate guards` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate unsafe` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate bindgen` | staged tree | executed now | exit 0, wasm-bindgen confined to ocelli-wasm |
| `bin/ocelli.sh gate content` | staged tree | executed now | exit 0, no patient data or build artefacts staged |
| `bin/ocelli.sh gate provenance` | staged tree | executed now | exit 0, 1072 files |
| `python3 scripts/sync_agent_skills.py --check` | staged tree | executed now | exit 0, 42 Codex and 19 OpenCode adapters match 21 canonical sources |
| `python3 -m unittest discover -s scripts/tests -p test_sprint_workflow.py` | staged tree | executed now | exit 0, 16 tests pass |
| `python3 scripts/guard_census.py` | staged tree | executed now | exit 0, 892 refusals in 73 files, all claimed, 0 watched by nothing |
| Mutation of `publish_cpu_frame` to the device height, then `bin/ocelli.sh test ocelli-wasm --lib a_reduced_presentation_replicates` | staged tree plus one local edit | executed now | exit 101, FAILED at the height assertion, `left: 2`, `right: 1`, then restored from a copy and `git diff` empty |
| Static evaluation of the installed 5.8.2 utilities | `tools/oracle/node_modules` | executed now | lower 39.75, upper 39.25, then width 1.5 and centre 40, transfer-function codes 53, 87, 128, 168, 202 |
| PS3.3 C.11.2.1.3.1 arithmetic at the five recorded points | width 0.5 and 1.5 against 5, 30, 127, 225, 250 | executed now | width 0.5 maximum error 0.5 codes, width 1.5 maximum error 56.507 codes |
| `grep` for `width 1.5`, `applied width` and `follows width` in `docs/`, `.claude/plans/` and the Codex evidence | staged tree | executed now | only qualified hits at `docs/lld/oracle.md:1242` and `:1245`, `.claude/plans/F-X003-design.md:512` and `:517`, and the Codex evidence `:42` and `:45` |
| `grep` for `A/C` in `docs/lld/oracle.md` | staged tree | executed now | only line 1221 |
| `grep` for `unsafe` in `crates/` | staged tree | executed now | only `crates/ocelli-wasm/src/ring.rs` and `crates/ocelli-core/src/cast.rs` |
| `git diff 2d5d101 -- ci/wasm-size-budget.json tools/oracle/src/tolerance.rs` | HEAD to staged tree | executed now | empty |
| `git diff 2d5d101 -- ci/guard-probe-budget.json` | HEAD to staged tree | executed now | additions only, no count lowered |
| `git diff 2d5d101 -- .github/workflows/ci.yml bin/ocelli.sh` | HEAD to staged tree | executed now | the unsafe step now runs `bin/ocelli.sh gate unsafe` and the gpu gate adds the tier A candidate proof, both strengthenings |
| Sprint disposition cross-check | `CURRENT_SPRINT.md`, `BACKLOG.md`, `S16-run.json`, `SPRINT_TRACKER.md`, `AS_BUILT.md`, `CHANGELOG.md` | read now | five dispositions agree, four completed stories have tracker, AS_BUILT and CHANGELOG entries, F-X003 has none and is carried |
| `docs/lld/oracle.md:1233-1249`, `.claude/plans/F-X003-design.md:509-518`, `.claude/reviews/S16-codex-completion-evidence.md:38-46` | S16 diagnosis and corrections | read now | corrected, internally consistent, no settled width claim |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The pass 2 D1 defect is closed. The oracle paragraph at `docs/lld/oracle.md:1233-1249` no longer says the finding agrees with the traced reference range round-trip. It records the rendered codes as fitting width 0.5, names the traced `toLowHighRange` result as lower 39.75 and upper 39.25, says the installed `createSigmoidRGBTransferFunction` on that range statically yields codes that fit width 1.5, says the rendered codes and the transfer-function path disagree, and says the reference's applied width stays unresolved pending an oracle run. The static evaluation confirms every number in the paragraph. The recorded codes fit width 0.5 with a maximum error of 0.5 codes, and width 1.5 misses them by up to 56.507 codes. The static transfer-function path yields 53, 87, 128, 168 and 202, which fit width 1.5 within 0.5 codes.

The pass 2 D2 defect is closed. `.claude/reviews/S16-codex-completion-evidence.md:38-46` and `.claude/plans/F-X003-design.md:509-518` record the same unresolved state with a correction dated 2026-10-26. Neither asserts the reference's applied or rendered width is 1.5 without the unresolved qualification. The corrections name the withdrawn sentences accurately. The design says the installed transfer function statically yields an applied width of 1.5 while the recorded rendered codes fit 0.5, and says the reference's applied width stays unresolved pending an oracle run. The evidence record says the recorded codes fit width 0.5, the installed path statically yields width 1.5, the two disagree, and the applied width stays unresolved pending an oracle run.

The pass 2 D3 defect is closed. The paragraph names the comparison `reference-versus-candidate`. `grep` over `docs/lld/oracle.md` finds `A/C` only at line 1221, which is the tier A against C run that is byte-identical on all 84 produced frames with 67 pixel passes, zero pixel failures and 17 unmeasured views. The diagnosis has no tier A arm.

The pass 2 N1 nitpick is closed. The Rust test at `crates/ocelli-wasm/src/session.rs:1523-1598` binds a 4x2 device, seeds a 4x2 RGB frame, draws a 1x1 reduced frame, asserts all eight device cells equal the drawn prefix, and asserts the Kind 6 width and height are 1 and 1 while `cpu_device` is `[4, 2]`. The mutation to the device height went red at the height assertion. The width assertion was already discriminating because the device width is 4. Both axes now discriminate.

No surviving sentence in `docs/`, `.claude/plans/` or `.claude/reviews/S16-codex-completion-evidence.md` asserts the reference's applied or rendered width is 1.5 without the unresolved qualification. The only width 1.5 hits are the static transfer-function claim at `docs/lld/oracle.md:1242`, the withdrawn claim at `:1245`, the design's static claim at `:512`, the design's correction at `:517`, the evidence's static claim at `:42` and the evidence's correction at `:45`. Each is qualified by the unresolved state or names the withdrawn claim. The F-X012 record at `docs/lld/oracle.md:358-366` and `tools/oracle/reference-divergence.json` record the observed codes as tracking the standard curve with 127 at the centre, which is consistent with the codes fitting width 0.5. `docs/lld/comparator.md:1028-1036` says the observed presented pixels remain standard-correct and monotonic. `docs/lld/corpus.md:135-139` records the fixture's five display values as coming from PS3.3 C.11.2.1.3.1. None of those records asserts an applied width of 1.5.

The remediation introduces no other new false claim. The oracle paragraph is internally consistent. The test's new comment says the seeded frame is nonuniform on both axes, which is true of the 4x2 fixture. The test's Kind 6 assertion now distinguishes the drawn height from the device height. The design and evidence corrections are dated and accurate. The paragraph's sentence that the remaining reference-versus-candidate difference is not explained by a settled SIGMOID width mismatch is true in the current unresolved state and is explicitly time-bound by the pending oracle run.

The sprint-wide consistency holds. The five story dispositions agree across `docs/sprints/CURRENT_SPRINT.md`, `docs/sprints/BACKLOG.md`, `.claude/scratch/S16-run.json`, `docs/sprints/SPRINT_TRACKER.md`, `docs/sprints/AS_BUILT.md` and `CHANGELOG.md`. The four completed stories have tracker rows, AS_BUILT entries and CHANGELOG bullets. F-X003 has none of the three and is `carried` in the run state and `in-progress` in the backlog. `docs/sprints/SPRINT_TRACKER.md` still reads `not measured` for every S16 row, and no capacity claim was invented. The sprint plan's S16 table holds the same five stories.

No gate was disabled or weakened. The remediation delta touches no gate script, no tolerance file and no baseline. `tools/oracle/src/tolerance.rs` is not in the union's changed files. `ci/wasm-size-budget.json` is unchanged and the recorded size stays 1,999,061 bytes. `ci/guard-probe-budget.json` only adds entries and raises counts. The `ci` gate is green. The union's `ci.yml` change replaces the bare unsafe script with `bin/ocelli.sh gate unsafe` and adds the tier A candidate proof to the `gpu` gate, both strengthenings.

No `unsafe` exists outside `crates/ocelli-wasm/src/ring.rs` and `crates/ocelli-core/src/cast.rs`. `grep` over `crates/` finds no other site outside comments and lint attributes, and the `unsafe` gate is green. `wasm-bindgen` appears only in `ocelli-wasm`, and the `bindgen` gate is green. The `content` gate finds no patient data or build artefact staged.

The command channel and the event ring were verified in pass 1 and pass 2 and the remediation does not touch either. The command ids are 1 to 11, the envelope is `[u16 kind][u16 viewport_id][u32 payload_len]`, the limit is 64 commands and 4096 bytes, and the payload sizes match. The ring header is 16 bytes, the stride is 48 bytes, the capacity is 256 events, and the Presented payload offsets are generation at 0, width at 4, height at 8 and flags at 12. The Rust test reads them at 36 and 40, which is the first event's payload at offset 32 plus 4 and 8. The state copy is 96 bytes with camera at 0, centre at 64, width at 68, slice at 72, count at 76, flags at 80 and padding at 84.

The unsafe allowlist, the bindgen isolation, the provenance policy, the skill adapters, the sprint workflow unit suite and the guard census are unchanged and green. The census claims 892 refusals in 73 files with 0 watched by nothing. The deep guard harness is a pass 1 receipt and was not re-run here.

## Limitations and handoff

This report is the only file added after the reviewed tree. Writing it changes the working tree. The consolidated sprint gate runs after staging this report and is not covered by this pass. The reviewed tree hash and the gate results above describe `319a693d`, not the tree that includes this report. The run state still records pass 2 against `9033a65f`, and this pass must be recorded against `319a693d` after the report is staged.

No oracle, GPU or browser run was permitted or performed. The recorded reference codes are a prior receipt. Their ignored outputs under `tools/oracle/compare-out` are absent from the tree. `gate --sprint` was not run. The static evaluation of the installed 5.8.2 utilities is not an oracle run and does not settle the reference's applied width. The applied width remains unresolved pending an oracle run, as the corrected paragraph, design and evidence say. Do not change DICOM arithmetic or the parity policy to settle it.

The JS CPU canvas harness still binds devices of height 1 at `packages/core/cpu-canvas.mjs:180`, `:270`, `:448`, `:611` and `:631`, so it does not independently exercise a device height greater than one. The Rust test now owns that axis. This is a bounded coverage limitation, not a finding, because the JS harness forwards the extent and the Kind 6 contract is asserted in Rust.

F-X003 is carried for the reasons in `docs/sprints/CURRENT_SPRINT.md:170-174`. AC5 is red with 67 matched views, 0 pixel failures and 17 unmeasured views under a `coverage-loss` verdict. AC6 has no representative GPU-less incumbent host and no accepted cost budget. The carry is honest and is not itself a finding.

The 2 nitpicks left by the F-X003 feature passes and the 3 nitpicks left by the F-101 passes remain carried. They are not re-reported here because this pass did not find a new trigger for them.

This is whole-sprint pass 3 against `max_review_passes: 3` in `.claude/scratch/S16-run.json`. The run-sprint host owns the limit decision. The per-feature microscope loop remains uncapped. The pass clears the whole-sprint review on this tree, not the sprint. The consolidated gate and the verification receipt must follow the staging of this report, and S16's close still depends on the carried F-X003 disposition.

The next owner should stage this report, run the consolidated sprint gate, record the pass with `python3 scripts/sprint_workflow.py record-sprint-review` against the new tree, and record the verification receipt. The pass must not reuse the `9033a65f` tree's receipt.
