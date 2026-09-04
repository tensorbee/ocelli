# Ocelli

A Rust and WebAssembly medical imaging core for the browser. Named after the
three simple eyes a bee carries alongside its compound eyes. Pronounced
**oh-SELL-eye**.

**Status: bootstrap.** The workspace, the workflow and the gates exist. No
imaging code has been written. See `docs/sprints/CURRENT_SPRINT.md`.

## Standing on cornerstone3D

[cornerstone3D](https://github.com/cornerstonejs/cornerstone3D) is the mature
open-source imaging engine in this space, it is MIT licensed, and Ocelli owes
it a great deal. We read it, we learn from it, and we measure ourselves against
it deliberately rather than incidentally.

It is our **reference oracle**. Every frame Ocelli renders is diffed against
the frame cornerstone3D renders from the same study, within a written
per-modality tolerance, before it can merge. A library that can serve as a
correctness reference for another implementation is a library that got a great
many hard details right, and reproducing them is most of the work here.

Ocelli is a different set of engineering trade-offs, not a verdict on that one:
a Rust core so the arithmetic is checked by the type system, WebGPU so compute
is available, and bounded GPU residency so memory stops being a function of
series size. Those trade-offs cost a new API and a migration, which is a real
price and one worth being honest about.

## What it is

The **shell** is TypeScript on the main thread: DOM, pointer events, the SVG
annotation layer, tool interaction state, framework bindings, DICOMweb fetch.
The **core** is Rust in workers: parsing, decode, the LUT chain, geometry,
rendering through wgpu.

Between them, a deliberately narrow boundary carrying three things and nothing
else: typed commands down, raw bytes into linear memory down, and events up
through a ring buffer drained once per animation frame. **Pixels never cross
it.** Decoded frames never become JavaScript objects on the way to the GPU,
which is the main architectural gain and the reason the boundary is shaped the
way it is.

## What we are trying to add

Ocelli runs on two very different machines, and it is honest about which claims
belong to which. GPU-less sessions are a first-class target, not a fallback:
virtualised desktops without GPU passthrough, locked-down builds where
acceleration is disabled by policy, and hosts whose driver is blocklisted are
all common in clinical estates.

**On a GPU client**, for diagnostic reading and advanced visualisation:

- **Bounded memory on unbounded data.** GPU residency becomes a configured
  number rather than a function of series size.
- **GPU compute.** Essentially nobody in medical imaging is taking WebGPU's
  compute advantage, and it is not a modest one.

**On a session with no GPU**, for review, triage and conference, where a CPU
rendering tier carries the work:

- **Annotations that are DICOM SR natively**, so every measurement is
  interoperable by construction rather than by export.
- **Attestable rendering.** A published, measured divergence bound and a
  reproducible output hash. A CPU renderer is the easiest target to make
  reproducible, because there is no driver stack underneath it.
- **Live multi-user sessions**, which is what a tumour board or a teaching
  session actually is.

**On both**: full parity, MPR, measurement, and real multi-monitor without a
browser extension.

Every one of these is measurable, and the measurements are the marketing.
**Every benchmark we publish names its tier**, because a number without the
hardware it ran on is not reproducible and, on a mixed estate, is not honest
either.

## Layout

```
crates/           13 Rust crates. wasm-bindgen appears in exactly one
  ocelli-core/      types, coordinate spaces, error model. No I/O
  ocelli-dicom/     parse, metadata, providers
  ocelli-codec/     decoder registry and adapters
  ocelli-pixel/     the LUT chain, frame model
  ocelli-volume/    volume assembly, reslicing
  ocelli-cache/     budgeted LRU across encoded, decoded and GPU tiers
  ocelli-compute/   WGSL compute kernels, sharing the renderer's device
  ocelli-render/    wgpu device, render graph, shaders, backend tiers
  ocelli-viewport/  viewport and scene model, camera, presentation state
  ocelli-geom/      hit-testing, projection, measurement mathematics
  ocelli-seg/       segmentation state and the three representations
  ocelli-wasm/      ** the only wasm-bindgen crate **
  ocelli-native/    desktop and server entry points. Phase 2 and 3, stubbed

packages/
  core/             @ocelli/core, the TypeScript shell
  react/            @ocelli/react

examples/
  viewer-react/     the example viewer, and the manual smoke test

tools/oracle/       the differential harness against cornerstone3D
corpus/             manifest only. The data is not in git

docs/hld/           the authoritative Markdown specification
docs/sprints/       backlog, sprint plan, trackers
.claude/            the workflow: commands, skills, plans, reviews
.agents/skills/     generated Codex adapters for the same workflow
```

## Getting started

```bash
git config core.hooksPath .githooks
npm ci
bin/ocelli.sh gate --floor
```

`docs/DEVELOPER_SETUP.md` has the rest.

## The one thing to know before contributing

**The dangerous defect here is not the crash, it is the pixel that is quietly
wrong.** Quietly wrong code is produced by reasonable people making locally
reasonable choices.

The worked example, from the specification: at the centre of a soft-tissue CT
window, the two DICOM VOI LUT functions `LINEAR` and `LINEAR_EXACT` differ by
0.32 of 255. Invisible to a human comparing screenshots. Immediate to a pixel
diff.

That is why the differential oracle is built before the code it validates, why
every function doing pixel arithmetic needs a fixture with hand-computed values
citing its DICOM section, and why the tolerance policy is written once and held
rather than tuned per failure.

## Workflow

One workflow, two hosts. Claude Code reads `.claude/commands/`, Codex reads the
generated adapters in `.agents/skills/`. They are the same instructions and a
gate refuses them when they drift.

```text
/design F-XXX  ->  /start-feature  ->  /implement-feature
               ->  /microscope     ->  /verify  ->  /complete-feature
```

`/run-sprint` drives a whole sprint. `/close-sprint` ends one. `/release` is
the only command that publishes anything. `.claude/WORKFLOW.md` is the law of
the project.

## Contributing

`CONTRIBUTING.md`, and please read it before writing code. Two rules there are
absolute rather than careful: no patient data ever enters this repository, and
the source-provenance policy governs which third-party projects may be read at
all, not merely depended on.

Security issues, including a wrong pixel value or wrong geometry, go through
`SECURITY.md`.

## Licence

Dual-licensed under **MIT** or **Apache-2.0** at your option, texts in
`LICENSE-MIT` and `LICENSE-APACHE`.

Permissive on purpose: integrators embed an engine like this inside closed
products, and a disclosure obligation ends that conversation before it starts.
