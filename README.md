<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/ocelli-slice-svg/ocelli-slice-logo-light.svg">
    <img alt="Ocelli" src="docs/ocelli-slice-svg/ocelli-slice-logo-dark.svg" width="320">
  </picture>
</p>

# Ocelli

A Rust and WebAssembly medical imaging core for the browser. Named after the
three simple eyes a bee carries alongside its compound eyes. Pronounced
**oh-SELL-eye**.

**Status: early.** The workspace, the workflow and the gates exist, and so now
do the differential oracle's two halves, the golden corpus, every build target,
the runtime tier resolution and the error model. Decision D7 still holds in
the narrow sense: the oracle has no Ocelli frame to judge. See
`docs/sprints/CURRENT_SPRINT.md` for the sprint and `CLAUDE.md` for what
exists in the order it matters.

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
  example-server/   loopback helper for the example viewer's frame-open, series-open and overlay panels
  folder-viewer/    local series page for a folder on this machine

tools/
  oracle/           the differential harness against cornerstone3D
  bench/            the benchmark registry and its runners
  spikes/           throwaway harnesses for the Appendix A gates

corpus/             manifest only. The data is not in git

docs/hld/           the authoritative Markdown specification
docs/sprints/       backlog, sprint plan, trackers
.claude/            the workflow: commands, skills, plans, reviews
.agents/skills/     generated Codex adapters for the same workflow
.opencode/commands/ generated OpenCode slash commands for the same workflow
```

## Getting started

```bash
git config core.hooksPath .githooks
npm ci
bin/ocelli.sh gate --floor
```

`docs/DEVELOPER_SETUP.md` has the toolchain, the hooks and the gates.

## Start the example viewer

The showcase is `examples/viewer-react`, package `@ocelli/example-viewer`.
From the repository root:

```bash
npm ci
bin/ocelli.sh wasm
npm run dev
```

Open <http://localhost:5173>. Vite prints that address when the server is ready.

`npm run dev` also starts `ocelli-example-server` on `127.0.0.1`. The helper
prints only its base URL. The Frame open section posts to it through Vite.
The synthetic button asks the helper to map stored 50 through LINEAR_EXACT
centre 100 and width 200, then shows the display value and the padding count.
The Series section posts three synthetic objects, or a multipart body of files
you choose, and shows each display group, its order, its timing and Loop or
Bounce. The Overlay section posts the synthetic 4 by 4 and shows whether image
pixel (1, 1) is set and whether the shutter hides it. The Interpolation
section posts the 2 by 2 fixture and shows 138.75 for linear and the
nearest corner for nearest. A file you choose is
posted and discarded. It is not written into this
repository. The header reads `core not built` until the page worker loads
the wasm module.

`bin/ocelli.sh wasm` writes the module to `crates/ocelli-wasm/pkg`. That
directory is a build artefact and is not committed, so a clean clone still
opens with the header on `core not built`. After that build, reload the page.
The worker loads the module, and the header reads `core ready` only after
that load succeeds. The build alone does not flip the header.

The page mounts one stack viewport. It is the showcase, so a library behaviour
is finished only when this page has a control for it. To open a folder of
DICOM files on this machine, use the local series page below.

## Start the local series viewer

`examples/folder-viewer` lists series from a folder you choose and shows one
image at a time. It listens on this machine only.

```bash
cargo run -p ocelli-folder-viewer
```

The process prints one line, the page URL, and nothing else. It has the form
`http://127.0.0.1:12345/`. The port is chosen at startup unless
`OCELLI_PORT` names a fixed one. `0` also means chosen at startup. A fixed
port keeps one bookmarkable address, and a second copy then fails to bind
instead of starting beside the first. A value that is not a port number,
including an empty one, stops the process with an error. Open that exact URL in
Chrome or Edge. The Folder control is a directory picker.

```bash
OCELLI_PORT=8742 cargo run -p ocelli-folder-viewer
```

1. Choose Folder and pick a directory. The browser reads the files. Leave them
   where they are. Do not copy them into this repository.
2. The left column has one row per Series Instance UID. A row shows the series
   number, the modality, the description and the image count. A file with no
   Series Instance UID keeps its own row.
3. Select a row. The viewport shows the first image and keeps it. The arrow
   keys move one image. A wheel without Ctrl moves one image after about 40
   pixels of travel. The scrubber reads `n / total`. Coming back to a series
   restores the image you left.
4. Space plays and pauses. Playback loops. The FPS control runs from 1 to 30
   and starts at 10.
5. The toolbar is a row of icons. Point at one to see its name, its key and
   what it does. The groups are the navigation tools (Scroll, Pan, Zoom,
   W/L), the drawing tools (Length, Rectangle, Ellipse, Freeform, Text), the
   view (Invert, Rotate 90, Flip horizontal, Flip vertical), the display
   (Pointer cross, Magnifier, Corners, Orientation letters), the layout
   (1 by 1, 2 by 2), and Key image, Headers, Fit, Reset and Full screen.
   Scroll starts active. The right column holds the window presets, the marks
   and the mouse and key lists. The icons at either end of the top bar, or
   the `[` and `]` keys, hide and show the two side columns.
6. The left button uses the active tool. The right button pans. Control with
   the left button also pans. The middle button zooms from the place you
   pressed. A pinch, or Ctrl with the wheel, zooms at the pointer. A sideways
   swipe does not change the image. Zoom keeps the image aspect ratio. The
   viewport clips an image that is larger than the pane.
7. Headers shows the data set as stored. The corners show the current image's
   labels, with a person name in family, given order and a date as
   year-month-day. Edge letters show the patient direction and hide
   while the picture is flipped or rotated.
8. Freeform draws a freehand outline and keeps it at three or more points.
   Rectangle and ellipse draw a region and do not fill it. Text places a note.
   Click, type, and press Enter to keep it. Escape drops the draft. Leaving
   the field keeps the note. Size and colour can be changed after the note
   exists. Length shows millimetres when the file gives Pixel Spacing,
   millimetres at the detector when it gives only Imager Pixel Spacing, and
   pixels when it gives no spacing. Click a mark to select it. While a drawing
   tool is active, drag moves that mark. Press Delete, Backspace or Remove to
   remove it. Marks last until you reload the page. They are drawings on that
   image. They are not written back into the file.

The top of the page offers Synthetic study or Folder. Synthetic study is a
generated CT and it is not a patient. More names library behaviour this image
does not draw. Use explains the JavaScript, React and Rust calls.

A file that is not a Part 10 DICOM object is skipped and counted. A colour
frame, a Modality LUT Sequence, or a VOI LUT Sequence without both a window
center and a window width stays on a fixed error sentence.

The page HTML is compiled into the binary. After a change under
`examples/folder-viewer`, build again and start that new process.

```bash
cargo build -p ocelli-folder-viewer
```

## Use the library

Rust crates do the parse, the decode and the pixel arithmetic. The TypeScript
packages are the shell a page embeds. Nothing here is published yet. Depend on
the crates by path inside this workspace.

```toml
[dependencies]
ocelli-core = { path = "crates/ocelli-core" }
ocelli-dicom = { path = "crates/ocelli-dicom" }
ocelli-codec = { path = "crates/ocelli-codec" }
ocelli-pixel = { path = "crates/ocelli-pixel" }
```

`ocelli_dicom::parse_part10` reads one Part 10 byte string. Encapsulated frame
bytes go to the decoder registry in `ocelli-codec`. Stored samples go through
`ocelli_pixel::LutChain` only. That type runs the modality stage, the VOI
stage and the presentation stage, in DICOM PS3.3 C.11 order. A second window
formula is a defect.

The `?` below belongs in a function whose error type accepts both
`ocelli_dicom::ParseError` and `ocelli_pixel::PixelError`.

```rust
use ocelli_core::{Display, Stored};
use ocelli_dicom::parse_part10;
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence,
    VoiFunction, VoiTransform,
};

let parsed = parse_part10(bytes)?;
let chain = LutChain::new(
    ModalityTransform::new(None, Some(1.0), Some(0.0))?,
    VoiTransform::new(
        None,
        &[100.0],
        &[200.0],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0,
    )?,
    PhotometricInterpretation::Monochrome2,
    PresentationLutEvidence::Absent,
)?;
let mut grey = [Display(0.0)];
chain.map_into(&[Stored(50.0)], &mut grey)?;
let _keep = parsed;
```

`wasm-bindgen` appears only in `ocelli-wasm`. Pixels stay on the worker side of
that boundary. The shell sends typed commands down and drains one event ring
per animation frame.

In a browser, use the workspace packages `@ocelli/core` and `@ocelli/react`.

`@ocelli/core` depends on dcmjs 0.52.2, which declares adm-zip 0.5.17 as a
runtime dependency. Nothing in `@ocelli/core` or in dcmjs's published files
loads adm-zip. `gate packages` checks that on every run. The check installs
the published tarballs outside the workspace, imports `@ocelli/core`, and
requires `dcmjs`, under a resolve hook that records ESM and CommonJS
resolutions on the main thread. It refuses a trace that does not record
`dcmjs/build/dcmjs.es.js` and `dcmjs/build/dcmjs.js`, a trace that records a
URL under the excepted package, and any other installed `.js`, `.mjs` or
`.cjs` file that names that package as a module specifier in any quote, with
or without a subpath, including a relative `../` or `../../` path. A load
whose specifier is built at run time in code the smoke run does not execute
on its main thread (for example a Worker or a child process, or a function
the import never calls) is outside the check. A consumer's own npm audit
also lists `@ocelli/core` and `@ocelli/react` as high, because they depend
on dcmjs. A project that needs a clean audit can add
`"overrides": { "adm-zip": "0.6.1" }` to its own root `package.json`, which
is safe because nothing calls it.

`OcelliViewport` renders a host `div`. Pass `session={null}` when you have no
wasm session. The frame pump starts only after the session's `memory` is the
built module. `examples/viewer-react/src/App.tsx` is the mount this repository
runs.

```tsx
import { viewportId } from "@ocelli/core";
import { OcelliViewport } from "@ocelli/react";
import type { OcelliSession } from "@ocelli/react";

const session: OcelliSession | null = null;

export function Viewer() {
  return (
    <OcelliViewport id="stack-1" viewport={viewportId(1)} session={session} />
  );
}
```

`DicomwebClient` in `@ocelli/core` is the QIDO-RS, WADO-RS and WADO-URI client.
Construct it with a studies service URL, a WADO-URI URL, `fetch`, wasm memory
and a response sink. A transport failure keeps a status code. It does not keep
the URL, the headers or the body. An option the client does not know is refused
with `OcelliError` code `UnknownOption`, which names it.

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

One workflow, three hosts. Claude Code reads `.claude/commands/`, Codex reads
the generated adapters in `.agents/skills/`, and OpenCode reads those as skills
and its slash commands from the generated `.opencode/commands/`. They are the
same instructions and a gate refuses them when they drift.

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
`LICENSE-MIT` and `LICENSE-APACHE`. The copyright holder is Tensorbee Limited.
`NOTICE` carries that line. Third-party notices are in `THIRD_PARTY_NOTICES`.
`THIRD_PARTY_LICENSES` carries the full licence texts of the Rust crates and
the Rust standard library compiled into the wasm module and the native
binaries, and `python3 scripts/licence_check.py --write binaries` regenerates
it after a change to `Cargo.lock` or the channel in `rust-toolchain.toml`.

Permissive on purpose: integrators embed an engine like this inside closed
products, and a disclosure obligation ends that conversation before it starts.
