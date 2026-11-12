# The benchmark harness

**F-IDs that contributed:** F-006, F-023, F-024, F-026, F-038, F-039, F-050, F-101, F-X003, F-X014, F-111, F-216, F-223, F-224, F-225, F-227, F-239, F-240, F-243, F-245
**Last updated:** 2026-11-11

HLD `docs/hld/23-performance-rules.md` section 26 ends with a rule that names an
instrument:

> Measure with the benchmark harness before optimising anything. The intuitions
> that work in JavaScript do not transfer.

Until the harness existed the rule was unenforceable, because there was nothing
to measure with, and a rule with no instrument is satisfied by whoever is
confident. `tools/bench` is the instrument. This file is the design behind it.

**It is an instrument and not a report.** `bin/ocelli.sh bench --list` is the
authority on each subject's blocking story and that story's backlog status.
It moves when the backlog moves. A count of runners or recorded numbers written
here goes stale the first time a story lands.

## The defect it exists to prevent

A benchmark harness under time pressure invents a workload, produces a plausible
number, and that number then sits in a tracked file describing nothing. That is
this project's "quietly wrong" shape moved from pixels onto cost, and it is
worse in one respect: a wrong pixel can be diffed against an oracle, and a wrong
duration cannot be diffed against anything.

Discipline is not a mechanism against it, so `scripts/bench_check.py`, which
is the `bench` gate, refuses:

1. a runner file for a subject whose story is pending, archived or superseded,
   because a runner for a subject that does not exist can only be timing a stub,
2. an entry in `ci/bench-baseline.json` for a subject in one of those three
   statuses, which is where an invented number would come to rest, and
3. a missing runner on a done story, and on a row that names no story. A done
   story requires the permanent instrument. A row that names no story exists
   today and must have one.

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
| `tools/bench/subjects.json` | tracked, the subject registry. One row per baseline subject |
| `tools/bench/run.mjs` | the driver, and `--help` says what each flag does |
| `tools/bench/src/paths.mjs` | every path the harness resolves, and whether it is the script node was asked to run |
| `tools/bench/src/registry.mjs` | parse, and resolve each row's blocking story |
| `tools/bench/src/state.mjs` | the four states, and the one that throws |
| `tools/bench/src/hostclass.mjs` | the fingerprint and the comparability rule |
| `tools/bench/src/record.mjs` | the run record, the comparison, the re-baseline |
| `tools/bench/src/runners/` | one file per subject that has a runner. A row whose `subject_story` is `null` must have one. A done story must have one. An in-progress story may lack one, which the harness reports as `no_runner` |
| `tools/bench/page/` | the page that times a cold start, `window.__bench` |
| `tools/bench/tests/` | node suites named exactly by both `npm test` in `tools/bench` and the `bench` arm of `bin/ocelli.sh gate`. The already-running path suite compares both lists with the files on disk, so adding an unregistered suite fails. The browser test inside `cold_start_test.mjs` is opted into with `OCELLI_BENCH_BROWSER=1` |
| `ci/bench-baseline.json` | tracked, the recorded measurement per subject per host class |
| `scripts/bench_check.py` | the `bench` gate |
| `tools/bench/out/` | ignored. One run's record and the page it served |

## The subject registry, and why the rows exist before the subjects do

`tools/bench/subjects.json` carries one row per baseline subject. The explicit
two-viewer mode beside it is defined by `tools/bench/comparison.json`. Each names the story that gives it a subject, the specification
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
A new unit is also an edit to `INCREASE_MEANS`, because a unit the table does
not name fails the record test. F-216 added `frames_per_second`, which is
better when it rises, and `bytes`, which is worse when it rises. The driver
accepts subject ids as positional arguments so one command can measure a
named subset. `--subject` still limits only the entries `--accept` writes.

The driver is also guarded directly. `run_test.mjs` reaches every argument
refusal in `parseArgs` and the refusal of one pending runner. The message
requires that story to be in progress or done. `runSubjects` is the exported
production seam used by that test, with no
test wrapper or duplicate decision path. The exact-list assertion above makes
the new suite part of both executable definitions of the bench tests.

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
| refused | a number for a pending, archived or superseded subject. Not a state, an exception, and the gate refuses the tracked half of the same mistake |

`unavailable` carries a reason as well as a state. `no_subject` names the
blocking story. `no_runner` means the subject exists and the runner file is
absent. `runner_failed` carries the failure rather than swallowing it.

**A missing runner fails the gate when the row names no story, and when the
story is done.** An in-progress story may still lack a runner. A pending,
archived or superseded story must not have one. The refusing branch is
`check_runners` in `scripts/bench_check.py`.

`docs/sprints/allocation.json`'s note on F-006 is "Numbers feed the P0 kill
criteria". Exactly two stories carry that marker, F-027 and F-028. Both are
codec questions and both are about decode. First frame and interaction latency
feed no kill criterion this repository has written down.

## The cold-start measurement

**`wasm.cold_start`, 2.3 ms**, recorded on 2026-09-05 with a 25 per cent
tolerance. `ci/bench-baseline.json` carries the figure, the host class, the
instrument versions, the artefact's digest and the reasoning for both the value
and the tolerance.

Appendix A gate A4 has two halves. The size half is recorded in
`ci/wasm-size-budget.json` at 2,151,196 bytes, the F-050 measurement of the
release module. F-039 measured 1,999,061 bytes when the module first reached
wgpu and Naga. **The cold-start half is the 2.3 ms
figure above, and that figure is the F-006 measurement.** Cold start was not
remeasured after F-039 or F-050. 2.3 ms is not a measurement of the current module.

**It is not an answer to gate A4 and must not be read as one.** A4 estimates
3 to 8 MB uncompressed with Naga dominating, and it also names cold start.
The F-039 release module that this measurement names exported six functions
and contains Naga. The functions the release module exports are listed in `docs/lld/build-targets.md`.
**A4 stays open.** The 2.3 ms figure remains the F-006 regression baseline
until a later story measures cold start on this module.

F-111 is not that measurement. `startupArtifact` names
`pkg/ocelli_wasm_bg.wasm` for a render load and
`pkg-decode/ocelli_wasm_bg.wasm` for a decode load. `summariseColdStart`
adds the five phases only when the profile is `release`.
`summariseFirstFrame` returns the caller interval only for that same
profile. Neither function reads this baseline. Gate A4 stays open.

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

## The first frame-decode measurement

**`decode.frame`, 0.0849 ms**, recorded on 2026-09-10 with a 30 per cent
tolerance. The runner measures one release `Decoder::decode` call through the
twelve-bit JPEG Extended `.51` adapter over the deterministic 96 by 64
synthetic corpus frame. It excludes process startup, fixture setup, decoder
construction, and caller output allocation. Dependency allocations that occur
inside `decode` remain inside the measurement boundary.

The Rust executable performs one warm-up decode, then times 31 individual
calls with `std::time::Instant` and reports their median. It rounds durations
to 0.0001 ms, which is below the observed run-to-run spread. `black_box` keeps
the source, frame description, and output observable. The record includes an
output checksum and the transfer syntax, rows, columns, bits stored, release
profile, and observed range.

The tolerance is derived from fifteen consecutive calibration runs on the
recording host. Their medians ranged from 0.0698 to 0.0891 ms, about 24 per
cent across the range. Thirty per cent covers that measured spread with
headroom. It is a regression baseline rather than a performance target.

## Timing is taken outside production code

The obvious wasm implementation is a `#[wasm_bindgen] pub fn bench_mark()` in
`ocelli-wasm`. It is refused.

- A timing export in `ocelli-wasm` adds a boundary function that exists for the
  harness and ships to every user, and F-101 (E16.2) owns what the boundary
  exports.
- A timestamp inside `Decoder::decode` would alter every production call and
  make the subject measure an instrumented implementation.

The browser half times with `performance.now()` from the page, outside the
module. The native decode half uses `Instant` in a release-only Cargo example,
outside the library. Neither timer changes production code or crosses a
product boundary.

### The rounding decision

`performance.now()` in this Chromium is quantised to 0.1 ms, so every reading is
a multiple of 0.1. Subtracting two of them in binary floating point turns 2.2
into 2.2000000029802322, and recording that would claim thirteen digits of
precision the clock does not have. Every duration is rounded to four decimal
places, which is a thousand times finer than the quantum and therefore cannot
move a figure across a tolerance boundary. It removes the arithmetic's artefact
and nothing else.

The native decode runner also records four decimal places in milliseconds.
Its calibration ranged over 0.0193 ms between run medians, much larger than
the 0.0001 ms recorded quantum. This avoids claiming irrelevant binary
floating-point digits without affecting its tolerance boundary.

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
every `subject_story` resolves to a real F-ID, that a row naming no story has
a runner, that a done story has a runner, that a story which is not in progress
or done has neither a runner nor a baseline entry, that no runner file
exists without a registry row, that every baseline entry names a host class and a tolerance
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

- **A named story is measurable only at `in-progress` or `done`.** A row that
  names no story is measurable too. Pending, archived and superseded rows
  remain unavailable. In-progress permits a real runner and baseline during
  independent review. A done story must have a runner, and so must a row that
  names no story, so status cannot substitute for permanent evidence. Python
  and JavaScript carry the same explicit five-status table and tests.

F-026 adds `decode.transfer_syntax.jpeg2000` without replacing the standing
JPEG `decode.frame` subject. Its release runner measures one `.91` decode of
the 64 by 96 manifest-backed mono16 synthetic frame. Each timing sample covers
four consecutive production decodes and divides elapsed time by four. This
keeps the one-call unit and the same setup and allocation exclusions while
reducing scheduling noise. A temp-only interleaved experiment found that extra
warm-ups through 32 did not remove the correlated slow mode. Raising kept
samples from 31 to 63 or 127 left upper deviations of 6.94 and 7.41 per cent
over 30 repeats. Four-decode samples reduced the observed upper deviation from
8.17 to 5.71 per cent. Batches of eight and sixteen reached 5.70 and 4.67 per
cent, so four was the smallest measured batch that materially improved the
instrument.

After one unrecorded warm-cache run, the controlled calibration series of 15
consecutive production runner medians, in run order, was
`0.6808, 0.7031, 0.6732, 0.7426, 0.7361, 0.7350, 0.6962, 0.6912, 0.6860, 0.6883, 0.6878, 0.6851, 0.6819, 0.6743, 0.6876`
ms. Its median is 0.6878 ms and its range is 0.6732 to 0.7426 ms on host class
`darwin|25.5.0|arm64|Apple_M4_Max|16|51539607552`. The extremes are 2.12 per
cent below and 7.97 per cent above the median. The retained 10 per cent is the
smallest declared symmetric band that covers both extremes, with 7.88 and 2.03
percentage points of lower and upper headroom. The checked baseline retains
exactly 15 values, and its standing test proves the recorded value is their
median, the structured and documented series agree, every point is inside the
band, and the tolerance remains exactly 10 per cent.

F-027 adds `decode.transfer_syntax.htj2k`, the P0 kill criterion's number and
the subject Appendix A gate A1 has been pointing at since F-006. Its release
runner measures one `.203` decode of the same 64 by 96 manifest-backed mono16
synthetic frame on the same four-decode normalized instrument. The irreversible
row is the one measured, because the subject's definition says the three HTJ2K
syntaxes are reported per syntax and never as one average, and a lossless and a
lossy codestream are different work.

After one unrecorded warm-cache run, the controlled calibration series of 15
consecutive production runner medians, in run order, was
`0.1697, 0.1726, 0.1715, 0.1696, 0.1622, 0.1631, 0.1685, 0.1663, 0.1697, 0.1646, 0.1692, 0.1683, 0.1677, 0.1630, 0.1726`
ms. Its median is 0.1685 ms and its range is 0.1622 to 0.1726 ms on host class
`darwin|25.5.0|arm64|Apple_M4_Max|16|51539607552`. The extremes are 3.74 per
cent below and 2.43 per cent above the median. The retained 5 per cent is the
smallest declared symmetric band that covers both extremes, with 1.26 and 2.57
percentage points of lower and upper headroom.

**That band is tighter than either other codec subject's and no protocol
changed to achieve it.** Same instrument, same fifteen-sample series, same
single discarded warm-cache run. JPEG 2000 needed 10 per cent and JPEG-LS 15,
and the three spreads together say the band is a property of the subject rather
than of the method, which is the reading a single number could not support.

F-028 adds `decode.transfer_syntax.jpegls`, the runner the subject registry had
declared and been waiting for since F-006. Its release runner measures one `.80`
decode of the same 64 by 96 manifest-backed mono16 synthetic frame, on the same
four-decode normalized instrument, so the two codec subjects differ only in the
transfer syntax. The lossless row is measured rather than the near-lossless one,
because it is the row with an encoder-independent anchor, and the two are not
averaged because they are different work.

After one unrecorded warm-cache run, the controlled calibration series of 15
consecutive production runner medians, in run order, was
`0.1297, 0.1199, 0.1152, 0.1180, 0.1178, 0.1162, 0.1114, 0.1140, 0.1132, 0.1156, 0.1156, 0.1179, 0.1188, 0.1145, 0.1124`
ms. Its median is 0.1156 ms and its range is 0.1114 to 0.1297 ms on host class
`darwin|25.5.0|arm64|Apple_M4_Max|16|51539607552`. The extremes are 3.63 per
cent below and 12.20 per cent above the median. The retained 15 per cent is the
smallest declared symmetric band that covers both extremes, with 11.37 and 2.80
percentage points of lower and upper headroom.

**That band is wider than JPEG 2000's 10 per cent and the reason is measured
rather than assumed.** This decode is about six times faster, so one discarded
warm-cache run leaves proportionally more process warm-up in the first retained
sample. Two confirmation series taken immediately afterwards under the same
protocol had first samples of 0.1323 and 0.1181 ms, each the highest value in
its own series, which is the same effect reproducing. **The protocol was not
changed to produce a tighter number**, and the recorded series is the first
controlled one taken rather than the best of the three.

**F-243 measured the patched decoder against its base**, with the same release
runner, 31 samples of four decodes after one warm-up, on one Apple M4 Max with
16 cores and 48 GiB under Darwin 27.0.0. That is not the host class the
baseline was recorded on, whose Darwin release is 25.5.0, so these figures are
compared base against tip and never against the baseline. The runner gained an
optional argument naming the corpus row, and with none it decodes the same row
and prints the same record as before, so the subject's invocation is
unchanged. Base is `aba1cb09` built from an archive of that commit with the
new runner and the two banded codestreams copied in. Three runs each,
interleaved, runner medians in ms:

| Row | Base `aba1cb09` | Tip `55cab08d` |
|-----|------|-----|
| `jpegls_lossless` | 0.1161, 0.1226, 0.1176 | 0.1219, 0.1223, 0.1222 |
| `jpegls_near_lossless` | 0.1226, 0.1184, 0.1182 | 0.1166, 0.1170, 0.1168 |
| `jpegls_banded_lossless` | 0.1163, 0.1171, 0.1155 | 0.0104, 0.0103, 0.0104 |
| `jpegls_banded_near_lossless` | 0.1253, 0.1237, 0.1235 | 0.0104, 0.0103, 0.0103 |

The middle values put the subject's own row 3.9 per cent above base on the same
host. The banded rows are about eleven times faster on the tip
because the base decoder lost synchronisation on them and decoded the rest of
the frame in regular mode, which is the defect rather than a speed-up.
`bin/ocelli.sh bench --compare` did not run, because the worktree it was tried
in has no benchmark browser installed, and on this host class it would report
incomparable. `ci/bench-baseline.json` is unedited and the comparison is owed.

A later serial four-row observation on the same Darwin 27 host is retained separately. Each base and tip command used the release runner with one warm-up followed by 31 samples of four decodes. All eight commands exited 0. The base lossless range, 0.1532 to 0.6489 ms, was broad, so these observations do not establish a stable speedup. Independent CharLS pixel checks qualify each row.

| Synthetic row | Later base ms | Later tip ms | Pixel qualification |
| --- | ---: | ---: | --- |
| lossless | 0.2581 | 0.1188 | Both exact CharLS |
| near lossless | 0.1092 | 0.1020 | Both exact CharLS, within NEAR |
| banded lossless | 0.1014 | 0.0091 | Base invalid, tip exact CharLS |
| banded near lossless | 0.1098 | 0.0089 | Base invalid, tip exact CharLS |

The two banded base times measure incorrect work. Those pixel checks do not show that the separate ramp cases or the immutable historical baseline decoded incorrectly. Eleven named current adapter, provider, runner and fixture payloads match the measured `0c327dd72326758516ec8bda41687554486311be` source byte-for-byte at current tree `0fad23dfbd206135c1dfd51d1123e04df2b79bf4`. This source binding does not prove an identical full dependency graph, compiler output, binary or current timing.

The registered comparison subsequently ran and recorded 0.1215 ms, with verdict `incomparable`. Darwin 27 differs from recorded Darwin 25.5.0, and observed Node 26.6 with null Chromium also differs from recorded Node 24.13.0 with Chromium 151. Numerical inclusion within 0.1156 plus or minus 15 percent is context, not an AC13 pass. Host labels, baseline and band are unchanged. On 2026-11-09 the operator approved a disposition permitting F-243 formal completion with this named external host and instrument gap, without reclassifying the verdict as a pass and without changing any recorded number. A matching host and instrument observation and the idle-host qualification remain owed after completion. Primary receipts and raw identities are retained under the local F-243 acceptance handoff.

Paired spike release wasm modules were 222,226 bytes with upstream defaults
and 222,166 bytes with the no-Rayon manifest patch, a 60-byte reduction.
Dependency graph absence and linked binary size are separate evidence.
The runner contract records exactly 31 kept timing samples of four decodes each
after one warm-up call. Its parser requires integer counts matching that
contract, a finite positive duration and checksum, and a finite positive
nondecreasing observed range that encloses the normalized median.
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
stopped replacing dots with underscores makes that false for every subject
whose id contains a dot. A subject that exists and whose runner file is then
missed reports `unavailable / no_runner`, `failedRunners` is empty, `main`
returns 0, and `bin/ocelli.sh bench` reports success having measured nothing. `isEntryPoint` produces the same outcome one
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

**The count of bench test files is not written here, because it has moved.**
It reached five in the S03 review's fourth pass and six in the eighth, when
`paths_test.mjs` joined the arm in the same commit that added the paragraph
above. A count in prose beside a list in a shell script goes stale the next
time the list changes, and it did. The pattern `[a-z_]` cannot match a digit,
so it drops `decode_htj2k_test.mjs` and `decode_jpeg2000_test.mjs`.
`grep -o 'tools/bench/tests/[A-Za-z0-9_]*\.mjs' bin/ocelli.sh | sort -u` is
what the `bench` arm names, and `ls tools/bench/tests/*.mjs` is what exists.
Compare the two lists rather than trusting a number.

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
compared as if it were one population.

`session.cpu_interactive`, `session.cpu_idle` and `cine.frame_change_rate` are
tier-C rows. `docs/spikes/A7-tier-c.md` says F-006 must measure tier C from the
start, including CPU cost per session. Idle is a separate row from interactive
because A7.3 says they answer to different people and only one is negotiable.
`docs/spikes/GATES.md` makes the two remaining A7 figures acceptance criteria
on F-X002 and F-X003. The permanent runners now use a dedicated Chromium
instance, release wasm, and a worker-owned 128 by 128 OffscreenCanvas at DPR 1.
The CPU collector sums `SystemInfo.getProcessInfo.cpuTime` deltas over every
process in that instance, divided by wall seconds. Counter rollback, invalid
values and a changed process inventory refuse the observation. Snapshot-call
midpoints define the wall interval. The result records boundary uncertainty
and workload wall time separately. This includes browser overhead, rather
than pretending to isolate Rust CPU time.

Interactive CPU dispatches DOM pointer events, batches commands on RAF and
settles after the workload. Idle schedules no drawing and checks zero changed
frames. Cine alternates two predecoded synthetic frames and verifies actual
black and white canvas pixels inside the owning worker before measurement.
The published rate divides changed frames by the duration of that cine call.
Snapshot midpoints stay the denominator of the CPU fraction only.
Pixels do not reach the shell. These fixtures do not model an incumbent
workload or establish its noise floor.

`render.first_frame`, `interaction.window_level_drag` and `interaction.scroll`
name F-101. The first starts before dispatching a fresh typed Fit and ends at
its presented event drained on a later RAF. Drag starts at an actual Playwright
pointer event. Scroll starts at a wheel event, applies Scroll to a configured
two-key stack, commits the newly selected resident frame and observes a later
ring drain. Latency reports the median after one warm-up. Tier A and B are
not measured by these runners. The instrument identity explicitly names C.
The native `presented_frame` example remains a separate diagnostic.

Release artifact reuse requires matching wasm, served-byte and source digests in ignored
`tools/bench/out/session-artifact.json`. Source identity includes transitive
workspace and vendor sources. Served identity includes wasm glue and compiled
core bytes. A default run rebuilds. Shared instrument code is under
`tools/bench/src/browser_session.mjs`, outside the registry-only runner directory. Neither a
source digest nor a local timing is a verification receipt.

CPU acceptance runners require `OCELLI_CPU_BASELINE_HOST` to name a JSON file
with `representativeGpuLess: true`, a nonempty `incumbent` description and
`host` matching every `HOST_CLASS_FIELDS` field from `hostClass(os)`. This is
operator attestation of physical-host suitability. Disabling the browser GPU
alone does not establish it. The operator has no such host available. Normal
CPU subjects therefore refuse without a number. Programmatic
`runBrowserSession(subject, { diagnostic: true })` permits local instrument
checks and explicitly sets `representativeHostAcceptance: false`. It does not
write a baseline. Collect the incumbent under the same workload and include
its noise floor before accepting any CPU budget.

Run the actual release browser controls with:

```sh
OCELLI_BENCH_BROWSER=1 node --test tools/bench/tests/run_test.mjs
```

`tier.startup_microbenchmark` is F-004's, recorded here and not reimplemented.
They are two artefacts sharing a word: F-004's runs on every production session
and returns a tier, this one is a development instrument that returns a recorded
number. Its runner invokes F-004's fill-rate instrument,
`measures_a_fill_rate_on_this_machine`, and reads the pixel rate that test
prints.

## Folder viewer measurements

F-216 measures the local series viewer. The five subjects are `viewer.open`,
`viewer.first_image`, `viewer.scroll_rate`, `viewer.window_step` and
`viewer.resident_memory`. Their definitions are in `tools/bench/subjects.json`.
The shared browser work lives in `tools/bench/src/viewer_work.mjs`, outside
the runner directory, so the registry does not treat it as a subject. Each
runner file only calls one measurement.

`viewer.scroll_rate` dispatches one ArrowDown inside each
`requestAnimationFrame` callback and counts a presented frame when the
image index the page drew changed since the previous callback. An empty
`setTimeout(16)` loop in the pinned headless Chromium stays below the 55
frame budget, so the run also reports that plain timer ceiling.

F-216 delivers `viewer.open`, `viewer.first_image`, `viewer.scroll_rate`,
`viewer.window_step` and `viewer.resident_memory`. `subject_story` on each
of those rows names F-216.

F-245 adds no subject and no timing axis. The five registered subjects keep
their definitions, runners, thresholds and bands. The F-245 plan appends a
performance acquisition procedure, and both its runs are
`bin/ocelli.sh bench viewer.open viewer.first_image viewer.window_step
viewer.scroll_rate viewer.resident_memory --compare --out` into a fresh
private directory, on the same host class and instrument, with no
`--accept` and no baseline or tolerance edit. The measured path is the
existing canonical local path only. Remote search and staging are not
measured, and no remote memory or timing claim exists. The DICOMweb workflow
is proved by `examples/folder-viewer/tests/browser/dicomweb.mjs`, which is a
correctness suite and measures nothing.

The synthetic Part 10 files are written under `tools/bench/out/`, which is
ignored. They are not staged.

F-227 adds one informational workload, `pet_no_window`, under
`tools/bench/out/viewer-pet-no-window/`: 267 files of 128 by 128, signed
16-bit, no window, a Rescale Slope of `0.5 + 0.125 (i mod 7)` for file `i`, and
stored 0 outside a disc of radius 50. Its first image waits for the series'
`/series-window` reply, so `viewer.first_image` reports it as
`pet_no_window_ms` and `pet_no_window_median_ms` in its detail, and
`viewer.open` reports the open of that folder the same way. Neither has a
threshold, because the operator accepted the cost of the extra decodes, and
neither moves the recorded value, which stays the CT median. F-216's CT and DX
workloads store Window Center 40, so they never ask for a series window.
`viewer.resident_memory` opens the `pet_no_window` folder three times, so the
run includes three series window rounds, and its bound carries F-227's named
term `P * 4 MiB` for the rounds' pools, `serverRssBound`'s
`SERIES_POOL_BYTES`, with the fixed constant at 24 MiB so the total stays
F-216's 528,194,192 bytes. The implementer's runs during the story are
informational only, taken on a loaded host.

The figures of record for F-227 are one run of `bin/ocelli.sh bench
viewer.open viewer.first_image` on 2026-11-11, release build, nine samples
each, on host class `darwin|27.0.0|arm64|Apple_M4_Max|16|51539607552`, the key
F-216's viewer baselines are recorded under. That names the host class and not
the physical machine. The run record is private evidence kept outside the
checkout.

| Row | Median | F-216 threshold |
| --- | ---: | ---: |
| `viewer.open` | 940 ms | 1,100 ms |
| `viewer.first_image`, CT | 10.1 ms | 20 ms |
| `viewer.first_image`, DX | 28.2 ms | 250 ms |
| `pet_no_window` first image, informational | 14 ms | none |
| `pet_no_window` open, informational | 106 ms | none |

`thresholds_met` was true and `restart_recovered` was true. The expectation
stated before measurement for the `pet_no_window` first image was under 30 ms.
The host was not otherwise idle: other agent work was paused, the operator's
foreground applications were open, and the load average was 3.20 before and
4.20 after. The thresholds were met under that load.

F-227's sampled-frame preview cost was observed separately on 2026-11-09. Three fresh release servers each received the existing 267-file synthetic no-window PET workload. The default cache was 256 MiB. `/held` reported zero decoded frames before and after `/series-window`. The first preview of sampled frame 0 reported `decoded`, and the identical next request reported `held`. Bodies and frame, dimensions and window headers matched, including the exact series-window strings sent.

| Pair | Second-decode-inclusive preview ms | Held preview ms | Difference ms |
| --- | ---: | ---: | ---: |
| 1 | 1.055917 | 0.645625 | 0.410292 |
| 2 | 1.094375 | 0.670916 | 0.423459 |
| 3 | 1.006416 | 0.645083 | 0.361333 |

These are informational monotonic HTTP request-to-complete-body durations, including scheduling, cache, rendering and transport. Fixed decoded-then-held order conditions the observations. They do not isolate decoder CPU cost, establish idle-host timing or replace the registered PET first-image observation. No threshold was introduced. The single invocation exited 0. All three owned servers stopped, and the observation procedure, binary and all 267 copied input hashes remained unchanged. The release binary matches root's source-0fad producer receipt. Historical progress-only 1.25/0.88 ms remains a separate observation with unrecovered primary evidence.

F-227's independent AC16 acceptance instrument is now [the pydicom calculator](../../tools/bench/auto_window_reference.md), paired with [the private viewer-order export helper](../../tools/bench/export_active_window_check.md). It compares exact binary32 window strings from an independently sampled pool. The standalone helper downloads local caller input and adds no product control or endpoint. Its one cached route observation is separate from manual first/middle/last, scrolling and labels. The explicit Python suite contains 42 synthetic controls. The 11 VM controls run in the benchmark gate through both registered runner lists. Actual native and Chromium download/CLI composition checks remain separate. They establish instrument readiness. The actual-study observation is the operator's, recorded as passed on 2026-11-10 in the F-227 AS_BUILT entry with no study detail, and the timing of record is the run above.

Playwright's `postDataBuffer` is empty for these fetch bodies. The window
subject also records the byte length of the body the page passes to `fetch`.
A preview body that is not empty fails the run. After the timed steps the
same subject asks for one more id-form preview and compares sixteen canvas
pixels with that reply. A listener started after the draw does not see the
reply that already arrived. The restart step clears the page grey cache
before the next key, so the stale id is sent and the file is posted again.

### F-225 conformance-route observation

On 2026-11-09, Node 24.13.0 recorded 27 native HTTP observations on two fresh release servers at the default 256 MiB cache. Producer tree `0fad23dfbd206135c1dfd51d1123e04df2b79bf4`, binary `85b4baaf65f5fe139dc9cc4f8e93a8bc1b5b400607103989f23175fc8fce082a`, and measured index `435a0ad410546550a11ad7f4b501e81be8039e63` are separately bound. Source, sent fixtures, helper, reader and binary identities matched before/after. Both owned servers stopped. Failed or incomplete observations invalidate medians.

| Route | N | Median ms | Range ms |
| --- | ---: | ---: | --- |
| CT encoded-held ID200 control before preview | 9 | 0.382875 | 0.239208 to 0.841167 |
| CT after untimed preview, ID410/body200 | 9 | 0.535041 | 0.476291 to 0.609709 |
| DX after untimed preview, ID410/body200 | 9 | 4.779375 | 4.641583 to 7.369375 |

All 45 legs remain, with no discarded warmup. Each fixture was posted once. The fallback sends its body directly without reposting `/file`. Page report caching normally deduplicates requests, while this instrument repeats each stratum nine times.

The clock spans ID request through complete final JSON. Each leg is fully drained, with artifact I/O and validation outside timing. The keepalive Agent uses maxSockets 1 per fixture, without guaranteeing socket reuse. Browser fetch does not explicitly drain 410 similarly. These are native-route figures, not exact browser latency, isolated checker CPU, idle-host or whole-AC34 evidence. Image benchmarks remain separate. No latency threshold applies.

Table values are rounded. Raw SHA256 `4033f47912aeea582325460e43fa9987eae22597af033b2f4f49c9ff23f312f4`.

### Current colour comparison cost distinctions

These controlled synthetic observations record the F239 AC32 and F240 AC16 target comparisons. They do not establish colour pixel parity, clinical representativeness or isolated causal costs. The informative calibration is CT only. Ocelli applies ICC while cornerstone draws device RGB.

The table gives Ocelli minus cornerstone median milliseconds. N is 9 per cold/warm timing and 9 window steps. A dagger marks overlapping IQRs, including touching endpoints: no demonstrated difference at this N under F223 AC6. It does not erase the numerical comparison or claim the target met. All IQRs, extrema and raw record hashes remain in [the saved comparison summaries](../benchmarks/s46-colour-comparison.json).

| Case | Open cold/warm delta | First cold/warm delta | Window delta |
|---|---:|---:|---:|
| rgb8 | +39.8 / +42.8 | -9.9 / -7.6 | +2.2† |
| ybr422 | +36.5 / +35.6 | -1.8† / +2.4† | +0.3† |
| palette16 | +45.8 / +45.4 | +8.7 / +7.4 | +16.9 |
| icc8-gamma22 | +71.5 / +63.0 | +4911.7 / +4983.7 | +6714.4 |
| icc16-table | +144.5 / +143.5 | +5065.2 / +5011.7 | +6946.6 |

Twenty-two latency medians are numerically above the comparator: 19 have separated IQRs and 3 overlap. Every case has higher open medians. RGB8 first-image medians are below with separated spreads. YBR422 first-cold is below with overlapping spreads.

The source shows different work placement. Ocelli redraws the complete colour frame in native CPU bands, transfers its three-channel byte response over loopback, expands RGB to RGBA and publishes ImageData on a 2D canvas. The comparator updates VOI properties on its retained WebGL viewport and renders. No backend/HTTP/canvas/GPU partition was measured, so quantitative attribution remains unisolated. Internal avoided texture-upload savings are not established by the consumer excerpt.

ICC recipe and device tables are retained per held frame, while channel lookups, matrix arithmetic and sRGB encoding execute per pixel per draw. Changed windows add a VOI pass that the initial identity window skips. That difference does not prove why window medians exceed first-image medians, and does not indicate table rebuilding.

Open ends at the first rAF after header grouping and the series sentence. First-image starts at that sentence, so intervals may overlap. Later draw-path excerpts do not isolate the open excess. Its specific quantitative cause remains unknown.

All 10 memory medians have no observed target miss, with N=3 sessions. Sampled concurrent physical footprint is a lower bound. Summed noncoincident process peaks are a separate upper bound. Neither establishes true simultaneous peak dominance. These target observations and unresolved attribution remain recorded, not converted to a blanket pass.

The saved summaries bind each matrix to its raw SHA-256, measurement index, producer binary, benchmark page and comparator. All five commands exited 0. Product source is tree `0fad23dfbd206135c1dfd51d1123e04df2b79bf4`, measured index is `a601d30620b727abda7225b3b215dea76e31d358`, and the binary is the same release recorded above. The controlled recipes use 200 frames of 512 square for RGB, YBR and palette, and one 8192-square frame for each ICC case, with a 512 MiB server cache for ICC16. These equipped browser comparator observations remain separate from the registered native four-case HTTP workload. No numeric target or baseline was changed.

## What is deliberately absent

**No `criterion` and no new Cargo dev-dependency.** The frame runner is a Rust
example with a named production subject and uses `std::time::Instant` directly.
The JPEG dependencies are production dependencies rather than benchmark-only
scaffolding. `tools/bench/package.json` and its lockfile continue to pin
playwright at exactly the version `tools/oracle` pins, which
`scripts/bench_check.py` asserts.

**No new trait and no new generic.** The runner lookup is a file path derived
from a subject id, and the two implementers a trait would need do not exist.

**No pixel arithmetic in the harness.** HLD 27.2 R3 requires a hand-computed
fixture for every function doing pixel arithmetic. Decode subjects invoke the
production decoder and read no sample value. The decoder's tests own the
hand-computed lossless fixture and independent `.51` reference. The folder
viewer subjects compare 16 canvas samples with the server reply and do not
compute a window of their own.

**No parity surface.** `docs/hld/B-parity-surface.md` enumerates viewport types,
tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation
representations, core events and adapters. A benchmark harness is on none of
those axes.


## Local viewer comparison, F-223

The current [generated comparison](../benchmarks/cornerstone-comparison.md) is a controlled synthetic instrument observation on 2026-11-09, not the operator-study acceptance. Its historical metric labels `pseudo-full`, `pseudo-ct` and `pseudo-dx` contain 201, 200 and 1 controlled files. The approved actual-root run, AC18/AC19 and decision 12 remain owed. The direct command exited 0 in 232.442 seconds with producer tree `0fad23dfbd206135c1dfd51d1123e04df2b79bf4` and the release binary recorded above. Independent review reconstructed all 44 statistic cells and 31 report rows. Raw SHA-256 is `30121e5b223951c6aeb635376b6373ac1079fdda3a4ed153ed51342fa10a806b`. No synthetic input is substituted for the required operator study.

`bin/ocelli.sh bench --compare-cornerstone --pseudo <local-root>` runs the two
local viewers with the same pinned full Chromium build and real inputs. It
requires the locked `tools/bench` and `tools/oracle` installs. `OCELLI_PSEUDO_DIR`
may supply the root. Neither this mode nor its supplemental mode changes a
baseline, and neither is an automatic floor or sprint timing gate.

The controller builds the release viewer and `ocelli-compare`, then builds the
separate `bench.html` and `bench.js` beside the existing oracle entries. Their
source and binary digests are recorded with the Git HEAD and staged tree. A
tracked or new measurement source that differs from the index refuses the run.
The existing stack and volume bundles are separate entries and were checked
byte-identical after adding the comparison entry.

Before any timed trial, the qualifying pseudo CT subset is opened in both
viewers at center 40, width 400, LINEAR. Their selected file names, file sizes,
native canvas dimensions and actual RGBA readbacks must agree. `ocelli-compare
pair` applies the existing monochrome and systematic-bias predicates. A failed
or low-information frame writes a refusal with no timings. Private frame files
are removed with the temporary input copies. This is one-frame agreement for
cost comparison, with none of the oracle corpus acceptance implied. The
opted-in prepared-subset browser regression calls this actual precheck with
the release comparator, alongside its upload-count and literal-pixel checks.
It therefore covers the controller capture callback as well as the pages.

The default matrix retains its five closed workload selection rules. Pseudo
full submits every file below the original root, including auxiliary files.
The subset directories use neutral regular-file copies in content-selected
order. Copy-on-write is attempted with `COPYFILE_FICLONE`, which falls back to
ordinary copying when the filesystem does not support cloning. Actual
directory uploads skip symbolic links, so selected subsets contain none. The
source files remain unchanged and the full-root workload retains its original
every-file policy.
Both page consumers list at most 65,536 frames per file and retain omitted
counts per series. There is no aggregate series cap. Matching that policy
preserves the largest-series tie and its selected first file. A
preflight read of every original file warms the operating system file cache on
both sides. A cold trial starts a fresh browser and native server. A warm trial
uses a new page in the retained browser and server after a completed priming
trial. Creating a page closes the actor's previous page and its automatically
owned context, including after a failed trial. Only the browser and native
server are retained. Window steps keep one drawn page. Both conditions have qualified labels
such as `pseudo-full/cold` and `pseudo-full/warm`, and neither overwrites the
other. N is per side, with alternation and the starting side recorded. Each
declared metric/workload visit shares one starting side across its cold and
warm conditions, then flips once for the next workload. The sequence continues
across metrics. Unavailable workloads reserve their place without observations,
and unavailable conditions do not cause extra flips. CPU reuses the measured
memory sessions. Missing
workloads and inapplicable conditions remain explicit unavailable cells.

Each duration except page-ready stays inside one page's performance clock.
Page-ready maps the controller spawn clock and browser first-ready rAF clock
through their respective `performance.timeOrigin + performance.now()` values.
Plain subtraction between context-relative timestamps and wall-clock durations
are not used. A new visible canvas commit is required for first-image and
window observations. Pending requests retain the prior presented index until
an actual commit. A retained visible image is not a blank sample. Native-stage
visibility and faults are sampled independently on every scroll rAF and after
the final settle.

The comparator provider adapts recognized valid VOI strings to the array shape
loader5.8.2 createImage expects. Otherwise it would derive `L` from `LINEAR`
and refuse subsequent actor updates. Arrays, unknown strings and absent values
are unchanged. Installed package bytes and shipped cache and worker defaults
remain unchanged. The fixed output caveats disclose this adaptation.

On macOS one serial footprint call samples every browser descendant, every
CDP-reported process and, for Ocelli, the native server. Sampling rounds are at
least 250 ms apart. Each round retains its actual PID inventory and timestamp.
The report names the sampled concurrent footprint as a lower bound and states
the minimum250ms sampling pause. Actual timestamps remain in raw evidence.
The separately reported sum of
per-process peaks is an upper bound because those peaks need not coincide.
Both bounds are summarized over the same three sessions. The Node static
server is infrastructure and recorded separately. Unattributed GPU-driver
memory is included on neither side. CPU is from those same sessions, and a
changed process inventory makes the CPU observation invalid. Before measured
actions, at most three inventory-plus-counter reads establish a coherent CPU
baseline. Only a locally identified missing PID permits another setup read.
Persistent incoherence refuses before navigation or sampling. Final snapshots,
actions and trials are never retried. Other platforms carry an explicit missing
memory collector, with no estimate.

Ignored `comparison.json` and `comparison.txt` carry the raw observations,
conditions, defaults, input digest and source identity. `--publish` also writes
`docs/benchmarks/cornerstone-comparison.json` and its generated Markdown pair.
Publication refuses pre-existing builds, injected tools and invalid cells. All
output strings, rendered text and serialized JSON are scanned against private
input headers and path components before writing. Foreign browser or I/O errors
are replaced by fixed refusal sentences. A failed new attempt removes the
previous ignored artifact names, so an older green file cannot describe a red
exit. Owned browsers, servers, input copies and the exclusive output lock are
closed or removed on success, failure and handled signals. Environment
refusals retain both browser and renderer strings in ignored reports, without a
pixel verdict or timing figures, and exit 2 after cleanup. Invalid measurements
keep their ignored raw records even when publication was requested. They never
replace the tracked pair. Privacy, interruption or cleanup failures remove the
attempt's ignored reports. Pre-launch flag refusals do not take ownership of
prior artifacts.

### Controlled colour and ICC supplement

`--colour-case rgb8|ybr422|palette16|icc8-gamma22|icc16-table` selects a separate
controlled mode and refuses `--publish`. Generate an isolated input root with
`.venv/bin/python scripts/corpus_synth.py --comparison-colour <case> --out
<empty-root>`. The exact closed manifest, inventories, file sizes and digests
must validate before use. The default five-workload matrix is unchanged.

The unchanged informative CT calibration runs first from `calibration-ct`.
Only then do the declared `colour` inputs run open, first-image, nine window
steps and three all-process memory sessions. The two fixed directory names are
public generator selectors. Every uncontrolled name below each selected root
and every identifying header remains in the output deny set. No colour pixel
score is inferred. ICC cases state that Ocelli applies the profile and
cornerstone renders device RGB, so their rendered images deliberately differ.
The ICC16 case declares and verifies an actual 512 MiB native budget. All other
cases verify 256 MiB through `/held`, rather than trusting configuration alone.

This mode writes ignored `colour-comparison.json`, `colour-comparison.md` and
`colour-comparison.sha256`. The digest binds the exact JSON bytes. Raw alternating
trials and the complete process receipts remain in the JSON, with separate
memory bounds. No equipped comparison or cost conclusion follows from the
fake-tool contract tests or bounded functional smoke checks.

## Measured largest colour frames, F-239 and F-240

The release viewer was measured on the Apple M4 Max, 16 CPUs, 48 GiB,
Darwin 27.0.0. All four inputs are generated 8192 by 8192 single frames.
Each first request decodes, and each following window request uses the held
frame. Nine complete window replies per case were checked against literal
hand markers. These are native HTTP observations, not browser frame rates.

The current observation uses measurement index `a601d30620b727abda7225b3b215dea76e31d358`, with product source from verified tree `0fad23dfbd206135c1dfd51d1123e04df2b79bf4`, HEAD `d34d9baff2477fae9f98e904dae643a4edcbb5f5`. The index differs only by the generated default comparison pair. Node 24.13.0 ran the unchanged registered instrument. Its release binary SHA-256 is `85b4baaf65f5fe139dc9cc4f8e93a8bc1b5b400607103989f23175fc8fce082a`. The direct command ran from 01:51:38 to 01:54:50 UTC on 2026-11-09 and exited zero in 192.331 seconds. The generic run record's `started` field is its record-assembly timestamp, so command times come from the direct receipt. No threshold was introduced.

| Input | First request ms | Median of nine window requests ms | Window range ms | Peak native RSS bytes | RSS samples |
| --- | ---: | ---: | --- | ---: | ---: |
| rgb8 | 2171.469 | 2699.735 | 2641.377 to 2728.456 | 614170624 | 1221 |
| palette16 | 2503.991 | 3087.126 | 3055.817 to 3182.705 | 484605952 | 1391 |
| icc8 | 6080.290 | 6567.662 | 6547.578 to 6579.278 | 614187008 | 3000 |
| icc16 | 6369.414 | 6748.776 | 6737.565 to 6766.356 | 1017839616 | 3088 |

Every RSS series has positive samples and zero collection failures. The
held-cache budget is 256 MiB except for the explicitly declared 512 MiB ICC16
case. That budget bounds retained cache entries, not the whole process RSS.
ICC8 retains 3072 bytes of tone-curve tables and ICC16 retains 786432 bytes.
The held-frame charge remains 201330640 bytes for ICC8 and 403440616 bytes
for ICC16 after the first draw and after all nine windows. Stable retained
charges do not prove that no temporary work occurred.

Palette validation was hoisted out of the draw-band loop after the earlier Node 26.6.0
instrument measured a 19023.172 ms palette window median on source
`0c327dd72326758516ec8bda41687554486311be`. The matching Node 26.6.0 after-hoist median is 5092.812 ms,
an observed 73.228 percent reduction. The descriptor borrows the same immutable
palette once per draw. The pixel mapping and tie rule are unchanged. Initial
and window hand-marker checks pass on both runs. These timings still show
slow updates for the largest admitted images.

The current final-byte store preserves the approved guard and ties-to-even
rule and reduces the conversion work. The current Node 24 and earlier Node 26
runs have different instrument identities, so their difference is not an
isolated byte-store speed measurement.

Other agent build, test and browser jobs were held during these measurements.
External desktop and security processes remained active. Load averages were
recorded in each run on the 16 CPU host, so the observations do not claim a fully idle host.
The records retain every sample and exact build identity. Raw run SHA-256:

- Before hoist: `631c8f5d7cc703494f7c8f4609054832a1dc03cbca65491402a32beb01c6a38e`.
- After hoist, Node 26: `e3886ccbecccaa3987d60ec6e8d005c716b009c9e640e83cdb81996d6326cd64`.
- Earlier assembled byte store, Node 24: `3b5fc0eb00d6173d45792745c91f4c110f64bdc84b351abcce19ea85051f48a9`.
- Current source, Node 24: `6f32db8e4cc0b525a28c95bb973747973ae9bed1a774440300d13bcfaeabd3c4`.

The equipped Cornerstone comparison and the declared monochrome regression
runs remain separate acceptance observations. No relative cost claim follows
from this one-sided largest-frame instrument.

### Same-host monochrome observations

The original source and assembled viewer were run serially with the same
Node 24.13.0, Playwright 1.62.1 and Chromium 151.0.7922.34 on Darwin 27.0.0,
Apple M4 Max, 16 CPUs, 48 GiB. The original 5451cb7d product used only the
reviewed shared-instrument adaptation. CT and DX fixture bytes and default
budgets were checked unchanged. The assembled product includes the conformance
report and automatic-window work, so these are stack observations.

The original measured tree is `68702951938d8d7194e8862739d0a51ffcf60244`,
HEAD `5451cb7d4ca6c9e1304b96e1e95c493acbbbd060`, with only the shared
instrument adaptation staged. The assembled measured tree is
`9b586d477dc5172969253f6a9d8bc44437214a63`. These identities name the
observations, not subsequent byte-store or completion edits.

| Subject | Original source | Assembled source | Unit |
| --- | ---: | ---: | --- |
| viewer.open | 933.000 | 970.000 | ms |
| viewer.first_image | 8.400 | 12.500 | ms |
| viewer.scroll_rate | 108.995 | 109.067 | frames_per_second |
| viewer.window_step | 2.100 | 3.400 | ms |
| viewer.resident_memory | 458555392.000 | 463585280.000 | bytes |

Every subject meets its answered absolute budget. Both sources report zero
release and debug blank scroll samples, successful restart recovery and zero
preview request bodies during held window updates. The assembled DX first
image median is 28.9 ms and its window median is 17.0 ms. Automatic-window PET
first image is 17.2 ms, informational with no added threshold.

The existing baseline comparator was also applied to these saved observations.
It matches the host and instrument exactly. The assembled first-image 12.5 ms
is 28.866 percent above the recorded 9.7 ms, and window 3.4 ms is 41.667 percent
above 2.4 ms. Both are `moved` with sense `worse`, beyond their existing 25 percent
bands, despite meeting the absolute 20 ms and 5 ms budgets. No new baseline,
tolerance change or relative-regression pass is claimed. The original source
open 933 ms is also outside its existing 6 percent band in the improving direction.

Other agent builds and browser jobs were held during these observations.
External processes remained, with load averages about 5.5 to 6 on 16 CPUs. These
records do not claim a fully idle machine. Source digests and all samples
remain in the run records. Earlier Node 26.6 runs are retained as diagnostics,
with correctly incomparable historical results due to their Node mismatch.

Raw `f239-monochrome-base-node24` SHA-256:
`ec83cc798e490c0d016ef97f1b7373e690526ab5d6482cfbd42e55ab7ec505b3`.

Raw `s46-monochrome-tip-node24` SHA-256:
`5f822caa093008d580d28250a545cc83bf0ffe2b4acc404b1cefcfaaede2d5cf`.

### Untagged colour against the pre-ICC product

The pre-ICC source `12797792d47afbd8aba2320cd27951c33645a125` was built
in release mode, binary SHA-256
`943dff2d8b2c61be5fe9ef0cf3c942c2e4a6aaae48ff31fcde53d138db442bdc`.
The assembled release source is `c1fe30375cdd8d8a0767ff7a941fe2fc4d80dd06`,
binary SHA-256
`b32c96cf33a3c719b49150b98040db4ffce8a02f82fe0e8e724be51ddaac2d59`.
Both products used the current registered subject's fixture and measurement
exports on the same M4 Max host with Node 24.13.0. A private driver bound by
its recorded hash selected only the two untagged cases for the old product.
It supplied the explicit binary path and did not overlay product source.
The old product has no ICC implementation and was never sent an ICC fixture.

| Input | Pre-ICC first ms | Assembled first ms | Pre-ICC window median ms | Assembled window median ms |
| --- | ---: | ---: | ---: | ---: |
| rgb8 | 4955.386 | 2231.128 | 4728.307 | 2751.412 |
| palette16 | 19287.031 | 2577.263 | 19257.557 | 3128.093 |

Each first request decoded and all nine following window requests used the
held frame. Complete replies and hand markers passed, with positive RSS
samples and zero collection failures. Both untagged cases used the default
256 MiB held budget. The source fixture module is byte-identical on the old
and assembled products. The driver records each old fixture's content digest,
helper hashes, host, binary hash and every sample.

The assembled product also contains the palette validation hoist and the
exact final byte-store repair. These observations establish the behaviour
and cost of the combined stack on untagged frames. They do not isolate the
cost of adding ICC support. Other agent builds and browser jobs were held,
while external desktop activity remained, so no fully idle host is claimed.
These measured untagged cases improve on the pre-ICC observations. The
earlier monochrome relative-band misses remain preserved. The later remediation
restores all five registered bands, as recorded below.

The current release re-observation above retains the same two untagged fixture and measurement helpers. It records RGB first 2171.469 ms and window median 2699.735 ms, palette first 2503.991 ms and window median 3087.126 ms, against the literal pre-ICC observations above. These are observed reductions, not values called within noise. The independently reviewed lead ruling in F-240 accepts these measured reductions as non-regression, while retaining that the literal noise wording was not met. The supplementary normalized control remains diagnostic and does not replace the story base.

Pre-ICC raw run SHA-256:
`61c638f921c418c6a19dbe3207c2930ec28ad6f7d81e6b7c33d21897c576f4fd`.

Assembled raw run SHA-256:
`3b5fc0eb00d6173d45792745c91f4c110f64bdc84b351abcce19ea85051f48a9`.

#### Supplementary normalized pre-ICC control

The literal `12797792` source, binary and historical observations above remain
unchanged. A separate private control starts from that exact source and adds
only the independently reviewed palette-validation hoist and exact final-byte
store. The resulting source is
`c4d6e4b3d17e35c54b4a2240cdb2ff10176cf336`, with release binary SHA-256
`0dc9860fe164d0842dd7a31262ea8ac46df0822a07e58a207d5c71d6ea94c25f`.
The complete delta changes only `cpu.rs` and `colour.rs`. It adds no ICC parser,
transform, profile policy, dependency, cache budget or window rule.

The unchanged two-case measurement body and shared fixture exports ran under
Node 24.13.0 on the same M4 Max host. Input content digests match the untouched
historical run. This private entry is bound by SHA-256
`2e06d1044e2777b76da40c1f98612dbd5733c21b239984e366c3586f222648bf`.

| Input | Normalized first ms | Nine held windows median ms | Window min to max ms | Sampled peak RSS bytes |
| --- | ---: | ---: | ---: | ---: |
| rgb8 | 2172.713 | 2561.928 | 2532.335 to 2655.332 | 614514688 |
| palette16 | 2586.171 | 3000.127 | 2988.722 to 3015.822 | 485064704 |

Both first requests decoded and all nine following requests used the held
frame. Complete-body and 30 hand-marker checks passed on every reply. RSS
collection used the existing requested 20 ms period, with 1175 RGB readings
and 1366 palette readings and zero failures. Both cases used the default
256 MiB held budget. Owned build and browser jobs were held. External desktop
activity remained, so no fully idle host is claimed. Start and end one-minute
load averages were 2.889 and 2.878.

The first attempted command exited one because the process sandbox denied
all RSS reads. It produced no valid result and its receipt remains separate.
After verifying and preserving its empty output directory, the exact unchanged
entry ran with process observation permitted and exited zero in 57.538 seconds.
Source, binary and driver hashes were unchanged before and after that run.

This supplementary control removes two known shared optimization differences.
It does not isolate every remaining product difference as ICC overhead. It
neither replaces the literal story-base observation nor calls the large
historical improvements within noise. No baseline, tolerance, numerical gold
or acceptance criterion changes. Any use as a substitute for AC14's stated
comparison requires its own explicit plan decision. Existing relative-band
movement flags remain separate.

Normalized raw run SHA-256:
`3bd1c59366540291875e78a5f8bb0b3a0cbd6f2cc2573c80677842631b418913`.


## S46 first-image and background file-probe remediation

The operator required the first-image movement fixed before sprint close.
The assembled run measured 12.7 ms against the recorded 9.7 ms baseline and
its unchanged 25 percent band. Deferring the active-series probe until after
the first draw's paint opportunity restored first-image time to 10.3 ms,
but its first full run and a focused repeat measured a 3.2 ms window step,
above the recorded 2.4 ms plus 25 percent band. Those observations are
retained, not accepted as a baseline or removed from the record.

A private four-arm diagnostic used the original registered consumer bodies
with transparent probe observations. Both old and new natural arms measured
3.2 ms, with file reads overlapping window requests. Settled-probe arms
measured 2.5 and 2.4 ms. This supports investigating overlap, not an isolated
claim that the new scheduler caused the movement. Its extra observer work
can affect timing. Its 36 samples are diagnostic only and never substitute
for registered acceptance samples. Input binding used the owned synthetic
file count and stamp, not a full per-file digest.

The measured implementation gives the first draw a paint opportunity before
constructing the probe list, and keeps a requested 100 ms fallback for a
failed or stalled draw or withheld animation callbacks. One pending owner
cancels an abandoned selection. Focus, visibility and viewport activation
consume that owner before their existing immediate probe. The background
sweep uses one read worker. This explicitly amends F-224's original four-worker
choice and exact-four observation policy. The independent ceiling of four,
every-file/one-byte checks and original one-second limit remain. No new
pause timer, pixel rule or cache identity is introduced.

On the F-216 host, Node 24.13.0, Playwright 1.62.1 and Chromium
151.0.7922.34, the complete registered release run on the private remediation
source measured:

| Subject | Fresh pre-ICC base | Prior assembled tip | Remediation | Existing baseline | Existing band |
| --- | ---: | ---: | ---: | ---: | ---: |
| viewer.open, ms | 930 | 972 | 967 | 1018 | 6 percent |
| viewer.first_image, ms | 8.7 | 12.7 | 11.3 | 9.7 | 25 percent |
| viewer.window_step, ms | 2.2 | 2.5 | 2.2 | 2.4 | 25 percent |
| viewer.scroll_rate, frames per second | 108.998 | 109.035 | 108.774 | 109.083 | 25 percent |
| viewer.resident_memory, bytes | 469434368 | 444497920 | 464125952 | 445956096 | 12 percent |

Every remediation row is within its existing recorded band, and all absolute
budgets pass. DX first image is 30.3 ms and DX window step is 18.1 ms, within
the existing 250 and 70 ms budgets. Release and debug scroll each observed
zero blank frames. The process restart recovered. The source remains slower
than the fresh pre-ICC base on first image, so this is a restored recorded
band, not a claim of equal pre-feature performance or an isolated ICC cost.
Host load and free-memory conditions are retained in each raw record, not
described as an otherwise idle desktop.

The full changed-file browser suite passed. It measured all 1693 one-byte
reads with peak one in 284.9 ms. Switching while that sweep was running,
rewriting a picked B file and coalescing selection with focus produced exactly
1693 A and three B reads. The B fault and count were observed, with completion
276.5 ms after selection. Eleven scheduling and lifecycle cases cover fast,
held, refused and animation-starved draws, focus, repeated selection, same-UID
folder replacement and viewport activation. Missing immediate cancellation,
missing fallback and dropped probe.again mutants fail semantically. The
upper-bound validator still rejects five reads in flight. Every accepted
source restoration was checked exactly.

The complete run is retained locally as
`first-image-serial-worker-five-subjects-node24/run.json`, SHA256
`5b7fc8cf2385f32e03059f82e3bed816ae6d949b5f953dd21c9b24c4fe71ba04`.

Its direct command receipt records exit zero and 102.558 seconds. The
implementation tree was `1294d421c041f6bd36fc600dc83cf4cd4baac507`, with
page source SHA256
`8de74678523d68b8ee32671229e34de0c57ad1a689bc9ece4ef03904c69107d7`.
The generic registered record does not self-report a product binary hash.
Build receipts, staged source identity and separate post-run artifact hashes
provide the qualified binding. Subsequent README, Use and evidence text edits
do not change this measured JavaScript. Final composed-tree verification,
colour comparisons and operator click-throughs remain separate obligations.
