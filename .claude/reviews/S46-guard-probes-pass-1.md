# S46 guard probes review, pass 1

**Reviewed**: the uncommitted working-tree diff on `sprint/s46` at HEAD
`965f6fb`, which is three paths: `scripts/guards/catalogue.py`,
`ci/guard-probe-budget.json` and `docs/runbooks/guard-verification.md`.
`git write-tree` of the index (the diff is unstaged) gave
`cada9970f0a90b02a6b29b576bc07e26b18a935a`, and `git diff | shasum` gave
`52cb28d5ee3369c8df026e79da170258fbf2d816`.
**Author / reviewer**: the diff was written by the session that launched this
review. This pass is an independent subagent that wrote none of it. The
generator change it adapts to, `scripts/gen_sprint_plan.py` in `965f6fb`, is
context and was not itself under review.
**Result**: 1 defect, 1 smell, 1 nitpick

## Acceptance challenged

- Each of the eleven `sprint-plan` probes, with its fixture applied to a
  scratch copy of `scripts/gen_sprint_plan.py`, `docs/sprints/allocation.json`
  and `docs/sprints/SPRINT_PLAN.md` from the working tree, and
  `gen_sprint_plan.py --check` run by hand. For each one the observation was
  the exit code, the number of problem lines and how many lines carry the
  expected fragment.
- Each of the four milestone refusal sites was deleted from the generator
  in turn, and its probe's fixture was rerun to confirm it goes green.
- The new `unplanned` probe was run against the pre-change generator, with
  the pre-change allocation and plan from `965f6fb^`, and was compared with a
  fixture that uses a real milestone's span, which is the hole the probe's
  note describes.
- A plausible wrong repair was tried: a generator that refuses an unplanned
  heading only when its span is not a real milestone's span, with the
  message unchanged.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| baseline `gen_sprint_plan.py --check` | scratch copy of the working tree | now | 0, 203 F-IDs, 20 summary lines, 76 goal lines |
| `existing-refuses-write` fixture, bare writer | same | now | 1, one FAIL line carrying `refuses to overwrite` |
| `two-sprint-tables` fixture | same | now | 1, 2 problems. The fragment is in the duplicate one only |
| `milestone-summary-absent` fixture | same | now | 1, 1 problem: `the allocation puts M1 at S01 to S05 and SPRINT_PLAN.md carries no summary line under its heading...` |
| `milestone-summary-spurious` fixture | same | now | 1, 1 problem: `M1 carries 2 summary lines and the generator writes one` |
| `milestone-summary-unplanned` fixture | same | now | 1, 1 problem: `SPRINT_PLAN.md carries a summary line under M99 and the allocation plans no story for it` |
| `wrong-milestone-summary` fixture | same | now | 1, 1 problem: `the summary line for M1 should read ... 68 engineer-weeks, and SPRINT_PLAN.md says ... 69` |
| `two-goal-lines`, `absent`, `wrong-sprint`, `wrong-estimate`, `stale-goal-line` | same | now | each 1, with the expected fragment in the intended refusal |
| the four milestone sites deleted one at a time, each probe's fixture rerun | mutated scratch generator | now | each 0 (`OK:`), so each probe goes green without its site |
| `unplanned` fixture (span S98 to S99) against the `965f6fb^` generator, allocation and plan | `git show 965f6fb^:` | now | 1, `has no milestone spanning those sprints` |
| M1's real summary line copied under `### M99, Unplanned`, against `965f6fb^` | same | now | **0**, the hole the note names |
| the same real-span copy against the current generator | working tree | now | 1, the unplanned refusal |
| span-filtered wrong repair, committed fixture | mutated scratch generator | now | 1, the probe stays red, so it passes the gate |
| span-filtered wrong repair, real-span copy | same | now | **0** |
| `bin/ocelli.sh gate guards` | real tree | now | 0, `ALL GREEN`, all eleven `sprint-plan` probes `red` as required |
| `python3 scripts/guard_census.py --check-runbook` | real tree | now | 0 |
| `python3 scripts/guard_census.py` | real tree | now | 0, 939 refusals, 472 probes, 0 watched by nothing |
| `python3 scripts/prose_check.py` | real tree | now | 0, 811 files clean |
| `python3 -m unittest scripts.tests.test_gen_sprint_plan` | real tree | now | 0, 19 tests |
| `python3 -m unittest scripts.tests.test_gen_sprint_plan scripts.tests.test_guard_catalogue scripts.tests.test_guard_readers` | real tree | now | 0, 144 tests |
| `census.discover()` filtered to `gen_sprint_plan.py` | real tree | now | 16 sites, which matches the re-recorded budget |

Scratch material is under
`/private/tmp/claude-501/-Users-atulsharma-Documents-projects-tensorbee-ocelli/7387c89c-42ee-4e98-93b8-dbe52072290d/scratchpad/review-guards/`.

## Defects

### D1, the `spurious` probe's note describes a comparison it no longer watches

**Where**: `scripts/guards/catalogue.py:9179`
**What**: the note still opens "The other direction of the same comparison".
That was true when the probe watched the span set difference
(`found - expected`). The diff points this probe at the count refusal, `{key}
carries N summary lines and the generator writes one`, which fires for a
second line under a milestone the allocation does plan. The other direction
of the keyed comparison is now `set(found) - expected` at
`scripts/gen_sprint_plan.py:373`, and the new `milestone-summary-unplanned`
probe watches that one. The fixture docstring at `catalogue.py:960` was
updated ("a second summary line under a milestone that already has one") and
the note beside it was not, so the two sentences now describe two different
refusals.
**Why it is wrong**: microscope section 3, a claim in prose that is false.
**Evidence**: the `spurious` fixture produces exactly one problem, `M1 carries
2 summary lines and the generator writes one`. The set-difference loop at
`gen_sprint_plan.py:373` produces nothing for it, because M1 is planned.
**Scope and regression**: the note only, since the runbook table does not
render notes. Smallest fix: reword the note, for example "`render` writes
one summary line per milestone heading, so a second one under a heading it
already wrote is a hand edit, and the count refusal names the milestone."

## Smells

### S1, the `unplanned` fixture uses a span the old generator already refused, so it does not exercise the hole its note names

**Where**: `scripts/guards/catalogue.py:985` (fixture) and `:9187` to
`:9190` (note)
**What**: the note says that before the S46 review "the span key let such a
line pass whenever its span matched a real milestone's". That is true, and
measured: M1's real line copied under `### M99, Unplanned` gives exit 0
against the `965f6fb^` generator. The fixture writes `_S98 to S99, 1 stories,
1 engineer-weeks._` instead, which matches no real span, and the old
generator refused it too (exit 1, `has no milestone spanning those
sprints`). So the probe proves that an unplanned heading is refused, and
does not prove that a real span under that heading is refused.
**Failure mechanism**: a repair that keeps the message and filters the
unplanned loop by span, refusing an unplanned heading only when one of its
spans is not a real milestone's, keeps this probe red and so passes the
guards gate. The real-span copy then exits 0 again. Both results were
measured with a mutated scratch generator.
**Smallest fix**: in `_sprint_plan_unplanned_milestone_summary`, append
`line.group(0)` (the real M1 summary line) under the new heading instead of
the S98 to S99 line. The fixture then fails under both the old generator and
the span-filtered repair, and the current generator refuses it with the same
single problem.

## Nitpicks

### N1, four generator refusals are still probed only at the entry level (pre-existing, outside the diff)

`scripts/gen_sprint_plan.py:315` (`appears in no SPRINT_PLAN.md sprint
table`), `:328` (`carries no sprint in BACKLOG.md`), `:383` (`carries no
**Goal** line`) and `:407` (goal order or punctuation drift) have no probe
whose fixture triggers them. The census counts them as watched, because it
buckets per entry and the entry carries probes, and the census output says
that itself ("ENTRY-level buckets summed over sites"). `stale-goal-line`'s
fragment `**Goal** line` is also a substring of the `:383` and `:386`
messages, so it does not tell those three apart. None of this was introduced
or changed by this diff, and it is recorded here because the brief asked
whether every site is watched.

## Verified clean

- Every changed expected fragment occurs in exactly the refusal its probe
  names, and in no other problem line that the probe's fixture produces. Each
  milestone fixture yields exactly one problem line.
- Removing each of the four milestone refusal sites turns its probe's fixture
  green, so none of the four probes passes for a reason other than its own
  site.
- `the summary line for` does not occur in the absent, duplicate or
  unplanned messages, and `summary lines and the generator writes one` and
  `the allocation plans no story for it` each occur in one message only.
- The `### M99, Unplanned` heading matches `MILESTONE_HEAD` (`^### (M\w+), `)
  and not the sprint `HEAD` regex, so it changes no sprint or goal parse. The
  inserted line follows M1's line, so M1's own entry is untouched.
- The budget change from 15 to 16 matches `census.discover()`: the old code
  had three milestone sites and the new code has four.
- The runbook is what `--render-runbook` produces (`--check-runbook` exit 0).
  The rows are the old rows renumbered from 337, plus the new row 337 and the
  three changed fragments.
- The absent probe's docstring and note, and the unplanned fixture's
  docstring, are true of the current generator. The note's history (position,
  then span, then heading) matches `git show 965f6fb^:` and `965f6fb`.
- Added lines contain no em-dash and no prose semicolon, and `prose_check.py`
  is clean.
- No other tracked file outside `docs/hld/` and `.claude/reviews/` still
  quotes the old messages `has no milestone spanning`, `carries no summary
  line for it` or `milestone summary line for`.

## Limitations and handoff

- The deep profile was not run. Every `sprint-plan` probe is `floor`, so the
  floor run covers them.
- The fixtures were run by hand through a stand-in object with the same
  `read`, `write`, `substitute` and `delete` semantics as `Sandbox`, not
  through a git sandbox. The gate run used the real harness and agrees.
- `scripts/gen_sprint_plan.py` itself was read as context and not reviewed as
  a change.
- Next action: the author fixes D1 and S1. A fresh independent pass then
  reviews the changed note and fixture and reruns `gate guards`. This pass is
  not clean, so it is not a sprint-readiness receipt.
