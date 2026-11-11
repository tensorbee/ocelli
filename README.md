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

`bin/ocelli.sh bench --compare-cornerstone --pseudo <local-root>` measures the
local folder viewer against the pinned cornerstone3D page on the same inputs.
A real CT canvas comparison must pass before timings run. The native parser
and CPU canvas on one side and browser decoding and WebGL on the other do
different work. The record names those architectures, defaults, all processes,
inputs by digest, and cold and warm conditions. It makes no cost claim before
an equipped run. See [the benchmark instrument](docs/lld/benchmarks.md).

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
  viewer-react/     the boundary demo, kept under the dev:boundary-demo script
  example-server/   loopback helper for the boundary demo's frame-open, series-open and overlay panels
  folder-viewer/    the showcase viewer, local series and DICOMweb sources

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

The showcase is the folder viewer, `examples/folder-viewer`. It is one
ordinary HTML page with one native backend, served the same way by the npm
dev path and by cargo. From the repository root:

```bash
npm ci
npm run dev
```

`npm run dev` runs `bin/folder-viewer-dev.mjs`. The launcher builds
`@ocelli/core`, builds the native backend with cargo, selects the binary from
cargo's compiler-artifact output rather than a hardcoded target path, and
runs it. A custom `CARGO_TARGET_DIR` is honoured. The backend prints its
address on stdout, an ephemeral port unless `OCELLI_PORT` is set, and opening
that address shows the viewer. `SIGINT` or `SIGTERM` to the launcher stops
the backend it started. The same page is served by
`cargo run -p ocelli-folder-viewer`, which is the native path.

The page opens a local folder or the synthetic study, and with
`OCELLI_VIEWER_STUDIES_SERVICE_URL` and `OCELLI_VIEWER_WADO_URI_URL` set it
also offers a DICOMweb source, described under Start the local series viewer
below. The Use book the viewer serves at `/use` and the More page at `/more`
carry the user-facing detail.

### The boundary demo

The earlier React Vite demo, `examples/viewer-react`, package
`@ocelli/example-viewer`, is kept under its own script and is not the
showcase:

```bash
npm run dev:boundary-demo
```

The demo page loads the wasm module, so build it first when it is missing:

```bash
bin/ocelli.sh wasm
```

Open <http://localhost:5173>. Vite prints that address when the server is ready.
The script is `bin/example-dev.sh`, which also starts
`ocelli-example-server` on `127.0.0.1`. The helper
prints only its base URL. The Frame open
section posts to it through Vite.
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
that load succeeds. The build alone does not flip the header. The module a
published consumer gets is `@ocelli/wasm`, which `npm pack` stages from this
build with the decode module, the two third-party files and a build id
naming the HEAD the modules were built from, the tree of the staged index,
and whether the working tree had unstaged tracked changes. The packages gate
refuses a build id recorded with unstaged tracked changes, or one that does
not name the index it was staged from.

### The Content Security Policy an embedding page needs

An embedding page that loads the Ocelli worker and the wasm module needs one
header on every response, the worker script included:

<!-- ocelli:csp -->
```text
Content-Security-Policy: default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self'
```

In Chromium the worker's policy comes from its own response and not from the
page's, so a header on the HTML document alone leaves the worker
unrestricted, and a header on the worker alone without `'wasm-unsafe-eval'`
breaks the compile. Send the same header on every response. The folder viewer's image page sets `connect-src` to exactly the studies service origin configured at startup, while Use and More keep the self-only policy, and IPv6 literal hosts are refused at startup because browsers do not match them in `connect-src`.
`worker-src 'self'` is an explicit pin rather than a requirement, because
the worker is also allowed through the fallback to `script-src 'self'`: the
pin keeps a later widening of `script-src` from widening worker construction
with it.

A DICOMweb service on another origin is added to `connect-src`:

<!-- ocelli:csp -->
```text
Content-Security-Policy: default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self' https://pacs.example
```

No `blob:`, no `data:`, no `'unsafe-eval'`, and no COOP or COEP. Ocelli needs
no SharedArrayBuffer (decision D5 in the decision log) and constructs
workers from URLs only, so an `openDecodePool` factory should too. A worker
constructed from `blob:` or `data:` inherits the page policy and breaks
under it, and loosening the page for it weakens every script on the page.

The probe that measures the header against the built example, and each
directive's minimality:

```bash
bin/ocelli.sh wasm
npm run build -w @ocelli/example-viewer
node examples/viewer-react/csp-probe.mjs
```

The probe serves the built example and the built wasm package through
Playwright routes on invented origins, so it binds no port and starts no
server. The policy exists once in
`examples/viewer-react/src/csp-policy.ts`. Its test checks marked policy
blocks in this document, `docs/lld/typescript-packaging.md` and the Use page.
Each block contains exactly one expected header, inside a Markdown text fence
or an HTML `pre` element.

The local folder viewer serves each of its three pages under a separate hash
policy. It hashes the inline script, import map and style text from its embedded
pages at first use, so rebuilding after a page edit also updates the hashes.
It needs no `'wasm-unsafe-eval'`, because decoding runs in the local process.
Run its release-browser probe with `node examples/folder-viewer/csp-probe.mjs`
after building the library with `npm run build`.

The page mounts one stack viewport. It is the showcase, so a library behaviour
is finished only when this page has a control for it. To open a folder of
DICOM files on this machine, use the local series page below.

## Start the local series viewer

`examples/folder-viewer` lists series from a folder you choose and shows them
in up to nine viewports, from 1 by 1 to 3 by 3. Each viewport has its own
series, image, zoom, pan and window, and Link can make the others follow the
outlined one's scroll, window and zoom. The preview registers all six decoder
families of `ocelli-codec`. It opens the monochrome frames of eighteen of the
nineteen known transfer syntaxes: the seventeen with a registered decoder and
Deflated Explicit VR Little Endian, whose whole data set `parse_part10`
inflates before the raw decoder reads it. JPEG XL JPEG Recompression is the
only one refused for every frame. Supported colour frames draw through the
library colour stage. A JPEG-LS frame whose LSE preset asks for a MAXVAL,
RESET or threshold the decoder does not apply is refused rather than drawn,
and so is a JPEG-LS scan that runs out of data or carries padding beyond its
final byte after its last sample. A damaged scan that reads as a valid scan
of another image cannot be detected by any decoder. Compressed files whose Basic Offset Table is empty open as PS3.5
A.4 requires, single-frame and multi-frame, including multi-frame JPEG,
JPEG-LS and JPEG 2000 files whose frames span several Fragments, which are
split at their codestream start and end markers. It listens on this
machine only.

```bash
cargo run --release -p ocelli-folder-viewer
```

With `OCELLI_VIEWER_STUDIES_SERVICE_URL` and `OCELLI_VIEWER_WADO_URI_URL` set to absolute http or https URLs, the viewer shows a DICOMweb source that searches studies, series and instances and fetches one selected instance the native backend parses and validates. Remote images are not displayed yet, and the local study, its marks, documents and checks stay as they are. With neither value set the viewer runs local only. Setting only one value, or an invalid value, stops startup with a fixed message that does not echo the value.

The debug build is about ten times slower on a window step. Each file is posted once while the process holds it, and posted again after eviction or a restart. `OCELLI_CACHE_MIB` sets that cache in mebibytes and the default is 256. A multiframe in which any frame's window comes from a per-frame functional group also keeps every frame's inspect reply, so the page can ask for a frame's window list by file id after the posted bytes are dropped. The parts no frame changes are kept once. No other file keeps one. This pool is outside `OCELLI_CACHE_MIB`, is charged at the bytes it holds, stays under a fixed 16 MiB across every file, and `GET /held` reports it as `kept`. Replies that would pass that cap after the files holding nothing are dropped are not kept, and that file's `/inspect` by id answers 410 once its bytes are gone. A forget drops a file's replies, and so does a file the cache has evicted, when it is next asked for. The folder viewer keeps at most four request bodies at once, which is the coefficient of `S_max` in the resident set bound, and decodes on two threads, which is `P` in that bound. Every preview decodes on those threads, one that carries the file's bytes included, and a decode permit ends with its decode, so a client that stops reading holds no permit. A request body is freed on the thread that allocated it. A decoded frame's sample buffer that leaves the cache is parked on a free list keyed by its exact byte length when cached bytes plus free bytes still fit the budget, and is freed otherwise. That list is inside the same 256 MiB budget, and the cache evicts to make room for a frame or for a free buffer. A decode takes a parked buffer of its length when there is one. When no free buffer has the length and a held file's frame does not fit beside the cached and free bytes, its decode takes the samples of the least recently used cached frame of that length only when no reply is still reading that frame, and allocates otherwise. A preview that carries the file's bytes never evicts and never takes a cached frame's samples, because its frame is not stored. At most two automatic series windows are computed at once, each holding up to 4 MiB outside `OCELLI_CACHE_MIB` while it runs, which is the bound's term `P * 4 MiB`, and a third waits for one of them to finish. Every grey byte the folder viewer holds counts against one fixed cap of 17,400,000 bytes, which is the bound's grey term `P * N_max`, with `N_max` the 3000 by 2900 frame the bound is stated for. A reply's grey counts from the moment its render is admitted until it is written. Each decode thread keeps at most one written grey to render into next, and that grey counts for as long as it is kept. A decode thread frees the grey it kept while any preview waits for room, and before it renders a grey less than half that size. A frame of more than 17,400,000 pixels, whose grey alone is over the cap, draws alone. Its grey is admitted only when no other grey is held, and while it is held no other grey is admitted, so grey bytes stay within the larger of the cap and that one frame. Its decode thread frees that grey after the write and does not keep it. Previews that wait for room are served in the order they first waited. A waiting preview is admitted only when every preview that waited before it has been admitted or has stopped waiting, and only when its grey fits. A preview that fits is admitted at once only when no preview waits, so it cannot pass one that waited first. No frame is refused for its size below the preview library's limit of 67,108,864 pixels. The resident bound is stated for frames within the design size. A frame over the design size can raise resident memory above that bound. Measured on the release binary with a synthetic 4740 by 5928 frame, the preview answered 200 with 28,098,720 grey bytes in 49.8 ms decoded and 25.5 ms cached. A smaller preview waited while that grey was held and was served 3.8 ms after it was released. Two such previews read together peaked at 28,098,720 grey bytes, one frame. With two clients previewing two such frames in a loop and reading every reply, twenty 512 by 512 previews sent among them all answered 200, the slowest in 42.0 ms. A preview whose grey does not fit waits for room for up to the 60 second write timeout, and answers 503 busy if none comes. A whole reply must be written within that timeout. `GET /held` reports `grey`, every grey byte under the cap, and `grey_replies`, the part held by replies not yet written. A page that abandons a request closes its connection, which releases its grey at once, within a millisecond when measured. A client that keeps a connection open without reading holds its grey until the write timeout, measured at 60.0 seconds. When such clients fill the cap, other previews wait up to the write timeout and then answer 503 busy. Measured with twenty such clients, five of six later previews answered 503 busy at 60.0 seconds and one was served at 58.95 seconds, when a stalled write timed out. While body-form previews wait, a file post can wait as long, because each of up to four waiting previews keeps its request body slot. Measured with four waiting, a file post waited 58.99 seconds. The folder viewer binary uses mimalloc as its global allocator. The pin is exact in the workspace manifest. No other crate does.

Opening another folder or the synthetic study tells the process to forget the folder the page left. The page sends that folder's file ids to `POST /forget`, and the process drops each posted file and every decoded frame of it, so `GET /held` then counts only the folder now open. A reply for the folder left that arrives late does not post its file again. A decode already running for a forgotten file stores nothing. Its request is still answered, with 410 Gone when its render had to wait for grey room, or 503 if that wait runs out. Each page load sends a random token with every file it posts, and leaving the page sends one short beacon naming that token, so the process forgets every file that page posted and no other page's. A list of ids would not do, because Chromium allows only 64 KiB of beacons in flight at once. A forget that fails is not retried, and the cache budget, which `OCELLI_CACHE_MIB` sets, and the 16 MiB reply pool still bound what the process holds.

Choosing the same folder again reads its files again, and every grey the page keeps is keyed by the file object that folder choice gave it, never by the file's name, size or time. The series list's thumbnails are keyed by series and drawn again on every open. A file rewritten on disk after the folder was opened is found when the window regains focus, when the page becomes visible again, when a series is selected or when another viewport becomes active. The page then reads one byte of each file of the active series, one at a time, and a file the browser refuses to read is forgotten by the process and shown as "This file changed or was removed on disk after the folder was opened. Open the folder again to see it." In Four images the sentence is in that file's tile and the other tiles still draw. The status line counts such files in the active series. A modification time that moved counts as a change even when the bytes are the same. Three limits hold. A rewrite that keeps the file's modification time is not detected, and the page keeps showing the bytes it already holds until the folder is opened again. A change is noticed at the next focus, return to the page, series selection or change of active viewport, not as it happens. Only Chromium was measured, and other browsers may behave differently.

The process prints one line, the page URL, and nothing else. It has the form
`http://127.0.0.1:12345/`. The port is chosen at startup unless
`OCELLI_PORT` names a fixed one. `0` also means chosen at startup. A fixed
port keeps one bookmarkable address, and a second copy then fails to bind
instead of starting beside the first. A value that is not a port number,
including an empty one, stops the process with an error. Open that exact URL in
Chrome or Edge. The Folder control is a directory picker.

```bash
OCELLI_PORT=8742 cargo run --release -p ocelli-folder-viewer
```

1. Choose Folder and pick a directory. The browser reads the files. Leave them
   where they are. Do not copy them into this repository.
2. The left column has one row per Series Instance UID. A row shows the series
   number, the modality, the description and the image count. A file with no
   Series Instance UID keeps its own row.
3. Select a row. The outlined viewport shows the first image and keeps it.
   The arrow keys move one image in the outlined viewport. A wheel without
   Ctrl moves one image in the viewport under the pointer after about 40
   pixels of travel, and makes that viewport the outlined one, as a press in
   it does. The scrubber reads `n / total` for the outlined viewport. Coming
   back to a series restores the image you left. With one viewport there is
   no outline.
4. Drag a series row onto a viewport to open it there, or click or
   double-click a row to open it in the outlined viewport. A click on the
   series already open there starts its window and fit again, and a drop of
   that series keeps its view. Opening a folder
   puts the series with the most images in the first viewport and the next
   series in list order into each other viewport. A viewport with no series
   reads "Drag a series here." Link opens three switches, Scroll, Window and
   Zoom, which are off at load and act on every visible viewport as one set,
   with the outlined viewport as the source. Scroll moves each other viewport
   to its image nearest the source image's position, when both series share a
   Frame of Reference UID and the cosine of the angle between their planes
   is above 0.9, which is under about 26 degrees. Window copies the source's
   window when a W/L drag ends, on a window row, on a region window and on
   Reset. A preset is not sent to a viewport whose image the local process
   does not offer it on, which is any image that is not CT. A file window is
   sent by its place in the list, and a viewport whose image lists fewer
   windows shows its first usable one. A window is copied only between images
   of the same kind, colour to colour or monochrome to monochrome. Reset is copied as Reset, so a monochrome viewport whose image stores no
   window returns to its own series' automatic window. Zoom
   copies the source's zoom over each viewport's own fit, and pan is not
   copied. A click on a series row is a new load and copies nothing.
   Reference lines, on at load, draw a yellow line in each other viewport
   where the source image's plane cuts it, when both share a Frame of
   Reference UID and the planes are not parallel. The local process chooses
   the image and computes the line, through `ocelli_viewport::link`.
5. Space plays and pauses. Playback loops. Between two frames of one
   multi-frame file, Play waits that file's Frame Time `(0018,1063)` in
   milliseconds, PS3.3 C.7.6.5, and the line beside the FPS control gives it,
   as in `Frame Time 40 ms, 25 per second, from the file`. The FPS control is
   disabled then, and Use the slider hands the rate back without stopping
   play. A step to another file, and any file without a usable Frame Time,
   uses the FPS control, and the line says why: a Frame Time Vector, which
   this page does not play, a frame rate with no Frame Time, no Frame Time,
   cine timing that is not usable, which is a Frame Time, Frame Time Vector,
   Recommended Display Frame Rate or Cine Rate the local process refuses, or a file
   whose frames this page could not read. A step that takes longer than the
   interval is not skipped, so play can run slower than the file asks, and the
   line says so. The FPS control runs from 1 to 30, starts at 10 and is kept
   in this browser.
6. The toolbar is a row of icons. Point at one to see its name, its key and
   what it does. The groups are the navigation tools (Scroll, Pan, Zoom,
   W/L, Region window `Y`), the drawing tools (Length, Angle `Q`, Cobb `J`,
   Bidirectional `B`, Rectangle, Ellipse, Freeform, Arrow `U`, Probe `X`,
   Text), the view (Invert, Rotate 90, Flip horizontal, Flip vertical), the
   display (Pointer cross, Magnifier, Corners, Orientation letters), the
   layout (Layout `G`, which opens a 3 by 3 picker, Four images `Shift G`,
   which shows four neighbouring images in the outlined viewport, Link and
   Reference lines, which have no key), and Key image, Headers, Fit, Reset
   and Full screen. Key image flags the image, and Save marks writes the
   flags as a Key Object Selection. The view, window, Fit, Reset, Key image
   and Headers act on the outlined viewport. The mouse profile picks the
   tool that starts active, Scroll or W/L, and choosing a profile also
   switches the current tool to that one, from a drawing tool too. The right
   column holds the window list, the marks and the mouse and key lists. The
   icons at either end of the top bar, or the `[` and `]` keys, hide and show
   the two side columns. On a CT the window list starts with the three
   presets, Soft tissue, Lung and Bone, and then lists the file's own windows
   in file order, each named by its Window Center & Width Explanation, or
   `Window n` when it has none. On any other image it lists the file's windows
   only. A monochrome image that stores no window lists one row, `Auto window`,
   after the presets on a CT, and it chooses the series' automatic window. The keys `1` to `9` choose rows 1 to 9, and `0` stays Fit. Each VOI
   LUT Sequence item is listed as a row that cannot be chosen, because this
   preview does not apply one, and while a file has one its other rows cannot
   be chosen either. A multiframe whose windows change per frame lists the
   shown frame's own, asked for by file id, which the process answers from the
   replies it keeps for such a file after the posted bytes are dropped. Until
   that list arrives the frame is drawn at its own first usable window, then at
   the chosen one. A file is read as changing per frame when any frame's window
   comes from its own per-frame group, not only frame 0's. Each tile of Four
   images follows the same rule. While its frame's list is on its way its label
   gives the WW and WL it is drawn at, and once the list arrives it is drawn at
   the chosen window and its label names it. A tile of a file that stores no
   window gives its numbers and says `Auto window`, as the corner does. A file
   opens at its first window that has both a centre and a width, and a slot
   missing either is listed as a row that cannot be chosen. A slice keeps its
   window, and a file window past the next image's list shows that image's
   first usable one. A series change and Reset return to the first usable file
   window, or to the automatic window when the monochrome file stores none. The bottom right corner names the window shown: the preset, the file
   window's name, `Auto window`, `Auto window, this image`, `region` or `manual`. A window row or a region window that
   the local process refuses leaves the image and the window as they were and
   puts the reason in the sentence line. Region window is a drag of a
   rectangle. The window then runs from the least to the greatest modality
   value whose pixel centre is inside it, with padding left out, and keeps the
   function already shown. The local process finds the two values through `POST
   /region`, and `windowRegion` from `@ocelli/core` makes the window. The
   process serves the built `packages/core/dist` under `/lib/`, so run `npm run
   build -w @ocelli/core` before starting the viewer, or the region window
   reads unavailable and says so. The library's SR modules import dcmjs, which
   `npm ci` installs, and the region window names that when it is the part that
   is missing. Its centre is the middle of that span. cornerstone3D 5.8.2's
   `WindowLevelRegionTool` centres on the mean and keeps a width of at least
   10, so the two can give different windows.
7. The left button uses the active tool in every profile. The Profile
   control in the MOUSE section of the right column chooses what the other
   buttons do. Ocelli, the default, starts on Scroll, with the middle button
   zooming from the place you pressed and the right button panning. Scroll on
   left starts on Scroll, with the middle button dragging the window and the
   right button zooming. Window on left starts on W/L, with the middle button
   panning and the right button zooming. Control with the left button pans in
   all three. The browser keeps the profile for this address, so a kept
   profile returns after a restart only on the same `OCELLI_PORT`. A pinch, or
   Ctrl with the wheel, zooms at the pointer. A sideways
   swipe does not change the image. Zoom keeps the image aspect ratio. The
   viewport clips an image that is larger than the pane.
8. Headers shows the data set as stored. A count on the Headers button is the
   number of findings in the shown file's conformance report, whether the file
   is drawn or shows a fault such as a colour or refused frame, and the sheet
   lists them in Checks above the table, each with its tag, rule and the DICOM
   section it cites, under the sentence that the report checks only the rules
   it lists and is not a statement that the file conforms to DICOM. It checks
   values against PS3.5 Table 6.2-1, value multiplicity for the attributes it
   lists, and Type 1, 1C, 2 and 2C presence for seven PS3.3 modules on CR, CT,
   MR and PET Image Storage. A finding names a length, a count or a character
   position and never a value. A finding never stops an image from showing.
   The page asks for a file's report 300 ms after that file is first drawn or
   shows its fault, so scrolling asks for nothing, and keeps it until the next
   open. Opening the sheet asks for its file at once. The
   checks need `npm run build` first, and without it Checks says so and the
   count stays hidden. The corners show the current image's
   labels, with a person name in family, given order and a date as
   year-month-day. They also show the birth date, sex, age, study time,
   institution, slice location and thickness, kVp, mAs, TR and TE when the
   file has them. With no Patient's Age, the age is worked out from the birth
   date at the study date.
   Edge letters show the patient direction from Image Orientation (Patient),
   or from Patient Orientation on an image without it, such as a projection
   radiograph, and hide while the picture is flipped or rotated. An image
   whose Anatomical Orientation Type is `QUADRUPED` shows no letters. A scale
   bar shows above the window line when the file gives a spacing a length can
   use, from that same spacing, and says `at the detector` when that spacing
   is at the detector: Imager Pixel Spacing with no magnification factor,
   alone or with a Pixel Spacing equal to it. It hides with the corners and in Four images. The pointer
   cross shows the column, the row and the modality value of the pixel under
   it with its unit: HU on a CT whose Rescale Type is HU, or is absent on a CT that is
   not multi-energy, `unspecified` for Rescale Type `US`, the Units
   attribute on a PET, `padding` for a padded sample, and `not held`
   for a file the process could not hold. The cross, its readout and the
   magnifier hide when the picture moves under a still pointer, after a
   zoom, a fit, a turn or flip, Four images, a layout change, a side
   panel, full screen or a window resize, and come back with the next
   pointer move. In Four images only the magnifier comes back, because the
   cross is drawn in the single view only. During a pan or zoom drag they
   stay with the pointer. When the image under a still pointer changes, by
   a key, the wheel or Play, the magnifier shows the new image, or hides.
9. Every number on a mark comes from the local process, and the page prints
   its text. Probe asks `POST /value`, and every other mark asks `POST
   /measure`. Length, Angle, Cobb and Bidirectional are in millimetres from
   Pixel Spacing, or from Imager Pixel Spacing divided by the Estimated
   Radiographic Magnification Factor, in millimetres at the detector from
   Imager Pixel Spacing alone, and in pixels when the file gives no spacing.
   PS3.3 10.7.1.1 says a Pixel Spacing that has not been corrected for
   magnification has the same values as the Imager Pixel Spacing, so a Pixel
   Spacing equal to it takes the imager path: it is divided by the factor when
   the file has one and is millimetres at the detector when it has none. On
   that path, and on Imager Pixel Spacing alone, a factor that is present but
   cannot be used is not measured. Beside a Pixel Spacing that differs from
   the Imager Pixel Spacing, the factor is not read and the Pixel Spacing
   decides. An angle without spacing says square pixels are assumed. A Pixel
   Spacing that is present but not usable is not measured. Angle is a drag
   along the first arm to the vertex and a click at the end of the second arm.
   Cobb is two drags, one per line, and reports the acute angle between them.
   Bidirectional is a drag along the long axis, with a short axis placed
   across its middle at half its length, and reports both lengths as drawn.
   Rectangle, Ellipse and Freeform report the area of the drawn shape and the
   mean, SD, minimum and maximum of the modality values whose pixel centres
   are inside it. Padding pixels are left out. A freeform that crosses or
   touches itself is not measured, and neither is one of more than 4000
   points. Freeform keeps an outline of three or more points. Probe is a click
   and reports the modality value of that pixel. Arrow is a drag from the tail
   to the head with an optional note. Text places a note. Click, type, and
   press Enter to keep it. Escape drops the draft. Leaving the field keeps the
   note. Size and colour can be changed after the note exists. The marks list
   shows each mark with its numbers, `measuring` while the reply is on its
   way, or the reason it was not measured. Click a mark to select it. A
   selected mark shows a handle on each of its points. Drag a handle to move
   that point, and the mark is measured again on release. While a drawing tool
   is active, drag moves the whole mark. Press Delete, Backspace or Remove to
   remove it. A moved mark stays inside its image. Save marks downloads one
   DICOM SR TID 1500 file per study for the Freeform, Rectangle, Ellipse,
   Length and Text marks, and one Key Object Selection file per study for the
   key images. Nothing is written into the opened folder. Put the downloaded
   files in the folder and open it again to see the marks on their images, or
   read them into the open study with Load marks. A length is saved with its
   number in mm when its spacing is a patient spacing, and without one at the
   detector or in pixels. Angle, Cobb, Bidirectional, Arrow and Probe are not
   saved, and the status line counts them. Colour and text size are not
   saved. One study's SR holds fewer than 4000 content items, about 700
   marks, and a larger save writes nothing and says so. Deleting every mark of
   a study is saved too, as an SR with no marks. A folder's marks are saved
   from that folder. Opening another folder clears the marks and every
   remembered document, replacement and identifier. Load marks adds documents
   to the open folder. A save replaces reports placed whole in that folder,
   including those read through Load marks, and reports skipped because a
   later document replaces them. A Save writes nothing for a study, and says
   so, when no mark or key image changed since the folder was opened or its
   last save, unless an earlier read into this folder hid a report. An
   SR is placed only in part when one of its marks is not placed or a group of
   it cannot be read, most often because the folder lacks an image a mark is
   on. Such an SR is not replaced by a save from this page, so its marks come
   back even after they were deleted, and the status line says so. A later SR
   that names it as replaced does replace it. Unflagging the last key image of
   a study is not saved, because a Key Object Selection must name at least one
   image, so the earlier file still flags its images, and the status line says
   so. Save marks and Load marks need `npm run build` first.

The top of the page offers Synthetic study or Folder. Synthetic study is a
generated CT and it is not a patient. More names library behaviour this image
does not draw. Use explains the JavaScript, React and Rust calls.

A file that is not a Part 10 DICOM object is skipped and counted. An
Enhanced SR, Comprehensive SR, Comprehensive 3D SR or Key Object Selection
file is read as a document and is not listed as a series. Any other SR, such
as an X-Ray Radiation Dose SR, still shows as a series. Each mark in a
document goes on the image whose SOP Instance UID and frame it
names, and the status line counts what was not placed. Supported colour
frames draw in the image, thumbnails and tiles. Their window starts at width
256 and centre 128. Window drag and invert act on each channel. Colour has
no presets or file windows. The pointer readout shows R, G and B before the
window, and a region reports area and separate R, G and B statistics. A
matrix or tone-curve ICC profile is applied before the colour window. Other
profile kinds keep device RGB and carry a sentence naming the profile not
applied. A colour shape this build cannot draw names its reason. A Modality
LUT Sequence, or a VOI LUT Sequence without both a window center and a
window width, stays on a fixed error sentence on monochrome frames. The local process refuses a window preset on an image whose
Modality is not CT, whatever the page sends. A series of more than 65536 images is not linked: it does
not scroll with the others and has no reference line.

A series whose images store no window is drawn at one automatic window for
the whole series, from the 1st to the 99th percentile of modality values over
up to 16 images spread through it, each through its own rescale, with stored
zero and the Pixel Padding Value left out, and the corner says `Auto window`.
Opening such a series decodes those images before its first image is drawn,
so scrolling keeps the same window and the same relative brightness. A window
in the file is always used as it is. A thumbnail, and a series whose window
the local process refuses, use each image's own automatic window, and the
corner then says `Auto window, this image`. The percentiles are nearest rank,
the function is LINEAR_EXACT, and `ocelli_pixel::AutoWindowPool` computes
them. cornerstone3D 5.8.2 takes each image's own minimum and maximum instead,
so this is not a parity row.

The page HTML is compiled into the binary. After a change under
`examples/folder-viewer`, build again and start that new process.

```bash
cargo build --release -p ocelli-folder-viewer
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

The wasm session keeps committed frames in a cache whose budget the caller
sets, and the default is zero, which is off. `setFrameBudget(wasm, bytes)`
moves it. A commit that carries a `content` id with its `stackIndex` and
`stackEpoch` fills the cache, and a commit that carries a `content` id
without both is refused with `InvalidValue` and commits nothing.
`frameResident` and `showResident` read it back without the bytes crossing
the boundary again. Under a wrong epoch `showResident` returns false at any
index. Under the current epoch it answers only for the slice the viewport has
selected, another index throws the coded `CommitRefused`, and at the selected
index a miss or a stale entry returns false.
`releaseSeries` and `releaseStudy` release the named series, one
`cache.released` event per series that held frames, a series with nothing
cached emits no event, and a release that crosses a pressure band pushes
`cache.pressure` after `cache.released`. The displayed frame is outside the
cache and stays on screen through a release.

In a browser, use the workspace packages `@ocelli/core`, `@ocelli/wasm` and
`@ocelli/react`.

`@ocelli/wasm` is the module `loadCore` initialises, built by `wasm-pack`
from `crates/ocelli-wasm`. `@ocelli/core` peer-depends on it at the same
version, so install both at once. Load the module in a worker and pass the
initialiser to `loadCore`:

```js
import init, * as wasm from "@ocelli/wasm";
import { loadCore } from "@ocelli/core";

await loadCore(async () => {
  await init();     // fetches the .wasm beside the glue in a browser
  wasm.install_panic_hook();
});
```

`@ocelli/wasm/decode` is the module built without the renderer. The
`@ocelli/wasm` tarball carries a build id that records the git SHA of the
HEAD the modules were built from, the tree of the staged index, whether the
working tree had unstaged tracked changes when they were built, and the
digest of each module. The packages gate refuses a build id recorded with
unstaged tracked changes, or one that does not name the index it was staged
from.

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
and a response sink. Every request is sent with the fetch cache mode from the
optional `cache` option. The default is `"no-cache"`, which revalidates a
stored response with the server before using it, so a re-processed object is
never served from the browser's HTTP cache without asking. `"no-store"`,
`"reload"` and `"default"` are accepted. `"force-cache"` and
`"only-if-cached"` can serve a stored response that is stale, and are refused
at construction with `DicomwebError` code `InvalidRequest`. A proxy or a
service worker between the browser and the server is outside what a cache
mode controls. A transport failure keeps a status code. It does not keep
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
The `@ocelli/wasm` tarball carries both third-party files beside the module,
so a consumer of the published package receives them without this repository.
The native build that ships is Metal-free, deviation D-33, so the objc2
crates generated from Apple SDK headers are not compiled into it and
`bin/ocelli.sh gate native` refuses any that reach the shipped graph.

Permissive on purpose: integrators embed an engine like this inside closed
products, and a disclosure obligation ends that conversation before it starts.
