# Current sprint, S03

**Milestone**: M1, foundations and the differential oracle.
**Branch**: `sprint/s03`
**Opened**: 2026-09-05
**Goal**: Turn the oracle from an instrument that renders into one that
returns a verdict, resolve the runtime tier, and close the two Appendix A
gates that decide the codec architecture.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-004 | E1.4 | Runtime capability detection and tiering (WebGPU / WebGL2 / SIMD / threads) | Build | 2w | pending |
| F-005 | E1.5 | Error model, panic-to-JS mapping, structured logging | Build | 2w | pending |
| F-006 | E1.6 | Benchmark harness: decode, first frame, interaction latency | Build | 2w | pending |
| F-011 | E2.3 | Pixel-diff comparator with per-modality tolerance policy | Test | 3w | pending |
| F-X006 | Y1.1 | Answer Appendix A gates A1 (HTJ2K) and A2 (JPEG-LS) against our own decoders | Test | 3w | pending |
| F-X007 | Y1.2 | Oracle volume and MPR reference renders, so the spacing rows are asked something | Test | 3w | pending |
| F-X009 | Y1.4 | A standing test for every repository guard, not a mutation run once at authoring time | Build | 3w | pending |

Count the rows rather than trusting a sentence:

```bash
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
```

## What this sprint is

S02 built an instrument that renders. S03 makes it answer. F-011 adds the
comparator and the tolerance policy, which is the first point in this project
where a machine says a pixel is right or wrong. Around it, F-004 resolves which
tier a session runs on, F-005 gives failures a shape the shell can act on, and
F-006 measures what things cost. F-X006 closes the two Appendix A gates that
decide the codec architecture, and F-X007 and F-X009 close gaps S02 recorded
rather than fixed.

## What is carried in

Nothing is carried forward incomplete. S02 closed all five of its stories and
`gate --sprint` was green over 23 gates with zero skips.

Three things arrive as knowledge rather than as code, and each has a story:

- **The reference is wrong about SIGMOID.** cornerstone3D 5.8.2 applies
  LINEAR's `(w - 1) / 2` to SIGMOID, where PS3.3 C.11.2.1.3.1 gives SIGMOID its
  own constraint. Unreachable today because all 85 windowed corpus rows resolve
  LINEAR. **F-X012**, S04.
- **Sixteen corpus frames are too saturated for a pixel diff to show a
  divergence in the clipped values.** F-010 names them in `run.json` and did not
  solve them, because a second render at a wider window is a comparator
  decision. **This sprint's, in F-011.**
- **A1 and A2 are not answered.** F-010 showed cornerstone3D renders the HTJ2K
  and JPEG-LS rows, and its own review caught the harness claiming that answers
  the gates. **F-X006.**

## The defect class this sprint is exposed to

**A verdict is only as good as its tolerance and its reference, and both fail
silently.**

The dangerous comparator defect is a tolerance that absorbs a real divergence.
HLD section 25.1 fixes the numbers in advance for exactly this reason:
monochrome 16-bit is maximum absolute difference of 1 LSB on at least 99.9% of
pixels with zero pixels differing by more than 2. A comparator that passes the
corpus on the first run is more suspicious than one that fails, and the answer
to a failure is never a wider tolerance. That is a design-plan decision with a
recorded rationale, reviewed like code.

The second half is subtler and S02 found it. **The reference can be wrong.**
Where cornerstone3D diverges from PS3.3, a diff measures our correct arithmetic
against its incorrect arithmetic and reports the difference as ours. Decision
D14 publishes a measured divergence, so the comparator has to be able to say
which side a difference is on. The SIGMOID case is the known instance and it
will not be the last.

The dangerous tiering defect is deviation D-07's, stated in
`docs/spikes/A7-tier-c.md`: on a host with no GPU a software rasteriser
presents a conforming WebGL2 context, so `Caps` as HLD section 7 specifies it
resolves tier B and runs GPU paths on a rasteriser slower than our own CPU
path. It is invisible, and it presents as "the viewer is slow" rather than as a
misdetection. Detect by renderer string, adapter type **and** a startup
micro-benchmark, and trust the benchmark.

The dangerous gate defect is the one S02 measured. **A guard mutated in the
same command that adds it has been observed to fire once and has nothing
watching it afterwards.** F-010's sweep found its own mutation harness was
broken, so every earlier "all refusals red" result had a red baseline and
proved nothing, and six of 26 refusals turned out to be watched by nothing.

## What done means

- **F-004** resolves a tier and distinguishes a hardware adapter from a
  software one, with the micro-benchmark, and an operator override exists.
- **F-005** maps a Rust panic to a JS error the shell can act on without
  poisoning the wasm instance, per HLD section 23.
- **F-006** measures decode, first frame and interaction latency, and records
  the numbers rather than describing them.
- **F-011** returns a per-row verdict against HLD section 25.1's written
  tolerances, decides what to do about the sixteen low-information rows, and
  can attribute a divergence to a side.
- **F-X006** produces a written answer per gate in `docs/spikes/`, not a
  passing test. A1 is bit-exactness of HTJ2K through openjp2 under wasm32
  against OpenJPH. A2 is the JPEG-LS architecture decision.
- **F-X007** reference renders volumes and MPR, so the ten non-uniform spacing
  rows exercise the volume builder's refusal path.
- **F-X009** gives every guard a standing test that fails when the guard stops
  guarding.

## Dependency order

Every declared dependency is `done`. F-004, F-005 and F-006 depend on S02's
build stories. F-011, F-X007 and F-X009 depend on F-010. F-X006 depends on
F-009 and needs no other story in this sprint.

F-011 is the one with a real ordering constraint inside the sprint: F-X007 adds
reference renders that F-011 will then compare, so a comparator written before
F-X007 lands must not assume stack-only input.

## Standing expectations

Read the tracked Markdown under `docs/hld/` before implementation. It is the
normative source. Record an implementation departure in
`docs/hld/DEVIATIONS.md` rather than changing a gate or tolerance to make a
check pass.

**A tolerance change is a pull request with a rationale, reviewed like code.**
That rule matters more in this sprint than in any before it, because this is
the sprint that first has a tolerance to change.

Every new guard is observed red before it is claimed, and the mutation that
proves it must not be run in the same command that adds it.
