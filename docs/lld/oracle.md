# The oracle, reference half

**F-IDs that contributed:** F-010
**Last updated:** 2026-09-05

HLD section 11 names cornerstone3D as the reference the differential harness
measures against, and decision D7 makes the oracle the thing that exists before
port code. This file is the design behind `tools/oracle`. Read it beside
`docs/lld/corpus.md`, because the corpus is what this renders and the two are
one instrument.

**This half compares nothing.** It produces, for every applicable corpus row, a
reference frame plus the metadata that explains it. Comparing Ocelli's own
output against those frames is F-011.

## The defect it exists to prevent

From `docs/sprints/CURRENT_SPRINT.md`:

> A headless page can start, load a test runner and exit successfully without
> decoding every corpus row, presenting a frame or reading back the rendered
> pixels.

Every design choice below is shaped by that sentence. In particular a blank
canvas reads back perfectly and hashes stably, so "the run produced 91 digests"
is not evidence of anything on its own.

## Layout

| Path | What it is |
|------|-----------|
| `tools/oracle/package.json` | the pins, with the reasoning per group in `$pins` |
| `tools/oracle/run.mjs` | the driver, and the whole gate |
| `tools/oracle/build-page.mjs` | esbuild, into `page/dist` |
| `tools/oracle/page/app.mjs` | the render page, `window.__oracle` |
| `tools/oracle/src/` | paths, manifest, params, voi, sidecar, output, pins, server, unsupported, faults |
| `tools/oracle/tests/` | ten `node:test` suites, and the fault runner |
| `tools/oracle/check_sidecars.py` | the pydicom cross-read of the sidecars |
| `tools/oracle/render-params.json` | committed, and it decides every frame |
| `tools/oracle/unsupported.json` | committed, what 5.8.2 cannot render |
| `tools/oracle/out/` | ignored, and refused by the pre-commit hook |

`tools/oracle/Cargo.toml` and `src/lib.rs` are the Rust side that will run
Ocelli under F-011 onwards. F-010 changed nothing there but the crate's doc
comment, which now names 5.8.2 and points here.

## The pins, and deviation D-11

`@cornerstonejs/core`, `@cornerstonejs/tools`, `@cornerstonejs/dicom-image-loader`,
`@cornerstonejs/metadata` and `@cornerstonejs/utils` are pinned at exactly
`5.8.2`, and `playwright` at exactly `1.62.1`. No caret and no tilde, for HLD
15.2's stated reason applied to the reference rather than to wgpu: **an oracle
that drifts is not an oracle.**

The pin is 5.8.2 and not Appendix B's v5.8.9, which does not exist. That is
deviation **D-11** and the reasoning is there, not repeated here.

`metadata` and `utils` are peer dependencies of `core`, so leaving them to
resolve themselves would leave part of the reference unpinned. The page's own
import list is three names: `@cornerstonejs/core`,
`@cornerstonejs/dicom-image-loader` and `dicom-parser`. The other two arrive
behind them, `metadata` as a peer of both `core` and `dicom-image-loader` and
`utils` as a direct dependency of `core`, so all five reach the bundle whether
or not the page names them.

**`tools` is pinned and is never loaded.** Nothing in the harness imports it and
nothing peer-depends on it. It is pinned because D-11 pins the three
cornerstone3D packages together and because the parity surface is measured
against it, so `bin/ocelli.sh oracle` refuses a tree where any of the three has
moved. `@kitware/vtk.js`, `gl-matrix`, `d3-array` and `d3-interpolate` are
declared for the same reason: they are `tools`' peers.

Three more pins exist for reasons of their own.

- **`dicom-parser`**, at `1.8.21`. The page imports it directly, to read the
  sidecar's `attributes` block straight from the bytes independently of
  anything cornerstone3D resolved, and HLD section 11 makes that block
  load-bearing. It is also a dependency of `dicom-image-loader`, so it is
  inside the renderer as well as beside it, and one version is pinned for both
  uses. A drifting metadata reader is the same problem as a drifting
  renderer.
- **The four `@cornerstonejs/codec-*` packages.** Nothing imports them by name
  at run time, `build-page.mjs` resolves them to copy their `.wasm` binaries
  into the page, and those binaries decide the decoded pixels for every JPEG,
  JPEG-LS, JPEG 2000 and HTJ2K row. Leaving them to npm's hoisting would make
  the decoders a phantom dependency, which is the one kind hoisting can stop
  providing without warning.
- **`esbuild`.** Not part of the reference either. It is what turns the
  reference into a page a browser can load, and a bundler that changed the
  module graph between two runs would move the output for a reason nobody
  could see.

`@kitware/vtk.js` and `gl-matrix` are pinned twice over: they are `tools`'
peers and they are also direct dependencies of `core`. vtk.js is what
cornerstone3D v5 actually renders through, so of everything here it is the
package whose drift would move the most reference pixels.

`checkPins` in `run.mjs` enforces all of this at run time, over all sixteen
declared dependencies with no exceptions, and it enforces two lists against
each other as well: what the driver requires and what `package.json` declares.
A package in one and not the other is refused, so a pin cannot be recorded in a
place nothing reads.

Six of the sixteen expose no `./package.json` in their exports map, so their
version is found by resolving the package's main entry and walking up to the
`package.json` whose `name` matches. That entry sits in `dist/` or `src/`, so
the walk starts one level below the root and finds the manifest on its second
step. Measured today, each of those six has exactly one manifest, and it is
that root one.

**The name predicate is a guard rather than a workaround, and it is worth
keeping for a reason that is not yet load-bearing.** Nested `package.json`
files carrying nothing but `{"type": "module"}` do exist in this tree, in five
of the `@cornerstonejs/*` packages, and all five of those take the direct route
and never enter the walk. A walk that took the first manifest it met would read
`undefined` from one of them, and record it as an installed version, the day an
exports map changes and moves a package from one route to the other.
`tests/pins_test.mjs` builds that shape explicitly, so the predicate is
exercised before the tree needs it.

Absent is refused by `bin/ocelli.sh`, present at the wrong version is refused
by the driver, and neither is a warning.

## How the reference is driven

Playwright drives headless Chromium. The page is **static**, built once per run
into `page/dist` and served from loopback on an ephemeral port. Three things
have to land there and each has a reason:

1. `app.js`, the bundle.
2. `decodeImageFrameWorker.js`, under exactly that name, beside `app.js`.
   `@cornerstonejs/dicom-image-loader`'s `init` starts its decode worker with
   `new Worker(new URL('./decodeImageFrameWorker.js', import.meta.url))`.
3. The four codec `.wasm` binaries under `wasm/`, named the way the decoders
   ask `locateFile` for them. The page passes `wasmBasePath` to `init` so they
   are found by name rather than through the bare-specifier `new URL` the
   sources fall back to, which no bundler resolves.

An HTTP origin rather than `file://`, because a module worker from `file://` is
refused by the browser's origin rules.

**The corpus is not reachable through that server.** Bytes reach the page as an
argument to `page.evaluate`, base64 encoded, so no server in this harness can
read `corpus/data`.

Two Emscripten details are worth knowing before touching `build-page.mjs`. The
codec glue files are built for node as well as the browser and name node
builtins on a branch the browser never enters, so those specifiers resolve to
an empty module. `events` is the exception: `xmlbuilder2`, which vtk.js pulls
in, does `class ... extends EventEmitter` at module scope, and extending
`undefined` throws while the bundle is still evaluating. That one gets a small
real EventEmitter.

## SwiftShader, and why the reference does not move

Chromium is launched with `--use-gl=angle --use-angle=swiftshader
--enable-unsafe-swiftshader`, so rasterisation is software. The adapter string
is read back from the page through `WEBGL_debug_renderer_info` and recorded in
`run.json`, and the run **refuses to proceed** unless three things cornerstone3D
itself reports are true, plus one the browser reports. From cornerstone3D's own
`getRenderingCapabilities` and `getShouldUseCPURendering`: a software
rasteriser, a WebGL2 context, and its GPU rendering path rather than its CPU
fallback. From the browser: `window.devicePixelRatio` is exactly one, which is
what makes the viewport canvas the size `render-params.json` declares.
cornerstone3D has no opinion about that fourth one, and the refusal says so.
The choice is therefore checked rather than implied by a flag nobody reads.

All four refusals have been observed red: the adapter one by dropping
`--use-angle=swiftshader`, the WebGL2 one by adding `--disable-webgl2`, the
device pixel ratio one by launching at scale factor two, and the CPU path one
by forcing `useCPURendering`.

Decision D14 says to claim measured divergence and never bit-exact
reproducibility, and the counter-argument to freezing the reference is real:
the divergence a user sees is on their hardware. The answer is that the
reference is held still HERE and the measured divergence F-011 publishes is
Ocelli's own render on real hardware against this stable reference.

The recorded adapter on the machine this was built on:

```
ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (LLVM 10.0.0) (0x0000C0DE)), SwiftShader driver)
```

`run.json` also records `norm16`, `float` and `halfFloat` texture support,
because those decide how a 16-bit frame is carried to the GPU and a machine
that answered differently would render differently for a reason worth seeing.

## The render parameters are declared, not defaulted

A frame is a function of the window, the camera, the canvas size and the
interpolation, and the DICOM file carries only the first of those. So
`render-params.json` is committed and it decides every frame:

- canvas 512 by 512, fixed and square, so no row's aspect ratio decides the
  output size,
- background black,
- interpolation `NEAREST`, so no resampling smears a difference,
- camera `reset`, which is cornerstone3D's own fit. It is the only mode the
  page implements and the page refuses any other value rather than ignoring it,
  so the declared mode acts. The resulting camera is recorded per row rather
  than assumed,
- VOI from the file where the file has one, and from the declared per-modality
  default where it does not.

Resolution is `base`, then every rule in `rules` whose `match` holds, in file
order, later winning. A rule **replaces** a key wholesale rather than merging
into it, so a rule that sets `voi.source` to `none` cannot carry along the
base's explanation of why the file's own window is used. The rules that fired,
and the reason each exists, are recorded per row in the sidecar.

A rule matching on a key nobody reads, or setting one, is **refused** rather
than quietly matching nothing, and a unit test fails a rule that matches no row
in the committed manifest. `canvas` and `background` are deliberately not
settable by a rule at all: one viewport serves every row and is sized and
coloured once, so a per-row value would be published in the sidecar as a
parameter that produced the frame, having produced nothing.

**Eighty-seven of the eighty-nine rendered rows are smaller than the canvas
and are magnified into it. Two are not.** `real/dx_varepop/00000001.dcm` is
879 by 1168 and `real/us_cmb_crc/00000001.dcm` is 590 by 819, both larger than
512 in both dimensions, so both are fitted DOWN under `NEAREST`, which discards
source pixels. A per-modality tolerance written against a magnified frame does
not automatically hold for a decimated one, so `run.json` lists them under
`downsampled` rather than leaving F-011 to work it out from the sidecar.

**The scale it lists is per axis and follows the physical fit.**
`resetCamera` fits the image by its extent in millimetres, not by its pixel
count, so a frame with non-square pixels is scaled differently in each
direction and neither factor is `canvasWidth / columns`. `canvasScale` in
`src/params.mjs` derives both from the camera's own `parallelScale` and the
row and column spacing. `syntax/reference_mono12.dcm` is the worked case: 64 by
96 at spacing [0.5, 0.25] with `parallelScale` 16 gives 8 canvas pixels per
source pixel vertically and 4 horizontally, so the image fills the height and
384 of the 512 columns, and the frame's recorded `blackFraction` is 0.25
exactly. A pixel-count model would have answered 5.333 for both axes and
matched neither.

**A change to that file changes every reference frame.** Treat it the way HLD
25.1 treats a tolerance: a pull request with a rationale, reviewed like code.

### The VOI rule, and the one number in it

`tools/oracle/src/voi.mjs` is bundled into the page by `build-page.mjs` and is
imported under node by the unit tests, so the function the tests exercise is
the function the browser executes. Inside the page it is called once per row,
and its result both sets the viewport's window and fills the sidecar's, so the
rendered window and the recorded window are one value rather than two that
happen to agree.

The declared CT default is the soft-tissue window, centre 40 and width 400.
Everything else falls to a `full-range` rule that derives the window from the
image's own minimum and maximum. PS3.3 C.11.2.1.2 places the LINEAR window's
lower edge at `c - 0.5 - (w - 1) / 2` and its upper at `c - 0.5 + (w - 1) / 2`,
so

```
w = max - min + 1
c = (max + min + 1) / 2
```

puts the lower edge exactly at `min` and the upper exactly at `max`. No stored
value clips and none of the display range is wasted.

A constant image answers `w = 1`. That is the smallest width C.11.2.1.2 allows
and it is also degenerate, because `w' = w - 1` is then zero. Nothing clamps it
to hide that: a constant image has no window that shows anything, and the
uniform frame it produces is refused by the read-back guard, which is where a
frame that shows nothing belongs. `max < min` is not a range at all and is
refused outright.

**The minimum width is per function, not shared.** C.11.2.1.2 requires
`w >= 1` for LINEAR because it divides by `w - 1`. C.11.2.1.3.1 (SIGMOID) and
C.11.2.1.3.2 (LINEAR_EXACT) require only `w > 0` because they divide by `w`.
One constant shared between them would be the same class of defect as one
branch shared between them, and the difference between LINEAR and LINEAR_EXACT
is a half and a one.

**One divergence between that rule and the reference, recorded for F-011.**
cornerstone3D 5.8.2's `toLowHighRange` applies LINEAR's `(w - 1) / 2` to
`SAMPLED_SIGMOID` as well, so a file declaring SIGMOID with a width between 0
and 1 would be accepted here, correctly under C.11.2.1.3.1, and produce an
inverted range in the reference: width 0.5 at centre 40 gives lower 39.75 and
upper 39.25. No corpus row reaches it, because all eighty-five windowed rows
resolve LINEAR. It is written down because the first SIGMOID row added to the
corpus will meet it, and because it is the reference's divergence from the
standard rather than this harness's.

**Neither the CT default nor the full-range rule is reached by the corpus as it
stands.** Eighty-five of the eighty-nine rendered rows carry their own window
and four are colour, so both fallback branches are exercised by the unit tests
alone. They are not dead code, they are the declared answer for a file that
arrives without a window, and the tests are what keep them honest until one
does.

Centre and width are converted to a `voiRange` by cornerstone3D's own
`utilities.windowLevel.toLowHighRange`, so no arithmetic of ours enters the
reference.

The VOI a file asks for is read from the top-level tags first and from
cornerstone3D's `voiLutModule` second, and which of the two answered is
recorded as `voi.origin` in the sidecar. The second source exists for
`synthetic/ct_multiframe_perframe.dcm`, whose window lives in per-frame
functional groups (PS3.3 C.7.6.16.2.10) and not at the top level at all.

It is NOT read from the loaded image's `voiLUTFunction`. In cornerstone3D
5.8.2, `createImage.js` computes that as
`(voiLutModule.voiLUTFunction?.length && voiLutModule.voiLUTFunction[0])`,
which indexes a **string** and yields `"L"` for a file saying `"LINEAR"`.
Feeding that back into the reference's own `toLowHighRange` throws `Invalid VOI
LUT function`. Recorded here because the next person to reach for that field
should know before they lose an afternoon.

## The four boundaries

| Boundary | Where | What fails it |
|----------|-------|---------------|
| reached | `run.mjs` | a manifest row the loop never attempted |
| decoded | `page/app.mjs` | the image load rejected, or resolved with no image |
| presented | `page/app.mjs` | cornerstone3D's own `IMAGE_RENDERED` did not fire inside 30 seconds, or the viewport is showing another row's image |
| read back | `page/app.mjs` | the canvas is not the declared size, or the frame is one value |

Three things about that table are load-bearing.

**Presented waits on the event, never on a sleep.** A fixed sleep is the exact
shape this defect class takes, and a timeout here is a failure rather than a
retry. The viewport's current image id is also checked against the row's, so a
frame left over from the previous row cannot satisfy it.

**Read back checks for degeneracy.** A blank canvas reads back perfectly. The
viewport canvas is also painted a sentinel magenta immediately before
`render()`, so a frame that comes back still sentinel says specifically that
`IMAGE_RENDERED` fired without anything reaching the canvas.

**The frame is hashed twice**, once in the page before it is base64 encoded out
of the browser and once by the driver over what arrived, and a mismatch is
refused. A truncation between the two would leave F-011 comparing against a
frame nobody rendered.

Each of the four has been observed red by fault injection, and the injections
are kept rather than described: `node tools/oracle/run.mjs --inject <name>`, or
`tools/oracle/tests/faults.mjs` for every one at once, which the oracle gate
runs. A guard nobody has watched fail is not a guard.

The catalogue is `tools/oracle/src/faults.mjs` and it is the authority. This
table is the same list in prose, and where the two disagree the code is right.
Some entries aim at a boundary's own check and some at a refusal inside one
that no other fault reaches. Every refusal in `page/app.mjs` is reached by one
of them.

| Fault | Boundary | What it breaks |
|-------|----------|----------------|
| `drop-row` | reached | the driver's own loop skips a row |
| `truncate` | decoded | the row's bytes are cut to 256 |
| `reject-syntax` | decoded | the Transfer Syntax UID becomes one no decoder claims |
| `no-render-event` | presented | the page never calls `render()` |
| `no-stack` | presented | the row never reaches the viewport |
| `stack-throws` | presented | `setStack` rejects |
| `bad-interpolation` | presented | an interpolation resolved from `Object.prototype` |
| `bad-camera` | presented | a camera mode the page does not implement |
| `stale-frame` | read back | the page fires `IMAGE_RENDERED` without drawing |
| `uniform-canvas` | read back | the frame is overwritten with one value |
| `wrong-canvas-size` | read back | the frame is read back against a size nobody declared |
| `bad-voi-source` | internal | an unexpected throw inside the page |

`stack-throws` says **presented** and not decoded, which was wrong here for
several rounds. By the time `setStack` runs, the image has loaded and
`stage.decoded` is already true, so a row counted in `decoded` cannot also have
failed there, and `unsupported.json` matches entries on the boundary name.

`stale-frame` is the one that exercises the sentinel specifically. Without it
the sentinel branch would be present, authoritative-looking and never reached,
which is the defect class it was written to catch.

**`no-stack` is the important one.** It aims at the check that the viewport is
showing THIS row's image and not the previous row's, which is the
quietly-wrong-pixel class this project names as its dangerous defect: a frame
from the wrong row reads back perfectly and hashes stably under the right row's
name. `bad-interpolation` and `bad-camera` aim at the two declared parameters
the page refuses rather than ignores, and `bad-interpolation` uses
`constructor` specifically, because `InterpolationType` is a numeric enum on
`Object.prototype` and a guard that looked the string up rather than checking
the name would accept it. `bad-voi-source` is the only one that proves an
unexpected throw inside the page arrives as a boundary rather than as a
Playwright evaluation error, which is why its boundary is `internal` and not
one of the four.

The catalogue lives in `src/faults.mjs` rather than in `tests/`, because two
entries mutate the bytes the driver sends, four mutate the render parameters
and one changes the driver's loop, so the declarations are production data.
`tests/faults.mjs` is only the runner. The cost is one extra browser launch per
fault on every oracle gate, and it buys a re-runnable proof rather than a note
saying somebody once watched them fail.

## Applicability is discovered and recorded, never skipped

Every row is attempted. A row that fails is checked against `unsupported.json`,
which is committed, and an entry has to match the row's transfer syntax, the
boundary, the row's own path, and a fragment of the error the reference
produced. Strict in both directions:

- a failure no entry describes fails the run, so the file cannot grow into a
  list of excuses,
- a row an entry claims that then succeeds fails the run, so a stale claim
  cannot read as a known limit and hide a coverage gain.

That is the difference between an oracle that covers 89 rows and knows it, and
one that covers 89 and reports 91.

### What cornerstone3D 5.8.2 could not render, measured

Two rows of ninety-one. Both are in `unsupported.json` with the full reasoning.

- **`synthetic/us_ybr_full_422.dcm`**, uncompressed YBR_FULL_422. PS3.3
  C.7.6.3.1.2 subsamples chroma two to one horizontally, so the frame is
  `Rows * Columns * 2` bytes and not `Rows * Columns * SamplesPerPixel`.
  Measured on the file: 480 bytes where the naive size is 720. cornerstone3D
  sizes the texture the naive way, the browser refuses the short upload, and
  the frame reads back blank. **The load resolves**, so this fails at read back
  and not at decode. Without the degeneracy check the run would have written a
  stable digest for a blank frame and called the row covered.
- **`syntax/deflated_explicit_vr_le.dcm`**, Deflated Explicit VR Little Endian.
  The default `loadImageFromNaturalizedMetadata` path does not inflate before
  parsing, so it reads compressed bytes as a data set and throws on a garbage
  length. The package does carry an inflater, but only the deprecated
  `useLegacyMetadataProvider` path reaches it. The harness runs the default,
  because the default is what a cornerstone3D 5.8.2 user gets and is therefore
  what the reference is.

**This does not answer Appendix A gates A1 or A2, and it is worth being exact
about why.** A1 asks whether HTJ2K decodes correctly "in openjp2 under wasm32,
bit-exact against OpenJPH", and A2 asks what the JPEG-LS answer is, "CharLS
bridge, self-compiled CharLS, or a young pure-Rust crate". Both are questions
about Ocelli's own codec build, which does not exist yet. What this story gives
them is the other half of the measurement: all three HTJ2K rows and both
JPEG-LS rows decode, present and read back through the reference, so there are
reference frames for those five rows to be compared against once there is
something to compare.

## Output, and why none of it is committed

**The output directory holds one complete run that passed every boundary, or
it holds nothing.** That is arranged from both ends. `prepareOutput` empties it
before the pins are checked and before the first row is read, so everything
that aborts early leaves nothing behind. The two checks that can only run after
the frames exist, the pydicom cross-read and the fault self test, record a
failure rather than throwing, and the single failure path discards the
directory again. `run.json` therefore carries no `ok` field: the file exists
only for a run that passed, and a field that could say only one thing would be
one more thing to keep true.

It will not empty a directory it did not write. `--rows` requires `--out`, so
an operator types a directory name on the normal path, and a non-empty
directory with no `run.json` in it is refused rather than deleted. So is an
`--out` that turns out to be a file, named as such, rather than swallowed as
"nothing there yet" and surfacing much later as an unexplained `EEXIST`.
`tests/output_test.mjs` covers all five cases.

Per row into ignored `tools/oracle/out/`:

- `<id>.raw`, the RGBA8 bytes the PNG encoder never touched. **F-011 compares
  against these.**
- `<id>.png`, for a human looking at a divergence.
- `<id>.json`, the sidecar.

Once per run, `run.json`: the manifest digest, the digests of
`render-params.json` and `unsupported.json`, the installed versions of all
sixteen pinned packages, the page's own environment, the host, the counts at
each of the four boundaries, the determinism result, and a digest per row.

It also records **which checks ran**, under `checks`. The `--no-*` flags are
development aids and the gate passes none of them, but a record that did not
say so would look identical whether they were passed or not, and `partial` is
in the file for exactly the same reason.

`<id>` is the manifest path with `/` replaced by `__` and `.dcm` dropped, so
the three files for a row always sit beside each other and no two rows collide.

**Nothing under `out/` is ever committed**, and the reason is the project's
first rule rather than tidiness. A reference frame of a real corpus row is a
rendered picture of patient data and every real row carries
`burned-in-unchecked`, because HLD story E22.3 is not built. The pre-commit
hook refuses staged DICOM by magic bytes and would not refuse a PNG, so
`scripts/staged_content_check.py` refuses anything under `tools/oracle/out/` by
path. `.gitignore` covers it too, and that is not enough on its own because
`git add -f` exists.

### The sidecar carries two readings of the same metadata

HLD section 11 diffs metadata alongside pixels "because a wrong rescale slope
can still produce a plausible image". So the sidecar carries:

- `attributes`, read straight from the bytes by `dicom-parser` in the page,
  independently of the render path,
- `cornerstoneMetadata`, the modules the reference itself resolved and used.

Two readings on purpose. A sidecar that only transcribed the reference's own
reading could not show the reference reading a file wrong, which is exactly
what happened with YBR_FULL_422 and with `voiLUTFunction`.

`attributes` is then cross-read a third time by `check_sidecars.py` under
pydicom, over every sidecar, plus a hand-written expectation table for nine
named synthetic rows whose values are known by construction from
`scripts/corpus_synth.py` and PS3.3. The table is required to ASSERT every
photometric interpretation and both values of Pixel Representation that the
corpus's sidecars carry, and the checker fails if that stops being true.
Asserting, not merely naming: a row can sit in the table with five assertions,
none of them about photometric interpretation, and a check that only asked
which rows were named would call that value covered.

`cornerstoneMetadata` is checked too, on two rows, and one of them is the whole
point. `synthetic/ct_multiframe_perframe.dcm` carries its rescale and its
window only in the per-frame functional groups (PS3.3 C.7.6.16), so the
independent top-level read correctly reports absent, both readers agree on
absent, and the values that actually drove the render would be verified by
nothing. Slope 1 and intercept -1024 for frame 0 are asserted against the
generator's own constants instead.

Real corpus rows report a mismatched attribute by name only and never by value,
following `corpus_check.py`'s convention.

## Determinism, measured and bounded

Every gate run renders the corpus **twice, in one browser and one page**, and
requires identical digests. That is the claim this story can support: stable output on
one machine and one browser build. It is not a claim of cross-machine
reproducibility, and D14 is why the stronger claim is not made. `run.json`
carries the host and the adapter string, so a cross-machine difference is
attributable rather than mysterious.

Decoding runs on **one** web worker, so decode order is fixed, and the image
cache is purged between rows, so eviction order cannot differ between the two
passes.

## Covered is not the same as measured

A frame can pass all four boundaries and still say very little. The synthetic
transfer-syntax rows are full-range ramps that declare a soft-tissue CT window,
so most of the ramp clips to white: `syntax/reference_mono12.dcm` comes back
25% black and 71% white, and a codec error in the clipped values would not show
in a pixel diff.

That is not a failure. The frame is exactly what the file asks for, and the
plan's rule is that the file's own window wins. So the run **counts and names**
these rows instead: any frame over `informationFloor.extremeFractionWarnAbove`
black and white together is listed in `run.json` under `lowInformation` and
noted on stdout. Sixteen of the eighty-nine rendered rows are on that list:
every rendered `syntax/` row whose content is a mono16 ramp, which is fifteen
of them, plus `synthetic/ct_unsigned_16.dcm`, which is a 16-bit ramp under the
same CT window. (`syntax/` holds eighteen rows. The sixteenth mono16 one,
`deflated_explicit_vr_le`, is not rendered at all, and the other two are
colour.) F-011 should weight them accordingly, and the
fuller fix is either a declared category rule here or a corrected declared
window in `scripts/corpus_synth.py`. Both are later stories, and neither should
be done by accident.

The two rows fitted DOWN into the canvas are the same kind of fact and are
counted the same way, under `downsampled`. Neither list fails a run. Both exist
so that "the row was covered" cannot be read as "the row was measured".

## Stack viewports only, and what that leaves undone

HLD section 28 says the goal of the first two weeks is to diff one windowed 2D
image, and F-011 is a pixel-diff comparator, so every reference frame here is a
**stack** render of one frame of one instance. Volume and MPR reference renders
are not produced.

The consequence is worth naming rather than inferring from an output directory.
The twenty synthetic spacing rows, ten under `synthetic/ct_series_uniform/` and
ten under `synthetic/ct_series_nonuniform/`, are rendered here as twenty
independent stacks. (Sixty-two manifest rows carry the `series` category
token: these twenty, plus the twenty-seven of `real/ct_cmb_mml` and the fifteen
of `real/mr_eay131`. The other two real directories are single instances and
carry no such token. The twenty are the ones written to exercise spacing.) **The volume builder's refusal path
that the non-uniform ten exist to exercise is asked nothing by this story.**
Those rows are covered as pixels and untouched as geometry. Volume reference
renders belong to the later E2 stories.

## Where it sits in the gates

`bin/ocelli.sh oracle` runs everything: the pins, the unit suites, two corpus
passes, the pydicom cross-read and the fault self test. `bin/ocelli.sh gate
oracle` calls it. It stays out of `--floor`, which is deviation D-04 and is
unchanged by this story.

**The S01 pre-oracle skip is gone.** It let `gate --sprint` report an absent
oracle as a named skip while S01 was building the corpus the oracle needs, and
its condition cannot be true now that the oracle exists. `s01_pre_oracle` has
been removed from `bin/ocelli.sh` rather than left with a condition that can no
longer fire, and `.claude/WORKFLOW.md`, `.claude/commands/verify.md` and
`.claude/commands/close-sprint.md` are corrected in the same change. A dead
exception is a live misreading.

## Prerequisites

```bash
cd tools/oracle && npm ci && npx playwright install chromium
uv sync --locked                 # pydicom, for the sidecar cross-read
```

The cross-read resolves its interpreter the way `corpus_tests.py` does, and
the important half is where it stops. `$OCELLI_PYTHON` is **authoritative**: if
it is set and cannot import pydicom, nothing else is tried, because answering
with a different interpreter from the one the operator asked for is its own
quiet failure. With it unset the implicit candidates are `.venv/bin/python`
then `python3`, and one that cannot import pydicom is passed over. Running out
of candidates is a failure and never a skip.
