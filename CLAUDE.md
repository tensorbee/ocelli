# Ocelli

A Rust and WebAssembly medical imaging core for the browser. Rust core in
workers, TypeScript shell on the main thread, a deliberately narrow boundary
between them.

**cornerstone3D is our reference oracle, and we read it and learn from it.** It
is MIT licensed and in bounds. Phase 1 targets feature parity with v5.8.9 and
every frame is diffed against it before merge, which is a debt to acknowledge
rather than a rivalry to advertise. Keep that tone in anything public.

**Read `.claude/WORKFLOW.md` before changing anything.** It wins on process.
`docs/hld/` wins on what to build.

## The thing to hold in mind

**The dangerous defect here is not the crash, it is the pixel that is quietly
wrong.** Quietly wrong code is produced by reasonable people making locally
reasonable choices, which is exactly what a competent agent does at speed.

The HLD's worked example, from `docs/hld/15-lut-chain.md` section 18.3: at the
centre of a soft-tissue CT window, VOI `LINEAR` and `LINEAR_EXACT` differ by
0.32 of 255. Invisible in a screenshot. Immediately visible in a pixel diff.
That single number is why the oracle is built before the code it validates.

## Architecture, in five sentences

The **shell** is TypeScript on the main thread: DOM, pointer events, the SVG
annotation layer, tool interaction state, framework bindings, DICOMweb fetch.
The **core** is Rust in workers: parsing, decode, the LUT chain, geometry,
rendering. **Three channels cross the boundary and nothing else**: typed
commands down, raw bytes into linear memory down, events up through a ring
buffer drained once per frame. **Pixels never cross it**, which is the main
architectural gain. **`wasm-bindgen` appears in exactly one crate**, which is
what makes the desktop and server targets new entry points rather than
rewrites.

## Standing decisions you must not relitigate

From `docs/hld/11-decision-log.md`. Raise a deviation in
`docs/hld/DEVIATIONS.md` rather than improvising around one.

| # | Decision |
|---|----------|
| D1 | Feature parity with a NEW API. Not a drop-in shim. |
| D2 | wasm-bindgen in one crate only. |
| D3 | Pixels never cross the boundary. |
| D4 | Events polled from a ring per frame, never a JS callback per event. |
| D5 | Single-threaded, one wasm instance per worker. Stable Rust, no SharedArrayBuffer. |
| D6 | WebGPU primary, WebGL2 a declared tier. Compute degrades, never fails. |
| D7 | The validation oracle exists before port code. |
| D11 | Chunked residency is the default path, not a fallback above a threshold. |
| D13 | The annotation type IS a DICOM SR content tree. |
| D14 | Claim MEASURED divergence, never bit-exact reproducibility. |

**Two numbering namespaces, and they nearly collide.** `D1` to `D14` above are
the HLD's own decisions. `D-01` to `D-07` in `docs/hld/DEVIATIONS.md` are
places this repository departs from the HLD. **`D7` and `D-07` are different
things**: D7 is the oracle-before-port-code decision, D-07 is the CPU tier.
Always write the hyphen for a deviation.

## Tiers, and the one the HLD does not have

HLD §7 gives two, both GPU: **A** is WebGPU, **B** is WebGL2 through wgpu's
downlevel profile. Deviation **D-07** adds **C, CPU**, because §7 leaves a
machine with neither rendering nothing at all.

Every rendering or compute feature declares an answer for **all three**, and
"not applicable" is a legitimate answer that an omitted row is not. A feature
that cannot run on the resolved tier **reports unavailable**. It never quietly
produces a different result, which is §31's rule generalised.

Tier C reuses `ocelli-pixel` rather than reimplementing the LUT chain. §18
requires that arithmetic to exist exactly once, and a second copy behind a tier
check is the same defect as a second copy anywhere else, except that it only
runs on hardware nobody develops on. Spike gate **A7** (`docs/spikes/GATES.md`)
decides whether tier C is worth building at all, and F-X001 to F-X004 are the
stories.

## Hard rules

- **No patient data in this repository, ever.** The corpus lives outside git
  behind `corpus/manifest.tsv`. The pre-commit hook refuses staged DICOM by
  magic bytes as well as by suffix. No allowlist.
- **dwv and Horos must not be opened**, by a person or an agent. GPL-3 and
  LGPL-3-plus-AGPL-3 respectively, and translating source into Rust is a
  translation. Take their ideas from DICOM PS3.3 and PS3.16 instead. Grok must
  not be depended on. `scripts/source_provenance_check.py` enforces this.
- **No `unsafe`** outside `ocelli-wasm/src/ring.rs` and
  `ocelli-core/src/cast.rs`.
- **wgpu is pinned exactly.** Treat GPU code that compiles first try with
  suspicion. Agents confidently emit APIs that have not existed for two years.
- **Never re-type file content from tool output.** Tool output truncates and
  the truncation is silent. Edit in place.
- **Tests derive from the specification or the oracle, never from reading the
  implementation.** An agent asked to test a function will assert what it does,
  not what it should do.
- **Every function doing pixel arithmetic needs a fixture with hand-computed
  values citing its DICOM section.** This is the defect class that reaches
  patients.
- **No em-dash, no prose semicolon** in tracked prose or commit messages.
  `docs/hld/` is exempt, it is the author's text.

## Commands

```bash
bin/ocelli.sh check <crate>        # inner loop
bin/ocelli.sh gate --list          # what each gate covers
bin/ocelli.sh gate --floor         # what CI runs: no GPU, no corpus
bin/ocelli.sh gate --all           # everything, including oracle and corpus
bin/ocelli.sh wasm                 # wasm-pack build + size budget
bin/ocelli.sh oracle               # the differential harness (needs a GPU)
npm run dev                        # the example viewer
```

Everything runs natively. There is no container path, because a container has
no GPU and no browser and Ocelli needs both.

## What a human must actually check

`docs/hld/24-agent-code-standards.md` section 27.3, and it is not a formality:

- Every `as` cast and every rounding decision.
- LUT and geometry arithmetic against the cited specification section, **not
  against the comment above it**, which was generated by the same process as
  the code.
- That a new test would actually fail if the code were wrong. Mutate one
  constant, re-run, confirm it goes red.
- Any wgpu call against the pinned version's documentation rather than from
  memory.

## Current state

Bootstrap. Thirteen crate skeletons, no implementation. S01 is F-001 (core
types) and F-009 (the corpus). See `docs/sprints/CURRENT_SPRINT.md`.

Nothing in `docs/hld/` Part II has been implemented yet, so where this file and
that directory disagree about what exists, that directory is describing the
target and this one is describing today.
