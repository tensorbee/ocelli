# F-006, benchmark harness: decode, first frame, interaction latency

**Status**: approved
**Epic ref**: E1.6
**Sprint**: S03
**Estimate**: 2w

Every verbatim transcription below sits in a fenced block. `docs/hld/` is
exempt from the voice rules and this file is not, so a blockquote carrying the
author's en-dashes and em-dashes would fail `scripts/prose_check.py` and force
a silent edit of the source text. A fence keeps the bytes.

## Normative source, transcribed

### `docs/hld/23-performance-rules.md`, section 26, in full

```text
## 26. Performance rules

- No allocation in the render loop.

- One queue.submit() per frame.

- Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload.

- Batch pointer events into one command buffer per animation frame. Never cross the boundary per event.

- Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer.
```

**The last rule names an instrument, and `CLAUDE.md` and `AGENTS.md` both
paraphrase it without one.** Both say "Measure before optimising. The
intuitions that work in JavaScript do not transfer." The HLD says "Measure
**with the benchmark harness** before optimising anything." That difference is
this story. Section 26's final rule is unenforceable until the harness exists,
because there is nothing to measure with, and a rule with no instrument is
satisfied by whoever is confident. The paraphrase is not wrong, it is just
missing the half that F-006 delivers.

Section 26 states **no number**. Not a millisecond, not a frame rate, not a
latency budget. It is a rule about the order of operations, and the search
described under "What the specification does not cover" confirms the HLD
states no performance target anywhere.

### `docs/hld/08-validation-architecture.md`, section 11, in full

```text
## 11. Validation architecture

Cornerstone3D is a correct reference implementation that can render any series you own. The harness pushes the same study through both stacks and compares frames within a written per-modality tolerance, with metadata diffed alongside pixels because a wrong rescale slope can still produce a plausible image.

Every pull request renders the corpus in CI. Every field bug becomes a permanent fixture. In production, shadow mode renders both libraries and alerts on divergence — the oracle running against real clinical traffic, and the same corpus a regulatory submission would want to see.
```

Section 11 is about **correctness**, not cost. It says nothing about timing,
and the differential harness it specifies compares frames rather than
durations. F-010 built that harness and it records exactly one duration,
`elapsedMs`, which is whole-run wall clock. Section 11 is transcribed here
because it establishes what this story must **not** do: the benchmark harness
is a second instrument with a second verdict, and folding a timing claim into
the oracle's pass or fail would make a correctness gate flaky for a reason
unrelated to correctness.

### `docs/hld/04-boundary-and-data-path.md`, sections 5.1, 5.3 and 6

```text
### 5.1 Control — typed commands, downward

Small, infrequent, one call per user intent: set camera, set VOI, activate tool, load series. **Never one call per pointer move.** Pointer streams are normalised in TypeScript and batched into a single packed command buffer per animation frame.

### 5.3 Events — a ring buffer, upward

A fixed-size ring in linear memory, drained once per frame rather than invoking a JavaScript callback per event. This removes a boundary crossing from the hot path and gives coalescing for free: a burst of camera changes during a drag collapses to one delivered event. Viewport state reads back as a small C-layout struct copy, not a JavaScript object graph.
```

This is where **interaction latency** gets its definition, and the definition
is available today even though the implementation is not. An interaction
begins at a pointer event on the main thread and ends when the resulting event
is drained from the ring in a later frame. It does not end when pixels appear
in JavaScript, because decision D3 says pixels never cross the boundary.

### `docs/hld/05-rendering.md`, section 7, the submit rule

```text
- **One submit per frame.** The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker.
```

This is where **first frame** gets its definition. First frame ends at the
first ring event reporting a presented frame, not at a pixel read-back.

### `docs/hld/18-codec-registry.md`, section 21, the decoder trait

```rust
pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}
```

This is where **decode** gets its definition. Decode is one call to
`Decoder::decode` over one frame, inside a decode worker, and section 24 says
the decode worker never touches the GPU. Decode time therefore has nothing to
do with a tier and everything to do with a transfer syntax.

### `docs/hld/A-spike-gates.md`, Appendix A, gate A4

```text
| A4 | Do binary size and cold start land within budget? | Estimated 3–8 MB uncompressed before tuning, Naga dominating. Unmeasured today |
```

A4 has two halves. F-002 measured the first, recording 14,104 bytes in
`ci/wasm-size-budget.json` on 2026-09-04, and `docs/lld/build-targets.md` is
explicit that this "is not an answer to Appendix A gate A4 and must not be read
as one". **The second half, cold start, has never been measured at all.** It is
the one subject in this story that has a real subject today.

### `docs/spikes/A7-tier-c.md`, which names F-006 directly

```text
- **F-006 (E1.6), the benchmark harness, must measure tier C from the start**,
  including CPU cost per session. A harness that only measures a GPU path
  cannot answer A7.3.
```

and A7.3's answered form, which is the only place in the repository that
states a measurement method for this harness:

```text
## A7.3, answered. Beat the incumbent, then ratchet

**The budget is set relative to the viewer the estate already permits**, not
from an invented absolute.

1. Measure the incumbent viewer on a representative GPU-less session. Record
   its interactive cost and its idle cost as fractions of one vCPU.
2. **Ocelli's budget is no worse than those two numbers.**
3. Record Ocelli's observed figures and gate on regression beyond a tolerance,
   the same mechanism `ci/wasm-size-budget.json` already uses for binary size.
```

```text
**Report interactive and idle separately.** They answer to different people and
only one is negotiable. **Ocelli's idle cost must be indistinguishable from
zero**, because it multiplies by every open session on the host, and a viewer
that costs anything while nobody touches it is the one a platform team
removes first.

Also report frame change rate under a cine loop, because on a remoted session
every changed frame has to be encoded and shipped.
```

and the CPU-budget method, which forbids inventing a number:

```text
Do not invent a number. Ask the platform team for the density target, sessions
per physical core, and derive from it.
```

### `docs/hld/DEVIATIONS.md`, the rows this story is bound by

D-04, CI runs no GPU and no corpus. D-07, tier C exists and every tier-gated
feature declares its CPU answer. D-12, wasm-bindgen isolation is direct
declaration on wasm32 and transitive reachability on the host. All three are
declared and none is changed here.

### `docs/sprints/allocation.json`, the F-006 note, verbatim

```text
Numbers feed the P0 kill criteria
```

**The P0 kill criteria are named in the same file and they are not latency
criteria.** Exactly two stories carry the marker:

```text
F-027 (E4.5), HTJ2K: spike, then integrate or bridge to openjph wasm
  "P0 KILL CRITERION — unverified in Rust today"

F-028 (E4.6), JPEG-LS: decide CharLS bridge vs pure Rust, then integrate
  "P0 KILL CRITERION — no credible pure-Rust path"
```

Both are codec questions, both are Appendix A gates A1 and A2, and both are
about **decode**. So the "numbers" that feed the kill criteria are decode
numbers per transfer syntax, and the two remaining subjects, first frame and
interaction latency, feed no kill criterion that this repository has written
down.

### `docs/sprints/CURRENT_SPRINT.md`, what done means, verbatim

```text
- **F-006** measures decode, first frame and interaction latency, and records
  the numbers rather than describing them.
```

## What the specification does not cover

Almost all of it, and the largest omission is a number.

**1. The HLD states no performance target of any kind.** This was searched
rather than assumed. Every file in `docs/hld/` was grepped for `ms`,
`milliseconds`, `fps`, `latency`, `budget`, `kill`, `cold start`,
`throughput`, `frame rate`, `benchmark`, `perf`, `MB`, `GB`, `P0`, `interactive`,
`first frame`, `60`, `30` and `responsive`. The complete set of numeric
figures the HLD states that bear on cost at all is four, and not one of them
is a target this harness can pass or fail against:

| Figure | Section | What it actually is |
|--------|---------|---------------------|
| "Estimated 3–8 MB uncompressed before tuning, Naga dominating. Unmeasured today" | Appendix A, gate A4 | An estimate of binary size, explicitly unmeasured, and A4 is open |
| "a guaranteed maximum buffer size of 256 MiB", "roughly 300 MB" for a 512x512x600 16-bit series | Section 7 | A GPU buffer limit and a series size, which is why bricking is the normal path |
| "a 300 MB volume load has no pause behaviour to tune around, only a budget to respect" | Section 8 | A memory budget, and the budget is the caller's |
| "Window/level is thirty-two bytes, not a re-upload" | Section 26 | A uniform block size, which is an argument for a technique |

**There is no stated decode time, no stated first-frame time, no stated
interaction latency, no stated frame rate and no stated cold-start figure
anywhere in `docs/hld/`.** That is a finding and this plan does not fill it
with invention. The only stated method for setting a budget in this repository
is A7.3's, which is relative to the incumbent viewer, and A7.3 says in terms
"Do not invent a number."

**2. Where the harness lives, what it is called, and what it runs on.** The HLD
names "the benchmark harness" once and gives it no directory, no entry point
and no subject list. Section 15.1's layout has `tools/oracle/` and nothing
else under `tools/`.

**3. What a measurement record looks like and how it survives a commit.**
Nothing in the HLD. `ci/wasm-size-budget.json` is the repository's own
precedent and A7.3 names it as the mechanism to copy.

**4. What to do when the subject of a measurement does not exist.** The HLD
assumes the code being measured. This story runs in S03. The first decoder
lands in S07, the first device init in S11, the render graph in S12 and the
boundary with its event ring in S16.

**5. How a duration is compared across machines.** The oracle's determinism
claim is scoped to one machine and one browser build, and decision D14 refuses
bit-exactness across machines for pixels. A duration is strictly worse.

## Approach

The harness is an instrument with a **registry of subjects**, a **runner per
subject that has a subject**, and a **recorded baseline with a tolerance**. The
subject list is data. Adding a measurement when its story lands is a data edit
plus one runner file, and no part of the driver, the record format or the gate
changes. That is the whole answer to "built so it measures the real thing later
without being rewritten".

### 1. The subject registry, `tools/bench/subjects.json`, tracked

One row per thing this project will ever measure, each naming the story that
gives it a subject and the HLD section that defines it. Shape:

```json
{
  "id": "decode.frame",
  "title": "One Decoder::decode call over one corpus frame",
  "definition_hld": "18-codec-registry.md section 21, 04-boundary-and-data-path.md section 6",
  "unit": "ms",
  "dimensions": ["transfer_syntax", "rows", "columns", "bits_stored"],
  "tiers": ["n/a"],
  "subject_story": "F-023",
  "feeds": ["A1", "A2", "F-027", "F-028"]
}
```

`subject_story` is `null` when the subject exists today. Nine rows at the
outset, and the "What has no subject yet" section below lists them.

**The definition is fixed now, from the specification, for every row including
the ones with no subject.** That is deliberate and it is the part that stops a
rewrite later. `interaction.window_level_drag` is defined against sections 5.1
and 5.3 as pointer event to ring drain, so the story that lands the runner has
no latitude to redefine it as something easier to measure. `render.first_frame`
is defined against section 7 as command to first presented-frame ring event,
which is not a pixel read-back, because D3 forbids one.

### 2. The three states a subject can be in, and the fourth that is refused

| State | Meaning |
|-------|---------|
| `measured` | A runner exists, it ran, and the record carries a number and a host class |
| `unavailable` | `subject_story` is set and that story is not `done`. The record carries the F-ID and no number |
| `incomparable` | A number exists but the host class does not match the recorded baseline, so no comparison is made |
| refused | A number recorded for a subject whose story is not `done`. This fails the gate |

The fourth row is the defect this story is most likely to produce and it is
the one the gate is built around. A benchmark harness under time pressure
invents a workload, produces a plausible number, and that number then sits in
a tracked file describing nothing. `unavailable` is the correct output and it
is a useful one, because it names the story a reader should go and read.

### 3. The one runner that has a subject today, `wasm.cold_start`

`bin/ocelli.sh wasm` produces a real release artefact under HLD section 15.2's
profile. The runner opens it in headless Chromium and times fetch, compile,
instantiate and the first call to `ocelli_version()`, which is the module's
entire exported surface. This is not a fabricated workload. It is the actual
artefact doing the actual thing a browser does at session start, and it is the
half of gate A4 that has never been measured.

**Release only, and the reason is F-002's.** `opt-level = "z"`, `lto = "fat"`,
`codegen-units = 1`, `panic = "abort"` and `strip = true` apply to release,
so a dev-profile cold start is a different number rather than a smaller one.
`bin/ocelli.sh wasm` already defaults to release and the runner refuses a
`pkg/` it did not build.

**The number will be tiny and it must be reported with the same caveat
`docs/lld/build-targets.md` puts on the size number.** A module with one
exported function and no wgpu and no Naga tells you nothing about the cold
start of a feature-complete module. **A4 stays open**, and the first recorded
cold start is a baseline for regression detection during build-out, not an
answer to the gate. Re-baselining is expected repeatedly and each one names its
design plan.

### 4. The run record and the baseline

Two files, following the oracle's split exactly.

- `tools/bench/out/run.json`, gitignored, is one run. It carries every
  subject with its state, the host class, and the pinned versions.
- `ci/bench-baseline.json`, **tracked**, is the recorded measurement per
  subject per host class, with a tolerance, exactly as
  `ci/wasm-size-budget.json` holds one for size. A7.3 names this mechanism.

### 5. The host class, and why a duration needs one

A byte count is deterministic and a duration is not. Every record carries a
host-class fingerprint and a comparison is **refused** rather than passed when
the fingerprint does not match.

The oracle already collects most of what is needed in `run.json`'s `host` and
`page` blocks: `platform`, `release`, `arch`, `node`, plus `page.rendering`
with `renderer`, `softwareRasterizer`, `useCPURendering`,
`effectiveRenderBackend` and `devicePixelRatio`. **It collects no CPU model, no
core count, no memory and no load average**, which is enough to identify a
reference environment for pixels and not enough to normalise a duration. The
harness adds those four, which is the one place it extends rather than reuses
what F-010 built.

### 6. Where it runs, and how a number survives a commit

This is deviation D-04's problem restated for durations, and it is worse than
D-04's, because a shared CI runner can execute a benchmark and produce a
number that means nothing. Two mechanisms, deliberately separated.

**A floor gate, `bench`, that asserts the instrument and never a duration.**
It costs no GPU, it is deterministic, and it cannot be satisfied by intention:

1. Every registry row parses, and every `subject_story` resolves to a real
   F-ID in `docs/sprints/allocation.json`.
2. Every row with `subject_story: null` has a runner file, and **every row
   with a `subject_story` that is not `done` has none.** This is the
   anti-fabrication rule and it is the point of the gate.
3. **No entry exists in `ci/bench-baseline.json` for a subject whose story is
   not `done`.** A recorded number for a thing that does not exist is the
   failure this story must not produce, and this is what catches it.
4. Every baseline entry names a host class and a tolerance.
5. The harness's own unit tests pass.

**A comparison that is not a gate.** `bin/ocelli.sh bench` runs what it can and
writes the record. `bin/ocelli.sh bench --compare` compares against the
baseline for the matching host class, and reports `incomparable` on any other
machine. `--accept` re-baselines, and the design plan that used it says why.

**The comparison is deliberately not in `--floor` or `--sprint`.** A duration
comparison on a machine that did not record the baseline is either noise or a
skip, and this project's rule is that a skip is not a pass. Putting it in the
floor would produce a gate that is permanently amber, and an amber gate is a
gate that gets disabled, which `AGENTS.md` makes a change to
`.claude/WORKFLOW.md`. Open question 3 asks whether `/verify --profile release`
should require it, which is where `/release` already puts the corpus.

### 7. Timing is taken from outside the core, never by instrumenting it

The obvious implementation is a `#[wasm_bindgen] pub fn bench_mark()` in
`ocelli-wasm` and an `Instant::now()` in each crate. **Both are refused here.**

- A timing export in `ocelli-wasm` adds a boundary function that exists for
  the harness and ships to every user, and F-101 (E16.2) owns what the
  boundary exports.
- `std::time::Instant` does not work on `wasm32-unknown-unknown`, so a Rust
  timing primitive would need a browser branch, which is browser-specific code
  in a core crate and is precisely what decision D2 exists to prevent.

The browser half times with `performance.now()` from the page, outside the
module. A future native half times with `Instant` in `tools/bench`, outside the
crates. Neither adds a line to `crates/`.

### 8. No new trait, no criterion, no dev-dependency

`criterion` is not added. There is no Rust subject to measure, and
`AGENTS.md` refuses a construct with no user today. There is a second reason
recorded in `docs/lld/build-targets.md`: a dev-dependency that is not
wasm32-portable breaks `cargo check --all-targets` on that target, which is
how `proptest` reaching `wait-timeout` already constrains this workspace. When
F-023 and F-024 land a decoder, those stories add the Rust runner and the
dependency it needs, with a named user.

No new trait and no new generic. The runner lookup is a table keyed on subject
id, and the two implementers a trait would need do not exist today.

### 9. The mutation proof

Per HLD 27.3 and this sprint's stated rule, every new guard is observed red
**in a separate command from the one that adds it**. Four mutations, all
reverted: add a runner for a subject whose story is pending, add a baseline
entry for a subject whose story is pending, point a `subject_story` at an F-ID
that does not exist, and remove a host class from a baseline entry.

## What has no subject to measure yet, stated plainly

The audit is not close. Eight of thirteen crates are byte-identical twenty-line
stubs. `ocelli-codec`, `ocelli-pixel`, `ocelli-dicom`, `ocelli-viewport`,
`ocelli-volume`, `ocelli-geom`, `ocelli-cache` and `ocelli-seg` contain no
function at all. `ocelli-render` and `ocelli-compute` are type declarations
enforcing a device-ownership contract with nothing on either side. There is no
decode function, no render function, no LUT arithmetic, no
`request_adapter` call, no WGSL file, no worker and no ring drain loop anywhere
in the workspace. This is decision D7 holding, and it is the reason this story
is an instrument rather than a report.

| Subject | State at merge | Blocked on |
|---------|----------------|------------|
| `wasm.cold_start` | **measured** | nothing, this is the deliverable number |
| `decode.frame` | unavailable | F-023 (E4.1) codec dispatch and registry, S07 |
| `decode.transfer_syntax.htj2k` | unavailable | F-027 (E4.5), S09, and it is a P0 kill criterion |
| `decode.transfer_syntax.jpegls` | unavailable | F-028 (E4.6), S09, and it is a P0 kill criterion |
| `render.first_frame` | unavailable | F-037 (E6.1) device init S11 and F-038 (E6.2) render graph S12 |
| `interaction.window_level_drag` | unavailable | F-101 (E16.2) the wasm boundary and event ring, S16 |
| `interaction.scroll` | unavailable | F-101 (E16.2), S16 |
| `session.cpu_interactive` | unavailable | F-X003 (X1.3) CPU raster, S16 |
| `session.cpu_idle` | unavailable | F-X003 (X1.3), S16 |
| `cine.frame_change_rate` | unavailable | F-X003 (X1.3), S16 |
| `tier.startup_microbenchmark` | unavailable | **F-004 (E1.4), this sprint.** See open question 1 |

**A caution for whoever writes `subjects.json`, and it is not a small one.**
Every `subject_story` above was resolved through `docs/sprints/allocation.json`
rather than copied from a comment, and the comments in this repository are
wrong. `crates/ocelli-wasm/src/lib.rs`, `packages/core/src/ring.ts`,
`packages/core/src/bulk.ts`, `docs/lld/build-targets.md`,
`docs/lld/typescript-packaging.md`, `bin/ocelli.sh` and three earlier design
plans all name **F-096** as the story that builds the boundary, two of them
writing "F-096 (E16.2)" explicitly. **F-096 is E15.3, "Measurement persistence
and serialisation format", in S36.** E16.2 is **F-101**, "wasm boundary:
command channel, event ring buffer, state readback", in S16, and the API-design
story cited as F-095 in `packages/react/README.md` is **F-100** (E16.1). The
cluster is off by five, which is the signature of an F-numbering carried over
from before five stories were inserted. The gate described in section 6 of the
approach resolves every `subject_story` against `allocation.json` for exactly
this reason, so the registry cannot inherit the error. **Correcting the other
files is not this story's write set** and is reported separately.

**What the harness does about it: it says `unavailable` and names the story.**
It does not substitute a proxy workload, it does not time a stub, and it does
not record a number for any of these rows. The gate refuses a baseline entry
for any of them, which makes the refusal mechanical rather than a matter of
discipline.

## Boundary and tier

- wasm-bindgen: **not touched.** The harness loads the module `wasm-pack`
  already generates and adds no export. Decision D2 and deviation D-12 are
  untouched, and `ci/check-bindgen-isolation.sh` is not modified.
- Pixels across the boundary: **no**, and this is a live constraint rather
  than a formality. `render.first_frame` is defined as time to the first
  presented-frame ring event, not time to a pixel read-back, because a
  read-back would violate D3 and would also measure something no user
  experiences.
- Render-loop allocation: none. There is no render loop, and the harness never
  runs inside one. When a runner is added for an interaction subject it
  observes from the main thread and allocates nothing in the worker.
- unsafe: **none.** No file under `crates/` is modified, so neither permitted
  file is touched.
- **Tier A (WebGPU)**: the harness resolves no tier and requires none. The
  resolved tier is recorded as a dimension of every measurement, and a
  baseline for a tier-A run is never compared against a tier-B or tier-C run.
  No subject is tier-A only today.
- **Tier B (WebGL2)**: identical to tier A. Recorded as a dimension, never
  required. The oracle's environment already reports `softwareRasterizer` and
  `effectiveRenderBackend`, and the harness records both so that a tier-B
  number taken on a software rasteriser is distinguishable from one taken on
  hardware. Deviation D-07 says that misdetection presents as "the viewer is
  slow", so a harness that did not record it would help produce the defect.
- **Tier C (CPU)**: not `n/a`. `docs/spikes/A7-tier-c.md` says F-006 "must
  measure tier C from the start, including CPU cost per session", so the
  registry carries `session.cpu_interactive`, `session.cpu_idle` and
  `cine.frame_change_rate` as tier-C rows from this story. All three are
  `unavailable` at merge and blocked on F-X003, and they exist now so that
  A7.3's method is written into the instrument rather than remembered later.
  Idle cost is a separate row from interactive cost because A7.3 says they
  answer to different people and only one is negotiable.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | The registry parses, every `subject_story` resolves in `allocation.json`, and an unknown F-ID is rejected | `tools/bench/tests/registry_test.mjs` |
| `unit` | A subject whose story is not `done` resolves to `unavailable` and never to a number | `tools/bench/tests/state_test.mjs` |
| `unit` | A host class that differs in any recorded field yields `incomparable`, and an identical one yields a comparison | `tools/bench/tests/hostclass_test.mjs` |
| `unit` | The baseline round-trips, the tolerance is applied on both sides of the baseline, and `--accept` rewrites only the named subject | `tools/bench/tests/record_test.mjs` |
| `unit` | The gate refuses a runner for a pending subject and refuses a baseline entry for one | `scripts/tests/test_bench_check.py` |
| `browser` | The real release `pkg/` module loads under headless Chromium and `ocelli_version()` returns the workspace version, with a duration attached | `tools/bench/tests/cold_start_test.mjs` and the runner itself |

**No `fixture` row, and it is named rather than omitted.** HLD 27.2 R3 requires
a hand-computed fixture citing a DICOM section for every function doing pixel
arithmetic. This story computes no pixel, no coordinate and no LUT value. It
records durations and it reads none of the corpus. An omitted row and a
deliberate "no arithmetic here" read identically six months later, which is
why the row says so.

A recorded measurement with a tolerance is not a test in the taxonomy's sense,
which is F-002's finding about the size budget and it applies unchanged here.
Its red path is observed by mutation, per section 9 of the approach.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` enumerates viewport types, tool classes,
blend modes, VOI LUT functions, transfer syntaxes, segmentation
representations, core events and adapters. A benchmark harness is on none of
those axes.

**The `Covered by` column that `/design` step 6 and `/parity` both describe
does not exist in the tracked file.** `docs/hld/B-parity-surface.md`'s table
has `Surface`, `Count` and `Notes` and nothing else, so an epic ref cannot be
resolved through it. F-010 recorded the same finding and `docs/sprints/AS_BUILT.md`
carries it. It is noted again rather than worked around, because a step that
cannot be performed should not be reported as performed.

## Deviations

**None required.** Each candidate was checked and each fails to be one.

- Section 26 names a harness and gives it no shape. That is an absence the
  plan fills, which is `## What the specification does not cover`, not a
  departure from a stated design.
- The HLD states no performance target, so the harness cannot depart from one.
- Recording no number for a subject that does not exist is what section 26
  and decision D7 together imply, not a weakening of either.
- D-04 already covers CI having no GPU. Keeping a duration comparison out of
  CI is that deviation's consequence rather than a new one.

Open question 3 describes the one change that **would** need a new row, and
this plan does not take it without an answer.

## LLD impact

A new `docs/lld/benchmarks.md`, covering the subject registry, the four
states, the host class and its comparability rule, the baseline mechanism and
the `bench` gate. A row is added to `docs/lld/README.md`'s table.

`docs/lld/build-targets.md` gains the cold-start half of gate A4 alongside the
size half it already carries, and repeats that A4 stays open. That is one
paragraph, not a second document, because the two numbers are the same gate's
two halves and separating them is how one gets read as answering it.

`docs/lld/oracle.md` is **not** modified. The harness reuses none of the
oracle's code.

## Write set

Created:

- `tools/bench/run.mjs`, the driver and its `--help`
- `tools/bench/subjects.json`, the tracked subject registry
- `tools/bench/src/registry.mjs`, parse and validate
- `tools/bench/src/state.mjs`, the four states
- `tools/bench/src/hostclass.mjs`, the fingerprint and the comparability rule
- `tools/bench/src/record.mjs`, the run record, the compare and the accept
- `tools/bench/src/runners/wasm_cold_start.mjs`
- `tools/bench/page/index.html` and `tools/bench/page/app.mjs`
- `tools/bench/package.json` and `tools/bench/package-lock.json`
- `tools/bench/tests/registry_test.mjs`, `state_test.mjs`,
  `hostclass_test.mjs`, `record_test.mjs`, `cold_start_test.mjs`
- `ci/bench-baseline.json`, the tracked recorded measurement
- `scripts/bench_check.py`, the gate's self-check
- `scripts/tests/test_bench_check.py`
- `docs/lld/benchmarks.md`

Modified:

- `bin/ocelli.sh`, a `bench` command and a `bench` entry in `GATES` with its
  `run_gate` arm
- `.github/workflows/ci.yml`, the matching floor step, which
  `scripts/ci_floor_check.py` requires and will otherwise fail on
- `.gitignore`, `tools/bench/out/`
- `package.json`, a `bench` script beside the existing `oracle` one
- `docs/lld/README.md`, the new row
- `docs/lld/build-targets.md`, the cold-start paragraph

## Open questions

**1. Does F-004 own the startup micro-benchmark, with F-006 only recording
it?** `docs/spikes/A7-tier-c.md` requires a startup micro-benchmark as the
signal that decides tier resolution, and says "The micro-benchmark is the one
to trust, and the strings are the hint." `CURRENT_SPRINT.md` puts it in F-004's
definition of done. It is production code that runs on every session and
returns a tier, which is a different artefact from a development instrument
that returns a recorded number. This plan assumes **F-004 owns it and F-006
declares `tier.startup_microbenchmark` as a subject blocked on F-004**, so
F-006 reads its output and never reimplements it.
*Blocks*: whether the registry row is `subject_story: "F-004"` or the runner
ships in this story. Also blocks whether the two stories can run in parallel
this sprint, which they can under the assumed answer and cannot under the
other.

**2. Is the incumbent baseline measured in F-006 or in F-X003?** A7.3 step 1
says to measure the incumbent viewer on a representative GPU-less session and
record interactive and idle cost. The oracle's rig is already exactly that:
headless Chromium on SwiftShader with cornerstone3D 5.8.2 pinned. So F-006
**could** produce the incumbent number today, and it would be a real
measurement of a real subject. But `docs/spikes/GATES.md` calls the two
remaining A7 figures "acceptance criteria on F-X002 and F-X003", which places
it there.
*Blocks*: whether `session.cpu_interactive` and `session.cpu_idle` carry a
second reference-side runner in this story, and roughly three days of the two
weeks. Recommendation is to defer the run to F-X003 and ship the registry rows
and the record shape now, so F-X003 fills a slot rather than designing one.

**3. Should `/verify --profile release` require `bin/ocelli.sh bench
--compare` to have run green?** `/release` already requires `--require-corpus`,
making release "the one moment the GPU tier is not optional". The same argument
applies to a benchmark: a release is the moment the numbers should be true. The
cost is that a release then requires the machine that owns the baseline.
*Blocks*: whether `.claude/commands/verify.md` and `docs/RELEASE.md` are in
this story's write set. **If the answer is instead to put the comparison into
`gate --floor` with a host-class skip, that skip is a named exception to this
project's "a skipped gate is NOT a pass" rule and needs a new `D-NN` row in
`docs/hld/DEVIATIONS.md`**, phrased as: HLD section 26 requires measurement
before optimisation, and CI cannot produce a comparable duration, so the
benchmark comparison is a named skip in the floor. This plan does not take
that option and the row is described here rather than added.

**4. One playwright install or two?** `tools/oracle` pins `playwright` at
`1.62.1` in its own `package.json` outside the npm workspace. `tools/bench`
following the same pattern means a second `node_modules` and a second pin that
can drift, and a browser build difference between the two would silently make
the harness and the oracle report different environments. Sharing one install
couples the benchmark's story to the comparator's.
*Blocks*: `tools/bench/package.json`. Recommendation is a separate directory
with the pin held identical to the oracle's, plus a check in
`scripts/bench_check.py` asserting the two pins match, so drift is mechanical
rather than remembered.

**5. Does `ci/bench-baseline.json` hold one host class or several?** A project
with one developer machine wants one. A project where CI, a laptop and a
reference box all record wants a map keyed on host class, and then the
question is who prunes it.
*Blocks*: the baseline file's schema and `--accept`'s semantics.
Recommendation is a map keyed on host class from the start, because migrating
a scalar to a map later rewrites every entry.

**6. Should the `bench` gate share a record schema with F-011's comparator?**
Both write a per-subject or per-row verdict with an environment block, and
F-011 lands in this same sprint. Two schemas that are almost the same is a
known cost, and coupling a correctness verdict to a cost verdict is a worse
one.
*Blocks*: nothing in F-006's critical path, but it is cheaper to answer before
both land than after. Recommendation is to keep them separate and to reuse
only the host-class fingerprint, which F-006 defines and F-011 may read.

**7. Sequencing against F-X009.** F-X009 gives every repository guard a
standing test that fails when the guard stops guarding, and it touches the
same gate machinery this story adds a gate to. If F-006 lands first, F-X009's
sweep must pick up `bench`. If F-X009 lands first, F-006 must add the standing
test itself.
*Blocks*: whether `scripts/tests/test_bench_check.py` is written to F-X009's
standing-guard shape or to the ad-hoc shape used today. Recommendation is that
F-006 lands first and F-X009 absorbs `bench` into its sweep, since F-X009 is
3w and F-006 is 2w.

---

## Decisions taken in the design round

Answers to `## Open questions`, taken in the S03 consolidated round.

**1. F-004 owns the startup micro-benchmark. F-006 records it.** Both plans
recommended this split independently. They are two artefacts sharing a word:
F-004's runs on every production session and returns a tier, F-006's is a
development instrument that returns a recorded number. `subjects.json` carries
`tier.startup_microbenchmark` with `subject_story: "F-004"`, F-006 reads its
output and reimplements nothing. The dependency runs one way, so the two
stories build in parallel.

**2. The A7.3 incumbent measurement is deferred to F-X003, and the registry
rows ship now.** `docs/spikes/GATES.md` states in terms that the two remaining
A7 figures are acceptance criteria on F-X002 and F-X003, and a story that
quietly took one would move a resolved spike's acceptance criteria without
saying so. Define `session.cpu_interactive`, `session.cpu_idle` and
`cine.frame_change_rate` from A7.3 now, so F-X003 fills a slot rather than
designing one.

**3. `/verify --profile release` does not require `bench --compare`, and the
comparison does not enter the floor.** So no new deviation row is needed and
`.claude/commands/verify.md` and `docs/RELEASE.md` stay out of the write set.
The `bench` floor gate asserts the instrument's integrity only, which is
deterministic and needs no GPU, and the duration comparison runs on demand on
the machine that owns the baseline. A permanently amber gate is a gate that
gets disabled, and a host-class skip in the floor would be a named exception to
this project's "a skipped gate is not a pass" rule for no gain.

**4. `tools/bench` gets its own Playwright, pinned identically to the oracle's
`1.62.1`, with `scripts/bench_check.py` asserting the two pins are equal.**
Sharing one install would couple the benchmark's story to the comparator's, and
this sprint already has enough coupling in that area. A drifting browser build
between two harnesses would silently make them report different environments,
so the equality is a mechanism rather than something to remember.

**5. `ci/bench-baseline.json` is a map keyed on host class from the start.**
Migrating a scalar to a map later rewrites every recorded entry, and a recorded
measurement that gets rewritten stops being a measurement.

**6. F-006 and F-011 keep separate record schemas**, sharing only the
host-class fingerprint that F-006 defines and F-011 may read. A correctness
verdict and a cost verdict answer different questions and fail for different
reasons, and coupling them lets a slow machine make a pixel diff look like a
regression.

**7. F-006 lands before F-X009**, which absorbs `bench` into its sweep. F-006
lists the guards it adds in its handoff. `scripts/tests/test_bench_check.py` is
written in the shape used today, and F-X009 converts it rather than F-006
guessing at a shape that does not exist yet.

**8. The section 26 paraphrase in `AGENTS.md` is corrected in this story.**
Section 26 reads "Measure **with the benchmark harness** before optimising
anything" and `AGENTS.md` line 138 drops the instrument, which is the
difference between a habit and a dependency. This story is the one that makes
the instrument exist, so it is the one that restores the sentence.
