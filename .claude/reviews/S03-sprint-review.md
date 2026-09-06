# S03 sprint review, the passes and what each one falsified

**Scope**: the whole S03 diff on `sprint/s03`, seven stories, reviewed by the
integrator rather than by any story's author.
**Result**: each remediation commit carries its own `Ocelli-Verify` trailer,
written by the pre-commit hook from the verify ledger and naming the profile,
the gate list, the corpus result and the tree it certifies. `git log
--format='%h %(trailers:key=Ocelli-Verify)' 36adc98..HEAD` prints them, and
`git rev-parse <commit>^{tree}` checks the binding, and it holds on every one.
**Every pass from 2 onward ended `profile=sprint corpus=pass` over 28 gates.**
Pass 1's two commits record `profile=feature` over 25, because the sprint
profile was not yet being run for review remediation. The sixth pass found that
sentence written here without the qualifier, in the paragraph whose whole
argument is that the trailer records what actually ran. **The number of gates is the only
figure written here**, because `bin/ocelli.sh gate --list` prints it and the
trailer records what actually ran. The comparator's mutation catalogue detects
every entry in `tools/oracle/src/mutations.rs`, which `grep -c '^    Mutation
{'` on that file counts.

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

**The passes do not map one to one onto commits, and this record said they did
until the fourth pass ran that command.** Pass 1 remediated in two commits, the
three blocking defects first and then the prose and claim sweep, and pass 2
landed its own fix in a commit of its own before its remediation. **The number
is deliberately not written here.** The fourth pass replaced "three" with
"five", and the fifth pass ran the same command and got six, because the
commit that wrote "five" carries the same subject and counted itself out. A
count written once and never re-run is the defect class this whole file
catalogues, and it is not made safe by being recounted once.

## The finding that matters more than the count

**Pass 3's sharpest finding was pass 2's headline fix.**

Pass 2 found that the bias bound this sprint added to HLD section 25.1, the
bound whose whole purpose is to make the LINEAR against LINEAR_EXACT divergence
visible, detected none of it. It was evaluated over the image rectangle, and a
pixel clipped to black or white on both sides differs by nothing whatever the
arithmetic underneath says, so the rectangle divides the divergence the
unclipped pixels do show by a denominator full of pixels that structurally
cannot show one. Measured over every gating class-one view, the largest
observable bias fell short of the 0.1 bound, so it caught nothing.
`./target/release/ocelli-compare census` prints the figure, which was itself
superseded once when the mutation it was measured under turned out to be
wrong.

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

That table is arithmetic and not a measurement, so it is written out rather
than deferred to a command: it is the count of `u` in `0..=255` for which
`round(u - u/w) != u`. **Rerun it with Rust's rounding and not Python's.**
Rust's `f64::round` goes half away from zero and Python's `round` goes half to
even, and they disagree at exactly one code here: at `w = 510` and `u = 255`
the value is `254.5`, which Rust returns to 255 and leaves unchanged while
Python returns 254 and reports a change. Python alone would put a 1 in the
second row and make the whole finding look weaker than it is. At any width from
510 up the mutation moved no pixel and the harness then refused, saying the
view could not show the divergence. Four tracked files repeated that claim.

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
| 3 | 10 defects and 6 smells, a tally that appears in no commit message and nowhere else in this tree, and one of the three rows that fail this file's own rule. See the note below the table. Pass 2's mutation was a caricature. The bound's blind spot starts at width 678 and not 2550. Three LLD updates were claimed and none existed | remediated |
| 4 | Four independent reviewers on disjoint areas returned 34 defects and 27 smells, which `8fbfc88`'s message carries in the same form passes 1 and 2 carry theirs. Pass 3's was the only unrecorded tally when this row was written, and it is not any more. See the note below the table. Pass 3's replacement mutation was still not the divergence. A group `#![allow]` switched off four of HLD 27.1's five denied lints with both gates green. The `covered_by` directory route was satisfied by the catalogue itself. This record stated a caught defect that never existed | remediated |
| 5 | Four reviewers again, on the same areas, with the pass-4 remediation as the primary target. A floor gate could be deleted from CI while the check said all 25 ran. `#![allow(clippy :: pedantic)]` with spaces defeated the fix for `#![allow(clippy::pedantic)]`. Four of the five refusals pass 4 added to the census were watched by nothing. The mutation's residue invariant was false below `w = 255` | remediated |
| 6 | Three reviewers. Six defects, against twenty-one and thirty-four before. The comparator's white-pixel exclusion carried a wrong consequence of PS3.3 for the fourth consecutive pass. A TOML trailing comment was the fifth route past the lint policy. Eight refusals in scanned guard files, including gate A4's wasm size ceiling, were invisible to the scanner | remediated |
| 7 | Three reviewers. Seventeen defects, and the count rose because the sweep widened. Six mutations left the suite green, two of them on paths whose rationale is written out at length in the source. The harness's own inverted-success refusal was watched by nothing. The lint policy took a sixth and a seventh route | remediated |
| 8 | Three reviewers. **The comparator returned zero defects**, the first clean area of the sprint, and reproduced all five of pass 7's claims independently. Six defects elsewhere: the eighth and ninth routes past the lint policy, a declared exception nothing ratcheted, and a 68-mutation sweep finding seven tests that cannot fail | remediated |
| 9 | Three reviewers. **The comparator was clean a second time.** Ten defects elsewhere, and the lint-policy sequence finally got a diagnosis rather than a tenth route: the guard RECONSTRUCTS the compiled file set by hand. HLD 17.2's central architectural rule was found guarded by nothing | remediated |
| 10 | Three reviewers, 12 defects, 8 smells, 6 nitpicks, and this row's tally is in `e2b11d8`'s successor because pass 10 found the sentence claiming pass 3's was the only unrecorded one to be false for passes 8 and 9 as well. **The comparator was clean a third time and the crates sweep converged at five survivors in ninety mutations.** The guard harness produced a backtick shape and two `#[path]` spellings | remediated |
| 11 | Two reviewers, 6 defects, 2 smells, 5 nitpicks. **The comparator and the crates were both declared done**, at four and two consecutive clean passes. The guard reviewer named the class ten passes had been patching and the remediation replaced two of three hand-rolled grammars with real parsers | remediated |
| 12 | Two reviewers, 6 defects, 8 smells, 3 nitpicks, aimed at the two grammars pass 11 left. **The prediction pass 11's remediation made against itself came true**: the YAML reader had four fail-open routes, one of them key order, which is not a spelling and cannot be patched. Both readers are replaced | remediated |

**Three of these tallies are recorded here and nowhere else, not one.** The
pass-4 row explained pass 3's missing tally and asserted it was the only one,
and then the rows for passes 8 and 9 were added afterwards with the same defect
in them, which is the shape that row exists to describe. Measured over
`git log --reverse --format=%h 36adc98..HEAD`, reading each message with
`git log -1 --format=%B <sha>`:

- Recorded: pass 1 in `a651821` ("23 defects and 32 smells"), pass 2 in
  `fe18a91` ("twenty defects and nineteen smells"), pass 4 in `8fbfc88`
  ("34 defects and 27 smells"), pass 5 in `f51a9ea` ("Twenty-one defects"),
  pass 6 in `a5a9a9c` ("Six defects"), pass 7 in `e33f824`
  ("Seventeen defects").
- Not recorded: pass 3, whose remediation `4139a54` gives no count. Pass 8,
  whose `828037e` opens "THE COMPARATOR RETURNED ZERO DEFECTS" and gives no
  count for the row's "Six defects elsewhere". Pass 9, whose `e2b11d8` says
  "Six documentation defects" about a strict subset and gives no count for the
  row's "Ten defects elsewhere".

The tallies are kept rather than deleted, because this file is where a reader
looks for them. What is corrected is the claim about how many of them stand
alone.

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
move at all once `w >= 510`. A fifth of the mutation's drops were landing on
pixels the divergence leaves alone, and the overstatement was worst on
white-heavy rows. **The figures are not transcribed here**, which is this
file's own rule and one it broke for two paragraphs:
`./target/release/ocelli-compare census` prints the per-view bias and
`tools/oracle/src/mutations.rs` carries the derivation at the site. Each time,
the mutation was checked for being DETECTED and never for being what it
claimed to be.

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

## What pass 5 found, and the shape it kept finding

Pass 5 aimed four reviewers at the pass-4 remediation, on the argument that the
newest code is the least reviewed code. It found twenty-one defects, and nearly
all of them are one shape: **the fix was written against the route somebody
demonstrated rather than against the rule.**

- Pass 4 refused `#![allow(clippy::pedantic)]`. `#![expect(...)]` silenced the
  same lints and was found before the commit landed. Then pass 5 found
  `#![allow(clippy :: pedantic)]`, which Rust tokenises identically and the
  guard compared as a string, and `#[allow(...)] pub mod x;`, which governs a
  whole module tree while the guard permitted it as "the visible local choice".
  And the walk read `crates/` while `Cargo.toml` declares fourteen members,
  the fourteenth being `tools/oracle` with thirteen unscanned source files.
- Pass 4 required every command in a gate's arm to be run by CI. The arm
  splitter ended an arm at the next case label rather than at its `;;`, so it
  read commands out of the comment block introducing the next arm.
  `arms['panic']` came out as `npm run test`, a command that appears nowhere in
  that arm, and the `panic` gate, which is the wasm panic-hook proof of HLD
  section 23 and the one property no native test can observe, could be deleted
  from CI while the check printed "all 25 floor gate(s) are invoked by CI".
- Pass 4 added twelve probes and changed `scripts/guards/census.py` by 224
  lines **and gave that file no probes at all**. Four of its five new
  refusals, including the ratchet whose stated purpose is that the widening
  ratchet cannot be disarmed in one green commit, could each be deleted with
  the census, the probe profile and the unit suite all green. They were
  invisible because the entry claims its file with `"*"`, so a new refusal in
  an already-catalogued file lands in the covered bucket. The census's own
  rule, that a guard arrives with its test, did not apply to the census.
- The mutation's residue invariant was false. `if accumulator >= w` rather than
  `while` means `acc < w` holds only where every participating value is below
  `w`, so below a window of 255 the mutation applied materially less than the
  divergence it claims to be. The corpus's narrowest window is 256.

**And the region decision that pass 2 called the sprint's central fix had no
test outside the corpus run.** Substituting the image rectangle for the
informative region left `cargo test` fully green and was caught only by a
mutation replay needing the rendered corpus and a GPU.

Three findings were the record's own. The commit count this file had already
corrected once was wrong again, because the commit that corrected it carries
the same subject and counted itself out, so the number is now not written at
all. The escape count in F-X017's title moved from four to five to at least
eight across two passes, so the title carries no count either. And a CHANGELOG
bullet claimed twelve reformats where nine are written, reintroducing a count
pass 1 had already fixed in two other files.

## What pass 6 found, and where the passes are going

Six defects, against twenty-one in pass 5 and thirty-four in pass 4, and the
areas are separating. The record and the crates came back with two defects
between them and a verification that mattered more than any finding: every one
of the twenty-one test mutations pass 5 claimed to have proved was applied
again and every one went red, and an independent sweep of fifteen further
mutations in `caps.rs` killed fourteen, the survivor being a provably
equivalent mutant. **No test in that area passes whether the code is right or
wrong.** That is the first time in this sprint a reviewer has been able to say
so.

The two places still producing defects are the two with the most arithmetic and
the most adversarial surface.

**The comparator's white-pixel exclusion was wrong for the fourth consecutive
pass.** Every version of that block reasoned only about stored values in
LINEAR's clamped region and treated "the reference rendered 255" as equivalent
to "LINEAR clamped". LINEAR also rounds to 255 over a band below the clamp, and
LINEAR_EXACT can fall under 254.5 there. The movable set is `y_L` in
`[254.5, min(255, 254.5 w/(w-1)))`, and what `w >= 510` buys is only that a
CLAMPED pixel cannot move. The remediation then derived what four passes of
prose had missed: **in stored-value units the band is exactly `509/510` of one
input unit wide at every width.** 510 moves width between the rounding part and
the clamp part rather than switching anything off, which is why an integer
sometimes falls in the band and sometimes does not, and why the count is zero at
both 255 and 510 and one at every other width tried. The exclusion is
conservative everywhere and exact nowhere, and there is now a fixture over
twelve widths that pins the boundary rather than a sentence asserting it.

**The lint policy took a fifth route.** A trailing TOML comment made a row
invisible to a regex anchored on end of line. On a required row that fails safe.
On the group row added one pass earlier it failed open, and the declared
constant that was supposed to be the backstop stopped at the first blank line,
so the two holes lined up and `pedantic = { level = "allow", priority = 1 }
# keeps noise down` after a blank line turned off four of HLD 27.1's five
denied lints with every gate green. Both halves are fixed, and one residual is
declared rather than hidden: a two-line inline table is still invisible to the
row regex, and the constant digest is what catches it.

**And the scanner could not see eight refusals that were already there**,
including gate A4's wasm size ceiling, because they are written as
`problems += [...]` or as a returned list literal. Deleting that ceiling left
the census reporting exactly the same number of refusals and exiting 0. So
`entry_sites`, added one pass earlier precisely to notice a refusal added to an
already-claimed file, could not see a refusal written in that shape. Three
shapes were added and the count moved from 544 to 566.

**Eight and twenty-two are different numbers and this record used to set them
beside each other without saying so.** Eight is what the pass found by hand in
scanned guard files. Twenty-two is what the three new shapes then surfaced, and
`git show a5a9a9c -- ci/guard-probe-budget.json` is where it is measured: eleven
`entry_sites` figures move and their deltas sum to exactly 22. `a5a9a9c`'s
message attributes the split, sixteen being pre-existing refusals that were
always invisible, the eight among them, and six being the new probes' own
refusals. So the eight are a subset of the sixteen and never were the whole of
the movement.

**The shape of the remaining work is now clear.** Six passes have not exhausted
the guard harness or the comparator, and each pass costs less than the one
before it. What has converged is everything else.

## What seven passes have actually shown

**The defect count is not converging, and that is not the same as the tree
getting worse.** Pass 6 found six, pass 7 found seventeen, and the difference is
that pass 7's reviewers mutation-tested code nobody had mutation-tested before.
Every pass so far has found a NEW CLASS rather than more of the last one:

| Pass | The class it found |
|------|--------------------|
| 1 | Records asserting things the tree did not do |
| 2 | A bound that detected none of what it was added for |
| 3 | A mutation that was not the divergence it claimed |
| 4 | A one-line attribute disabling four of five denied lints |
| 5 | A fix written against the demonstrated route rather than the rule |
| 6 | A refusal shape the scanner could not see |
| 7 | **A test that asserts the implementation is itself** |

Pass 7's class is the sharpest because it is invisible to every gate. Two tests
recomputed the implementation's own expression in the test body and asserted
the result: `fill` then `seal` written by the test rather than driven through
`record`, and `edge * edge * passes` rebuilt from the same constants. Both
passed under a mutation that inverts the behaviour they exist to protect. HLD
27.2 R2 warns about a test written from reading the implementation, and this is
its sharper form: a test written from reading the implementation and then
*restating* it.

**What has converged.** The record and the planning data now carry almost no
transcribed numbers, because every count that could be replaced by the command
that prints it has been. That policy paid three times inside single sessions:
`grep -c 'limit='` read 13, then 14, then 15 while the passes ran, and the
escape count in one story title moved from four to five to at least eight.

**What has not.** The guard harness and the comparator's arithmetic are
adversarial surfaces, and a reviewer who invents a new input will keep finding
something. That is a property of the surface rather than a defect in the work,
and the honest statement is that these two areas are not finished, not that
they are broken. What is true of them today is that every bypass anyone has
demonstrated is closed, every closure has a probe that has been watched to
report `HARNESS` against its own broken guard, and every limit that could not
be closed is declared and printed on a green run.

## Pass 8, and the first clean area

**The comparator returned zero defects.** Its reviewer re-derived the movable
band from PS3.3 in exact rationals before reading the code, reproduced all
twelve mover rows and both new assertions, confirmed the rational refactor left
the four HLD 18.3 rows unchanged, and confirmed the MONOCHROME2 narrowing fails
loudly rather than selecting nothing. It said so plainly rather than
manufacturing a finding, which is the first time an area has been able to.

Its smells then produced a result stronger than the finding that prompted them.
The two empty rows in the mover table were said to be empty because "the
integers happen to fall" there. They are empty because **255 divides the
window**, which is exactly when the band's open upper endpoint is an integer,
and a band `509/510` of a unit wide ending open at an integer holds none. That
is now the assertion, over 43212 display values, and it is the sharpest form
the four-pass argument about 510 has taken.

**The lint policy took an eighth and a ninth route.** `--cap-lints allow` in
`rustflags` caps every lint including all five of HLD 27.1's, and was in
neither the refused set nor the declared out-of-scope list, so the OK line
positively asserted the false thing. And `[lib] path` in a member manifest puts
the crate ROOT anywhere, so the guard scanned a now-unused `src/lib.rs` and
never opened the real one, which is pass 5's defect through a third key. That
second fix is the first structural one in this sequence: the member's source
set now comes from `cargo metadata`'s own `targets[].src_path` rather than from
assuming `src/`, so the class closes rather than the route.

A tenth was found while fixing the eighth. The remediation was told to leave
`--force-warn` permitted "since it raises", measured it, and found it forces
the level to warn and outranks the `clippy` gate's own `-D warnings`. It
refused it and said so. That is the discipline working in the direction it is
hardest to work: against the instruction.

**Seven tests that cannot fail**, from a 68-mutation sweep across the crates
and the shell. The TypeScript fixture claimed to be copied from the Rust one
"character for character" and differed at the one byte that matters, giving it
the same level-equals-arity symmetry pass 4 had already fixed on the Rust side.
The panic code could be read out of the version word, latent only because both
are 1 today and live at the first version bump the layout is documented to
take. Deleting the GPU warm-up left the crate green, where its own comment
carries twenty measured runs showing the calibration then times first-submission
cost and the recorded fill rate collapses about seventeen times, which is
deviation D-07's misdetection arriving from the direction the resolver exists to
catch.

## Pass 9, and the two answers worth keeping

**The comparator was clean a second time**, and its reviewer derived the
`255 | w` result independently and found it stronger than stated: it holds for
every integer width, not only the twelve tabulated, because `u_end` lies in
`(1/255)Z` and the largest non-integer fraction available is `254/255`, which
is below `509/510`. So a band of that measure ending open at an integer holds
no integer exactly when the endpoint is one. The area has now been reviewed
twice with no defect found either time.

**The lint policy got a diagnosis instead of a tenth route.** Nine rounds in,
the reviewer named the class rather than the input: *"a file clippy compiles
that the guard does not read"*, and the guard still RECONSTRUCTS that set by
hand, now from three hand-picked keys rather than two. Every pass since the
fifth found a new key to the same set, and pass 9 found a fourth, `include!`.

rustc reports the exact set in its dep-info files, and the remediation
**measured four things and declined to use it**: a stale `.d` narrows in
silence, freshness by mtime refuses after every keystroke, making it fresh
costs a `cargo check --workspace --all-targets` measured at 10.8 seconds inside
every probe's sandbox on a gate that compiles nothing today, and a workspace
that does not compile yields no dep-info at all. So the reconstruction stays and
is now DECLARED as one: `member_sources`'s docstring, the OK line and
`docs/lld/guards.md` all say four keys have been found, in place of "Every
`.rs` file a workspace member compiles", which is what they said and which was
false. **A guard that says what it does not know is worth more than one that
overstates what it covers**, and that is the honest end of this sequence rather
than a claim that it is closed.

Two structural fixes did land. The cargo config is parsed with `tomllib` rather
than matched with a regex anchored at the start of a line, which was defeated by
TOML's dotted and quoted key spellings, and `NESTED_CASE`'s keyword list is now
derived from `SHELL_INTRODUCERS` rather than written a second time beside it, so
the two cannot drift again.

**And `guards-deep` moved onto every pull request.** Its trigger had been
justified twice, first by a clock and then by a toolchain, and neither survived
contact with `ci.yml`: the `guards` job that runs on every pull request already
installs the pinned toolchain, and the deep job added only a wasm32 target no
deep probe needs. The consequence had been that every `lint-policy` probe, the
ones this review kept finding holes in, was unwatched on the pull request that
would weaken them. It costs about 24 seconds on a runner that already has the
toolchain.

**HLD 17.2's rule was guarded by nothing.** `packages/core/src/bulk.ts` could be
rewritten into the exact shape its own header quotes as "the classic failure"
with `eslint` and `vitest` both green. The two guards cancelled: `bulk.ts` is in
`ALLOWED_TO_DISABLE`, correctly, because it is one of the two files permitted
to build the view, and there was no `bulk.test.ts`. `writeFrame` ships from the
published index. The test now makes the ordering assertable by growing linear
memory inside `alloc`, which detaches the previous buffer, so a view hoisted
above the allocation reads a buffer that no longer exists. Seven of its eight
cases go red against that shape and the eighth is a bounds case that needs a
memory which does not grow, which the tenth pass corrected in three places that
said "every assertion".

**Two files, not one, and F-005 is what made it two.**
`git show 36adc98:eslint.config.js` shows a single-entry allowance at the
sprint base, and `ALLOWED_TO_DISABLE` now holds
`["packages/core/src/bulk.ts", "packages/core/src/panic.ts"]` plus a separate
`["packages/core/src/*.test.ts"]`. `bulk.ts`'s own header still said it was the
only one until the tenth pass, which is S03 falsifying a claim inside the file
the claim is about, and the pass-9 commit message carries the same wrong
version.

## Pass 10, and two areas out of three are done

**The comparator is finished.** Three consecutive passes found no defect in its
substance, and the third derived the mutation's distribution independently from
PS3.3 in exact rationals and confirmed the accumulator produces what the
standard implies. It also explained the original failure in one line: the
repudiated `round(u - u/w)` is the same expression evaluated at `e = 0` for
every pixel, where `e` is the rounding residue the reference frame no longer
carries, which is why it degenerated into a threshold.

**The crates sweep converged.** Five survivors in ninety mutations, three of
them one-line fixture gaps, and `ocelli-core`, `ocelli-wasm/src/lib.rs`,
`state.mjs`, `page/app.mjs` and `examples/` came back explicitly clean. The one
genuinely new finding was pass 8's latent-constant class recurring on the
TypeScript side: `fatal.test.ts` could not fail on the code it names, because
`PANIC_FALLBACK_CODE` equals `ERROR_CODE.Panicked` and the only fixture used
that value, so both branches produced the same number.

**The guard harness has not converged, and the reason is now legible.** Pass 10
found a nested `case` inside BACKTICK substitution, and two `#[path]` spellings
the guard skips, `cfg_attr`-wrapped and raw-string. Neither is new ignorance:
the same file already matches through `cfg_attr` in `INNER_ALLOW` and already
handles the raw-string prefix in `INCLUDE_PATH` ten lines below `MODULE_PATH`.
**The failure is knowledge that did not propagate between three parsers in one
file**, which is why the fixes now derive one from another rather than restating
it: `NESTED_CASE`'s alternation comes from `SHELL_INTRODUCERS`, and all three
shell parsers share one `_quote_spans`.

**A ratchet digest moved for a prose edit.** The `Cargo.toml:workspace.lints`
capture backtracks to the last `key =` line before the next header, and an awk
snippet in a nearby comment matched, so the recorded value ran twelve lines past
the table. The lints table was byte-identical across that change. The fix
anchors the backtrack to a line start and the digest returns to the value the
table has had all along. **The point is not the bypass, it is that a ratchet
which moves for unrelated prose trains the next author to re-record on sight**,
which is the erosion the mechanism exists to prevent.

**Two probes were found not to discriminate**, the first such finding in the
sprint, and they were accept probes whose expected fragment was the OK line's
fixed prose. With `#[path]` following removed entirely both stayed green. Their
expects now carry the count the run derives, so the same mutation drives both to
`HARNESS`.

## Pass 11, and the class gets named

**Two areas were declared done.** The comparator has four consecutive passes
with no defect in its substance, and the crates and bench sweep two, with every
pass-10 figure reproducing to the digit: `left: 4`, `0 of 8` against `8 of 8`,
`7 failed | 1 passed`, and a 27-name published list that catches a deletion, an
addition and a de-callabling. The delivery record was checked against 62 further
numeric claims with no disagreement.

**The guard reviewer named the class.** Ten passes had been closing spellings,
and the diagnosis is one sentence:

> These two files parse three foreign grammars, TOML, Rust attributes and POSIX
> shell, with regexes and hand-rolled scanners, and each pass closes the
> spelling that pass happened to think of. Deriving one alternation from
> another makes two parsers agree. It does not make either of them agree with
> the GRAMMAR.

Three findings, three grammars, one defect. A QUOTED TOML key defeated
`LINT_ROW` and the ratchet meant to back it up, in a file whose own comment
sixty-four lines below names that spelling. `$( )` defeated the shell comment
scanner. And `[a-z-]` in two Python readers of the `GATES` array did not match
what bash accepts, so a gate named `prose2` would be selected by `gate --floor`,
run by no CI step, claimed by no catalogue entry, and counted by neither ratchet.

**And pass 10's fix for a fail-open had created one.** Adding `)` to the
comment word-start set was the right RULE, since a `#` begins a comment after an
unquoted operator, and the scanner could not tell an operator `)` from the one
closing `$(...)`. The proof is a parent-commit control: the same input fails
CLOSED at `e2b11d8` and OPEN at `a83dbae`, and the declared limit beside it
asserted the space was closed.

**So the remediation was structural rather than another alternation.** Both
`[workspace.lints.*]` tables are parsed with `tomllib`, which the same file
already used for cargo configs twenty lines away, and the declared constant now
CALLS the guard's own reader rather than matching its own regex, so the ratchet
and the guard cannot disagree. The shell scanner is one tokenizer with a span
stack over quote, backtick, `$( )`, `${ }` and here-document, replacing a
delimiter set plus three copies of a close rule that agreed only because one
commit had touched all three. The `GATES` array has one reader, shared by both
checks, tested against bash itself.

**What the remediation says is still unbounded, in its own words.** `ci.yml` is
read by hand-rolled YAML in `run_commands`, `workflow_events` and
`steps_running`, which is the same shape as the three that failed, and nobody
has planted an input at it. The statement-head policy is still an enumeration
and has been wrong four times, though a missed head fails closed. And the
declared limits are prose asserting properties of code, three of which were
false this pass, where the real answer is fewer sentences a mechanism could
carry instead.

## Pass 12, and a prediction a remediation made against itself

Pass 11's remediation ended by naming what it had not fixed: `ci.yml` is read by
hand-rolled YAML, it is the same shape as the three that failed, nobody has
planted an input at it, and pass 12 should expect to find one. **That is what
happened**, and the discipline in how it was found is worth as much as the
finding. Every planted input was parsed with a real YAML library and the
resulting structure printed, and three routes the reviewer could not establish
GitHub accepts, anchors, multi-document files and a Unicode line separator, it
declined to report at all.

**Four fail-open routes, and the first is not a spelling.** `run_commands`
attached whatever `if:` it had seen SO FAR to a `run:`, so writing the condition
AFTER the command hid it, and the control is the same two lines reversed, which
refuses. **Key order is the entire difference and a YAML mapping has no key
order.** A folded `run: >` split one command into two and defeated the argv
equality pass 7 added, by one character. A quoted key in `on:` narrowed the
event set the floor is checked against. And a gate name inside ANY `run:` text
satisfied the gate, so `echo "... run bin/ocelli.sh gate panic locally"` stood in
for HLD section 23's wasm panic-hook proof.

**Five LEGITIMATE spellings were refused**, one of them with a message quoting a
condition that plainly does run on the events it said it did not.

The remediation argued the dependency rather than assuming it, and the argument
is the one that settles it: the fail-closed alternative closes three routes,
provably leaves key order open, and three of the five spellings it would have to
refuse are the ones already being wrongly refused, so it makes a guard refusing a
legitimate state permanent BY DESIGN at the moment it leaves the widest route
open. `BaseLoader` turned out to be the more restrictive loader and it retires
the `(?:on|"on"|'on'|true)` alternation rather than carrying it forward.
Deviation **D-17** records a floor gate acquiring a third-party import.

It then found a fifth route of its own, an unterminated flow sequence no parser
accepts and therefore a workflow GitHub cannot run, read as all twenty-five
gates invoked. It found **three existing probes planting a workflow GitHub
Actions rejects**, which only a parse could see. And it wrote a span-mask
defence and then deleted it, because with the match anchored at a statement head
no probe could reach it: **a defence no probe can reach is a comment claiming a
property.**

**The shell half then went the other way, and the reasoning is the interesting
part.** The obvious move was to ask bash, since `bin/ocelli.sh` is bash and the
`GATES` array already has a test that does. The remediation measured three
things and refused: `declare -f` needs the function DEFINED, which needs the file
EXECUTED, and the catalogue plants adversarial shell into that very file inside a
disposable clone, so a reader that asked bash would turn "this file is misparsed"
into "this file is run". Its output is a pretty-printer with no stability
contract that differs between the two bash versions on this machine. And it
normalises neither of the two spellings that were failing open.

**So bash is not the reader, bash is the test oracle**, comparing what bash
PRINTS from a synthetic arm against what the scanner attributes to it. That
found something nobody had asked about: the backtick's `opens` flag was wrong in
the fail-open direction, and the scanner was accepting a file bash refuses.

The last hand-written production, the here-document delimiter as a regex with an
invented character class, is gone. It is a shell WORD now, read by the same rule
that reads the `GATES` array.

## What is still open, and it is declared rather than hidden

- **G-02 and G-04**, the two remaining declared guard holes. F-X014.
- **Bash's compound-command grammar is not modelled**, so the shell scanner
  cannot tell a subshell `(` from a `case` pattern's, nor `((` arithmetic from
  `( (`, which bash resolves by attempting the arithmetic parse and backtracking.
  Every consequence is fail-closed and asserted where the refusal carries the
  weight. That is one unmodelled thing rather than a list, which is the
  difference between this and the eleven passes before it.
- **`GpuContext::supports_compute` is a forwarder no test reaches.** The
  decision it forwards to is in `caps.rs` with a six-row truth table, and
  reaching the forwarder needs a real device, which deviation D-04 leaves the
  floor without. Declared at the site and in `docs/lld/gpu-ownership.md` and
  listed here because the eleventh pass pointed out that this list opens with
  "it is declared rather than hidden" and this one was declared elsewhere.
  F-037 or F-X002.
- **`close-preflight` reports on a stale source.** It reads each feature's
  last per-feature review, all recorded at pass 1 before nine sprint-level
  passes and nine remediation commits, and its verification check accepts a
  recorded sprint profile with no tree hash. Its green is true about a
  different question. F-X018, and the mitigation is that `/close-sprint` runs
  `gate --sprint` itself.
- **A device is requested on one adapter and no other is tried**, so a
  host with a broken driver beside a working one resolves a tier that
  renders nothing. F-X016, and the cost is the evidence design rather
  than the loop.
- **Measured escapes from the wasm linear memory view ban**, and the count is
  deliberately not written because it moved three times in two passes. The
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
