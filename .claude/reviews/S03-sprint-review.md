# S03 sprint review, the passes and what each one falsified

**Scope**: the whole S03 diff on `sprint/s03`, seven stories, reviewed by the
integrator rather than by any story's author.
**Result after pass 3 remediation**: `bin/ocelli.sh gate --sprint` ALL GREEN
over 28 gates, and the comparator's mutation catalogue detects every entry in
`tools/oracle/src/mutations.rs`, which `grep -c '^    Mutation {'` on that file
counts.

**A fourth pass then measured every number in this record and in the sprint
ledgers, and this file is corrected by it.** It found the wrong commit count
here, a caught defect described that never existed, and a landing claim that
was true of two files out of three. Each correction below says what was written
and what is true, because deleting the false sentence and leaving the true one
would hide that the record was wrong. The rule the fourth pass applied
throughout: **where a number is not reproducible by a command, the number is
deleted and the command is named.**

## How this record was made, and why it is one file

S01 and S02 each left a per-pass file written at the time. S03's passes left
their findings in commit messages and in an untracked integrator's notebook,
and no tracked review record at all. A sprint's review history is a tracked
artefact and this sprint's was about to disappear with the scratch directory.

**This file was assembled after pass 3 from those commits and that notebook,
and it is not three files written at three times.** Writing it as three would
say something about when it was made that is not true. The per-pass detail is
in `git log`, and this is the command that lists exactly those commits:

```bash
git log --oneline --grep='^S03, sprint review pass' 36adc98..HEAD
```

**Three passes did not leave three commits, and this record said they did until
the fourth pass ran that command.** It returns five. Four carry `remediation` in
the subject, `a651821`, `e962144`, `fe18a91` and `4139a54`, and the fifth,
`f04e07d`, is pass 2 landing its own fix. Pass 1 remediated in two commits, the
three blocking defects first and then the prose and claim sweep. A record whose
stated purpose is to point at the evidence pointed at the wrong number of
commits, which is the defect class this whole file catalogues: a count written
once and never re-run.

## The finding that matters more than the count

**Pass 3's sharpest finding was pass 2's headline fix.**

Pass 2 found that the bias bound this sprint added to HLD section 25.1, the
bound whose whole purpose is to make the LINEAR against LINEAR_EXACT divergence
visible, detected none of it. It was evaluated over the image rectangle, and a
pixel clipped to black or white on both sides differs by nothing whatever the
arithmetic underneath says, so the rectangle divides the divergence the
unclipped pixels do show by a denominator full of pixels that structurally
cannot show one. Measured over every gating class-one view, the largest
observable bias was 0.0825 against a bound of 0.1. It caught nothing.

**No view count is transcribed into this record.** `bin/ocelli.sh gate oracle`
summarises the run as `<n> views: <n> pass, <n> fail, <n> unmeasured, <n>
absent`, `bin/ocelli.sh compare census` prints the class-one detectability
census, and the per-view detail with each record's `toleranceClass`, `outcome`
and `qualifiers` is in `tools/oracle/compare-out/compare.json`. The gating
class-one count written into three passes of this record and into
`tolerance.rs` was wrong, and the fourth pass found it by running the census
rather than by reading the sentence.

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
| 1 | 23 defects and 32 smells, three of them blocking. The rest were records asserting things the tree did not do, swept in one commit | remediated |
| 2 | 20 defects, 19 smells. The bias bound detected nothing. The census counted a refusal as watched when its entry named a test that never opened the file. In-plane spacing was compared by nothing. Two DICOM skills reproduced HLD 18.3's wrong `LINEAR_EXACT(-160)` | remediated |
| 3 | 10 defects, 6 smells. Pass 2's mutation was a caricature. The bound's blind spot starts at width 678 and not 2550. Three LLD updates were claimed and none existed | remediated |
| 4 | 34 defects, 27 smells, over four independent reviewers on disjoint areas. Pass 3's replacement mutation was still not the divergence. A group `#![allow]` switched off four of HLD 27.1's five denied lints with both gates green. The `covered_by` directory route was satisfied by the catalogue itself. This record stated a caught defect that never existed | remediated |

### Pass 1's three blocking defects, which this record used to omit

The row above used to read "20 prose and claim defects" and leave out the three
findings that actually held the push. They are sharper than any of the prose
defects that row named, they are in `a651821`'s message, and they belong here.

- **The cached-wasm-view ban did not ban what this repository writes.** HLD
  17.2's rule was enforced by an eslint selector matching only
  `new DataView(wasm.memory.buffer)`. `packages/core/src/panic.ts` destructures
  first, which makes the argument's object a bare identifier, and the selector
  missed it. Measured: appending both shapes to a file with no allowance
  produced exactly one error, on the literal form. So any file could cache a
  view over wasm linear memory with a green gate, and F-005's widened allowance
  was protecting nothing. A second selector catches the destructured shape and
  the residual limit is stated rather than left to be found.
- **The only real volume reference shipped with its divergence switched off.**
  The early return for a subject that declines to judge uniformity skipped the
  reference comparison too, so `real/mr_eay131` published `referenceDivergence`
  null while its gaps ran 5 to 50 mm against a resolved 10 mm. The measurement
  is hoisted above the return. That fix exposed a latent defect one rung down:
  F-011's rung 3 attributed ANY failing pixel comparison to the reference
  whenever a divergence was declared, whatever field it named, and it could
  never fire before because no volume subject carried one.
- **An operator override could construct a tier the machine cannot run.**
  `OCELLI_TIER=a` returned `Tier::A` with `Applied(A)` on a host where no device
  could be opened, because the override arm checked only that a tier-A adapter
  had been enumerated. `RefusedUnconstructible` already existed for this and was
  unreachable, and the totality property that should have caught it was guarded
  by `request == Auto`.

## Numbers that moved in the honest direction, and one claim that did not hold

**The census's uncovered count went UP, from zero to a real figure.** It had
counted a refusal as watched whenever the ENTRY it belongs to named a watcher,
without checking that the named test reaches the file. `bench.runner` named a
suite that never opens `tools/bench/run.mjs`. Pass 3 then found that a
`covered_by` naming a bare DIRECTORY put the same refusals back into the covered
bucket and printed `0 watched by nothing` again, so a directory is now resolved
to its files and must hold one that reaches the guarded file. An honest larger
number is the point of the story, and it is a downward ratchet from here.
`python3 scripts/guard_census.py` prints it and `ci/guard-probe-budget.json`
records it, which is why no figure for it is written into this file.

**The bound's blind spot is content and not only width.** The structural figure
`255 / w` says no window wider than 2550 can reach the bound, which is true and
nearly useless, because the bound depends on `mean(u) / w`. Measured on this
corpus, most gating class-one views can fail the bound and a minority cannot,
the minority being the `real/mr_eay131` family and the wide-window rows, and the
smallest blind window is 678 rather than 2550. The split itself is deliberately
not transcribed here, because it is exactly the kind of number this review keeps
finding stale, and pass 4 found the numerator wrong again. `bin/ocelli.sh
compare census` re-derives it from the rendered corpus by applying the
catalogue's own swap to each view and reading the signed mean back,
`bin/ocelli.sh gate oracle` prints the run census, and
`tools/oracle/compare-out/compare.json` carries the per-view records both are
computed from.

**Pass 3 claimed that measurement landed in three places and it landed in two.**
`git show --stat 4139a54` lists `tools/oracle/src/tolerance.rs` and
`docs/hld/22-testing-and-tolerance.md` and does not list
`docs/lld/comparator.md`, which still carried only the structural `255 / w`
figure when the fourth pass read it. The comparator LLD is being brought into
line separately. A claim about which files hold a fact is checkable with one
command, and pass 3 did not run it.

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
  nowhere, so `--check` now compares the estimate, and two probes watch the
  disagreement branch that the guard has claimed since it was written and
  nothing had ever driven red. **This record used to justify that with a caught
  defect, and there was no defect.** It said F-X014 held `1w` in the plan
  against `2w` in the backlog and the allocation for the length of this review.
  It did not. `git show fe18a91:docs/sprints/BACKLOG.md`,
  `:docs/sprints/SPRINT_PLAN.md` and `:docs/sprints/allocation.json` all read
  `1w`, and `4139a54` moved all three to `2w` in one commit. **No committed
  state ever held the disagreement.** What happened is that pass 3 widened
  F-X014 while writing that same commit, the plan row lagged the other two
  inside the edit, and the author added the estimate comparison because nothing
  in the repository would have caught it had the edit been committed half done.
  That is a real reason to add a check and it is a weaker one than a latent
  defect, and writing the weaker reason as the stronger one is the same failure
  the rest of this file is about.
- **`ci_floor_check.py` asked whether ONE step covered every automatic event**,
  so two steps with complementary conditions, which cover the floor between
  them, were refused with a message naming no missing event at all. Coverage is
  now asked per event, with an accept probe.
- **`sandbox.py`'s safety banner miscounted its own choke points**, saying two
  functions name `REPO_ROOT` and that nothing else below refers to it. Three
  do. A safety argument that miscounts is not one.

## What pass 4 found, and why it needed four reviewers

Passes 1 to 3 were one reviewer each over the whole diff. Pass 4 was four, on
disjoint areas, and it found more than the three before it combined. That is
evidence about the method rather than about the sprint: a single reviewer over
a diff this size reads everything once and measures a little of it, and four
reviewers with a narrow area each measure most of what they read.

**The mutation was wrong a third time, and the error was on the same side
every time.** Pass 3's accumulator drops clipped-WHITE pixels. The lower clamps
of LINEAR and LINEAR_EXACT coincide exactly, which is why excluding black is
right, and **the upper clamps do not**: LINEAR clamps at `x > c + w/2 - 1` and
LINEAR_EXACT at `x > c + w/2`, so a pixel the reference rendered 255 cannot
move at all once `w >= 510`. Measured on the target, 5801 of 26084 drops were
on reference value 255, and on white-heavy rows the mutation overstated the
divergence by 2.7 times. Each time, the mutation was checked for being
DETECTED and never for being what it claimed to be.

**A one-line attribute switched off the defect class this project exists to
prevent.** `scripts/lint_policy_check.py` matched an `#![allow]` by lint name,
so `#![allow(clippy::pedantic)]` in a crate root disabled
`cast_possible_truncation`, `cast_precision_loss`, `cast_sign_loss` and
`float_cmp` together, and `cargo clippy -D warnings` asserts nothing about
what is enabled, so both gates stayed green. Measured with cargo on a minimal
crate: exit 101 without the attribute and exit 0 with it. The check now
refuses a blanket allow and walks every `.rs` file rather than `src/lib.rs`.

**Two hatches were reopened one indirection out from where they were closed.**
Pass 3 required a `covered_by` directory to hold a file that reaches the
guarded file, and `scripts/guards/catalogue.py` names every guarded file by
construction, so any directory holding the catalogue covered everything. The
route is now removed rather than narrowed, since no entry used it. Pass 3
closed a floor gate hidden behind a condition, and a gate could still be
removed from CI by deleting one of its arm's several commands, or by deleting
its whole job when the gate sits outside the floor.

**This record asserted a caught defect that never existed.** It said F-X014
held `1w` in the plan against `2w` in the backlog and the allocation for the
length of the review. At `fe18a91` all three files read `1w` and at `4139a54`
all three read `2w`. No committed state ever held the disagreement: it was an
uncommitted intermediate state pass 3 created itself while widening the story.
The estimate check is still worth having and it has never caught anything, and
those are two different sentences.

**What the fourth pass then earned.** Filing the two stories it deferred added
two rows to the backlog, and `gen_sprint_plan.py --check`, extended in the same
pass to read the milestone summary lines and the goal paragraphs it had never
read, refused the tree with all three drift classes named before a human looked
at it. That is the first time in this sprint a check caught a real edit rather
than a mutation written to prove it.

## What is still open, and it is declared rather than hidden

- **G-02 and G-04**, the two remaining declared guard holes. F-X014.
- **A device is requested on one adapter and no other is tried**, so a
  host with a broken driver beside a working one resolves a tier that
  renders nothing. F-X016, and the cost is the evidence design rather
  than the loop.
- **Four measured escapes from the wasm linear memory view ban.** The
  function-parameter route is HLD 17.2's named failure with one
  indirection and a green lint. The cheap fourth selector costs one
  real site and the only mechanism for sparing it switches every
  selector off in that file, so the answer is type-aware linting.
  F-X017.
- **The refusals in `tools/bench/run.mjs` watched by nothing**, counted by
  `python3 scripts/guard_census.py` and ratcheted in
  `ci/guard-probe-budget.json`. F-X014, whose
  notes now name the recorded limits too. They had all named F-X010, whose
  recorded scope is CI floor equivalence and mentions none of those files, so
  the ownership was as false as the coverage claim it replaced. Pass 3 called
  those "seven limits" and one of the seven is not a limit: `spikes.compare`
  carries `kind="not-a-guard"` with a `reason`, so the census prints it in the
  out-of-scope bucket and never beside an owner. Six limits and one out-of-scope
  reason. `python3 scripts/guard_census.py` is what prints them, and no count
  for that harness belongs in this file.
- **The skills gate asserts nothing about the numbers in the skills.** F-X015.
- **The HTJ2K route after gate A1 failed.** F-X013.
