# S43 review, pass 2

**Reviewed**: staged tree `1309ddbace3a80e9029e7374a7076e3d7baa688f` on `sprint/s43`, which is HEAD `fa655ed` plus the pass 1 remediation.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 2 defects, 1 smell, 4 nitpicks

## Acceptance challenged

Pass 1's D1 and S1 to S4 were checked against the staged source. The title, the goal and "What done means" now describe one worker, one stored sample, the resolved tier and one posted envelope. F-202 had no sprint and no code. `stored_tier_label` takes the unavailable flag, an open adapter still wins, and the `Ok(None)` arm stores `C`. `loadCore` assigns `pending` before a synchronous throw can settle, and the catch does not set the flag. The key-object and secondary-capture writers gained no tag. `TiltedVolume` stores the longest agreeing step. `LATERAL_BOUND_MM` is still `1e-6`. The new tilted test pins the subtraction `2.1e-6 - 6e-7`, not the decimal `1.5e-6`. The two constructors still do not both accept that stack.

## Evidence

| Check | Source | Executed now / prior receipt | Observation |
|---|---|---|---|
| `gate --sprint` | tree 1309ddba | prior receipt | ALL GREEN, 31 gates |
| compare gate | candidate 2bf5c039, reference e117f11b | prior receipt | 71 pass, 0 fail, 31 unmeasured, verdict pass |
| `prepare` and the new tilted test | source | read by the reviewer | longest step is stored, first step is the agreement reference |
| both Part 10 datasets | `tid2010.ts` and `secondary-capture.ts` | read by the reviewer | secondary capture writes Instance Number 1, key object omits it |
| allocation versus F-202 | `allocation.json` and `BACKLOG.md` | read by the reviewer | the backlog row had no allocation entry |

## Defects

### D1, the S4 change left the volumes LLD and the F-197 test count false

**Where**: `docs/lld/volumes.md` and `docs/sprints/AS_BUILT.md` F-197.

**What**: the LLD still said the lateral step is the first inter-slice component, and that a lateral step at or under 1e-6 mm is `OrthogonalStack`. The code stores the longest step and compares that length with the bound. In the new fixture the first step is 6e-7 mm and the stack is still accepted. AS_BUILT said ten tests in `tilted.rs`. The file has eleven.

**Why it is wrong**: `docs/lld/` is the record of what exists. A reader using the LLD to predict which constructor takes a stack near a micrometre gets the pre-remediation answer.

**Evidence**: source reading of `prepare` and the new test. The rustdoc was updated. The LLD paragraph was not.

**Scope and regression**: prose only. Say the stored step is the longest, that equal lengths keep the earlier step, and that the longest step at or under 1e-6 mm is `OrthogonalStack`. Make the test count eleven.

### D2, the new key-object sentence says both writers omit Instance Number, and the secondary-capture writer writes it

**Where**: `docs/lld/key-object.md` against `packages/core/src/secondary-capture.ts`.

**What**: the sentence's subject is both writers, and it lists Instance Number as omitted. `writeImage` writes `InstanceNumber: 1`. The key-object writer does omit it.

**Why it is wrong**: the sentence is new in the remediation and it is false for one of its two subjects.

**Evidence**: both dataset literals read in full.

**Scope and regression**: prose only. Split the list per writer. Do not remove the tag from the writer to make the sentence true.

## Smells

### S1, F-202 is in BACKLOG.md and not in allocation.json, so the per-phase totals leave it out

**Where**: the F-202 backlog row, the backlog summary, and `docs/sprints/allocation.json`.

**What**: every other roadmap row has an allocation entry with an empty sprint and milestone. F-202 had none. The summary names the allocation one-liner as the source for stories and engineer-weeks per phase. That one-liner left out 4 weeks of tracked Phase 1 work. `backlog_check.py` cannot catch this, because it only checks that allocation entries exist in the backlog. The counting rule said Phase 2 and Phase 3 rows are the ones with no milestone and no sprint, while F-202 is Phase 1 under the Phase 2 and Phase 3 heading.

**Why it is wrong**: two commands the backlog names as authoritative disagree, and nothing reports the asymmetry.

**Scope and regression**: add an F-202 allocation entry with phase P1 and an empty milestone and sprint, which leaves M19 and F-198's weeks unchanged. `gen_sprint_plan.py --check` must still pass.

## Nitpicks

1. The new AS_BUILT sprint-review paragraph said a synchronous throw comes from `loadCore`. The initialiser throws, and `loadCore` does not.
2. The key-object omission list is not a complete IOD inventory. Study Date, Study Time, Referring Physician's Name, Study ID, Accession Number, Patient Birth Date, Patient Sex and the key-object Referenced Performed Procedure Step Sequence are also absent. This is from memory of PS3.3 and was not checked against the standard. The sentence does not claim to be complete.
3. "F-202 is the one Phase 1 row" puts a count in prose beside the table that holds it.
4. `table.unavailable = false` in `Ok(Some)` cannot clear a true flag, because `ensure_device` returns before a second open once the flag is set. It is harmless. AS_BUILT's F-198 "Tests added" line also does not list `load-retry.test.ts`.

## Verified clean

- **Tolerances.** `LATERAL_BOUND_MM` is `1e-6`. The `1e-9` duplicate bound and the `0.005` spacing factor are unchanged. Agreement is still measured against the first step.
- **`unsafe`.** None was added. The two allowed files are not in the diff.
- **wasm-bindgen.** It appears only in `ocelli-wasm`, and the remediation adds no attribute.
- **Boundary.** The worker's messages still carry a boolean, a tier string, a published flag and a byte count. No pixels.
- **Writers.** Neither writer gained a tag. The secondary-capture reader still refuses a present patient name or ID.
- **Tier names.** There are still exactly three, A, B and C.
- **Sprint ledgers for the pass 1 title defect.** The title, the goal, "What done means", the CHANGELOG and the dependents are consistent. F-202 had no sprint and no code.
- **Prose read in this pass.** No em dash and no prose semicolon in the remediation.

## Limitations and handoff

The reviewer ran nothing. All gate, compare, test and wasm results are host receipts on `1309ddba`. PS3.3 was not opened, so nitpick 2 is from memory. The live page was not clicked. Pass 1's nitpicks, the production `/@fs` URL and the empty failed-load tier line were not reopened. D1 and D2 are prose-only fixes. S1 is a ledger decision. A third pass should check those three edits and confirm `backlog_check.py` and `gen_sprint_plan.py --check` stay green if F-202 gets an allocation entry.
