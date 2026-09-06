# The benchmark harness

**F-IDs that contributed:** F-006
**Last updated:** 2026-09-06

HLD `docs/hld/23-performance-rules.md` section 26 ends with a rule that names an
instrument:

> Measure with the benchmark harness before optimising anything. The intuitions
> that work in JavaScript do not transfer.

Until the harness existed the rule was unenforceable, because there was nothing
to measure with, and a rule with no instrument is satisfied by whoever is
confident. `tools/bench` is the instrument. This file is the design behind it.

**It is an instrument and not a report.** One of its eleven subjects has a
runner and a recorded number. Nine are blocked on a story that has not landed,
because there is no decoder, no renderer, no worker and no boundary, which is
decision D7 holding: the oracle and the instruments exist before the port code.
The eleventh, `tier.startup_microbenchmark`, is the awkward one and is written
down rather than rounded off: F-004 landed in this same sprint, so its story is
`done` and it still has no runner. The harness says so, names the story a reader
should go and read, and records no number for any of the ten.

**`bin/ocelli.sh bench --list` is the authority on that split and this
paragraph is not.** It reports each subject's blocking story and that story's
current backlog status, so it moves when the backlog moves. A count written
here goes stale the first time a story lands, which is exactly what happened to
the sentence this one replaces.

## The defect it exists to prevent

A benchmark harness under time pressure invents a workload, produces a plausible
number, and that number then sits in a tracked file describing nothing. That is
this project's "quietly wrong" shape moved from pixels onto cost, and it is
worse in one respect: a wrong pixel can be diffed against an oracle, and a wrong
duration cannot be diffed against anything.

Discipline is not a mechanism against it, so two things are refused
mechanically by `scripts/bench_check.py`, which is the `bench` gate:

1. a runner file for a subject whose story has not landed, because a runner for
   a subject that does not exist can only be timing a stub, and
2. an entry in `ci/bench-baseline.json` for such a subject, which is where an
   invented number would come to rest.

## The HLD states no performance target, and this harness invents none

Searched rather than assumed. Every file under `docs/hld/` was read for `ms`,
`fps`, `latency`, `budget`, `cold start`, `throughput`, `frame rate`,
`benchmark` and the rest. The complete set of numeric figures the HLD states
that bear on cost at all is five, and not one is a target this harness can pass
or fail against: an explicitly unmeasured binary-size estimate in gate A4, a GPU
buffer limit and a series size in section 7 which are two figures rather than
one, a memory budget in section 8 that belongs to the caller, and a uniform
block size in section 26 that is an argument for a technique.

The only budget-setting method written down in this repository is spike gate
A7.3's, and it is relative to the incumbent viewer:

> Do not invent a number. Ask the platform team for the density target, sessions
> per physical core, and derive from it.

So this harness compares a figure against a figure **it recorded**, on the
machine that recorded it, and against nothing else.

## Layout

| Path | What it is |
|------|-----------|
| `tools/bench/subjects.json` | tracked, the subject registry. One row per thing this project will ever measure |
| `tools/bench/run.mjs` | the driver, and `--help` says what each flag does |
| `tools/bench/src/paths.mjs` | every path the harness resolves, and whether it is the script node was asked to run |
| `tools/bench/src/registry.mjs` | parse, and resolve each row's blocking story |
| `tools/bench/src/state.mjs` | the four states, and the one that throws |
| `tools/bench/src/hostclass.mjs` | the fingerprint and the comparability rule |
| `tools/bench/src/record.mjs` | the run record, the comparison, the re-baseline |
| `tools/bench/src/runners/` | one file per subject that has a runner. A row whose `subject_story` is `null` must have one. A row whose story has landed may not have one yet, which is `no_runner` |
| `tools/bench/page/` | the page that times a cold start, `window.__bench` |
| `tools/bench/tests/` | six suites, and `npm test` in `tools/bench` and the `bench` arm of `bin/ocelli.sh gate` now name the same six. `grep -o 'tools/bench/tests/[a-z_]*\.mjs' bin/ocelli.sh \| sort -u \| wc -l` prints what the arm names and `ls tools/bench/tests/*.mjs \| wc -l` prints what exists. The one browser test inside `cold_start_test.mjs` is opted into with `OCELLI_BENCH_BROWSER=1` |
| `ci/bench-baseline.json` | tracked, the recorded measurement per subject per host class |
| `scripts/bench_check.py` | the `bench` gate |
| `tools/bench/out/` | ignored. One run's record and the page it served |

## The subject registry, and why the rows exist before the subjects do

`tools/bench/subjects.json` carries one row per thing this project will ever
measure. Each names the story that gives it a subject, the specification
sections that define it, and **where the measurement starts and stops, in
words**.

The definition is fixed now, from the specification, for every row including the
ones with no subject. That is the part that stops a rewrite later.
`interaction.window_level_drag` is defined against sections 5.1 and 5.3 as
pointer event to ring drain, so the story that lands the runner has no latitude
to redefine it as something easier to measure. `render.first_frame` is defined
against section 7 as command to first presented-frame ring event, which is not a
pixel read-back, because decision D3 forbids one and a read-back would also
measure something no user experiences.

Adding a measurement when its story lands is a data edit plus one runner file.
No part of the driver, the record format or the gate changes.

**Every `subject_story` is resolved against `docs/sprints/allocation.json`
rather than copied from a comment**, and the gate enforces that. The reason is a
mistake this repository has already made once. Comments across the tree named
**F-096** as the story that builds the boundary, and F-096 is E15.3,
"Measurement persistence and serialisation format", in S36. E16.2 is **F-101**,
in S16. The cluster is off by five, which is the signature of an F-numbering
carried over from before five stories were inserted.

**Those comments were corrected in S03, by commit `d74ad3a`, and every live one
of them now names F-101.** Six occurrences of the old number survive in the
code tree, in `bin/ocelli.sh`, `crates/ocelli-wasm/Cargo.toml`,
`scripts/bench_check.py`, `tools/bench/src/registry.mjs` and two blocks of
`tools/bench/subjects.json`. Every one of them is an account of the past error
rather than a live attribution, so correcting the number there would falsify an
accurate record. Count them rather than trusting this sentence:

```bash
grep -rn F-096 bin/ ci/ crates/ scripts/ tools/ .githooks/
```

`docs/sprints/AS_BUILT.md` keeps its own instances for the same reason, since
it is append-only history and no story edits it. What this
story does instead is make the class of mistake unrepeatable in the registry: a
`subject_story` that names no real F-ID fails the gate, and one that names the
wrong real F-ID is why the rows carry a definition and a specification citation
beside the story number rather than the number alone.

## The four states, and the one that throws

| State | Meaning |
|-------|---------|
| `measured` | a runner exists, it ran, and the record carries a number and a host class |
| `unavailable` | there is no number, and the record says why. When a story blocks the subject the record names that F-ID |
| `incomparable` | a number exists and the recorded baseline was taken on a different host class or with a different instrument, so no comparison is made |
| refused | a number for a subject whose story has not landed. Not a state, an exception, and the gate refuses the tracked half of the same mistake |

`unavailable` carries a reason as well as a state. `no_subject` names the
blocking story. `no_runner` means the subject exists and nobody has written the
runner yet, which is the honest state between a story landing and its runner
arriving. `runner_failed` carries the failure rather than swallowing it.

**A `no_runner` row does not fail the gate**, deliberately. Requiring a runner
the moment a story is marked done would fail the CI floor on the day an
unrelated story completes, for a reason nobody in that story can act on. What
the gate requires is a runner for a row whose `subject_story` is `null`, which
is the registry saying the subject exists today. **A row whose story is DONE
and whose runner has not been written is not that case**, and the wording here
used to read as though it were, which would have made
`tier.startup_microbenchmark` a gate failure the day F-004 landed. It is
`unavailable` with reason `no_runner` instead, which is what the opening
section calls the awkward eleventh row.

## The state of every subject at merge

| Subject | State | Blocked on |
|---------|-------|------------|
| `wasm.cold_start` | **measured** | nothing, this is the deliverable number |
| `decode.frame` | unavailable | F-023 (E4.1), codec dispatch and registry, S07 |
| `decode.transfer_syntax.htj2k` | unavailable | F-027 (E4.5), S09, a P0 kill criterion |
| `decode.transfer_syntax.jpegls` | unavailable | F-028 (E4.6), S09, a P0 kill criterion |
| `render.first_frame` | unavailable | F-038 (E6.2), S12, and F-037 (E6.1), S11 |
| `interaction.window_level_drag` | unavailable | F-101 (E16.2), the boundary and event ring, S16 |
| `interaction.scroll` | unavailable | F-101 (E16.2), S16 |
| `session.cpu_interactive` | unavailable | F-X003 (X1.3), S16 |
| `session.cpu_idle` | unavailable | F-X003 (X1.3), S16 |
| `cine.frame_change_rate` | unavailable | F-X003 (X1.3), S16 |
| `tier.startup_microbenchmark` | unavailable, reason `no_runner` | nothing. F-004 (E1.4), S03 is **done**, and no runner has been written |

`docs/sprints/allocation.json`'s note on F-006 is "Numbers feed the P0 kill
criteria". Exactly two stories carry that marker, F-027 and F-028, both are
codec questions and both are about decode, so the numbers that marker points at
are the three decode rows. First frame and interaction latency feed no kill
criterion this repository has written down.

## The one measurement this story took

**`wasm.cold_start`, 2.3 ms**, recorded on 2026-09-05 with a 25 per cent
tolerance. `ci/bench-baseline.json` carries the figure, the host class, the
instrument versions, the artefact's digest and the reasoning for both the value
and the tolerance.

Appendix A gate A4 has two halves. F-002 measured the first, recording 14,104
bytes, which F-005 re-baselined to 16,388. **The second half, cold start, had
never been measured at all.** This is that half, and it is not a fabricated
workload: it is the actual release artefact doing the actual thing a browser
does at session start.

**It is not an answer to gate A4 and must not be read as one.** The same caveat
`docs/lld/build-targets.md` puts on the size number applies here unchanged. A4
estimates 3 to 8 MB uncompressed with Naga dominating, and this module holds
four exported functions, no wgpu and no Naga. **A4 stays open.** The figure is a
regression baseline for the build-out phase, and re-baselining is expected
repeatedly, each one naming its design plan.

### How it is taken

Release profile only. HLD section 15.2's `opt-level = "z"`, fat LTO,
`codegen-units = 1`, `panic = "abort"` and `strip = true` apply to release, so a
dev-profile cold start is a different number rather than a smaller one. The
runner builds the artefact through `bin/ocelli.sh wasm`, which is the same
command the size gate uses, and `--accept` refuses `--no-build` so a baseline
can never be recorded against whatever happened to be lying in `pkg/`.

Five phases are timed separately, because a total on its own hides which half
moved: the wasm-pack glue's module script, the fetch of `ocelli_wasm_bg.wasm`,
`WebAssembly.compile`, `initSync`, and the first `ocelli_version()` call. A
module that grew would move `compile`. A module that gained an import would move
`instantiate`.

Sixteen iterations, each in a **freshly launched Chromium**, with the first
discarded. A fresh browser and not a fresh page: a compiled WebAssembly module
is cached by the browser process, so a second page would report a compile phase
that measured a cache lookup. Sixteen is measured rather than chosen: at six,
the median swung between 2.2 and 2.9 ms across ten consecutive runs, and at
sixteen it sat between 2.3 and 2.5 ms across eleven, on a machine carrying a load
average near six. The whole subject costs about two seconds either way.

**That calibration is recorded in two places and they disagree**, which the S03
review's fourth pass found. The paragraph above and the comment beside
`ITERATIONS` in `tools/bench/src/runners/wasm_cold_start.mjs` both say eleven
runs between 2.3 and 2.5 ms. `ci/bench-baseline.json`'s
`tolerance_provenance` said fifteen runs between 2.2 and 2.5 ms. Neither can be
recovered after the fact, so the disagreement is recorded in that field rather
than resolved by preference, and the recorded 25 per cent tolerance covers
either. The next story to re-baseline this subject takes its own calibration
and replaces both accounts with one.

### What it does not measure, stated plainly

A user's real cold start. The artefact is served from loopback, so the fetch
phase is a lower bound and nothing else, and the discarded warm-up means the
figure is taken with the artefact already in the operating system's page cache.
The record says both, in `detail.not_measured`.

## Timing is taken from outside the core, never by instrumenting it

The obvious implementation is a `#[wasm_bindgen] pub fn bench_mark()` in
`ocelli-wasm` and an `Instant::now()` in each crate. Both are refused.

- A timing export in `ocelli-wasm` adds a boundary function that exists for the
  harness and ships to every user, and F-101 (E16.2) owns what the boundary
  exports.
- `std::time::Instant` does not work on `wasm32-unknown-unknown`, so a Rust
  timing primitive would need a browser branch, which is browser-specific code
  in a core crate and is precisely what decision D2 exists to prevent.

The browser half times with `performance.now()` from the page, outside the
module. A future native half times with `Instant` in `tools/bench`, outside the
crates. **No file under `crates/` was modified by this story.**

### The rounding decision

`performance.now()` in this Chromium is quantised to 0.1 ms, so every reading is
a multiple of 0.1. Subtracting two of them in binary floating point turns 2.2
into 2.2000000029802322, and recording that would claim thirteen digits of
precision the clock does not have. Every duration is rounded to four decimal
places, which is a thousand times finer than the quantum and therefore cannot
move a figure across a tolerance boundary. It removes the arithmetic's artefact
and nothing else.

## The host class, and why a duration needs one

A byte count is deterministic and any machine can check it. A duration belongs
to a CPU, an operating system build and a browser build. Comparing one machine's
milliseconds against another's is the numeric equivalent of the bit-exactness
claim decision D14 already refuses for pixels.

So every recorded figure carries a host class, and a comparison against a
baseline recorded under a different one is **refused** rather than made. A
harness that quietly compared across machines would report a regression on a
slower laptop and an improvement on a faster one, and both would be noise
wearing a verdict's clothes.

| Field | Why |
|-------|-----|
| `platform`, `release`, `arch` | the oracle's `run.json` already carries these three |
| `cpu_model`, `cpu_count`, `memory_bytes` | it carries none of these, and they are what a duration needs |

That is the one place this harness extends rather than reuses what F-010 built.
F-011 may read the fingerprint. It shares nothing else with F-006: a correctness
verdict and a cost verdict answer different questions and fail for different
reasons, and coupling them would let a slow machine make a pixel diff look like
a regression.

**Load average is recorded and is not part of the class.** It changes between
two runs on one machine, so putting it in the class would make every run
incomparable with every other, which is a comparison mechanism that never
compares. It is recorded because a figure taken on a loaded machine is worth less
than the same figure taken on an idle one, and a reader can only know that if the
number is there.

**The instrument is compared separately.** Node, playwright and the Chromium
build. The same machine running a different Chromium produces a different cold
start for a reason that has nothing to do with the artefact, so a mismatch there
is `incomparable` exactly as a host-class mismatch is.

## The baseline, and the tolerance

`ci/bench-baseline.json` is **a map keyed on host class from the start**, and
not a scalar. Migrating a scalar to a map later rewrites every recorded entry,
and a recorded measurement that gets rewritten stops being a measurement.

Spike gate A7.3 names the mechanism to copy: gate on regression beyond a
tolerance, "the same mechanism `ci/wasm-size-budget.json` already uses for binary
size". So the tolerance is applied **on both sides**. An unexplained improvement
fails as loudly as an unexplained regression, because during build-out the check
does not mean "you exceeded a budget", it means **"the figure moved and the move
was not declared"**, and a runner that started timing less work reads as an
improvement.

`--accept` re-baselines, and it requires `--story` and `--why`. A new tolerance
requires `--tolerance-why` of its own, because `ci/tier-thresholds.json` states
the provenance of each figure separately, saying which is measured, which is
derived and which is absent rather than guessed, and a tolerance is a derived
figure like any other.

## Where it runs, and the two mechanisms that are deliberately separate

This is deviation D-04's problem restated for durations, and it is worse than
D-04's, because a shared CI runner can execute a benchmark and produce a number
that means nothing.

**A floor gate, `bench`, that asserts the instrument and never a duration.** It
costs no GPU and no browser, it is deterministic, and it cannot be satisfied by
intention. `scripts/bench_check.py` asserts that every registry row parses, that
every `subject_story` resolves to a real F-ID, that every row naming no story has
a runner and every row whose story is not done has none, that no runner file
exists without a registry row, that no baseline entry exists for a subject whose
story is not done, that every baseline entry names a host class and a tolerance
with its provenance, and that the two harnesses' playwright pins are equal.

**The node suites carry one assertion the Python gate does not.** `INCREASE_MEANS`
in `tools/bench/src/record.mjs` and the units declared in
`tools/bench/subjects.json` are asserted to be the same set, in both directions.
Before that binding, changing a subject's unit to a word `INCREASE_MEANS` does
not know left `bin/ocelli.sh gate bench` at exit 0, because the throw fires
inside `--compare` and only on a machine that already owns a baseline for that
subject. It is the same shape as `the_recorded_bands_match_the_checked_in_file`,
a constant in source asserted equal to the checked-in file it claims to follow.

That gate then runs the guard's own negative cases and the node suites, all of them since `828037e` added `paths_test.mjs` to the arm. This sentence said five while the arm named six, which is the third stale count this file has carried about the same list, so the arm is named rather than counted here too.

**Three decisions the node suites state in prose were held by nothing until the
seventh review pass**, and all three were found by mutating the source rather
than by reading it.

- **`subjectExists` is `status === "done"` and not "not pending".**
  `resolveSubjects` was only ever driven with `done` and `pending`, so
  `in-progress`, `archived` and `superseded` never reached it and the two
  spellings were indistinguishable. Reading either of the last two as delivered
  is exactly the `REFUSED` state `src/state.mjs` calls the defect this harness
  is most likely to produce, and a superseded story is one whose subject was
  replaced rather than built. Every status now goes through it against a table,
  and the authority for that table is `scripts/bench_check.py`, which refuses a
  runner file and a baseline entry on `status != "done"` twice over.
- **The comparison reads the RECORDED unit and not the run's.** Nothing built a
  run whose unit differed from its baseline's, so the two were the same value.
  The two units in the test now have opposite polarity in `INCREASE_MEANS`, so
  taking the wrong one prints a duration that rose as an improvement.
- **A figure exactly on the tolerance is inside it.** The existing case stepped
  to either side of a 0.1 tolerance and never onto it. The boundary follows the
  mechanism A7.3 names: `scripts/pin_and_size_check.py` refuses on
  `size > ceiling`, so a figure at the ceiling passes there, and a tolerance
  stated as a fraction that excluded its own value would mean a 10 per cent
  tolerance admits less than 10 per cent.

`sameInstrument` gained the fourth. It compares the UNION of both sides' keys,
and only one direction was covered: a key the run carries and the baseline does
not. The other direction is the one that happens, because the baseline is the
older record, so a baseline naming a tool this run never captured would have
compared as identical and been given a verdict. The parallel guard on
`sameHostClass` was already covered both ways.

**The fifth arrived in the ninth pass, in `backlogStatuses`.** The status-table
heading rule is `/^M\d+,/` OR a heading starting `Roadmap`, and every fixture in
`registry_test.mjs` used an `M<n>` heading, so dropping the `Roadmap` clause
left every test in the harness green while `docs/sprints/BACKLOG.md` carries
`### Roadmap, Phase 2 and Phase 3` with F-158 and F-159 under it. The rule is
duplicated in `scripts/bench_check.py` and `scripts/backlog_check.py`, and this
module's header says a rule that exists on one side only is a defect, so a
Roadmap-blocked subject would have been refused by the driver and accepted by
the gate. **The failure is loud rather than quiet**, because the driver throws
on a story it cannot resolve instead of labelling one wrongly, and that is what
caps the severity rather than what makes the clause optional. Two tests now hold
it, one asserting a Roadmap row IS read and one asserting a heading that is
neither shape is not, because a clause satisfied by reading every `### ` heading
would be no clause.

**The path rules decide whether the harness measures anything, and nothing
executed them until the S03 review's eighth pass.** `run.mjs` asks
`existsSync(runnerPath(id))` for every subject, so a `runnerBasename` that
stopped replacing dots with underscores makes that false for the one subject
that has a runner. Every subject then reports `unavailable / no_runner`,
`failedRunners` is empty, `main` returns 0, and `bin/ocelli.sh bench` reports
success having measured nothing. `isEntryPoint` produces the same outcome one
level up, which is what its own comment describes. Both rules, and the two
shapes that break the obvious `import.meta.url === argv[1]` idiom, a path
needing percent-encoding and a path reached through a symlink, are asserted in
`tools/bench/tests/paths_test.mjs`.

The convention itself is the registry's. `subjects.json` states it in the `id`
field's documentation and `scripts/bench_check.py` applies the same rule from
Python, so it must be spelled once on the JavaScript side: `state.mjs` builds
its `no_runner` note from `runnerBasename` rather than from a second
`replaceAll(".", "_")`, because a note naming a path the gate does not look at
is the unseen runner the anti-fabrication rule is about.

**Six, and the count has moved twice, which is why it is written with the
command that prints it rather than as a number.** It reached five in the S03
review's fourth pass and six in the eighth, when `paths_test.mjs` joined the
arm in the same commit that added the paragraph above. A count in prose beside
a list in a shell script goes stale the next time the list changes, and it did.
`grep -o 'tools/bench/tests/[a-z_]*\.mjs' bin/ocelli.sh | sort -u | wc -l` is
what the `bench` arm names, and `ls tools/bench/tests/*.mjs | wc -l` is what
exists. They agree today, and the whole point of the two commands is that a
reader can see whether they still do.

The fourth pass was `cold_start_test.mjs`.
It holds seven tests and only the last
needs a browser, but `wasm_cold_start.mjs` imported `playwright` at module
scope, so the file could not be loaded at all without a playwright install and
the whole suite sat outside the gate. `workspaceVersion`, `median`,
`atClockPrecision`, which is this story's one stated rounding decision, and
`resolveServedPath`, which refuses `/..`, `/%2e%2e/`, a malformed percent
escape and a sibling directory sharing a prefix, were watched by nothing. The
runner now takes playwright with `await import` inside `run()`, and the browser
test is opted into with `OCELLI_BENCH_BROWSER=1`, which
`npm run test:browser` in `tools/bench` sets. The gate still needs no browser.

**The page's own arithmetic joined the floor the same way, in the seventh
pass.** `tools/bench/page/app.mjs` computes the five phase durations from six
marks, and that loop was reachable only through the browser test the floor
skips, so nothing observed it. The relationship it depends on,
`PHASES.length === marks.length - 1`, was stated nowhere and asserted nowhere,
and a mark added without a phase name beside it leaves the TOTAL correct while
shifting every label onto a neighbour's duration. That defeats the reason the
five phases exist at all: a module that grew is supposed to move `compile` and
nothing else, so a misaligned label sends a reader chasing the wrong half. The
arithmetic is now `phaseTable`, exported and pure, it refuses a mark count that
does not match the phase list, and the one line needing a DOM is guarded on
`typeof document` so the module imports outside a browser. That is the same
one-line trade the runner already makes with `await import`, and the page
behaves identically because `document` is always defined in a browser.

**A comparison that is not a gate.** `bin/ocelli.sh bench` runs what it can and
writes the record. `bin/ocelli.sh bench --compare` compares against the baseline
for the matching host class and reports `incomparable` on any other machine.

The comparison is deliberately not in `--floor`, not in `--sprint` and not
required by `/verify --profile release`. A duration comparison on a machine that
did not record the baseline is either noise or a skip, and this project's rule is
that a skip is not a pass. Putting it in the floor would produce a permanently
amber gate, an amber gate is a gate that gets disabled, and disabling a gate is a
change to `.claude/WORKFLOW.md`. It would also be a named exception to the
skip rule, which would need a new deviation row for no gain.

## Two playwright installs, held equal

`tools/bench` has its own `package.json` and its own `node_modules`, pinned at
exactly the version `tools/oracle` pins, and `scripts/bench_check.py` asserts the
two are equal.

Sharing one install would couple the benchmark's story to the comparator's. Two
installs cost two pins that can drift, and a drifting browser build between them
would silently make the two harnesses report different environments, which is
exactly the kind of difference a host class exists to catch. The equality is
therefore a mechanism rather than something to remember.

## Tiers

The harness resolves no tier and requires none, **and nothing in it records
one, so no comparison is scoped by tier today.** An earlier sentence here
claimed that a tier-A baseline is never compared against a tier-B or tier-C
run, and the harness has no mechanism that could make that true.
`hostClass()` returns `platform`, `release`, `arch`, `cpu_model`, `cpu_count`
and `memory_bytes`, `sameHostClass` and `sameInstrument` compare only those,
and `acceptRecord` stores `host_class`, `instrument`, `conditions` and
`detail`. The only `tiers` field anywhere is the registry's declared
applicability list, copied into the run entry by `state.mjs` and never
compared. Check rather than trusting this paragraph:

```bash
grep -n tier ci/bench-baseline.json
```

**So this is a requirement on the story that lands the first tier-bearing
runner, and not a property of the instrument.** Adding the resolved tier to the
host class, or to a comparison key beside it, is that story's design decision,
because the runner that reports a tier is the first thing that can put one in
the record. Until then a subject measured on two tiers on one machine would be
compared as if it were one population, which is why the three tier-C rows and
`tier.startup_microbenchmark` below have no runner rather than an approximate
one.

`session.cpu_interactive`, `session.cpu_idle` and `cine.frame_change_rate` are
tier-C rows and they exist from this story, because `docs/spikes/A7-tier-c.md`
says F-006 "must measure tier C from the start, including CPU cost per session".
Idle is a separate row from interactive because A7.3 says they answer to
different people and only one is negotiable. All three are `unavailable` and
blocked on F-X003, because `docs/spikes/GATES.md` makes the two remaining A7
figures acceptance criteria on F-X002 and F-X003, and a story that quietly took
one would move a resolved spike's acceptance criteria without saying so.

`tier.startup_microbenchmark` is F-004's, recorded here and not reimplemented.
They are two artefacts sharing a word: F-004's runs on every production session
and returns a tier, this one is a development instrument that returns a recorded
number. The runner, when it is written, invokes F-004's own instrument and reads
`crates/ocelli-render/src/probe.rs`'s output.

## What is deliberately absent

**No `criterion` and no new CARGO dev-dependency**, and the qualifier is not
pedantry: F-006 did add a node one. `tools/bench/package.json` and its lockfile
are this story's, and they pin playwright at exactly the version `tools/oracle`
pins, which `scripts/bench_check.py` asserts. What is untouched is the cargo
side. There is no Rust subject to measure, `AGENTS.md` refuses a construct with
no user today, and a dev-dependency that is not wasm32-portable breaks
`cargo check --all-targets` on that target, which is how `proptest` reaching
`wait-timeout` already constrains this workspace. F-023 and F-024 add the Rust
runner and the cargo dependency it needs, with a named user.

**No new trait and no new generic.** The runner lookup is a file path derived
from a subject id, and the two implementers a trait would need do not exist.

**No hand-computed pixel fixture, and it is named rather than omitted.** HLD 27.2
R3 requires one for every function doing pixel arithmetic. This harness computes
no pixel, no coordinate and no LUT value, and it reads none of the corpus. An
omitted row and a deliberate "no arithmetic here" read identically six months
later.

**No parity surface.** `docs/hld/B-parity-surface.md` enumerates viewport types,
tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation
representations, core events and adapters. A benchmark harness is on none of
those axes.
