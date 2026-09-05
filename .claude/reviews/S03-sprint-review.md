# S03 sprint review, the three passes and what each one falsified

**Scope**: the whole S03 diff on `sprint/s03`, seven stories, reviewed by the
integrator rather than by any story's author.
**Result after pass 3 remediation**: `bin/ocelli.sh gate --sprint` ALL GREEN
over 28 gates, and the comparator's mutation catalogue detects all 21.

## How this record was made, and why it is one file

S01 and S02 each left a per-pass file written at the time. S03's three passes
left their findings in the commit messages of the three remediation commits and
in an untracked integrator's notebook, and no tracked review record at all. A
sprint's review history is a tracked artefact and this sprint's was about to
disappear with the scratch directory.

**This file was assembled after pass 3 from those commits and that notebook,
and it is not three files written at three times.** Writing it as three would
say something about when it was made that is not true. The per-pass detail is
in `git log` on the three commits whose subjects begin `S03, sprint review
pass`.

## The finding that matters more than the count

**Pass 3's sharpest finding was pass 2's headline fix.**

Pass 2 found that the bias bound this sprint added to HLD section 25.1, the
bound whose whole purpose is to make the LINEAR against LINEAR_EXACT divergence
visible, detected none of it. It was evaluated over the image rectangle, and a
pixel clipped to black or white on both sides differs by nothing whatever the
arithmetic underneath says, so the rectangle divides the divergence the
unclipped pixels do show by a denominator full of pixels that structurally
cannot show one. Measured over all seventy-one gating monochrome views, the
largest observable bias was 0.0825 against a bound of 0.1. It caught nothing.

Pass 2 fixed the region and added what it called "a mutation that IS the
divergence". **Pass 3 measured that mutation and it was not.** It applied
`round(u - u/w)` to the already-quantised byte, which makes it a threshold at
`u >= w/2` rather than a proportional effect:

| window | codes the mutation changed |
|--------|----------------------------|
| 400 | 55 of 256, all at `u >= 201` |
| 510 | none |
| 678 | none |
| 4096 | none |

At any width from 510 up it moved no pixel and the harness then refused, saying
the view could not show the divergence. Four tracked files repeated that claim.

The real divergence is proportional and PRE-quantisation. LINEAR_EXACT sits
`u / w` below LINEAR before the renderer rounds, so a pixel drops one code with
probability `u / w`. The replacement is an accumulator that produces exactly
that distribution, deterministically, at every width, in integer arithmetic
with no float and no cast.

**The lesson generalises past this bug.** Both passes checked that the mutation
was DETECTED and neither checked that it was the thing it claimed to be. A test
whose input is wrong tests nothing, however green it goes. HLD 27.2 R2 says a
test must derive from the specification rather than from the implementation,
and a mutation is an input to a test: it derives from the specification too.

## What each pass found, in one line

| Pass | Found | Verdict |
|------|-------|---------|
| 1 | 20 prose and claim defects: records asserting things the tree did not do | remediated |
| 2 | 20 defects, 19 smells. The bias bound detected nothing. The census counted a refusal as watched when its entry named a test that never opened the file. In-plane spacing was compared by nothing. Two DICOM skills reproduced HLD 18.3's wrong `LINEAR_EXACT(-160)` | remediated |
| 3 | 10 defects, 6 smells. Pass 2's mutation was a caricature. The bound's blind spot starts at width 678 and not 2550. Three LLD updates were claimed and none existed | remediated |

## Three numbers that moved in the honest direction

**The census's uncovered count went from zero to nine.** It had counted a
refusal as watched whenever the ENTRY it belongs to named a watcher, without
checking that the named test reaches the file. `bench.runner` named a suite
that never opens `tools/bench/run.mjs`. Pass 3 then found that a `covered_by`
naming a bare DIRECTORY put the same nine back into the covered bucket and
printed `0 watched by nothing` again, so a directory is now resolved to its
files and must hold one that reaches the guarded file. An honest smaller number
is the point of the story.

**The bound's blind spot is content and not only width.** The structural figure
`255 / w` says no window wider than 2550 can reach the bound, which is true and
nearly useless, because the bound depends on `mean(u) / w`. Measured on this
corpus: 51 of the 71 gating monochrome views can fail it and 20 cannot, the
smallest blind window being 678. The 20 are the MR family and the wide-window
rows. That measurement is now in `tolerance.rs`, in HLD 25.1 and in the
comparator LLD, in place of the reassuring structural figure.

**Two of the four declared guard holes closed inside the sprint that declared
them**, and their declarations went with them in the same change, which the
census forces by failing when a declared defect's probe starts passing.

## What pass 3 fixed that no gate could have caught

- **`docs/lld/guards.md` did not exist** and `AS_BUILT.md` said it did, along
  with two other LLD updates. `/complete-feature` step 9 was not run and
  nothing noticed, so the sprint's largest story had no living-architecture
  document while the record claimed three. It was written from the code and not
  from the design plan, which mattered: the plan proposes three census checks
  and the census has six.
- **`SECTION_25_1_BIAS` claimed to be a verbatim transcription of HLD 25.1 and
  had stopped being one**, still saying "image rectangle" for a bullet this
  same sprint changed to "informative region". A test now compares the two.
- **`backlog_check.py` documented a check `main()` has never run.** The sprint
  half of it lives in `gen_sprint_plan.py --check`. The estimate half lived
  nowhere, and F-X014 held `1w` in the plan against `2w` in the backlog and the
  allocation for the length of this review. `--check` now compares the
  estimate, and two probes watch the disagreement branch that the guard has
  claimed since it was written and nothing had ever driven red.
- **`ci_floor_check.py` asked whether ONE step covered every automatic event**,
  so two steps with complementary conditions, which cover the floor between
  them, were refused with a message naming no missing event at all. Coverage is
  now asked per event, with an accept probe.
- **`sandbox.py`'s safety banner miscounted its own choke points**, saying two
  functions name `REPO_ROOT` and that nothing else below refers to it. Three
  do. A safety argument that miscounts is not one.

## What is still open, and it is declared rather than hidden

- **G-02 and G-04**, the two remaining declared guard holes. F-X014.
- **Nine refusals in `tools/bench/run.mjs` watched by nothing.** F-X014, whose
  notes now name the seven recorded limits too. They had all named F-X010,
  whose recorded scope is CI floor equivalence and mentions none of those
  files, so the ownership was as false as the coverage claim it replaced.
- **The skills gate asserts nothing about the numbers in the skills.** F-X015.
- **The HTJ2K route after gate A1 failed.** F-X013.
