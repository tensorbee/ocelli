# S46 guard probes review, pass 2

**Reviewed**: the uncommitted working-tree diff on `sprint/s46` at HEAD
`965f6fb`, the same three paths as pass 1: `scripts/guards/catalogue.py`,
`ci/guard-probe-budget.json` and `docs/runbooks/guard-verification.md`. The
diff is unstaged, so `git write-tree` of the index still gives
`cada9970f0a90b02a6b29b576bc07e26b18a935a`. `git diff | shasum` gave
`e02bb091823b12c49de7de405b92f76cd9188f22` at the start and at the end of
this pass, so the tree did not move under it.
**Author / reviewer**: the diff and its pass 1 remediation were written by the
session that launched this review. This pass is a fresh independent subagent
that wrote none of it and did not write pass 1. `scripts/gen_sprint_plan.py`
in `965f6fb` is context and was not under review.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 1 D1** (the `spurious` note described the wrong comparison). The new
  note was checked against `render` (`scripts/gen_sprint_plan.py:132` to
  `:143`), which writes one `### Mn` heading, its goal and exactly one summary
  line per entry of `rendered_milestones`, and against the fixture's single
  observed problem.
- **Pass 1 S1** (the `unplanned` fixture did not build the hole its note
  names). Both author claims were re-derived in a fresh scratch copy: the
  `965f6fb^` generator, allocation and plan on the new fixture, and the
  current ones on the same fixture.
- **Pass 1's plausible wrong repair** was replayed: a generator that skips an
  unplanned heading whenever every summary line under it names a real
  milestone's span, message unchanged. The question is whether the probe now
  goes green under it, which is what makes the gate catch that repair.
- **Site deletion** for the new refusal: the unplanned loop's `append` was
  made unreachable and the fixture rerun.
- **Every `sprint-plan` probe** was driven against the current generator to
  confirm the fragment it names lands in the refusal it is meant to watch and
  in no other problem line.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| baseline `gen_sprint_plan.py --check` | `base-cur`, working-tree generator, allocation and plan | now | 0, 203 F-IDs, 20 summary lines, 76 goal lines |
| baseline `gen_sprint_plan.py --check` | `base-old`, `git show 965f6fb^:` of all three files | now | 0, 193 F-IDs |
| new `unplanned` fixture (M1's real line under `### M99, Unplanned`) | `base-old` | now | **0**, `OK: ... 20 milestone summary line(s)`. The old span key absorbs the copy, so the author's first claim holds |
| same fixture | `base-cur` | now | 1, one problem: `SPRINT_PLAN.md carries a summary line under M99 and the allocation plans no story for it`. The author's second claim holds |
| span-filtered wrong repair, new fixture | `base-wrong-repair`, current generator with the unplanned loop skipping headings whose spans are all real | now | **0**, so the probe goes green and the guards gate would fail on that repair |
| span-filtered wrong repair, pass 1's S98 to S99 fixture under `### M99` | same | now | 1, still refused. This is the case that let the repair through before, and it confirms S1 was real |
| unplanned `append` made unreachable, new fixture | `base-delete-site` | now | 0, the probe goes green without its site |
| all eleven `sprint-plan` fixtures | `base-cur` | now | each exit 1 with the fragment on its own refusal: `refuses to overwrite`, `SPRINT_PLAN.md sprint tables`, `carries no summary line under its heading`, `summary lines and the generator writes one` (one problem, `M1 carries 2 summary lines`), `the allocation plans no story for it`, `**Goal** lines`, `does not exist`, `is in sprint`, `is estimated`, `the summary line for`, `**Goal** line` (two lines, both the stale-title refusal) |
| `spurious` fixture | `base-old` | now | 1, the old span refusal only, which is why the fragment had to change |
| `bin/ocelli.sh gate guards` | real tree | now | 0, `ALL GREEN`, all eleven `sprint-plan` probes `red` |
| `python3 scripts/guard_census.py --check-runbook` | real tree | now | 0 |
| `python3 scripts/guard_census.py` | real tree | now | 0, 939 refusals, 472 probes, 0 watched by nothing |
| `python3 -m unittest scripts.tests.test_gen_sprint_plan scripts.tests.test_guard_catalogue` | real tree | now | 0, 68 tests |
| `python3 scripts/prose_check.py` | real tree | now | 0, 811 files clean |
| `census.discover()` filtered to `gen_sprint_plan.py` | real tree | now | 16 sites, matching the budget's `sprint-plan: 16` |

The fixtures were applied by `drive.py`, a stand-in with `Sandbox`'s `read`,
`write`, `substitute` and `delete` semantics. The gate run used the real
harness and agrees. Scratch material is under
`/private/tmp/claude-501/-Users-atulsharma-Documents-projects-tensorbee-ocelli/7387c89c-42ee-4e98-93b8-dbe52072290d/scratchpad/review-guards2/`.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, four generator refusals are still probed only at the entry level (carried from pass 1, pre-existing, outside the diff)

`scripts/gen_sprint_plan.py:315` (`appears in no SPRINT_PLAN.md sprint
table`), `:328` (`carries no sprint in BACKLOG.md`), `:383` (`carries no
**Goal** line`) and `:407` (goal order or punctuation drift) are still
triggered by no `sprint-plan` fixture: none of the eleven outputs above
contains any of the four messages. `scripts/tests/test_gen_sprint_plan.py`
does not contain those message fragments either. It stays a nitpick for this
diff. The diff neither added nor changed these sites, and the census already
says its buckets are per entry rather than per refusal driven red. It is the
section 4 defect class, though, so it belongs in a story of its own rather
than being left as a review footnote. Four small fixtures (drop one table
row, add one row the allocation lacks, delete one `**Goal**` line, swap two
titles in one goal line) would close it.

## Verified clean

- **D1 closed.** The `spurious` note now reads "render writes one summary
  line per milestone heading, so a second one under a heading it already wrote
  is a hand edit." That is true of `render`, and it matches both the fixture
  docstring and the one problem the fixture produces.
- **S1 closed.** The `unplanned` fixture now copies the first real summary
  line under `### M99, Unplanned`. The old generator accepts it and the
  current one refuses it, so the fixture builds the hole the note names. The
  span-filtered wrong repair turns the probe green, so the gate now catches
  it.
- The `unplanned` note ("Before the S46 review the span key let such a line
  pass whenever its span matched a real milestone's") is now backed by an
  observed exit 0 on the old generator.
- The `absent` note's history (position, then span, which collapses two
  milestones sharing a span, then heading) matches the `965f6fb^` code
  (`found_by_span` keyed by `(first, last)`) and the current code (`found`
  keyed by `MILESTONE_HEAD`). The `absent` docstring is true of the current
  generator.
- The `### M99, Unplanned` insertion yields exactly one problem under the
  current generator, so it does not disturb a sprint table, a goal line or
  M1's own entry.
- Each milestone fixture produces exactly one problem line, and each of the
  four milestone fragments occurs in only its own refusal's message.
- The runbook rows 335 to 337 and 342 carry the catalogue's fragments, and
  `--check-runbook` is clean. The budget's 16 equals the discovered site
  count.
- The added catalogue lines contain no em-dash and no prose semicolon. The
  semicolons the diff shows in the runbook are in pre-existing table rows
  renumbered by the renderer.

## Limitations and handoff

- The deep profile was not run. Every `sprint-plan` probe is `floor`.
- `scripts/gen_sprint_plan.py` was read as context and not reviewed as a
  change.
- `README.md`, `use.html` and `more.html` describe none of the changed
  material and were not rechecked.
- This is a clean bounded pass over the three-path diff at the pinned tree.
  It is not a sprint-readiness receipt. Staging and the sprint review record
  are still the author's to do, and a later edit to these paths makes this
  pass stale. N1 is open for a follow-up story.
