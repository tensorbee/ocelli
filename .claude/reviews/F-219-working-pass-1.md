# F-219 review, pass 1

**Reviewed**: worktree `ocelli-wt-f-219`, branch `work/f-219-xai`, HEAD `e21bc4c4d9790594a613d493d76a889d37966db4` plus the uncommitted diff over five paths. `git diff | shasum -a 256` was `1c34660803f43d2693d05e0ca811c780a5c1c0a0e1055c4096b20ab914a64e6f` at the start of the pass and at the end. Per-file sha256 at the end: `scripts/guards/catalogue.py` `268a8815...`, `scripts/tests/test_gen_sprint_plan.py` `09105632...`, `ci/guard-probe-budget.json` `8e56721b...`, `docs/lld/guards.md` `081c677b...`, `docs/runbooks/guard-verification.md` `9fba9ecb...`. `scripts/gen_sprint_plan.py` is unchanged at `937d9e97...`. The untracked `tools/oracle/out` is not part of the story and was not reviewed.
**Author / reviewer**: implemented by an external coding agent, whose report is the scratchpad file `grok-F-219.log`. Reviewed by an independent Claude reviewer that did not write the plan or the code. This file is the only file the reviewer wrote in the worktree.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **AC1 to AC4, one refusal per fixture on the live plan.** Each new fixture was applied by a stand-in box to a temp copy of the live `allocation.json` and `SPRINT_PLAN.md`, and `--check` was run against it. Each control exited 0. Each mutation exited 1 with exactly one `  - ` problem line, and that line was the named refusal: R1 `F-001 ... appears in no SPRINT_PLAN.md sprint table`, R2 `F-999 ... carries no sprint in BACKLOG.md`, R3 `S01 carries no **Goal** line`, R4 `S01's **Goal** line carries every story title ... its order or its punctuation has drifted ...`. This rejects a fixture that is red for a neighbouring refusal.
- **AC5, the narrowed fragment.** The unchanged `stale-goal-line` fixture prints two R6 lines (one per S01 title) and nothing else. `two-goal-lines` prints one R5 line.
- **AC6, fragment uniqueness, derived independently.** `guards.discover._sites_in` over `scripts/gen_sprint_plan.py` gives 15 sites inside `check`: the `does not exist` print, thirteen `problems.append` calls, and the `FAIL: SPRINT_PLAN.md and BACKLOG.md disagree` header. Every one of the fifteen `sprint-plan` probe fragments was tested against all fifteen normalised messages. Each of the fourteen `--check` fragments matches exactly one site, and `refuses to overwrite` matches none, which is correct because that refusal is in `main` at line 442. No executable check of this exists in the tree. See N1.
- **AC9, site deletion, re-run in a scratch clone.** Clone at `review-f219/clone`, diff applied and committed there. Each of R1 (315), R2 (328), R3 (383), R4 (407) and R6 (403) had its `problems.append(...)` replaced by `pass` with the `continue` kept, and was committed so the git sandbox saw it. All fifteen `sprint-plan` probes were run each time. Exactly one probe went `HARNESS` ("exited 0. The guard it aims at did not fire") each time, and it was the named one. The other fourteen stayed red. The scratch digests match the implementer's record (`c521e9da`, `fdf271e7`, `e9929b19`, `110e837a`, `1b8a7b66`). The baseline run of the fifteen was `OK`. The clone was restored to `937d9e97...` after.
- **AC10, unit test mutation.** Under the same five deletions, `python3 -m unittest scripts.tests.test_gen_sprint_plan` failed exactly one test each time and it was the matching one. The other fifteen of sixteen passed.
- **AC8, exactly one problem line.** `assert_lone` counts lines starting with `  - ` and asserts one, asserts the own fragment in that line, and asserts the other five fragments absent from the whole output. Every test also asserts the healthy plan exits 0.
- **Fixture guards, driven.** Planting `F-999` into sprint `S99` in a temp allocation made `_sprint_plan_row_unplanned` raise `SandboxError` with its named message. An `F-999` with an empty sprint did not raise, which is correct because that row still reaches R2. Rewriting every goal line to a single title made `_sprint_plan_goal_order_drift` raise `SandboxError`. On the live plan the order-drift fixture chooses S01, which holds two stories and one goal line. 46 sprints hold two or more stories, so the fixture is not one story away from its raise.
- **Census honesty.** The site sets of `catalogue.py` at `e21bc4c` and in the working tree were diffed by key: 79 to 82, exactly three added and none removed. The added three are `SPRINT_PLAN.md carries no sprint table`, `the allocation plans F-999 into a sprint ...` and `no sprint goal line contains two adjacent distinct story titles`. The other four new raises dedupe onto messages already present (`the first sprint table carries no story row` twice at base, `SPRINT_PLAN.md carries no **Goal** line` once at base). `sprint-plan` stays 16. The only change to `ci/guard-probe-budget.json` is `catalogue` 79 to 82.
- **Runbook.** With row numbers stripped, the removed and added row multisets differ by exactly the old `stale-goal-line` row against its narrowed form plus the four new rows. No prose outside the table changed.
- **LLD.** "Every `--check` refusal now has a probe of its own, keyed by a fragment only that refusal prints" is true today: fourteen refusals, fourteen probes, each fragment unique. See N3 for its durability.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `python3 -m unittest scripts.tests.test_gen_sprint_plan scripts.tests.test_guard_catalogue` | worktree, diff hash `1c346608` | now | 0, `Ran 73 tests`, `OK` |
| `python3 scripts/gen_sprint_plan.py --check` | worktree | now | 0, `OK: 205 planned F-IDs ... 20 milestone summary line(s) and 76 goal line(s)` |
| `bin/ocelli.sh gate guards` | worktree, `CARGO_TARGET_DIR` set | now | 0, `396 refusal probe(s) drove 36 guard(s) red ... 65 control(s) green, 98.2s`, all fifteen `sprint-plan` probes red, `ALL GREEN` |
| `bin/ocelli.sh gate prose` | worktree | now | `OK: voice rules clean over 825 file(s)`, `ALL GREEN` |
| `python3 scripts/guard_census.py` | worktree | now | 0, `1035 refusal(s) in 77 file(s), all claimed`, `0 watched by nothing` |
| `python3 scripts/guard_census.py --profile deep` | worktree | now | 0 |
| `python3 scripts/guard_census.py --check-runbook` | worktree | now | 0 |
| `python3 scripts/backlog_check.py` | worktree | now | 0, `OK: 244 F-IDs, 154 done ...` |
| stand-in fixture driver, six fixtures | clone `b185759` (e21bc4c plus the diff) | now | listed under Acceptance |
| `guard_probe.py` with fifteen `--only sprint-plan.*`, five site deletions | clone, one scratch commit per deletion | now | exactly the named probe `HARNESS` each time |
| unit tests under the five deletions | clone | now | exactly the named test fails each time |
| `_sites_in` diff of `catalogue.py`, base against tree | clone | now | 79 to 82, three named additions |
| implementer's AC6, AC9, AC10 tables | `grok-F-219.log` | prior receipt | agrees with every reproduced number |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, fragment uniqueness has no executable check over all fourteen messages

**Where**: `scripts/tests/test_gen_sprint_plan.py:218` (`ROW_AND_GOAL_FRAGMENTS`).
**What**: The plan's AC6 is a recorded hand check, and the implementer recorded it. The unit tests enforce non-collision only among the six row and goal fragments, and only in the outputs those five tests produce. A later reword of one of the other eight messages, say the milestone ones, to contain one of the fifteen fragments would pass every test and leave a probe that cannot tell two refusals apart, which is the shape N1 of the S46 guard review found. The plan chose a recorded check, so this does not block.
**Smallest fix**: one unit test that runs `guards.discover._sites_in` over `scripts/gen_sprint_plan.py` and asserts each `sprint-plan` probe's `expect` matches exactly one site message. The reviewer's ad hoc version is about ten lines.

### N2, the three new catalogue sites read as watched at entry level and nothing drives them

**Where**: `scripts/guards/catalogue.py:1232`, `:1252`, `:1350`.
**What**: The census counts them watched because the `catalogue` entry names `scripts/tests/test_guard_catalogue.py`, which opens the file and does not run these fixtures. This is the per-entry shape the story's own source names. It does not block, because if either `SandboxError` were deleted the probe would still go red for a reason the harness names (no mutation means the guard did not fire, and the wrong-sprint state lacks the fragment). The reviewer drove both raises by hand, as above.

### N3, the LLD sentence is a universal claim with "now" and nothing holding it

**Where**: `docs/lld/guards.md:436` to `:438`.
**What**: "Every `--check` refusal now has a probe of its own" is true today and goes false the next time a refusal lands without a probe. The `sprint-plan: 16` count would force a `--record`, but nothing would force a probe or an edit to this sentence. "reads as watched until a probe names it" also suggests the census tells the two states apart, and it does not. The census reads the same either way.
**Smallest fix**: drop "now", and replace the second sentence with "The census counts per entry and cannot tell a refusal with a fixture from one without."

## Verified clean

- All four new probes fire exactly their named refusal on the live plan, and controls exit 0.
- The narrowed `stale-goal-line` fragment matches only R6. All fifteen `sprint-plan` fragments are unique over the fifteen sites in `check`.
- Deleting each of R1, R2, R3, R4 and R6 turns exactly its probe and exactly its unit test green, and nothing else. Reproduced independently in a scratch clone, with digests matching the implementer's.
- Each unit test asserts a healthy exit 0, a mutated exit 1, one problem line, its own fragment, and that the other five are absent.
- `PlanFixture` holds `story`, `write_plan_check`, `shared_sprint` and `shared_plan` with unchanged bodies. `RowAndGoalRefusals` does not inherit `MilestoneSummary`. 16 tests run, not 21.
- The fixture regexes (`head`, `story`, `goal`) match the generator's `HEAD`, `ROW` and `GOAL_LINE`. The order-drift fixture re-derives `sprint_groups` order but only swaps a pair it finds verbatim on the goal line, so a future order change cannot produce a silent pass.
- Both `SandboxError` guards fire on their planted states. `F-999` is not in the live allocation.
- Census 79 to 82 is exactly three newly distinct messages. `sprint-plan: 16` and `uncovered` 0 are unchanged. The deep census passes.
- The runbook diff is the generated table only: four new rows, one changed fragment, renumbering. `--check-runbook` is clean.
- The `refuses` text names both table refusals (AC11).
- Prose gate is clean. No added line carries an em-dash or a prose semicolon outside generated table rows that only moved.
- `README.md`, `use.html` and `more.html` mention none of the touched surfaces.
- The worktree diff hash was identical at start and end.

## Limitations and handoff

- The plan's consumer map says `guard_census.py --record` re-measures `wall_clock_seconds`. It does not. `guard_probe.py --record-budget` does. The implementer left wall clock unrecorded. The gate ran in 98.2s against the recorded floor of 103.5s. This is a plan sentence, not a tree defect, and no budget re-record was asked for.
- The fixtures were run against the live plan only. The plan generator, oracle, GPU and cargo gates were not exercised because the story touches none of them.
- This is a clean bounded review of the working diff. It is not a verify receipt and not sprint readiness. Next owner: the implementer may take N1 to N3 or leave them, then `/verify` and `/complete-feature` as usual.
