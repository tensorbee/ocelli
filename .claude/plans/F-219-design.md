# F-219, Sprint plan checker refusals that no guard probe triggers

**Status**: approved
**Epic ref**: E38.29
**Sprint**: S46
**Estimate**: 1w

Story row, `docs/sprints/BACKLOG.md`: "Sprint plan checker refusals that no guard probe triggers: the four refusals in gen_sprint_plan.py the S46 guard review found unwatched, each given a fixture that turns it red". Layer Test, no declared dependency, milestone M20.

`allocation.json` notes: "Found by the S46 guard probe review, pass 1 nitpick N1. The census counts per entry, so these four read as watched without any probe triggering them."

## Normative source, transcribed

The HLD says nothing about the sprint-plan checker or the guard harness. The normative sources for this story are the repository's own workflow and the two review records that found the gap.

### `.claude/WORKFLOW.md`, "Where things live"

`docs/sprints/SPRINT_PLAN.md` is the "Sprint roadmap", written "Hand-curated", lifetime "Live".

### `scripts/gen_sprint_plan.py`, module docstring, the rule `--check` enforces

"After that SPRINT_PLAN.md is hand-curated prose and this script normally runs in `--check` mode, where it asserts the planning data and nothing else: every planned F-ID appears in exactly one sprint table, and the sprint and the estimate it appears with match the allocation the backlog is rendered from."

### `.claude/commands/design.md`, "Evidence feasibility before implementation", the guard rule

"For a guard, map authoritative input, derived state and the command that must refuse. Construct negative cases with otherwise valid receipts and identities, so an unrelated stale-tree failure cannot masquerade as inventory enforcement. Give a positive control and name the intended failure reason."

### `docs/runbooks/guard-verification.md`, "The probes"

"Run the control afterwards and confirm the guard goes green again, because a guard that fails on everything is as useless as one that fails on nothing."

### `.claude/reviews/S46-guard-probes-pass-1.md`, nitpick N1, verbatim

"`scripts/gen_sprint_plan.py:315` (`appears in no SPRINT_PLAN.md sprint table`), `:328` (`carries no sprint in BACKLOG.md`), `:383` (`carries no **Goal** line`) and `:407` (goal order or punctuation drift) have no probe whose fixture triggers them. The census counts them as watched, because it buckets per entry and the entry carries probes, and the census output says that itself (\"ENTRY-level buckets summed over sites\"). `stale-goal-line`'s fragment `**Goal** line` is also a substring of the `:383` and `:386` messages, so it does not tell those three apart."

### `.claude/reviews/S46-guard-probes-pass-2.md`, nitpick N1, the closing suggestion, verbatim

"Four small fixtures (drop one table row, add one row the allocation lacks, delete one `**Goal**` line, swap two titles in one goal line) would close it."

### The four refusals, re-located on `sprint/s46` HEAD `ed383c7`

`scripts/gen_sprint_plan.py`, `check`, unchanged from HEAD in the working tree. Each `problems.append` call starts on the line given. The message text is quoted from the source, with the f-string pieces joined.

| Site | Line | Message as printed |
|------|------|--------------------|
| R1, planned story absent from every table | 315 | `{fid} is in BACKLOG.md sprint {sprint} but appears in no SPRINT_PLAN.md sprint table` |
| R2, table row the allocation does not plan | 328 | `{fid} appears in SPRINT_PLAN.md sprint {s} but carries no sprint in BACKLOG.md` |
| R3, sprint with no goal line | 383 | `{sprint} carries no **Goal** line` |
| R4, goal line with every title, still unequal | 407 | `{sprint}'s **Goal** line carries every story title and still does not match, so its order or its punctuation has drifted from what the generator writes` |

The two messages that share the `**Goal** line` substring with R3:

| Site | Line | Message as printed |
|------|------|--------------------|
| R5, duplicate goal lines | 386 | `{sprint} carries {n} **Goal** lines. ...` |
| R6, a title absent from the goal line | 403 | `{sprint}'s **Goal** line does not carry the story title it is built from: {story!r}` |

R6 is the refusal the existing `sprint-plan.stale-goal-line` fixture triggers. Its fragment `**Goal** line` also matches R3, R4 and R5.

## Facts measured for this plan

Measured on 2026-11-05 in a scratchpad copy of `scripts/gen_sprint_plan.py`, `docs/sprints/allocation.json` and `docs/sprints/SPRINT_PLAN.md` from the `sprint/s46` working tree, whose baseline `--check` prints `OK: 205 planned F-IDs, ... 20 milestone summary line(s) and 76 goal line(s)`. The fixtures were applied by a stand-in with `Sandbox`'s `read` and `write` semantics, not by the git sandbox.

| Candidate fixture | Exit | Problem lines |
|-------------------|------|---------------|
| delete the first story row of the first sprint table | 1 | exactly one, R1: `F-001 is in BACKLOG.md sprint S01 but appears in no SPRINT_PLAN.md sprint table` |
| insert `\| F-999 \| E0.0 \| A row the allocation plans nowhere \| Test \| 1w \|` after the first sprint table's last row | 1 | exactly one, R2: `F-999 appears in SPRINT_PLAN.md sprint S01 but carries no sprint in BACKLOG.md` |
| delete the first `**Goal**:` line and its trailing blank line | 1 | exactly one, R3: `S01 carries no **Goal** line` |
| swap the first adjacent pair of distinct titles in S01's goal line | 1 | exactly one, R4: `S01's **Goal** line carries every story title and still does not match, ...` |

**Each goes green with its own refusal deleted.** With the `problems.append(...)` call of R1, R2, R3 and R4 in turn replaced by `pass`, keeping the `continue` that follows R1 and R3, each matching fixture exits 0 with the baseline `OK:` line. The `continue` matters: deleting the whole `if` block instead makes the next line raise `KeyError`, which is a red for an unrelated reason.

## What the specification does not cover

1. **One probe per refusal, keyed by a fragment only that refusal prints.** The fragments are `appears in no SPRINT_PLAN.md sprint table` (R1), `carries no sprint in BACKLOG.md` (R2), `carries no **Goal** line` (R3), `its order or its punctuation has drifted` (R4). None is a substring of any other message `check` prints. That was checked against all fourteen message texts in `check`, not only these six.
2. **`stale-goal-line`'s fragment narrows** from `**Goal** line` to `does not carry the story title it is built from`, which only R6 prints. The fixture is unchanged. `two-goal-lines` keeps `**Goal** lines`, which only R5 prints.
3. **The R4 fixture swaps titles rather than dropping the final period.** Both reach R4. A swap is the drift a hand edit actually produces, and it is the review's own suggestion. The fixture reads the sandbox's `allocation.json` for the sprint's story titles in allocation order, applies the generator's documented `;` to `,` substitution that the plan file itself shows, finds the first sprint whose goal line contains two adjacent distinct titles as `a, b`, and swaps them on that goal line only. It raises `SandboxError` if no such pair exists, so a plan that cannot express the drift is a broken fixture and never a silent pass.
4. **The R2 fixture's F-ID is fixed at `F-999`** and the fixture raises `SandboxError` if the sandbox allocation plans `F-999` into a sprint, so a future story with that ID breaks the fixture loudly rather than turning R2 into the wrong-sprint refusal.
5. **Site deletion is recorded, and the unit tests make it permanent in a weaker form.** The harness checks a fragment and an exit status. It cannot itself prove that a probe is red only because of its own site. The proof that it would go green with that site deleted is a recorded measurement (AC9). The permanent proxy is that each unit test asserts its mutation yields **exactly one** problem line and that the line is the intended refusal, because a fixture with exactly one problem goes green when that one refusal is removed.
6. **The unit tests share the existing hand-written plan helpers.** `write_plan_check`, `shared_sprint` and `shared_plan` move from `MilestoneSummary` into a mixin `PlanFixture` that both test classes use, with no change to their bodies. The new class does not inherit `MilestoneSummary`, which would run its tests twice.
7. **No census change in kind.** The census's per-entry buckets are its documented design, and changing it to per-site coverage is a different story. Nothing is filed by this plan for it.

## Problem and reproducer

```bash
python3 scripts/guard_census.py                       # 0 watched by nothing, per entry
grep -n 'appears in no SPRINT_PLAN\|carries no sprint in\|carries no \*\*Goal\|order or its punctuation' scripts/guards/catalogue.py scripts/tests/test_gen_sprint_plan.py   # nothing
```

Today, deleting any of R1 to R4 from `check` leaves `bin/ocelli.sh gate guards` and `gate backlog` green, because no fixture triggers them. Deleting R3 also leaves `sprint-plan.stale-goal-line` green for a reason it does not intend, since R6 still fires and its fragment matches R3's message shape. After this story, deleting any of R1 to R4 turns exactly one probe red, the probe named for it.

## Scope, API and compatibility

In scope: four new probes and their fixtures, one narrowed fragment, the `refuses` text of the `sprint-plan` entry, five unit tests and the helper mixin, the census record, the runbook render, and the LLD paragraph.

Non-goals:

- Any change to `scripts/gen_sprint_plan.py`. The refusals are correct. Only their watchers are missing.
- Rewording any refusal message to make a fragment unique. Every fragment above is already unique, so no message moves.
- Per-site census coverage (decision 7).

No public signature changes. No product code is touched.

New probes, appended to the `sprint-plan` entry's tuple after `sprint-plan.stale-goal-line`, in source order of the refusals:

```text
Probe("sprint-plan.row-absent",        _sprint_plan_row_absent,        --check, "appears in no SPRINT_PLAN.md sprint table")
Probe("sprint-plan.row-unplanned",     _sprint_plan_row_unplanned,     --check, "carries no sprint in BACKLOG.md")
Probe("sprint-plan.goal-line-absent",  _sprint_plan_goal_line_absent,  --check, "carries no **Goal** line")
Probe("sprint-plan.goal-order-drift",  _sprint_plan_goal_order_drift,  --check, "its order or its punctuation has drifted")
```

Each invoke is `script("python3", "scripts/gen_sprint_plan.py", "--check")`, each is floor and level 3, and each uses the default control, the same invoke on the unmutated sandbox, which must exit 0.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `bin/ocelli.sh gate guards`, floor harness | the four new probes and the narrowed one | floor, git sandbox | each probe `red` as required, control green |
| `bin/ocelli.sh gate guards`, `test_gen_sprint_plan.py` | the five new unit tests | floor | pass |
| `python3 scripts/guard_census.py --record` | `ci/guard-probe-budget.json` | after the catalogue change | `entry_sites` `sprint-plan: 16` unchanged. `wall_clock_seconds` is re-measured by `guard_probe.py --record-budget`, not by this command, and is left as recorded because the gate stays inside it |
| `python3 scripts/guard_census.py --render-runbook`, `--check-runbook` | `docs/runbooks/guard-verification.md` | after the catalogue change | four new rows, one changed fragment, later rows renumbered |
| `bin/ocelli.sh gate backlog` | `gen_sprint_plan.py --check`, unchanged | floor | unchanged OK |

## State transitions and invariants

Not applicable. The probes mutate a disposable sandbox and the unit tests write temporary files. The invariant this story adds across consumers: within the `sprint-plan` entry, every probe's fragment matches exactly one of the messages `check` can print.

## Ownership, commit and recovery

Not applicable. No runtime resource is acquired.

## Numbered acceptance inventory

| ID | Trigger and expected behaviour | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | R1. The first sprint table loses its first story row. `--check` exits 1 with one problem line carrying `appears in no SPRINT_PLAN.md sprint table` | probe `sprint-plan.row-absent` | a check that iterates the plan's rows and never the allocation's, so an absent row is invisible | guards |
| AC2 | R2. A row for `F-999`, which the allocation does not plan, is added to the first sprint table. One problem line carrying `carries no sprint in BACKLOG.md` | probe `sprint-plan.row-unplanned` | a check that iterates only the allocation, so an extra row is invisible | guards |
| AC3 | R3. The first `**Goal**:` line is deleted. One problem line carrying `carries no **Goal** line` | probe `sprint-plan.goal-line-absent` | a goal comparison over only the sprints the parser found | guards |
| AC4 | R4. Two adjacent distinct titles in one sprint's goal line are swapped. One problem line carrying `its order or its punctuation has drifted` | probe `sprint-plan.goal-order-drift` | a set or substring comparison that ignores order | guards |
| AC5 | `sprint-plan.stale-goal-line` expects `does not carry the story title it is built from`, and its fixture still produces only R6 lines | the probe, `gate guards` | the old fragment, which R3, R4 and R5 also match | guards |
| AC6 | Fragment uniqueness. Each of the six fragments in AC1 to AC5 and `**Goal** lines` occurs in exactly one of the fourteen message texts `check` can print, checked by hand against the source and recorded in the handoff | handoff table | a fragment that a neighbouring refusal also prints | implementation, recorded |
| AC7 | Controls. Every new probe's unmutated sandbox exits 0 under `--check` | default control in the harness | a fixture that breaks the plan for every probe | guards |
| AC8 | Unit tests, one per message, in a new class using the `PlanFixture` mixin over the hand-written two-story plan. Each asserts the healthy plan exits 0, the mutated plan exits 1, exactly one problem line, that line carries its fragment, and none of the other five fragments appears. R1 deletes `\| F-002 ...`, R2 adds `\| F-009 \| E9.9 \| Extra \| Rust \| 1w \|`, R3 deletes the goal line, R4 writes `**Goal**: Second, First.`, R6 writes `**Goal**: First, Other.` | `test_gen_sprint_plan.py` | a test that asserts only the exit status | implementation |
| AC9 | Site deletion. For each of R1 to R4 and R6, in a scratch copy of the generator with that `problems.append(...)` call replaced by `pass` and every `continue` kept, the matching probe's fixture applied to the real tree exits 0. The unmodified generator exits 1 with the fragment. Recorded as RED and GREEN in the handoff, with the five scratch generator digests | manual, recorded | a probe red for an unrelated reason, or a deletion that turns into a `KeyError` | implementation, recorded |
| AC10 | Mutation of the tests. Each unit test from AC8 goes red when its refusal's `append` is replaced by `pass` in the real module, and green when restored | manual, recorded | a unit test that passes without its refusal | implementation, recorded |
| AC11 | The `sprint-plan` entry's `refuses` text names the two table refusals explicitly, "a planned story absent from every sprint table, a table row for a story the allocation does not plan", beside its existing goal-line clause | `gate guards` | an entry description that still omits two of its refusals | guards |
| AC12 | Census and runbook. `guard_census.py --record` re-run, `entry_sites` still `sprint-plan: 16`, the uncovered ratchet unchanged at zero. `--render-runbook` regenerates the table and `--check-runbook` is clean | `gate guards`, `guard_census.py --check-runbook` | a hand-edited runbook row | guards |
| AC13 | Prose. The fixtures' docstrings and probe notes carry no em-dash and no prose semicolon | `gate prose` | | implementation |

## Approach

1. **Unit tests first (AC8).** Lift the three helpers into `PlanFixture`, add the class with five tests, run them against the unchanged generator. They pass, because the refusals exist. Then AC10 by hand: replace each `append` with `pass` in turn, see the matching test go red, restore.
2. **Fixtures.** In `scripts/guards/catalogue.py`, beside the existing `_sprint_plan_*` helpers, add the four fixture functions. Each reuses the existing `head` and `story` regex shapes already used by `_sprint_plan_wrong_sprint` and `_sprint_plan_row_in_two_sprints`, and each docstring says which refusal it reaches and why it reaches only that one. `_sprint_plan_goal_order_drift` reads `docs/sprints/allocation.json` from the box as decision 3 describes.
3. **Probes.** Append the four probes, narrow the stale-goal-line fragment, update `refuses`.
4. **Harness.** `bin/ocelli.sh gate guards`, confirm all fifteen `sprint-plan` probes `red` and their controls green.
5. **AC6 and AC9** by hand in the scratchpad, recorded.
6. **Record.** `python3 scripts/guard_census.py --record`, `--render-runbook`, `--check-runbook`.

Rejected alternatives:

- **Rewording R3 to drop `**Goal** line`.** Message churn for a uniqueness problem the fragment choice solves.
- **Level-1 snippets calling `check` with in-memory text.** The level-3 fixtures run against the real plan and allocation, which is the state that drifts. The unit tests already cover the in-memory form.
- **A permanent site-deletion meta-test in `test_guard_catalogue.py`.** Each case would build a sandbox and mutate the generator, roughly doubling these probes' cost, for a property the one-problem assertion in AC8 already pins for the synthetic plan.

## Dependencies, write ownership and resources

No declared dependency. Write set:

```text
scripts/guards/catalogue.py
scripts/tests/test_gen_sprint_plan.py
ci/guard-probe-budget.json            (guard_census.py --record)
docs/runbooks/guard-verification.md   (guard_census.py --render-runbook)
docs/lld/guards.md
```

Exclusive files: `scripts/guards/catalogue.py`, `ci/guard-probe-budget.json`, `docs/runbooks/guard-verification.md`. F-217 and F-218 write all three. F-219 runs after F-217 is integrated and before F-218, serially, so each re-records the census over a settled catalogue.

The probes read `docs/sprints/SPRINT_PLAN.md` and `allocation.json`, which every S46 story's completion edits. The fixtures locate their target by structure (first table, first goal line, first swappable pair), never by a named F-ID or sprint, so a ledger edit does not break them.

No network, GPU, browser or corpus.

## Verification and evidence reuse

Focused:

```bash
python3 -B -m unittest discover -s scripts/tests -p test_gen_sprint_plan.py
python3 scripts/gen_sprint_plan.py --check
bin/ocelli.sh gate guards backlog prose
python3 scripts/guard_census.py
python3 scripts/guard_census.py --check-runbook
```

Final profile: `bin/ocelli.sh gate --sprint`, recorded by `/verify` against the staged tree.

What invalidates evidence: any change to `check`'s messages, to the four fixtures, or to the plan's first sprint table or first goal line between AC9's measurement and verification. AC9 is re-measured if any of those move.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a, guard harness and planning tooling, no rendering or compute
- Tier B (WebGL2): n/a, guard harness and planning tooling, no rendering or compute
- Tier C (CPU): n/a, guard harness and planning tooling, no rendering or compute

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | each of R1, R2, R3, R4 and R6 fires alone, with its own fragment, on a hand-written plan | `scripts/tests/test_gen_sprint_plan.py` |
| guard probe | each of R1 to R4 fires on the real plan in a sandbox, and the narrowed R6 fragment | `scripts/guards/catalogue.py`, `gate guards` |
| mutation | AC9 and AC10, recorded RED and GREEN | handoff |

No `fixture` row is owed. No function doing pixel or geometry arithmetic is touched.

## Parity surface covered

None. Repository tooling with no rendered output.

## Deviations

None.

## LLD impact

- `docs/lld/guards.md`, section "The sprint-plan writer is bootstrap-only": one paragraph saying every `--check` refusal now has a probe of its own, keyed by a fragment only that refusal prints, and that the census counts per entry, so a refusal with no fixture reads as watched until a probe names it.

## User-facing documentation impact

No change. `README.md`, `examples/folder-viewer/src/use.html` and `examples/folder-viewer/src/more.html` describe the library and the viewers, and none of them mentions the sprint plan, the guard harness or the census.

## Open questions

None. The fixture shapes follow the review's own suggestion, and the choices made locally (swap rather than punctuation for R4, a fixed `F-999` guarded by `SandboxError`, a shared test mixin) are recorded under "What the specification does not cover" with their reasons. No operator decision is needed.
