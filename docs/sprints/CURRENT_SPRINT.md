# Current sprint, S04

**Milestone**: M1, foundations and the differential oracle.
**Branch**: `sprint/s04`
**Opened**: 2026-09-06
**Goal**: Turn the comparator's verdict into a gate every pull request must
pass, extend the diff beyond pixels to metadata and geometry, and clear the
follow-up debt S03 declared rather than carrying it into the port.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-012 | E2.4 | CI gate: every PR renders the full corpus | Test | 3w | pending |
| F-013 | E2.5 | Metadata diff harness (LUT values, geometry, spacing) | Test | 2w | pending |
| F-015 | E2.7 | Stable render-hash emission from the comparator | Test | 2w | pending |
| F-X001 | X1.1 | Tier C, software-adapter detection, and the feature-availability contract | Rust | 4w | pending |
| F-X008 | Y1.3 | One parity-target version string, and a licence in the published wasm package | Build | 1w | pending |
| F-X010 | Y1.5 | CI floor equivalence, and the identical --sprint and --all gate profiles | Build | 2w | pending |
| F-X011 | Y1.6 | Cross-machine reference determinism, and what the oracle claims about it | Test | 2w | pending |
| F-X012 | Y1.7 | The reference's own SIGMOID width divergence, and what D14's bound says about it | Test | 2w | pending |
| F-X013 | Y1.8 | Price the HTJ2K decoder route after gate A1 failed | Test | 3w | pending |
| F-X014 | Y1.9 | Close the two guard holes F-X009 declared, and watch the refusals its census leaves to nothing | Build | 2w | pending |
| F-X015 | Y1.10 | Execute the skills' worked examples, because the skills gate asserts nothing about their numbers | Build | 1w | pending |
| F-X016 | Y1.11 | Try the next adapter when the best candidate cannot open a device, and record what was attempted | Rust | 1w | pending |
| F-X017 | Y1.12 | Close the measured escapes from the wasm linear memory view ban, which needs type-aware linting | Build | 2w | pending |
| F-X018 | Y1.13 | Make close-preflight see the sprint review, and key its verification on the tree it is about | Build | 1w | pending |
| F-X019 | Y1.14 | Decide whether a CI step that is not guaranteed to run counts as CI running the gate | Build | 1w | pending |
| F-X020 | Y1.15 | Stop gen_sprint_plan.py's write mode silently overwriting a hand-curated SPRINT_PLAN.md | Build | 1w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** No script reads this file's table: `scripts/backlog_check.py` never
opens it, and `scripts/sprint_workflow.py` reads only the sprint name from the
title. `docs/sprints/BACKLOG.md` is the authority on status. Read the two
together rather than trusting either sentence:

```bash
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S04 / {print $2, $9}'
```

## What this sprint is

S03 made the oracle answer. **S04 makes its answer binding.** F-012 puts a full
corpus render behind every pull request, F-013 extends the diff from pixels to
the metadata and geometry the pixels are derived from, and F-015 emits a stable
hash so a change in output is detectable without storing every frame. F-X001
builds tier C, which is deviation D-07's CPU path and the reason a machine with
no GPU renders anything at all.

**This sprint is packed well past the stated caps and that is deliberate but
worth seeing.** The allocation's own rule at `docs/sprints/SPRINT_PLAN.md` is
at most six stories and at most sixteen estimated engineer-weeks per sprint.
S04 carries sixteen stories and thirty weeks, because eight of them are `F-X`
follow-ups S03 declared rather than hid. Run the commands above rather than
trusting these numbers. **An operator splitting this sprint is making a
reasonable call, not overriding the plan.** The nine debt stories are
individually small, mostly one or two weeks, and none blocks the four that
carry the milestone forward.

## What is carried in

Nothing is carried forward incomplete. S03 closed all seven of its stories and
`gate --sprint` was green over 28 gates with corpus=pass.

Three things arrive as knowledge rather than as code:

- **Gate A1 failed.** `openjp2` does not link for `wasm32-unknown-unknown` and
  traps on every codestream when forced to, so HTJ2K reports unavailable. HLD
  section 15.2 names `openjp2` as the wasm choice and that is measured not to
  work. **F-X013** prices a route.
- **The reference is wrong about SIGMOID.** cornerstone3D 5.8.2 applies
  LINEAR's `(w - 1) / 2` to SIGMOID, where PS3.3 C.11.2.1.3.1 gives SIGMOID its
  own constraint. Unreachable while every windowed corpus row resolves LINEAR.
  **F-X012.**
- **The guard harness has declared holes.** `python3 scripts/guard_census.py`
  prints the bucket watched by nothing, and two guards are declared defective
  rather than fixed. **F-X014.**

## The defect class this sprint is exposed to

**The danger changes shape this sprint. Until now the question was whether a
measurement is right. From F-012 onward it is whether a GREEN GATE MEANS
ANYTHING.**

That is not generic. S03 already produced the exact shape: the comparator
reports `98 views: 70 pass, 0 fail, 28 unmeasured`. A gate that reads `0 fail`
as success is claiming a verdict over 98 views when it has one for 70. The 28
are class two, where HLD 25.1 states no threshold, and decision D14 forbids a
pass against a bound nobody wrote. **F-012 must fail when coverage drops, not
only when a comparison fails**, or it becomes a gate that passes because it
measured nothing. The same trap sank F-010's first sweep, where a broken
mutation harness gave every "all refusals red" result a red baseline.

Three more, each specific:

- **F-X001 is exposed to a second copy of the LUT chain.** HLD section 18
  requires that arithmetic to exist exactly once, and tier C must reuse
  `ocelli-pixel` rather than reimplement it. A second copy behind a tier check
  is the same defect as a second copy anywhere else, except that it only runs
  on hardware nobody develops on, so no reviewer will see it fail.
- **F-013 is exposed to the transposed spacing index.** `PixelSpacing[0]` is
  the spacing between ROWS and multiplies the COLUMN direction cosine, per
  PS3.3 C.7.6.2.1.1. Getting it backwards is invisible on the square-pixel
  studies that are most of any corpus and wrong on every non-square one. The
  corpus carries non-square rows precisely so this cannot pass unnoticed.
- **F-015 is exposed to a hash that is stable for the wrong reason.** A hash
  taken after quantisation, or over a buffer whose padding is not deterministic,
  is stable and says nothing. It must change when the pixels change, and the
  proof of that is a mutation observed red, not an argument.

## What done means

- **F-012** fails a pull request when a comparison fails AND when coverage
  drops, and its output distinguishes the two. It states how many views it
  claimed a verdict over rather than how many it looked at.
- **F-013** compares LUT parameters, geometry and spacing against values
  computed independently from PS3.3, not against anything Ocelli produces, and
  its fixtures cite their section.
- **F-015** emits a hash whose change is demonstrated by a mutation observed
  red before the story is claimed.
- **F-X001** produces a written answer on the feature-availability contract,
  and tier C reuses `ocelli-pixel`. A feature that cannot run on the resolved
  tier reports unavailable and never quietly produces a different result.
- **F-X013** produces a written decision in `docs/spikes/`, priced, not a
  passing test. Gate A1 is answered `Fail` and this story says what replaces
  the route.
- **F-X014** closes G-02 and G-04 by making their declared-defect probes start
  passing, and the census's uncovered count goes DOWN. The ratchet fails until
  each declaration is removed in the same change that closes it.
- **F-X019** decides one question: whether the right-hand side of `&&` counts
  as CI running a gate. Its acceptance test is already written and asserts the
  hole is still open, so it goes red when the story closes.

## Dependency order

Every declared dependency is `done`, so nothing in this sprint is blocked at
its start. F-012, F-013 and F-015 depend on F-011. F-X001 and F-X016 depend on
F-004. F-X008, F-X010, F-X011 and F-X012 depend on F-010. F-X013 depends on
F-X006. F-X014, F-X015, F-X018, F-X019 and F-X020 depend on F-X009. F-X017
depends on F-005.

F-012 and F-015 both read the comparator's output and F-013 adds a second
comparison beside it, so those three share `tools/oracle` and must not run
concurrently in separate worktrees without a plan for the collision.

**The oracle is a serial resource.** Two workers running WebGPU or headless
Chromium tests concurrently on one machine contend for the device and produce
timeouts that read exactly like rendering failures.

## Standing expectations

Read the tracked Markdown under `docs/hld/` before implementation. It is the
normative source. Record an implementation departure in
`docs/hld/DEVIATIONS.md` rather than changing a gate or tolerance to make a
check pass.

**A tolerance change is a pull request with a rationale, reviewed like code.**
S03 added a signed-mean bias bound to section 25.1 by operator decision, and
that is the shape such a change takes.

Every new guard is observed red before it is claimed, and the mutation that
proves it must not be run in the same command that adds it.

**A count written in prose is a claim with no owner.** Sixteen review passes in
S03 falsified counts in this repository's own records repeatedly, including in
the paragraph warning against them. Where a number matters, write the command
that prints it.
