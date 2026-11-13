# F-204, One stored pixel per device pixel in the single showcase viewer

**Status**: approved
**Epic ref**: E38.14
**Sprint**: S47
**Estimate**: 2w

Scheduling and one-viewer scope were authorized by the operator on 2026-11-08.
The estimate is a provisional lead engineering estimate, not measured capacity
or a calendar promise. This is revision 4 of the plan, a short round revised
in place after the round 3 independent review returned 3 defects, 6 smells
and 7 nitpicks. Every finding is applied in this revision and listed with its
landing place in the round 3 section below. The round 2 amendment section and
the decisions it carries stand unchanged. The material changes from revision
3 are: the linked-zoom acceptance row split into an answer (a) form and an
answer (b) form, the held-mode drop on a remote display defined at one named
point with a named page field, the F-205 overlap re-read from the revised
F-205 plan, the refusal reason given clear sites and a no-stale-reason
assertion, the ratio watcher proved by counting its own handler, the suite
pinned to sRGB for its exact-byte rows, the remote harness named, a
large-frame zoom-out row added, and the fractional right-edge device column
of the anisotropic readback excluded by name.

This plan is written against the landed viewer at `ac962c64`, the F-245
integration commit, which is this worktree's HEAD on `work/f-204-glm`
(`git rev-parse --abbrev-ref HEAD`, `git log --oneline -1 HEAD`, run this
session). The adopted landing order below has F-202 land first, so this
story's implementation starts on the post-F-202 tree and re-verifies the page
facts named here against that tree at implementation start. The decisions are
unchanged by the rebase, and implement-feature's rule for a tree that
contradicts a plan's facts applies as written.

## Lead amendment after design round 2

Each ruling below is applied in this revision. The section names where.

- **D1, AC6 contradicted decision 5.** With fixture C at ratio 2 the extended
  floor `min(fitted * 0.05, 1 / ratio)` is exactly the identity, so a
  zoom-out tick clamps back to it and the old row's "goes below the identity"
  fails a correct implementation. Decision 5 stands. AC6 now asserts the
  identity is reached from above and then held, with the precondition measured
  in the suite. See AC6.
- **D2, the 1.44 probe cannot fail.** Chromium stores the device scale factor
  as f32, so `devicePixelRatio` reads 1.440000057220459 and
  `(1 / r) * r` is exactly 1 there, as it is at 1.27 and 1.58. Verified with
  `node` this session. The probes are now 1.38, 1.45, 1.54 and 1.57, where the
  product is one ulp below 1, and each case asserts that precondition in the
  page before trusting itself. See fact 23 and AC10.
- **D3, pixel readback at the fractional ratios.** AC3 now reads device bytes
  at ratios 1.25 and 1.5 on both fixtures, where a truncated layout unit or a
  128 versus 129 device column drops or doubles a column. The Tests table
  names exactly the cases that run. See AC3 and Tests.
- **D4, the F-202 seam is answerable now.** F-202's round 2 plan draws the
  remote frame into its own transferred canvas inside the active cell's stage,
  bound through `bindCpuCanvas` and `bind_cpu` with a device-pixel backing
  store (F-202 plan lines 428 to 434 and 1063 to 1067, `session.rs` 806 to
  824 and `canvas.rs` 718 verified in this session). Lead ruling adopted: on
  a remote frame the mode is refused by name with a visible reason until a
  later story routes it through the core factor. Decision 15 designs the
  refusal, AC16 fails if the corner reads `Zoom 1:1` over a remote raster,
  the More row names the source coverage, open question 5 is closed, and the
  shared-file order adopts F-202's request that F-202 land first.
- **D5, open question 4 rested on a false premise.** vtk renders in world
  space, so with `spacingWorld[1]` fixing the rows, cornerstone 5.8.2 does
  enter its scale mode on anisotropic images: rows draw one to one and
  columns draw at `spacing[0] / spacing[1]` device pixels each, nearest
  dropped at 0.714 on the synthetic. The false sentence at plan line 1025 of
  revision 2 is removed. Open question 4 is re-posed under "cornerstone is
  the floor" with the lead recommendation to match the floor, and both
  answers are designed with their own acceptance rows. See the cornerstone
  transcription, decisions 1, 4, 5 and 7, AC13, AC17 to AC19 and open
  question 4.

Smells, all resolved. **S1**, the snap and the assertions now use the same
element, the canvas, and the epsilon is derived from the layout unit
(decision 2, fact 23, AC2). **S2**, AC5 counts resize events in-page as a
precondition so it proves the `matchMedia` watcher (AC5). **S3**, the drag
quantisation is dropped because the origin snap covers every pan (decision 2,
AC11). **S4**, the refusal reason has a state field, a show rule, a clear
rule and a stated corners-hidden answer (decision 7). **S5**, mode
propagation onto unsuitable frames is defined and one case is tested
(decision 9, AC7). **S6**, the write-set overlap is re-grepped and the order
is stated across F-267, F-202, F-205 and F-241 (facts 19 to 21,
Dependencies). **S7**, open question 6 is closed, the mode is not stored
(decision 14). **S8**, the readback captures an enclosing integer CSS region
on the ratio's device-aligned grid and crops in-page by an exact device
offset (AC3).

Nitpicks, all fixed. Decision 7's case count is now three silent no-ops plus
the named refusals, counted. The output quantisation step
(`display_index` rounds to nearest, `crates/ocelli-render/src/colour.rs` 518
via `examples/folder-viewer/src/preview.rs` 1531) is transcribed into the
fixture, with the verified count of 95 f32 values in 256 that miss the
identity without it, and fixture P now carries a stored 255 so the top
boundary value is exercised. The half-pixel shift paragraph now says the
shift is keyed on the canvas device box's parity and ignores the image's
parity, and the 8-pixel floor is no longer offered as an analogy, because it
bounds a different quantity. The 1/64 CSS px rationale is restated as 1/64 of
a device pixel per the lead ruling. AC14 names its ratio. AC1 is marked the
coarse guard with AC3 the real proof.

## Round 3 review, applied in revision 4

Each finding below is applied in this revision at the named place. The
review reproduced round 3's arithmetic and confirmed about 40 citations, so
only the findings changed text here.

- **D1, the linked-zoom row assumed answer (b).** AC7, its mutation control
  and the linked state row all named the anisotropic linked viewport as the
  one that must not enter, which is true only under open question 4 answer
  (b). AC7 is split into AC7a and AC7b. Under answer (a) the unsuitable
  linked viewport is a stored frame smaller than the device pixel ratio or a
  remote-displayed cell, and an anisotropic target enters with the
  `Zoom 1:1 rows` label. See AC7a, AC7b, the linked row of the state table
  and the Tests mutation controls.
- **D2, a mode already held is not dropped when a remote frame is
  displayed.** F-202 routes the local draw path away from the cell while the
  remote source is active, so the guards in `layoutViewport` never re-run
  there. The transition is now defined: the remote activation site sets that
  viewport to fit, clears the refusal and rewrites the label, and the guard
  reads a named page field. AC16 gains the held-mode case. See decision 15,
  the remote rows of the state table and AC16.
- **D3, the F-205 write set was read from a stale copy.** The revised F-205
  plan names a full write set. Fact 21 is rewritten from it and every
  overlap list under Dependencies is corrected, including `use.html`, which
  F-202, F-205 and F-241 all touch. See facts 19 and 21 and Dependencies.
- **S1, `vp.zoomRefusal` was never cleared.** It is now cleared on every
  frame change and at any paint where the refusal predicate is false, and
  AC13 and AC16 assert the no-stale-reason case. See decision 7, the
  ownership table and those rows.
- **S2, AC5's zero-resize-events requirement proved the wrong listener.**
  The init script now wraps `MediaQueryList.prototype.addEventListener` and
  counts invocations of the page's own watcher handler, and the zero-resize
  requirement is gone. See AC5.
- **S3, the exact-byte rows need a pinned colour profile.** The suite
  launches Chromium with `--force-color-profile=srgb`, and AC3, AC14 and
  AC19 say why. See the acceptance preamble and those rows.
- **S4, AC16's harness was unnamed.** It reuses the synthetic upstream
  `dicomweb.mjs` defines, extracted unchanged into a shared module both
  suites import. See AC16 and the harness decision.
- **S5, the clamp floor had no large-frame case.** AC20 zooms out below the
  identity on a 2048 px frame, which a floor of `identityScale` alone
  refuses to do. See AC20 and fixture E.
- **S6, the anisotropic readback's fractional right edge.** AC19 excludes
  the partially covered device column 91 and says why. See AC19.
- **Nitpicks, all applied.** `.view`'s pixelated rule is page.html line 254.
  `drawGrey`'s layout call is line 3464. `showFault` is about line 3751 and
  the block at 1866 to 1875 is `processBack`. The window header token maps
  through `window_function_from_token` at preview.rs 2283 to 2285. The
  `bind_cpu` range is session.rs 806 to 824 everywhere. Fixture B's CSS box
  is fractional and its device box at the identity is integral. The pattern
  is distinct along each row and each column, not across a fixture.
  `pictureW * dpr` equals the stored width up to one ulp, not exactly. The
  applied translate is removed from the layout stamp, because it is an
  output. And a note for F-202's designer records that F-202's plan wrongly
  says F-204 writes `use.html`.

## Normative source, transcribed

There is no HLD Part II section for the viewer page's display scale. Part II
specifies the library. The normative sources for this story are the tracked
More page statements, the DICOM window identity the fixture rests on, the
output quantisation rule the drawn byte follows, the cornerstone3D 5.8.2
reference behaviour, the HLD's tier, tolerance and fixture rules, and the
core type contract the page's mapping must agree with. Each is transcribed
before any design sentence.

`examples/folder-viewer/src/more.html` lines 524 to 537, the One to one
section. This is the story's own tracked definition of done:

```text
<h2>One to one</h2>
<span class="status none">Not yet in the viewer.</span>
<p>One to one shows each stored pixel as exactly one screen pixel, so nothing
is resampled and fine texture is seen as acquired.</p>
<p class="where">There is no code for it. A <code>Zoom</code> command in
<code>@ocelli/core</code> scales the camera by a factor, and nothing computes
the factor that maps one stored pixel to one device pixel.</p>
...
<p class="tiers">The example page reads One to one unavailable.</p>
```

`examples/folder-viewer/src/more.html` line 156, the matrix row carrying
`data-features="F-204"`:

```text
| One stored pixel to one device pixel | Display-area scale API | Zoom building blocks | Not yet: A dedicated one-to-one mode is missing | S47: F-204 |
```

The matrix columns are Feature, Cornerstone, Ocelli library, Ocelli viewer,
Delivery (more.html line 149). Its caption reads "Viewer status is the
delivery status. A library-only implementation stays Not yet or Partial
here." The More lede at line 135 reads: "A feature is implemented here only
when it has a usable workflow on Images." More line 214, the S47 wave row,
names "one-to-one display" among the sprint's outcomes.

### cornerstone3D 5.8.2, read this session

The round 1 draft said the pinned packages were not installed and no
cornerstone source was read. That was wrong for this machine: the reference is
readable read-only at
`/Users/atulsharma/Documents/projects/tensorbee/ocelli/tools/oracle/node_modules/@cornerstonejs/core`,
`package.json` `version` `5.8.2` (read this session). Nothing outside the
worktree was read except that path. `tools/oracle/node_modules` is absent in
this worktree (`ls tools/oracle/node_modules`, no such directory), so the
citations below name the absolute path. The two transcripts below were
re-read this session and match revision 2's transcription.

`dist/esm/RenderingEngine/Viewport.js` lines 429 to 460,
`setDisplayAreaScale`, verbatim minus two unchanged tail lines:

```js
    setDisplayAreaScale(displayArea) {
        const { scale = 1 } = displayArea;
        const canvas = this.canvas;
        const height = canvas.height;
        const width = canvas.width;
        if (height < 8 || width < 8) {
            return;
        }
        const imageData = this.getDefaultImageData();
        const spacingWorld = imageData.getSpacing();
        const spacing = spacingWorld[1];
        this.setInterpolationType(InterpolationType.NEAREST);
        this.setCamera({ parallelScale: (height * spacing) / (2 * scale) });
        delete displayArea.imageArea;
        this.setDisplayAreaFit(displayArea);
        const { focalPoint, position, viewUp, viewPlaneNormal } = this.getCamera();
        const focalChange = vec3.create();
        if (canvas.height % 2) {
            vec3.scaleAndAdd(focalChange, focalChange, viewUp, scale * 0.5 * spacing);
        }
        if (canvas.width % 2) {
            const viewRight = vec3.cross(vec3.create(), viewUp, viewPlaneNormal);
            vec3.scaleAndAdd(focalChange, focalChange, viewRight, scale * 0.5 * spacing);
        }
```

What this says, in order. At `scale` 1, `parallelScale` is
`height * spacing / 2`, which maps one stored pixel onto one canvas backing
pixel, because the backing store is device-pixel sized:
`dist/esm/RenderingEngine/TiledRenderingEngine.js` lines 150 to 152:

```js
            const devicePixelRatio = window.devicePixelRatio || 1;
            canvas.width = canvas.clientWidth * devicePixelRatio;
            canvas.height = canvas.clientHeight * devicePixelRatio;
```

Cornerstone forces `InterpolationType.NEAREST` for the scale path. It uses
`spacingWorld[1]`, the row axis, as the one spacing that defines the 1:1
scale. Its half-pixel focal shift is keyed on the canvas device box's parity
alone and ignores the image's parity: when `canvas.height` is odd it shifts
the focal point along `viewUp` by `scale * 0.5 * spacing`, and when
`canvas.width` is odd along `viewRight`, whatever the stored extents are. The
shift puts the image's samples on device-pixel centres when the device box
cannot be centred exactly. That half-pixel shift is the origin alignment this
story exists to match.

**Cornerstone is the floor on anisotropic images too.** vtk renders in world
space, so `parallelScale` fixed by `spacingWorld[1]` gives one device pixel
per stored row on every image, and one stored column draws at
`spacingWorld[0] / spacingWorld[1]` device pixels. That `spacingWorld[1]` is
the row axis is pinned in the reference itself:
`dist/esm/RenderingEngine/StackViewport.js` lines 1371 to 1381 equate
`ySpacing` from `imageData.getSpacing()` with `image.rowPixelSpacing` and
`xSpacing` with `image.columnPixelSpacing` (read this session), and
`setDisplayAreaScale` takes `spacing = spacingWorld[1]`. On the built-in
synthetic
study, Pixel Spacing `0.7\0.5` is 0.7 between rows and 0.5 between columns,
so columns draw at `0.5 / 0.7 = 0.714` device pixels each and nearest
sampling drops columns. Cornerstone does not refuse the scale path on an
anisotropic image. It enters it, draws the rows exactly and resamples the
columns by nearest. That is the reference behaviour this story is measured
against, and open question 4 asks whether Ocelli matches it or refuses with a
named reason.

Cornerstone's own floor is on a different quantity from this story's. It
refuses when the canvas backing store is under 8 device pixels on either axis
(Viewport.js lines 433 to 435), a bound on the viewport's device box. This
story's floor is on the stored frame dimension. The two are not the same rule
and this plan does not claim cornerstone's floor as its reference.

Ocelli's differences, stated. Ocelli keeps the canvas backing store at the
stored frame size and maps through CSS, where cornerstone sizes the backing
store to `clientWidth * devicePixelRatio`. Ocelli's mode is exact on both
axes at once for square pixels, where cornerstone's scale is defined by the
row spacing alone and is 1:1 on the column axis only when the in-plane voxels
are square. Under open question 4's recommended answer the anisotropic
behaviour matches the reference: rows 1:1, columns nearest-resampled at
`spacing[0] / spacing[1]` device pixels each, with the corner saying which
axis is exact. Ocelli reaches the identity as one reachable zoom state beside
arbitrary factors and fit, where cornerstone's `setDisplayArea` takes an
arbitrary scale with 1 as the identity.

### DICOM PS3.3 C.11.2.1.2.1, the fixture's identity

Fetched this session from
`dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.2.html`
(PS3.3 2026d). The LINEAR pseudo-code, quoted:

```text
if (x <= c - 0.5 - (w-1)/2), then y = ymin
else if (x > c - 0.5 + (w-1)/2), then y = ymax
else y = ((x - (c - 0.5)) / (w-1) + 0.5) * (ymax - ymin) + ymin
```

Note 4 to that section, quoted with the page's flat rendering of exponents:
"A Window Center of 2n-1 and a Window Width of 2n selects the range of input
values from 0 to 2n-1. This represents a mathematical identity VOI LUT
transformation over the possible input values (whether used or not) in the
case where no Modality LUT is specified and the stored pixel data are n bit
unsigned integers." The page renders superscripts inline, so the same flat
string carries two meanings. The identity reading is Window Center
`2^(n-1)`, Window Width `2^n`, range `0` to `2^n - 1`, and the pseudo-code
confirms it: at `c` 128, `w` 256 the clamp bounds are `127.5 - 127.5 = 0` and
`127.5 + 127.5 = 255`, and the linear branch is
`((x - 127.5) / 255 + 0.5) * 255 = x`. The rival reading, Centre 255, is not
the identity: it clamps every `x <= 127` to `ymin`.

The drawn byte is not the f32 display value. The output quantisation step
belongs to the fixture and is transcribed here. The preview's display byte is
`display_index(display, 255)` (`crates/ocelli-render/src/colour.rs` 518,
reached through `display_byte` at
`examples/folder-viewer/src/preview.rs` 1531), and `display_index` rounds:
it adds `0.5` and searches, rather than truncating, and its own doc comment
states that the pinned reference quantises with `Math.floor(value * 255 +
0.5)` so rounding is the definition of correct. The distinction is load
bearing. Computed with `node` this session, the f32 evaluation of the linear
branch misses the stored value for 95 of the 256 inputs, each by one ulp low
(input 1 yields 0.9999999403953552, input 2 yields 1.9999998807907104, and so
on). Truncation would draw those 95 values one code dark. Round to nearest
restores the identity, so the expected byte is the stored value across `0` to
`255`.

The fixture therefore uses an 8-bit unsigned image, no Modality LUT and no
rescale, Window Center 128, Window Width 256, VOI LUT Function LINEAR, and
the expected display byte is the stored value across `0` to `255`. What the
clamps can and cannot show, stated exactly. The bottom clamp bound `x <= 0`
is satisfied at the stored value 0, which the fixture carries at `(0, 0)`, and
the bound agrees with the linear branch there, since
`((-127.5) / 255 + 0.5) * 255 = 0`. The top clamp condition is `x > 255`,
which no 8-bit stored value can satisfy, so no fixture value can exercise it
and this plan claims no more than that. The stored value 255 is carried by
fixture P at `(127, 95)` so the linear branch is exercised at the top
boundary, where it produces `ymax`. `LutChain` applies this exact function:
`crates/ocelli-pixel/src/lut.rs` `apply_window`, `VoiFunction::Linear` arm
(`center - 0.5`, `width - 1.0`, the same clamp comparisons), and the grey
route validates the window over the output range `0.0, 255.0`
(`examples/folder-viewer/src/preview.rs`, the `VoiTransform::new` call in the
monochrome preview path).

### The HLD and the tree's own contracts

`docs/hld/05-rendering.md`, section 7:

```text
**Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders,
storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's
downlevel profile: fragment shaders only, no compute, no storage buffers, a
conservative 3D-texture floor of 256. Every feature declares the tier it
needs; the tier resolves once at startup.
```

`docs/hld/23-performance-rules.md`, section 26:

```text
- No allocation in the render loop.
- One queue.submit() per frame.
- Prefer a uniform update to a texture update. Window/level is thirty-two
  bytes, not a re-upload.
- Batch pointer events into one command buffer per animation frame. Never
  cross the boundary per event.
- Measure with the benchmark harness before optimising anything. The
  intuitions that work in JavaScript do not transfer.
```

`docs/hld/13-core-types.md`, section 16:

```text
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
```

`docs/lld/viewport.md` lines 67 to 72, the device-pixel doctrine of the core
camera:

```text
`Pt<Canvas>` stays in CSS pixels. A device coordinate divides by the same
dpr as `device_extent` once, through `css_from_device`, before this mapping.
`View` does not store device pixels. The GPU camera uniform stores that CSS
map and the same ratio. The fragment shader divides a device-pixel position
by the ratio before the CSS multiply.
```

`docs/hld/22-testing-and-tolerance.md` section 25.1 sets the geometry
tolerance, "canvas coordinates within a quarter pixel", and section 25's
table roots unit evidence in "Hand-computed fixtures citing the DICOM
section". `docs/hld/24-agent-code-standards.md` 27.2 R2: tests derive from
the specification or the oracle, never from reading the implementation. R3:
every function doing pixel arithmetic needs a fixture with hand-computed
values citing the DICOM section.

`examples/folder-viewer/src/page.html` lines 2489 to 2493, the aspect rule,
and `examples/folder-viewer/tests/browser/aspect.mjs` lines 3 to 8, its
DICOM citation, which the landed test asserts as `64 : 67.2` on the synthetic
study:

```text
function greyAspect(facts) {
  if (!facts || !(facts.row > 0) || !(facts.column > 0) || facts.row === facts.column) return SQUARE_ASPECT;
  const least = Math.min(facts.row, facts.column);
  return { x: facts.column / least, y: facts.row / least };
}
```

```text
// The synthetic CT has 128 columns, 96 rows and Pixel Spacing `0.7\0.5`.
// PS3.3 10.7.1.3: the first value is the row spacing, the distance between
// the centres of adjacent rows, and the second is the column spacing.
```

`examples/folder-viewer/src/page.html` lines 3266 to 3274, the grey reply
contract the display scale sits on:

```text
const width = Number(response.headers.get("X-Ocelli-Width"));
const height = Number(response.headers.get("X-Ocelli-Height"));
const channels = Number(response.headers.get("X-Ocelli-Channels"));
const bytes = new Uint8Array(await response.arrayBuffer());
if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width <= 0 || height <= 0 ||
    (channels !== 1 && channels !== 3) || channels !== (image.info.colour ? 3 : 1) ||
    bytes.length !== width * height * channels) {
  throw new Error("the preview response did not match the frame");
}
```

## Tree facts, and the commands that checked them

Every claim below was checked in this session at HEAD `ac962c64` on
`work/f-204-glm`. The command is named beside each group. Claims sourced from
a sibling plan name the plan and the lines read there, and are marked as plan
facts rather than tree facts.

1. The display scale today is CSS pixels per stored pixel.
   `layoutViewport` (page.html lines 3579 to 3627) sets
   `pictureW = Math.max(1, imageWidth * scale * aspectX)` and
   `pictureH = Math.max(1, imageHeight * scale * aspectY)` at lines 3599 to
   3600, writes `dom.canvas.style.width/height` from them at 3614 to 3615,
   and `paintGrey` (lines 3414 to 3441) sets `node.width` and `node.height`
   from `grey.width` and `grey.height`. The canvas backing store is the
   stored frame size in every mode. Checked by reading the block.
2. The device pixel ratio enters the layout in two places, and nowhere else
   in layout: the stamp carries it (page.html line 3588) and the
   `image-rendering` choice uses it (line 3601,
   `scale * Math.min(aspectX, aspectY) * window.devicePixelRatio < 1 ?
   "auto" : "pixelated"`). The magnifier also reads it
   (`updateMag`, lines 4861 to 4864, `mag.width = Math.round(side * ratio)`)
   but the magnifier is not layout. Checked by reading both sites.
3. There is no one-to-one mode, control, key or scale anywhere in the page.
   `grep -c "1:1\|one.to.one\|oneToOne\|actual size\|native size"
   examples/folder-viewer/src/page.html` prints 0 (exit 1), and
   `grep -rn "oneToOneView" examples/ crates/ packages/` finds nothing.
4. The zoom label is fit-relative. `vp.zoomLabel` is set at page.html line
   3621, `(fitted > 0 ? Math.round((scale / fitted) * 100) : 100) + "%")`,
   and printed in the corner at line 3694,
   `assignText(corners.br, windowLine + "\nZoom " + vp.zoomLabel)`.
5. The zoom clamp excludes the 1:1 scale for small images on high-ratio
   displays. `zoomAt` (page.html lines 4314 to 4328) clamps at 4316 to 4317:
   `const next = clamp(vp.scale * factor, fitted * 0.05, fitted * 40)`. For
   a 64 by 64 frame in a 1600 by 1000 cell, `fitScale` (lines 3568 to 3577)
   gives `min(1600/64, 1000/64) = 15.625`, so the floor is `0.78125`, while
   the one-to-one scale at ratio 2 is `0.5`, unreachable by wheel or keys.
   The arithmetic was recomputed with `node` this session.
6. The pointer mappers, under their correct names. `imagePoint` is page.html
   lines 4629 to 4634 and wraps `clientToImage`. `clientToImage` is lines
   4645 to 4661 and `imageToClient` is lines 4663 to 4674. Marks, ink,
   magnifier and scale-bar geometry all derive from `vp.scale` (lines 3710,
   4870 to 4871, and the ink builders), so a scale change moves them
   consistently and none reads the ratio. Checked by reading the mappers.
7. The complete `fitMode` census is 13 sites, all in `page.html`:
   1367 (`blankViewport` init), 2518 (`loadSeries`), 3584 (the stamp),
   3593 (the layout fit branch), 3843 (`fitView`), 3858 (`resetView`),
   4289 and 4294 (the live-redraw keep block), 4326 (`zoomAt`), 4430 (the
   pan branch), 7006, 7007 and 7009 (`copyZoom`). Checked with
   `grep -n "fitMode" examples/folder-viewer/src/page.html`.
   `fitView` is at line 3842 and is reached by the Fit button (line 1639)
   and by the `0` key (lines 6033 to 6038). `grep -rln "fitMode"
   --include="*.mjs" --include="*.ts" --include="*.rs"` over the tree,
   excluding `node_modules`, finds no other file (exit 1).
8. The tile canvases are always fitted. `sizeTile` fits the stored-size
   canvas into the cell (page.html lines 3551 to 3566) and `zoomAt` returns
   early when the stage is hidden (line 4315).
9. The synthetic study is anisotropic. `examples/folder-viewer/src/synthetic.rs`
   lines 10 to 11 and 43: `ROWS = 96`, `COLUMNS = 128`, Pixel Spacing
   `0.7\0.5`, file window centre 40, width 400, `LINEAR`. `greyAspect` gives
   x 1 and y 1.4, so `SQUARE_ASPECT` (`{ x: 1, y: 1 }`, page.html line 1124)
   does not apply. Under nearest sampling a stored row at 1.4 device rows
   maps alternately to 1 and 2 device rows in the aspect-stretch display,
   which is the artefact the mode must not present as 1:1.
10. The browser suites start the viewer with `OCELLI_PORT=0`, drive it
    through tools/bench's pinned Playwright 1.62.1
    (`tools/bench/package.json` line 22, read this session), and stop it by
    PID (`aspect.mjs` `loadPlaywright` at line 45 and `stopByPid` at line
    118, imported by `window.mjs` line 76). No existing browser suite sets
    `deviceScaleFactor`: `grep -rn deviceScaleFactor
    examples/folder-viewer/tests/browser/` finds nothing. No existing suite
    or harness calls `page.screenshot`:
    `grep -rn "screenshot" examples/folder-viewer/tests/browser/
    tools/bench/src/ tools/oracle/*.mjs` finds nothing (exit 1). The
    equipped comparison pins the ratio to 1
    (`tools/bench/src/comparison_run.mjs` line 322
    `--force-device-scale-factor=1`, line 328 `deviceScaleFactor: 1`, and
    `tools/oracle/run.mjs` line 504).
11. The gate list and registration point are `bin/ocelli.sh` line 70 (the
    `viewer-browser` description string) and lines 214 to 231 (the case
    arm). The arm's comment at lines 207 to 210 says "F-245's
    launcher.mjs check is last", and `launcher.mjs` is the last entry at
    line 231. Checked by reading the arm.
12. The keyboard space is full. Every letter a to z is bound in `onKey`
    (page.html lines 6013 to 6062, read this session, through the two maps
    and the named branches), digits `1` to `9` choose window rows and `0`
    calls `fitView` (lines 6033 to 6038), `= + - _` zoom (lines 6027 to
    6032), `[ ]` panels, Shift R, Shift F and Shift G are taken. The
    backslash is bound nowhere in the chain. The KEYS section is page.html
    lines 919 to 932, its key rows 921 to 930, the scope note F-205 rewords
    is line 917, and the `SENTENCE` tip for fit is line 1024.
13. Pan clears fit mode and nothing else (page.html line 4430). The pan
    branch adds raw client deltas with no rounding
    (`vp.panX += event.clientX - drag.x`, lines 4431 to 4434) and calls
    `layoutViewport(vp)` synchronously per move (line 4437). Chromium
    `clientX` is fractional, so `panX` is fractional after any drag. That
    premise is browser behaviour stated by the review, not a fact readable
    from this tree, and the design below does not depend on proving it.
14. The stamp encodes pan only outside fit (page.html line 3584,
    `(vp.fitMode ? "fit" : String(vp.scale) + ":" + vp.panX + ":" +
    vp.panY)`). A mode that stamped scale without pan would never re-layout
    on a pan drag, because the stamp short-circuit at line 3589 would
    return. The stamp also carries the image size, the aspect, the rotation,
    the flips and the ratio (lines 3582 to 3588).
15. The page listens for layout-affecting environment changes only through
    `window.addEventListener("resize", relayout)` (page.html line 1779) and
    a `ResizeObserver` on `wrap` (lines 1780 to 1781). `relayout` itself
    sets `vp.layoutDirty = true` for every shown viewport before calling
    `layoutViewport` (lines 1787 to 1794), so a resize bypasses the stamp
    short-circuit. `grep -n "matchMedia" examples/folder-viewer/src/page.html`
    finds nothing (exit 1). A device pixel ratio change with an unchanged
    CSS viewport fires neither existing listener.
16. `canvasMatchesReply` is `tools/bench/src/viewer_work.mjs` line 772, used
    at lines 1036 and 1043 inside the equipped comparison. It runs in no
    gate: the `viewer-browser` arm runs the twelve `tests/browser/*.mjs`
    files, and the comparison is not one of them.
17. The overlay surfaces a readback must silence. `showCorners` (page.html
    line 1254) is toggled by the `toggle-corners` button (line 786, listener
    lines 1670 to 1672), which hides the corner labels (line 3640) and the
    scale bar (line 3709, `const show = showCorners && ...`). `paintSelection`
    draws only for a selected note, and `.ref-lines` draws nothing in a
    single unlinked view (`makeLineNodes` at line 6933 starts with
    `refs.style.display = "none"` and only linked scroll clears it).
    Orientation letters are placed at the cell edges (`.edge.top` and
    friends), outside a picture box that is smaller than the cell.
18. The local process applies VOI exactly as the standard says. The preview
    route validates the chosen window over output range `0.0` to `255.0`
    (`examples/folder-viewer/src/preview.rs`, `VoiTransform::new` in the
    monochrome path) and honours `X-Ocelli-Window-Function`: the page reads
    the header at page.html line 3286 and the local process maps the token
    through `window_function_from_token`, preview.rs lines 2283 to 2285
    with the `LINEAR` arm at 2285, called from serve.rs line 3853.
    `apply_window`'s Linear arm (`crates/ocelli-pixel/src/lut.rs`)
    is the standard pseudo-code with `c - 0.5` and `w - 1.0`. A file with no
    window takes index 0 with no header (page.html lines 3233 to 3236).
19. F-202's plan, `.claude/plans/F-202-design.md` in its own design
    worktree, is `**Status**: draft` (its line 3) and is under uncommitted
    revision there (`git status` shows the file modified, `git log --oneline
    -1` still at `ac962c64`). The copy read this session is 914 lines and
    ends mid-plan at a `<!-- CONTINUE -->` marker, so its write set, its
    landing-order request and the section the round 3 review cites as
    routing fit, zoom and pan to the worker while the remote source is
    active are **not re-readable in this session's copy**. What is visible
    and read: its ruling R4 (plan lines 63 to 69) hides the four overlay
    corners, `.scale`, the `.edge` markers, `.ref-lines` and `.ink` under
    one named visibility owner while the remote image is shown, with AC31
    failing if any local-image label is visible over the remote canvas. Its
    Scope item 6 (plan lines 676 to 682) decides the remote display's place
    on the page: the remote frame draws into its own canvas element inside
    the active cell's stage, created once and transferred to the render
    worker, shown while the remote source is active and hidden when a local
    source is active, and the local canvas is never transferred. Its Scope
    item 8 (plan lines 696 to 702) refuses readout, measurement, region
    window, marks, conformance, series window pooling and links on the
    remote source with a named unavailable line. Its F-245 boundary section
    (plan lines 422 to 434) records the page's status sentence at
    `page.html:1592` and `remoteRelease` at 1467. The visible Scope item 6
    alone already yields the D2 premise: with the remote source active the
    cell's local draw path does not run, so the guards in `layoutViewport`
    never re-run there. The tree cites behind the remote canvas's backing
    store were verified directly this session: `pub fn bind_cpu` is
    `crates/ocelli-wasm/src/session.rs` lines 806 to 824 and opens a
    `CpuRaster` for the CSS box and ratio, taking the device extent from
    `raster.device()` and pre-sizing `cpu_rgba` and `cpu_present` to the
    device-pixel byte count, and `publishCpuCanvas` is the js_name on
    `publish_cpu_canvas` at `crates/ocelli-wasm/src/canvas.rs` lines 717 to
    718, with the device extent computed by `device_extent` at the
    `resize_cpu_canvas` call, canvas.rs line 694. The landing order below
    stands on the lead ruling adopted in round 3, and the F-202 line that
    wrongly gives `use.html` to F-204 is recorded for F-202's designer
    under Dependencies.
20. F-267's plan, `.claude/plans/F-267-design.md`, `**Status**: approved`,
    writes `src/preview.rs`, `src/corners.rs`, `src/lib.rs`, `src/serve.rs`,
    `src/page.html`, `tests/preview.rs`, a new `tests/browser/` check, the
    `viewer-browser` case arm and its description string in `bin/ocelli.sh`,
    and `README.md` (plan lines 815 to 818). Its documentation section names
    the README corner sentence and states no `more.html` change is needed
    (plan lines 921 to 934). Its corner work extends the same
    `paintOverlays` corner statements this story's label change touches
    (page.html line 3694 region).
21. The F-241 plan, `.claude/plans/F-241-design.md`, writes `README.md`
    (plan line 368), `examples/folder-viewer/src/use.html` (plan line 369,
    the video route, the crates and functions it calls and the WebCodecs
    requirement), conditionally a `more.html` video card (plan line 370,
    "if the page has a card list for formats at implementation, else no
    change"), records in `docs/lld/build-targets.md` (AC21 and AC27),
    cites `bin/ocelli.sh` throughout its checks, may add a
    `bin/ocelli.sh wasm-video` step conditionally (AC21), and adds
    `tests/browser/video.mjs` (AC10, AC13, AC15, AC26). Checked with
    `grep -n "video.mjs\|build-targets" .claude/plans/F-241-design.md` and
    by reading lines 355 to 375 this session. The revised F-205 plan,
    `.claude/plans/F-205-design.md` in its own design worktree (663 lines,
    `**Status**: draft` at its line 3, under uncommitted revision), names a
    full write set (its lines 461 to 471): `page.html`,
    `tests/preview.rs`, a new `tests/browser/preferences.mjs`,
    `bin/ocelli.sh` line 70 and the case arm at lines 214 to 231,
    `more.html`, `use.html` for the section 7 search with no sentence
    change expected, and `README.md`. Its page edits include the scope note
    at page.html line 917, beside the KEYS panel (page.html lines 919 to
    932), and its preference constants, sections and startup lines. Its
    order section (its lines 479 to 502) recommends F-267 first, takes
    F-241's viewer phase after or as a rebase, and puts F-202 later. Its
    AC1 persists the mouse profile, the cine rate and both side columns,
    and no zoom. Read with `grep -n` over its write-set and order headings
    and `sed` over its lines 455 to 514 this session.
22. The operator's F-202 and F-245 AC8 answers of 2026-11-12 are recorded in
    `docs/sprints/CURRENT_SPRINT.md` by commit `864f3ed7`, which is one
    commit ahead of this worktree's HEAD: read with
    `git show 864f3ed7:docs/sprints/CURRENT_SPRINT.md`, line 88, "The
    idle-host performance acquisition runs at sprint end, with GLM paused,
    beside the consolidated `gate --sprint`."
23. Floating point and layout units, checked with `node -e` this session.
    Chromium stores the device scale factor as f32, so `devicePixelRatio`
    returns the stored value widened to double. After `Math.fround`, the
    product `(1 / r) * r` is exactly 1 at ratios 1.25, 1.27, 1.44, 1.5,
    1.58 and 2, and one ulp below 1 at 1.38, 1.45, 1.54 and 1.57. This
    corrects revision 2, which computed the products on exact doubles and
    named 1.27, 1.44 and 1.58 as failures. The expression at page.html line
    3601 therefore evaluates below 1 at the exact identity for the ratios
    whose f32 store rounds away, and the page selects `auto` there. The
    device box rounds back to the stored size at every ratio used below:
    `129/1.25 * 1.25` and `129/1.5 * 1.5` and `96/1.25 * 1.25` all return
    the stored count. A 1001 CSS px cell and a 128 px picture put the
    unsnapped centring at `(1001 - 128) / 2 = 436.5` CSS px, which at ratio
    1 is half a device pixel off grid. A 129 px picture at ratio 2 in a
    1001 px cell gives `(1001 - 64.5) / 2 * 2 = 936.5` device px. At ratio
    1.25 the same picture gives `561.125` device px, and at ratio 1.5 it
    gives `686.25`. Under Chromium's zoom for the device scale factor the
    layout unit is 1/64 of a device pixel, not of a CSS pixel. That is
    engine behaviour stated by the round 2 review, not something readable
    from this tree, and the design does not rest on its exact mechanism:
    the readback verifies the rendered result however the engine quantises
    the written value.
24. The cell, frame, stage and canvas nesting is
    `cell > frame > stage > canvas.view`, from the template at page.html
    lines 818 to 826 and the `dom` wiring at `ensureViewport` (lines 6558
    to 6571, read this session). `.cell` is the flex centring box (lines
    224 to 233, `align-items: center`, `justify-content: center`). `.frame`
    is `position: relative` (line 251), is the element the flex centres,
    and is the only element carrying the pan translate, written once at
    line 3605. `.stage` is `position: absolute, left: 50%, top: 50%`
    (line 252) inside the frame, positioned by negative half-picture
    margins written at lines 3608 to 3609, and carries the rotation and
    flips at line 3612. `.view` is at the stage origin with the picture CSS
    size, and its CSS rule already defaults `image-rendering: pixelated`
    (line 254), which the inline style at line 3601 can override to `auto`.
25. `drawGrey` (page.html line 3443) writes `vp.imageWidth`,
    `vp.imageHeight`, `vp.aspectX` and `vp.aspectY` from the grey (lines
    3444 to 3447) and calls `layoutViewport(vp)` (line 3464), so a drawn
    frame whose facts differ from the previous one produces a different
    stamp and a full layout. `showIndex` (line 2639) is the scroll and cine
    route through it. The live-redraw keep block (lines 4288 to 4296)
    restores scale, pan and mode after `drawGrey` for a same-frame window
    change.
26. The remote machinery in the landed page is fetch, validate and release
    only. `remoteFetchInstance` (page.html lines 1587 to 1596) ends at the
    fetched-and-validated sentence at 1592, `remoteRelease` (lines 1467 to
    1476) cancels and releases, `remoteEpoch` is line 1461, and the panel
    status is `#remote-state` (lines 753 and 1494). Nothing displays a
    remote frame. `grep -n "remoteFetchInstance\|remoteRelease\|remoteEpoch" 
    examples/folder-viewer/src/page.html` names those sites and no canvas.
27. The refusal surface that exists is per-viewport paint, not state.
    `paintOverlays` writes the bottom-right corner line at page.html line
    3694 and replaces the window line with the fixed string
    `"Window not applicable"` when the shown info is not drawable (line
    3690), derived at paint time from the frame's own facts. `showFault`
    (page.html line 3751, with `processBack` at lines 1866 to 1875) is
    different: it is for an undrawable image, hides
    the stage and the frame and writes the cell's fault paragraph, so it is
    not a surface for a control refusal that must leave the picture alone.
28. The output quantisation and the device-pixel backing store of the
    sibling surface, verified directly this session. `display_index` is
    `crates/ocelli-render/src/colour.rs` line 518 and rounds by adding
    `0.5` before its search. `display_byte` is
    `examples/folder-viewer/src/preview.rs` line 1531.
    `pub fn bind_cpu` is `crates/ocelli-wasm/src/session.rs` line 806 and
    pre-sizes to the device extent. `publishCpuCanvas` is
    `crates/ocelli-wasm/src/canvas.rs` line 718. `resize_cpu_canvas`
    computes `device_extent(css_width, css_height, dpr)` near canvas.rs
    line 694 and refuses when the bitmap and the extent disagree.
29. The reference's persistence surface, read this session.
    `Viewport.CameraViewPresentation` at
    `dist/esm/RenderingEngine/Viewport.js` lines 1211 to 1217 declares
    `rotation, pan, zoom, aspectRatio, displayArea` as capturable view
    presentation, and `getViewPresentation` is called at line 310. It is an
    explicit capture and restore that an application must drive. Nothing in
    the reference stores a zoom by itself across loads.

## What the specification does not cover

The HLD does not specify the viewer page's zoom model, its controls, its
label, its clamp, its link semantics, its origin handling or its test
inventory. The More section and the backlog row state the intent and nothing
else. Every concrete decision below is therefore made by this plan and is
listed here rather than attributed to a specification that does not contain
it.

1. **The scale rule.** One stored pixel per device pixel is the display scale
   `1 / window.devicePixelRatio` CSS pixels per stored pixel. The mode
   applies when the drawn frame is square-pixel, that is
   `vp.aspectX === 1 && vp.aspectY === 1`, which is `SQUARE_ASPECT`
   (page.html line 1124). Device pixels per stored pixel are then exactly 1
   on both axes. On an anisotropic frame the mode's behaviour is open
   question 4, and both answers are designed: under the recommended answer
   the mode enters with the row axis at the identity and the columns
   resampled, under the other it refuses with a named visible reason. A
   displayed remote frame refuses by name in both answers (decision 15).
2. **The origin rule.** In the one-to-one mode the drawn frame's device-space
   origin snaps to the device grid. The picture is centred by flex in the
   cell through the frame, the stage's `left: 50%` plus negative half
   margins, and the frame's translate rides on top (fact 24). The snapping
   rule, stated exactly:
   - The snap operates on the canvas, `dom.canvas`, which is the element the
     acceptance rows assert. Revision 2 snapped the frame's rect while the
     rows asserted the transformed canvas box, and the stage's positioning
     chain can offset one from the other by about a layout unit at
     fractional ratios. Snapping the asserted element removes the question.
   - No drag rounding exists. Revision 2 also quantised drag pans to whole
     device pixels while the mode held. That second rounding is dropped as
     redundant: the snap rounds the total laid-out position each layout,
     whatever fractional value the pan carries, so it covers every pan
     including drags. The pan branch keeps its accumulate-per-move deltas
     in every mode, and its one mode write at page.html line 4430 keeps
     `one` when the mode is held instead of writing free.
   - At layout time in the mode, the page writes the frame, stage and canvas
     sizes and the stage transform first, then measures
     `dom.canvas.getBoundingClientRect()`, then subtracts the translate last
     written, which the page records in `vp.panAppliedX` and
     `vp.panAppliedY` (two page-internal floats written at the single
     translate write site, line 3605, whose write moves after the canvas
     size writes). The untransformed canvas position is the measured rect
     minus the applied translate.
   - The target device origin is
     `Math.round((untransformedX + vp.panX) * dpr)` for x and the same for
     y. Rounding rule: nearest integer device pixel, ties toward positive
     infinity, which is `Math.round`. The written translate is
     `target / dpr - untransformed`, so the rendered device origin is the
     target up to engine quantisation, and the recorded `panApplied` is the
     value written.
   - The picture's own extent is an integer number of device pixels in this
     mode on a square-pixel frame, because `pictureW * dpr` equals
     `imageWidth` and `pictureH * dpr` equals `imageHeight` up to one ulp
     of the double product. The ulp is real: 448 of the 3001 ratios from
     1.000 to 4.000 make `128 * (1/r) * r` one ulp below 128 in double
     (`node`, this session), so no row asserts equality without the AC1
     half-device-pixel epsilon. With an origin on the grid all four corners
     of the content are on the grid, including under the 90 degree
     rotations and flips, where the content's corners are the box's
     corners. Under open question 4's recommended answer on an anisotropic
     frame the row axis is exact and the column axis is resampled by
     design, so only the row grid is asserted there.
   - The epsilon is derived from the engine's layout unit. Under Chromium's
     zoom for the device scale factor the layout unit is 1/64 of a device
     pixel (fact 23, stated by the review). A correct snap can be rendered
     up to half a unit off the written target, so AC2 asserts a distance of
     at most 1/128 device pixel from the integer grid, half the unit, on
     both axes. Revision 2's 0.001 device pixel epsilon was tighter than
     the grid the engine renders on and is withdrawn.
   - The measured-position variant is chosen over computing the centring
     from the cell rect because flexbox and the stage's margins quantise
     through the same unit grid and the ideal `(cellW - frameW) / 2` can
     differ from the laid-out position. One extra `getBoundingClientRect`
     per mode-one layout matches the file's existing habit (`fitScale`
     reads the cell rect on every layout, line 3569).
3. **The mode state.** `vp.fitMode` becomes a tri-state `vp.zoomMode` of
   `"fit"`, `"one"` and `"free"`. Two booleans with a never-both invariant
   would leave an impossible state representable at every read site.
4. **The label.** In the one-to-one mode on a square-pixel frame the corner
   line reads `Zoom 1:1`. Under open question 4's recommended answer on an
   anisotropic frame it reads `Zoom 1:1 rows`, naming the exact axis.
   Outside the mode, the existing fit-relative percent is unchanged. The
   percent is not redefined to be device-relative, because that would
   silently change the meaning the viewer's tips and README document today.
5. **The clamp.** `zoomAt`'s range extends to include the frame's own
   identity scale, by widening the floor to
   `min(fitted * 0.05, identityScale)` and the ceiling to
   `max(fitted * 40, identityScale)`, where `identityScale` is
   `1 / ratio` on a square-pixel frame and, under open question 4's
   recommended answer, `(1 / ratio) / vp.aspectY` on an anisotropic one. The
   identity must be reachable by the same gestures that reach every other
   scale.
6. **Entering the mode** keeps the image centre as `zoomCentre` does, by
   reusing `zoomAt`'s pan arithmetic at the cell centre with the factor
   `identityScale / vp.scale`, then setting the mode and laying out, which
   snaps the origin. `zoomAt`'s early return when the scale cannot move
   makes a repeated press idempotent. Pan keeps the mode, because the
   one-to-one scale does not depend on the cell size, unlike fit, whose next
   layout would discard the pan. That is the one behavioural difference
   from fit and it is deliberate.
7. **Refusals.** The control refuses, preserving the last usable mode, scale
   and layout. Three cases are silent no-ops with no fault and no layout
   write: no drawn image, hidden stage, tile mode. The named refusals
   return with a visible reason in the corner line, where the page already
   prints reasons such as `Window not applicable` (line 3690). They are,
   counted: a stored dimension smaller than the device pixel ratio on
   either axis, recommended text `1:1 needs a frame at least the device
   pixel ratio big`, which is where `Math.max(1, ...)` (lines 3599 to 3600)
   would floor the picture at 1 CSS px and destroy the identity, and a
   displayed remote frame, recommended text `1:1 is not available on the
   remote frame` (decision 15). Under open question 4's refusing answer a
   third named refusal is added for a non-square-pixel frame, recommended
   text `1:1 needs square pixels`. Under the matching answer there is no
   such refusal and the label names the exact axis instead.
   **The reason's lifecycle.** The reason is held in `vp.zoomRefusal`, a
   page-internal string on the viewport record beside `zoomLabel`. It is set
   only by a named refusal in `oneToOneView`, and it is cleared at three
   named sites so it can never outlive its condition: `drawGrey` writes it
   empty on every frame change before the layout, `paintOverlays` writes it
   empty at any paint where the refusal predicate is false, re-checked from
   the same predicate the guard evaluated, and the remote activation site
   clears it in the held-mode drop of decision 15. `paintOverlays` shows the
   reason in place of the `Zoom ` suffix only while the predicate holds, so
   the corner shows the percent at the first paint after the condition
   clears and the string itself is empty from that paint on. AC13 and AC16
   assert the no-stale-reason case: after the condition clears, further
   paints never show the refusal text again. A refusal writes that one
   string and nothing else:
   no style, no mode, no scale and no layout call. That is what "returns
   before any write" means here, and AC13 asserts the layout state holds
   while the corner shows the reason. When corners are hidden the reason is
   not visible anywhere. That is the same property the page's existing
   condition line has, `Window not applicable` is equally invisible under
   hidden corners, the toggle is an explicit choice to hide labels, and the
   control's tip states the refusals on hover. Surfacing it elsewhere would
   need the status line, whose lifecycle belongs to reading and opening, or
   `showFault`, which hides the stage (fact 27), and both are a larger
   change than the refusal justifies.
8. **Series open and Reset return to fit**, as they do today, so the mode
   does not survive a series change. Scroll, cine and window changes keep it
   while the drawn frame stays suitable (decision 9). **Leaving through
   Fit** works through both of `fitView`'s triggers, the Fit button (line
   1639) and the `0` key (lines 6036 to 6038), because `fitView` sets the
   mode.
9. **Linked zoom and unsuitable frames.** The source's mode propagates, as
   fit propagates today (open question 3). For each linked viewport the
   guards run against that viewport's own frame, cell and ratio. A suitable
   viewport enters the mode over its own image with its own identity scale
   and snap. Which frames are unsuitable follows open question 4's answer:
   under answer (a) a stored frame smaller than the device pixel ratio and
   a displayed remote frame, under answer (b) also a non-square-pixel
   frame. An unsuitable one keeps its current mode and scale, as
   `copyZoom` skips unsuitable viewports today (lines 7004 to 7012). A held
   mode that meets an unsuitable frame through scroll or cine drops before
   any style write: the guards re-run in the one branch of `layoutViewport`
   whenever the drawn frame's facts change, which the stamp forces because
   `drawGrey` writes the size and aspect facts and calls the layout (fact
   25), and a failed guard sets the mode to fit with `fitScale` and zero pan
   before the style writes, so no style write ever happens at a refused
   identity and the label honestly reads the fit percent. No reason line is
   shown for the drop, because nothing claims 1:1 any more, and
   re-entering is one key press.
10. **The canvas backing store stays the stored frame size in every mode.**
    The browser maps it onto the device grid through the CSS size. A
    device-sized backing store with `drawImage` was rejected: it would
    break the standing grey-length contract, double the buffer, and move
    the paint path that F-202 is redesigning.
11. **The pointer mappers are unchanged.** `clientToImage` and
    `imageToClient` keep their formulas (lines 4645 to 4674). Their model,
    image centre at cell centre plus pan, differs from the snapped truth by
    at most half a device pixel, which is below the precision any mark or
    readout consumes, and keeping them avoids a second copy of the mapping.
12. **A device pixel ratio change re-lays out.** The page arms a
    `matchMedia("(resolution:Xdppx)")` listener for the current ratio on
    start and re-arms it inside its own change handler, and the handler
    calls the existing `relayout` (page.html lines 1786 to 1793, the
    `layoutDirty` write at 1789), which
    sets `layoutDirty` and bypasses the stamp. The mode recomputes the
    scale from the live ratio on every layout, so the listener is the only
    new trigger the mode needs. AC5 counts invocations of the watcher's own
    handler, through the wrapped registration of Scope, so the row proves
    the watcher ran and does not depend on the resize listener.
13. **The core route is out of scope.** The `@ocelli/core` camera still
    accepts only an explicit zoom factor. This is open question 2.
14. **The mode is not stored.** `vp.zoomMode` is per-view session state like
    the rest of the zoom model. The pinned reference stores no zoom by
    itself either: its view presentation is an explicit capture and restore
    an application must call (fact 29), and the reference viewer does none
    on load. F-205's draft persists the mouse profile, the cine rate and
    the side columns and names no zoom (fact 21). If persistence is ever
    wanted it is F-205's explicit-capture shape, decided there. This closes
    revision 2's open question 6.
15. **A displayed remote frame refuses the mode by name, and a held mode is
    dropped when a remote frame is displayed.** The lead ruling after
    design round 2, adopted, now extended by the round 3 D2 ruling. The
    remote frame is drawn into its own transferred canvas inside the active
    cell's stage with a worker-owned device-pixel backing store (fact 19),
    so the identity this story promises is not expressible by rewriting
    that canvas's CSS size, which would fight the worker's own sizing on
    every publish, and the page cannot assert the identity over a raster it
    does not size. The control refuses with the visible reason `1:1 is not
    available on the remote frame` and changes nothing, until a later story
    routes the remote frame through the core factor. **The page field the
    guard reads is `vp.remoteDisplayed`**, a boolean on the viewport record
    beside `zoomMode`. It is set true at the remote activation site, the
    page function that shows the transferred canvas in the cell's stage and
    hides the local-derived overlays (F-202's Scope item 6 and R4 owners,
    fact 19), and cleared in `drawGrey`, the local draw path. F-202's
    visible plan does not name the field or that function, so this plan
    names them and implementation adapts to the landed names, reporting any
    contradiction per implement-feature. **The held-mode drop.** While the
    remote source is active in a cell, `drawGrey` and `layoutViewport` do
    not run there (fact 19), so the guards would never re-run and a mode
    held on the previous local image would survive. The remote activation
    site therefore also drops the mode, as its one named extra duty beside
    the canvas show: it sets that viewport's `zoomMode` to `fit`, sets
    `vp.remoteDisplayed` true, clears `vp.zoomRefusal`, sets `vp.scale` to
    `fitScale(vp)` with zero pan, and rewrites
    `vp.zoomLabel` from the fit percent computed from `fitScale` at that
    moment, the same expression the layout writes. The drop is record
    writes only, with no `layoutViewport` call and no style write, because
    the local picture is not displayed, and the next local layout
    recomputes the same fit scale from the record's mode. Nothing can read
    a stale
    `Zoom 1:1` from the record after it. Before F-202 lands the page
    displays no remote frame at all (fact 26), and the adopted landing
    order below has F-202 land first, so the guard, the field and the drop
    are written and tested against the tree that has the state they read.

## Problem and reproducer

At this worktree's HEAD `ac962c64`, run `npm run dev` (the F-245 launcher) or
`cargo run -p ocelli-folder-viewer`, open the built-in synthetic study, and
try to display it with one stored pixel on one device pixel. There is no
control, no key and no scale value that targets the device grid (fact 3). The
zoom label is fit-relative, so no displayed value names the device identity
either (fact 4). On a ratio 2 display, zooming until one stored pixel covers
one CSS pixel still shows two device pixels per stored pixel. For a small
image in a large cell the clamp of fact 5 makes the scale unreachable by any
gesture at all. And there is a second, silent failure the mode must prevent:
even with the right scale, the flex centring of the frame in the cell plus a
fractional pan leaves the image's top-left corner between device pixels. A
1001 CSS px cell and a 128 px picture at ratio 1 put the origin at 436.5
device px (fact 23). The picture is then resampled onto the grid: with a
fixture whose neighbouring pixels all differ, the device readback shows each
stored column doubled or shifted, which is the failure the story exists to
prevent and which revision 1's tests could not see.

This is a story because the tracked More section names the gap as not yet in
the viewer, the matrix row schedules it to S47 as F-204, and the S47 wave row
of more.html line 214 names one-to-one display among the sprint's outcomes.

## Scope, API and compatibility

In scope, all inside the canonical page, its own test and the two shared
test files the remote row needs:

- A tri-state `vp.zoomMode` replacing `vp.fitMode` at all 13 census sites
  (fact 7), with the three transitions fit, one and free, plus
  `vp.panAppliedX`, `vp.panAppliedY`, `vp.zoomRefusal` and
  `vp.remoteDisplayed`.
- A `oneToOneView(vp)` function: the refusal guards of decisions 7 and 15,
  the frame's identity scale from decision 5, `zoomAt`'s pan arithmetic at
  the cell centre, the mode set, `layoutViewport(vp)` and `copyZoom(vp)`.
- The `layoutViewport` changes: the stamp carries the mode and the pan, and
  never the applied translate, which is an output of the layout and not an
  input to its comparison, the one branch begins with the guards and drops
  to fit on a failure before any style write, recomputes the scale from the
  live ratio on every layout, sets `image-rendering: pixelated` explicitly
  instead of trusting the comparison at line 3601, moves the translate
  write after the size writes, and snaps the origin per decision 2. The
  corner label reads `Zoom 1:1`, or `Zoom 1:1 rows` under open question 4's
  recommended answer, and the reason line replaces it per decision 7.
- `drawGrey` clears `vp.zoomRefusal` on every frame change and clears
  `vp.remoteDisplayed` before its layout call (decision 15).
- The remote activation site gains the one mode-drop statement of decision
  15, beside F-202's canvas-show statement in the remote block of
  `page.html`. That single statement is this story's only edit to F-202's
  block, and it is serialised by the adopted order.
- The `zoomAt` clamp extension of decision 5, and its mode set to free.
- The pan rule: pan keeps the mode (decision 6). No rounding is added to
  the pan branch.
- The `fitView` mode write (page.html line 3843) joins the replacement list,
  so Fit and `0` leave the mode.
- The toolbar button beside Fit (`page.html` line 797 region), the `\` key
  branch in `onKey` beside the fit branch, the `SENTENCE` tip entry, the
  KEYS panel row, and the `copyZoom` propagation of decision 9.
- The `matchMedia` ratio watcher of decision 12.
- The new browser suite `examples/folder-viewer/tests/browser/one-to-one.mjs`
  and its registration in the `viewer-browser` gate list.
- The remote harness: `startSyntheticDicomweb` moves unchanged out of
  `dicomweb.mjs` into a new `examples/folder-viewer/tests/browser/dicomweb-upstream.mjs`
  that both `dicomweb.mjs` and the new suite import. `dicomweb.mjs` keeps
  every assertion it owns today. If F-202's landed tree already exports a
  reusable display driver from its own suite, that is imported instead and
  the extraction is dropped.

Out of scope, explicitly:

- No Rust crate, no `crates/` file, no `packages/` file, no wasm export, no
  wire format change and no dependency change. `Cargo.lock` is untouched.
  The `@ocelli/core` camera route keeps taking an explicit factor.
- No tile-mode one-to-one. Tiles stay fitted.
- No snapping of gestures onto the identity, and no persistence of the mode
  across sessions, which is decided out by decision 14.
- No change to the grey production path, the preview route, or the local
  process. `requestGrey` (line 3219) and `postPreview` (line 1978) and
  `paintGrey` (line 3414) are untouched.

Compatibility: `fitMode` is page-internal and appears in no other file
(fact 7), so the tri-state rename has no external consumer. The layout stamp
is page-internal. The grey reply contract and the canvas backing store size
are unchanged, so every existing suite keeps its meaning. The fit path's
observable behaviour is unchanged, which AC15 pins.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context and policy | Observable output |
|------------------------|-------------------|---------------------------|-------------------|
| Toolbar button and the `\` key | `oneToOneView(vp)` in page.html | Active viewport, single view, a drawn suitable local frame at least the ratio big | Canvas CSS size lands the stored frame on the device grid, origin on the device grid, corner reads `Zoom 1:1` or `Zoom 1:1 rows` |
| Wheel, Ctrl wheel, pinch, zoom drag, `=` and `-` | `zoomAt(vp, x, y, factor)` | Clamp range extended to include the frame's identity scale, mode set free | Percent label returns, extent leaves the identity |
| Pan drag in the mode | the pan branch of `onPointerMove` | Pan keeps the mode, the layout re-snaps the origin | Label stays, extent unchanged, image moved by the drag with the origin on the grid |
| Pan drag outside the mode | the same branch | Existing behaviour, mode to free | Unchanged from today |
| Fit button and `0` key | `fitView(vp)` (page.html line 3843) | Mode set to fit as today | Percent label, fitted extent, mode left |
| Linked viewports | `copyZoom(source)` | Source mode propagates, guards per target (decision 9, open question 3) | Each suitable linked viewport enters the mode over its own image and cell, an unsuitable one keeps its own mode |
| Scroll and cine in the mode | `showIndex` through `drawGrey` and `layoutViewport` | Guards re-run when the drawn frame's facts change (decision 9) | A suitable next frame keeps the mode, an unsuitable one drops to fit before any style write |
| Series open and Reset | `loadSeries` (line 2518), `resetView` (line 3858) | Mode returns to fit as today | Percent label, fitted extent |
| A displayed remote frame | the refusal guard and the activation-site drop of decision 15 | Named visible reason on the control, the held mode dropped at the activation point, nothing written to the remote canvas | The corner never reads `Zoom 1:1` over the remote raster, and the label reads the fit percent after the local source returns |
| `clientToImage`, `imageToClient`, marks, ink, magnifier, scale bar, readout | existing consumers of `vp.scale` | Unchanged code, all derive from the scale, residual error under half a device pixel (decision 11) | Positions stay correct in the mode with no per-consumer change |
| `devicePixelRatio` change | the `matchMedia` watcher of decision 12 | Page re-arms per change, calls `relayout` | Extent identity holds again with no test-fired layout |
| More matrix and One to one section | the More page edit | States the viewer workflow, which source it covers, the refusal reasons and where the factor is computed | Row reads Implemented with truthful cells |

The page owns the CSS-to-device mapping, the ratio read and the origin snap.
The local process owns the grey values and the spacing facts. Units are CSS
pixels per stored pixel for `vp.scale`, device pixels only inside the one
branch of `layoutViewport`, and millimetres in `paintScale` as today.
Rounding enters at two named places: the origin snap's nearest-integer
round, and the browser's own layout unit and device raster, which the
readback observes. No rounding is added to the pan.

## State transitions and invariants

| Entry point and starting state | Input or event | Resulting state and output | Refusal and preserved state |
|--------------------------------|----------------|----------------------------|-----------------------------|
| Fit or free, single view, drawn suitable local frame at least the ratio big | One-to-one button or `\` | Mode `one`, identity scale, image centred as `zoomCentre` does, origin snapped, label per decision 4 | No image, hidden stage or tile mode is a silent no-op with no layout write |
| Fit or free, drawn anisotropic local frame | One-to-one button or `\` | Under open question 4 answer (b), named reason in the corner line, no mode or layout change. Under answer (a), the mode enters with the row axis exact and the label `Zoom 1:1 rows` | Under (b) the last usable mode and scale are preserved |
| Fit or free, stored dimension below the ratio | One-to-one button or `\` | Named reason in the corner line, no mode or layout change | The last usable mode and scale are preserved |
| Any state, remote source active and displayed | One-to-one button or `\` | Named reason `1:1 is not available on the remote frame`, nothing written | The remote canvas and its CSS size are untouched |
| Mode `one` on a local image | A remote frame displayed in the same cell | The activation site drops the viewport to fit, clears the refusal and rewrites the label from the fit percent, per decision 15. The corner is hidden while the remote source is active per F-202's R4 | No layout call and no style write at the drop. After the local source returns, `drawGrey` clears `vp.remoteDisplayed` and the corner reads the fit percent |
| Any mode with a held refusal reason | A frame change, or any paint where the refusal predicate is false | `vp.zoomRefusal` is written empty and the corner shows the percent | No stale reason at any later paint |
| Mode `one` | Any DPR change, including browser zoom and monitor change | The `matchMedia` watcher fires, `relayout` runs, the scale and origin recompute from the live ratio, extent identity holds again | A ratio of 0 or absent falls back to 1 through the existing `\|\| 1` pattern |
| Mode `one` | Pan drag | Mode stays `one`, pan unquantised, origin re-snapped each layout, label unchanged | None, pan is always valid with a drawn image |
| Mode `one` | Wheel, zoom drag, `=` or `-` | Mode `free`, scale clamped into the extended range, percent label | A clamp that cannot move the scale leaves it unchanged as today |
| Mode `one` | Fit button or `0` key | Mode `fit` via `fitView`, fitted scale, percent label | None, this is the documented return |
| Mode `one` | Scroll or cine to a frame whose facts still pass the guards | Mode and pan kept through `drawGrey` and the layout, which re-snaps the origin | A refused window change leaves the mode as the live block does today |
| Mode `one` | Scroll or cine to a frame that fails a guard | The one branch drops the viewport to fit before any style write, percent label | No style write at a refused identity |
| Mode `one` | Series open or Reset | Mode `fit`, fitted scale, percent label | None, this is the documented return |
| Mode `one`, linked zoom on | The outlined viewport's mode change | Each linked viewport takes the mode over its own image and cell if its own guards pass, per open question 3. Under answer (a) an anisotropic target enters with the `Zoom 1:1 rows` label | A viewport with no image, a hidden stage, a tile mode, a stored frame below the device pixel ratio or a displayed remote frame is skipped, as `copyZoom` skips unsuitable viewports today, and under answer (b) a non-square-pixel frame is skipped too |
| Any mode | One-to-one pressed in tile mode | No-op, tiles stay fitted | No fault, no layout write, selection unchanged |

Invariants spanning consumers: the canvas backing store holds exactly the
stored frame's pixel dimensions in every mode, and the display scale is the
only stored-to-device mapping. In the one-to-one mode on a square-pixel
frame the rendered device origin is an integer device pixel and the rendered
device extent is exactly the stored extent, on both axes. Under open
question 4's recommended answer the same holds on the row axis and the
column axis is resampled by design with the label saying so. Every scale
consumer derives from `vp.scale`, so no consumer carries a second copy of
the rule.

## Ownership, commit and recovery

The work is stateless DOM and style writes inside the page, so the table
covers the one cached output, the layout.

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate behaviour |
|---------------------------|-----------------------|-------------------|-------------------------|--------------------------------|
| The viewport's layout, keyed by the stamp | `layoutViewport(vp)` | The stamp string over image size, aspect, mode, pan, rotation, flips and the ratio, page internal. The applied translate is an output and is in no stamp | The style writes and label assignment before `vp.layoutDirty = false` (page.html line 3625) | A guard return before any style write preserves the last usable layout. Repeating the button press with the mode already `one` recomputes the same scale, snap and translate, and rewrites the same styles, which is idempotent |
| `vp.zoomRefusal` | `oneToOneView`'s named refusals, cleared by `drawGrey` on frame change, by `paintOverlays` when the predicate is false and by the activation-site drop | Page-internal string beside `zoomLabel` | The next `paintOverlays` showing the reason, or the percent when cleared | Shown only while the refusal condition holds at paint time, written empty at the first paint or frame change after it clears, so no later paint shows a stale reason. A refused request writes this one string and no layout state |

Every input that determines validity is in the stamp, which the landed code
already compares before doing work (page.html lines 3582 to 3589). The mode
and the pan join it, so a pan drag in the mode
re-lays out instead of short-circuiting (fact 14). The applied translate
joins nothing, because it is produced by the layout it would otherwise
compare against. `relayout` sets
`layoutDirty` on resize so the stamp cannot hide a cell change (fact 15).
A refused request returns before any style write, so the last usable mode
and configuration are never replaced. The valid draw, refused request, valid
retry sequence is AC12, AC13 and AC16. There is no external resource, no
worker, no device and no browser setter in this story, so no other side
effect exists before the commit point.

## Numbered acceptance inventory

All rows are observed through the new browser suite
`examples/folder-viewer/tests/browser/one-to-one.mjs`, run by
`node examples/folder-viewer/tests/browser/one-to-one.mjs` and registered in
the `viewer-browser` gate list in `bin/ocelli.sh` immediately before
`launcher.mjs`, which stays last per the arm's own comment. The suite is
launched through the pinned Playwright with Chromium's
`--force-color-profile=srgb` argument, and AC3, AC14 and AC19 depend on it:
the decoded bytes of a canvas screenshot are transformed by Chromium to the
host's device colour profile, so an unpinned profile would shift the exact
byte rows on a display whose profile is not sRGB, and the pin makes the
exact-byte rows read the same values on every host. The suite writes its own
square-pixel synthetic DICOM files into a temporary folder, starts
the release viewer with `OCELLI_PORT=0`, and stops it by PID, in the pattern
of `window.mjs` lines 63 to 76. Five fixture files, all 8-bit explicit VR
little endian MONOCHROME2, no rescale, Window Center 128, Window Width 256,
VOI LUT Function LINEAR, written like `window.mjs`'s MR file so the
non-CT window list has the file window as its one row:

- Fixture P, 128 by 96, no Pixel Spacing. Stored value at `(x, y)` is
  `(x * 29 + y * 57) mod 251`, except the stored value at `(127, 95)` is
  255. The exception exists so the linear branch is exercised at the top
  boundary, since the pure pattern's maximum is 250 (computed this session).
  The override keeps every neighbour distinct: its only in-picture
  neighbours are 33 and 5 (computed this session). Corner values,
  hand-computed and re-derived this session from the formula: `(0, 0)` is 0,
  `(127, 0)` is 169, `(0, 95)` is 144, `(127, 95)` is 255.
- Fixture B, 129 by 65, no Pixel Spacing, same value rule with no override.
  Odd extents, so its CSS box is fractional at the identity scale of every
  ratio used below, 64.5 by 32.5 CSS px at ratio 2, 103.2 by 52 at 1.25 and
  86 by 43 at 1.5, while its device box at the identity is integral, 129 by
  65 device px, and an integral cell generally gives a fractional unsnapped
  origin. The generality has exceptions, measured this session: at ratio
  1.25 a cell of 1004 CSS px puts the unsnapped origin at exactly 563
  device px, so each origin case asserts its own precondition from the
  measured cell rather than trusting the fixture alone. Corner values:
  `(0, 0)` is 0, `(128, 0)` is 198, `(0, 64)` is 134, `(128, 64)` is 81.
- Fixture C, 64 by 64, no Pixel Spacing, same value rule. It exists for
  AC6, where a small frame in a large cell is what makes the unmodified
  clamp floor bind. Corner values: `(0, 0)` is 0, `(63, 0)` is 70,
  `(0, 63)` is 77, `(63, 63)` is 147.
- Fixture D, 128 by 96, Pixel Spacing `0.7\0.5`, same value rule with no
  override. It exists for the open question 4 recommended-answer rows, and
  matches the built-in synthetic's spacing so the expected column count is
  `128 * 0.5 / 0.7 = 91.43`.
- Fixture E, 2048 by 8, no Pixel Spacing, same value rule. It exists for
  AC20, where a frame wider than its cell makes the correct clamp floor sit
  far below the identity. Its rows are longer than the pattern's 251-value
  cycle, so values repeat along a row of this fixture, which AC20's
  extent-only assertion does not consume. Adjacent neighbours still differ
  everywhere, the property the byte rows need (computed this session).

The value rule makes the stored value distinct along each row and along
each column of every fixture whose longest side is at most 251, verified
this session for P, B, C and D, and it does not make the values distinct
across a fixture, which no row consumes. The expected display byte at every
device pixel is the stored value, by
PS3.3 C.11.2.1.2.1 and its Note 4 with the output quantisation of
`display_index` as transcribed above. The suite may read the page's grey
reply only to assert it equals the stored bytes, which guards the window
pipeline, never as the expected table. Each row names its case. Evidence
stages are `integrated browser` for the suite rows and `integrated` for the
gate row. AC1 is the coarse guard at half a device pixel. AC3's byte-exact
readback is the real proof of the identity.

| ID | Trigger and expected behaviour | Test and observation | Wrong implementation rejected | Evidence stage and owner |
|----|--------------------------------|----------------------|-------------------------------|--------------------------|
| AC1 | Enter the mode on fixture P. `abs(rect.width * devicePixelRatio - expected)` and the height form are at most 0.5 device px, where expected is `canvas.width` unturned and `canvas.height` under a 90 degree rotation, and the rect is `dom.canvas.getBoundingClientRect()`, the transformed canvas's own box. Cases at `deviceScaleFactor` 1, 1.25, 1.5 and 2, and a rotated case at ratio 2 where the swap is observable because the fixture is 128 by 96 | Cases `f-204-extent-dpr1`, `f-204-extent-dpr125`, `f-204-extent-dpr15`, `f-204-extent-dpr2`, `f-204-extent-rot` | Entering CSS scale 1 and ignoring the ratio, which shows two device pixels per stored pixel at ratio 2, or measuring the unrotated axes under rotation | integrated browser, this story |
| AC2 | In the mode, `abs(rect.left * devicePixelRatio - Math.round(rect.left * devicePixelRatio))` and the top form are at most 1/128 device px, half the 1/64 device px layout unit of fact 23, at `deviceScaleFactor` 1 with a cell whose width minus 128 is odd, at ratio 2 on fixture B, and at 1.25 and 1.5 on fixture B. Each case first asserts its own precondition, that the unsnapped centring would be fractional, computed from the measured cell | Cases `f-204-origin-dpr1-odd`, `f-204-origin-dpr2-odd`, `f-204-origin-dpr125`, `f-204-origin-dpr15` | Scaling without snapping the origin: the centring offset of the frame in the cell plus an unquantised pan lands between device pixels, at 436.5, 936.5, 561.125 or 686.25 device px per fact 23 | integrated browser, this story |
| AC3 | In the mode, hide corners and scale bar with the `toggle-corners` click, ensure no marks, no selection and no pointer over the cell, then capture an enclosing region with `page.screenshot({ scale: "device", clip })` whose CSS bounds are integers on the ratio's device-aligned grid, the smallest integer step whose product with the ratio is integral, which is 1 at ratios 1 and 2, 4 at 1.25 and 2 at 1.5, so the crop offset in the decoded PNG is an exact integer device count. Decode the PNG in-page through `Image` and a scratch canvas with no new dependency, crop by that integer device offset, and every device pixel equals the stored pattern byte at the same index. Runs on fixture P at ratio 1 in the odd cell, on fixture B at ratio 2, on both fixtures at 1.25 and at 1.5, and again on fixture B at ratio 2 after a pan drag of 40 device px, re-clipped from the moved rect | Cases `f-204-pixels-dpr1-odd`, `f-204-pixels-dpr2-odd`, `f-204-pixels-dpr125-p`, `f-204-pixels-dpr125-b`, `f-204-pixels-dpr15-p`, `f-204-pixels-dpr15-b`, `f-204-pixels-midpan` | A half-pixel origin, which nearest-samples the fixture into doubled or shifted columns and rows and mismatches the pattern, a truncated layout unit that drops or doubles one device column at the fractional ratios, or any compositor resampling | integrated browser, this story |
| AC4 | The corner line reads `Zoom 1:1` in the mode on a square-pixel frame, and one zoom drag tick returns the fit-relative percent | Case `f-204-label`, reading the corner text before and after | A stale mode flag that keeps the 1:1 label after the scale moved | integrated browser, this story |
| AC5 | With the mode held, change `deviceScaleFactor` from 1 to 2 and back through a CDP session's `Emulation.setDeviceMetricsOverride` with the CSS viewport held constant, touch nothing else, and the page re-lays out through its own `matchMedia` watcher: poll until the extent identity and origin integrality of AC1 and AC2 hold again, with the test never calling `layoutViewport` or `relayout`. An init script installed before the page load wraps `MediaQueryList.prototype.addEventListener`, so the page's resolution listener is registered through the wrapper and every call of that handler is counted in a page array. After each ratio change the count has increased by at least one, so the row proves the watcher's own handler ran and does not depend on the resize listener at all | Case `f-204-dpr-change` | A scale or ratio cached at entry, which leaves twice the identity after the change, or a test that fires the layout itself and credits the watcher with it | integrated browser, this story |
| AC6 | Fixture C, 64 by 64 square pixels, in a single 1 by 1 layout at ratio 2. The suite measures the cell, computes `fitScale` from it as page.html lines 3568 to 3577 do, and asserts the precondition `fitted * 0.05 > 1 / ratio`, which holds at 0.78125 above 0.5 in a 1600 by 1000 cell (fact 5), so the unmodified floor sits above the identity. Enter the mode, one zoom-in tick to leave it, then `-` key ticks until the scale stops changing: the settled extent is the identity, `rect.width * devicePixelRatio` within the AC1 epsilon of 64, because the extended floor `min(fitted * 0.05, 1 / ratio)` is exactly the identity 0.5 in this cell. One further zoom-out tick by `-` and one by a wheel tick each hold the extent at the identity, and the percent label follows the small scale | Case `f-204-clamp`, the floor and target computed from the measured cell in the suite | Leaving `fitted * 0.05` as the floor: the ticks settle at 0.78125, the extent reads 100 device px instead of 64, and the identity is unreachable by any gesture for a small frame | integrated browser, this story |
| AC7a | Applies only under open question 4 answer (a). With link-zoom on and the outlined viewport in the mode over a suitable frame, a linked viewport whose own guards pass enters the mode over its own image and cell, with fixture D as one target so the anisotropic case is covered and its corner reads `Zoom 1:1 rows`, and an unsuitable linked viewport, a stored frame smaller than the device pixel ratio or a remote-displayed cell, does not enter and keeps its own mode and percent label | Case `f-204-link-a`, per open question 3 and decision 9, with one suitable target, one below-ratio target and, on the post-F-202 tree, one remote-displayed target | Propagating the source's fit-relative ratio as a free scale, which is not the mode, or propagating without re-running the target's own guards, which would pull a below-ratio frame or a remote cell into the mode | integrated browser, this story, pending open questions 3 and 4 |
| AC7b | Applies only under open question 4 answer (b). With link-zoom on and the outlined viewport in the mode over a suitable square-pixel frame, a suitable linked viewport enters the mode over its own image and cell, and the anisotropic linked viewport does not enter and keeps its own mode and percent label | Case `f-204-link-b`, per open question 3 and decision 9, with one suitable and one anisotropic linked viewport | Propagating the source's fit-relative ratio as a free scale, or propagating without re-running the target's own guards, which would pull the anisotropic frame into the mode while the control refuses it standalone | integrated browser, this story, pending open questions 3 and 4 |
| AC8 | Opening another series returns the viewport to fit and the percent label | Case `f-204-series-open` | Keeping mode `one` across a series open, diverging from the landed open-at-fit behaviour | integrated browser, this story |
| AC9 | `canvas.width` and `canvas.height` stay at the stored frame size in the mode, and the in-page grey equals the stored bytes at every pixel, a full-frame form of the grey-length comparison `canvasMatchesReply` makes, asserted inside this suite | Case `f-204-backing-store`, run by the `viewer-browser` gate | Resizing the backing store to the device extent, which breaks the grey-length check and doubles the buffer | integrated browser, this story. Revision 1 leaned on `canvasMatchesReply`, which runs in no gate (fact 16) |
| AC10 | At the identity, `getComputedStyle(canvas).imageRendering` is `pixelated`, at `deviceScaleFactor` 1.38, 1.45, 1.54 and 1.57. Each case first asserts its precondition in the page, `(1 / devicePixelRatio) * devicePixelRatio < 1`, so the case is trusted only where the landed comparison provably fails (fact 23) | Case `f-204-pixelated`, the four probes set through the same CDP override as AC5 | Trusting the comparison at page.html line 3601, which evaluates one ulp below 1 at the identity at these ratios and selects `auto`, silently resampling the exact identity | integrated browser, this story |
| AC11 | A pan drag in the mode keeps the mode, the label and the extent, and moves the image by the drag delta quantised to whole device pixels by the origin snap: with a whole-device-px drag of 40 device px at ratio 2, `rect.left` after minus before equals the delta within the AC2 epsilon, and the origin stays on the grid | Case `f-204-pan-keeps`: the image must actually move, which the stamp carrying the pan allows (fact 14) | Copying the pan-clears-fit rule and dropping the mode on pan, or stamping the mode without the pan so the drag short-circuits at page.html line 3589 | integrated browser, this story |
| AC12 | The control in Four images tile mode, or with no image drawn, is a no-op with no fault and no layout write | Case `f-204-refusal` | A layout at scale 0 or a fault line, instead of the guard return | integrated browser, this story |
| AC13 | Applies only under open question 4 answer (b). On the built-in synthetic study, whose pixels are 0.7 by 0.5 mm (fact 9), the control shows the named refusal reason in the corner line in place of the zoom suffix and changes nothing: mode, scale, extent and the stored zoom label all hold their prior values. The no-stale-reason case: after a frame change or any other event that makes the predicate false, the corner shows the percent, and no later paint, including a pan and a window change, ever shows the refusal text again | Case `f-204-anisotropic` | Entering the mode on an anisotropic frame and resampling rows alternately onto 1 and 2 device rows while the corner claims 1:1, a silent no-op that shows no reason, or a reason string that outlives its condition | integrated browser, this story, pending open question 4 |
| AC14 | At the identity with the 90-degree rotate and each flip applied on fixture P at `deviceScaleFactor` 2, the clipped screenshot equals the permuted pattern, because every reachable rotation is an exact permutation | Case `f-204-transforms` | Resampling introduced at the identity under the CSS transform, which a blend shows at the pattern's edges | integrated browser, this story |
| AC15 | The existing aspect, window, readout, colour and marks suites pass unchanged on the final tree | The `viewer-browser` gate list, run whole | A tri-state refactor that changes fit layout, the stamp semantics or the aspect contract | integrated, the gate |
| AC16 | With F-202 landed first per the adopted order and a remote frame displayed through the worker canvas, the One-to-one control refuses with the named reason `1:1 is not available on the remote frame`, writes nothing, and the corner line never reads `Zoom 1:1` over the remote raster. The held-mode case: with mode `one` held on a local image in a cell, displaying a remote frame in that same cell drops the viewport to fit at the activation site per decision 15, and after the local source returns the corner reads the fit percent, never `Zoom 1:1`, with no refusal reason shown | Case `f-204-remote`, runnable only on the post-F-202 tree the adopted order guarantees. If this story must land first contrary to the order, the case is recorded blocked with that named dependency and runs when F-202's page lands. The harness is the synthetic upstream `dicomweb.mjs` defines, extracted unchanged into `tests/browser/dicomweb-upstream.mjs` per Scope, serving one synthetic Part 10 instance that the page's DICOMweb panel displays through F-202's worker path | Entering the mode over the remote canvas and rewriting its CSS size, which fights the worker-owned device-pixel backing store pre-sized at `bind_cpu` (session.rs 806 to 824) and published by `publishCpuCanvas` (canvas.rs 718), leaving the corner claiming 1:1 over a raster the page does not size, or leaving a mode held before the display claiming 1:1 after the remote frame is shown | integrated browser, this story after F-202 |
| AC17 | Applies only under open question 4 answer (a). On fixture D, Pixel Spacing `0.7\0.5`, the mode enters at ratio 2: `rect.height * devicePixelRatio` is within the AC1 epsilon of 96, `rect.width * devicePixelRatio` is within the AC1 epsilon of 91.43, and the corner line reads `Zoom 1:1 rows` | Case `f-204-aniso-enter` | Refusing anisotropic frames while the reference enters them, or entering with both axes stretched, which draws rows at 1.4 device rows | integrated browser, this story, pending open question 4 |
| AC18 | Applies only under answer (a). On fixture D at ratio 2 in a cell that makes the unsnapped centring fractional, the row origin is on the device grid within the AC2 epsilon, and each stored row occupies exactly one device row | Case `f-204-aniso-origin` | Skipping the snap on the axis that is exact, which shifts rows off the grid | integrated browser, this story, pending open question 4 |
| AC19 | Applies only under answer (a). The device readback on fixture D at ratio 2: exactly 96 device rows, device row j equals stored row j of the suite's own file read as an order-preserving subsample of that row's bytes, because `image-rendering: pixelated` copies source pixels and never blends, and the drawn device column count is 91 or 92. The exact dropped-column pattern is not asserted, because CSS pixelated pins nearest sampling and not the sampling formula. The byte equality covers device columns 0 to 90, the fully covered columns: the drawn box is 91.43 device px wide, so column 91 is only 0.43 covered and its sampled source column is the sampling formula's choice, the same thing this row declines to pin, so that one partially covered right-edge device column is **excluded** from the byte assertion rather than tolerated in it. The row-count and row-order assertions run over every row in the included columns | Case `f-204-aniso-pixels` | Blending or row shifting on the exact axis, a row count that is not the stored row count, or an asserted byte in the partial column, which would fail a correct implementation whose sampling formula picks either neighbour there | integrated browser, this story, pending open question 4 |
| AC20 | Fixture E, 2048 by 8 square pixels, in a single 1 by 1 layout at ratio 2. The suite measures the cell, computes `fitScale` from it as page.html lines 3568 to 3577 do, and asserts the precondition `fitted * 0.05 < 1 / ratio`, which holds at about 0.0244 below 0.5 in a 1000 px cell, so the correct floor is `fitted * 0.05` and sits far below the identity. Enter the mode, where the extent is the identity 2048 device px, then `-` key ticks until the scale stops changing: the first tick below the identity already reads an extent below 2048 device px, and the settled extent is `2048 * fitted * 0.05 * devicePixelRatio`, 100 device px in a 1000 px cell, computed in the suite from the measured cell | Case `f-204-clamp-large` | A clamp floor set to `identityScale` alone, which holds the extent at the identity 2048 device px and refuses to zoom out below 1:1 on a large frame. AC6 is the complementary small-frame case, and the pair pins the `min` | integrated browser, this story |

The pixel-identity fixture values are hand-computed in the suite and cited
in the Tests section. No AC depends on unimplemented external equipment.
The equipped cornerstone comparison and the release bench are not evidence
for these rows, because the comparison pins the ratio to 1 and the timed
subjects are covered by the sprint-end acquisition below.

## Approach

The smallest implementation in the landed page, in this order.

1. Write the new suite's fixtures and expectations first, per
   implement-feature step 2: the five fixture writers, the expected-value
   rule taken from PS3.3 C.11.2.1.2.1 and its Note 4 with the
   `display_index` quantisation, the origin preconditions computed from
   measured cells, the device-aligned enclosing region and in-page crop
   helper, the sRGB launch argument, and the clamp arithmetic of AC6 and
   AC20. Watch the suite fail with the control absent.
2. Replace `vp.fitMode` with `vp.zoomMode` at all 13 census sites of fact 7,
   including `fitView` at page.html line 3843, and add `vp.panAppliedX`,
   `vp.panAppliedY`, `vp.zoomRefusal` and `vp.remoteDisplayed`. The refusal
   string is written at the single translate write site and set and cleared
   per decision 7. The behaviour of the fit path
   is unchanged, which AC15 pins.
3. Add the one branch to `layoutViewport`: the guards first with the drop to
   fit on failure, the scale recomputed from the live ratio on every
   layout, `image-rendering: pixelated` set explicitly, the translate write
   moved after the size writes, the origin snapped per decision 2's rule,
   the label and the reason line set. `drawGrey` gains the two clears of
   decision 15 before its layout call. Add `oneToOneView(vp)` with the
   guards, the pan-preserving entry and the mode set.
4. Extend the `zoomAt` clamp. The pan branch's only change is its mode
   write at page.html line 4430, which keeps `one` when the mode is held
   and writes `free` otherwise, with no rounding added to the deltas.
5. Add the `matchMedia` ratio watcher of decision 12 beside the existing
   listeners at page.html lines 1779 to 1781.
6. Add the button, the icon symbol, the `\` branch in `onKey` beside the fit
   branch, the `SENTENCE` tip, the KEYS row, and the refusal reason text.
7. Wire `copyZoom` per decision 9 and open question 3's answer, and add the
   activation-site drop statement of decision 15 to the remote block.
8. Extract `startSyntheticDicomweb` into `tests/browser/dicomweb-upstream.mjs`,
   import it from `dicomweb.mjs`, and drive AC16's remote case through it.
9. Update `more.html` and `README.md`, and register the suite in
   `bin/ocelli.sh` before `launcher.mjs` and in `docs/lld/build-targets.md`.

Rejected alternatives, with reasons. A device-sized backing store with
`drawImage` breaks the grey-length contract and duplicates the paint path
F-202 owns. A `@ocelli/core` command or export is library code with no
showcase consumer and would put a second writer into `ocelli-wasm` beside
F-202. Gesture snapping onto the identity adds hidden state to every zoom
for a benefit the key already delivers. Redefining the percent label as
device-relative changes documented behaviour for no gain the mode does not
deliver. Snapping by adjusting the stage margins instead of the canvas
position was rejected because the translate is the pan mechanism the pan
branch already drives, and because the asserted element is the canvas.
Computing the centring ideally instead of measuring it was rejected because
flex and the stage margins quantise through the layout unit and the ideal
can be off, which the AC2 epsilon would then have to absorb. Quantising
drag pans per move was rejected as redundant with the snap (decision 2).
Storing the refusal reason as derived-only with no field was rejected
because the reason must appear after a request and not on every anisotropic
image unprompted, which is the difference from `Window not applicable`.

## Dependencies, write ownership and resources

Declared dependencies: F-245, integrated at `ac962c64` and recorded
in-progress with AC8 owed (`docs/sprints/BACKLOG.md` line 424, and the
operator's answer recorded at `864f3ed7`, fact 22), and F-207, done at S45
(`docs/sprints/BACKLOG.md` line 353). Both are satisfied for this story's
start on this tree, which is what `/start-feature` requires.

Source-level dependencies and the F-202 seam, decided. F-202's round 2 plan
(fact 19) puts the remote frame in its own transferred canvas inside the
active cell's stage, with a worker-owned device-pixel backing store
pre-sized at `bind_cpu` and published by `publishCpuCanvas`. The lead
ruling adopted by this plan: on a remote frame the mode is refused by name
with a visible reason until a later story routes it through the core factor
(decision 15, AC16). The question revision 2 put to F-202's designer, which
surface the scale rule lives in, is closed by that refusal for this story:
the local canvas keeps the stored-frame contract and the mode's rule, and
the remote surface is out of the mode's reach until a story owns the core
factor route. What F-202's landing changes for this story is the tree the
page edits sit on and the liveness of the remote guard, which the order
below fixes.

The shared-file order, adopted from the lead ruling of round 3 (fact 19:
F-202's own landing-order request is in the truncated part of its plan and
is not re-readable this session): **F-202 first**, then F-241's library
part, then F-241's
viewer part, with F-204, F-205 and F-267 serialised around the page edits
by the sprint's one-writer rule. F-205's own order section (its lines 479
to 502) recommends F-267 first and takes F-241's viewer phase after or as a
rebase, which the same serialisation absorbs. This story's implementation starts after
F-202's page edits land, re-verifies the page facts named here against that
tree at implementation start, and reports any contradiction per
implement-feature. Revision 2's "either order is safe" is withdrawn.

The write-set overlap, re-read this session from the revised sibling plans
(facts 19 to 21). The contested files and their writers:

- `README.md`: F-202 (worker decode sentences), F-204 (this story's
  toolbar, key and zoom sentences), F-205 (its write set, fact 21), F-241
  (video sentences, plan line 368),
  F-267 (the step 8 corner sentence, plan line 921). One writer at a time
  under the order above, the later story rebases.
- `examples/folder-viewer/src/more.html`: F-202 (its matrix row and worker
  section), F-204 (row 156 and the section at 524 to 537), F-205 (its
  write set, fact 21), F-241
  conditionally a video card (plan line 370). F-267 names no `more.html`
  change (fact 20).
- `bin/ocelli.sh`: F-202 (suite registration and the wasm gate additions),
  F-204 (suite registration before `launcher.mjs`), F-205 (the description
  at line 70 and the case arm at lines 214 to 231, fact 21), F-267 (the
  case arm and its description string), F-241 conditionally a `wasm-video`
  step (AC21). The arm's comment fixes `launcher.mjs` as last (fact 11),
  all five insert before it, and the later story rebases and re-inserts.
- `docs/lld/build-targets.md`: F-202 (its write set names it in the round 3
  record, fact 19), F-204 and F-241 (AC21 and AC27). F-205's write set
  names no LLD file and F-267's names none either (fact 20).
- `examples/folder-viewer/src/page.html`: F-202 (the remote section), F-204
  (the zoom, layout, pan, key, tip and listener blocks, plus the one
  activation-site drop statement of decision 15 in F-202's remote block),
  F-205 (the preference constants, sections, startup lines and the scope
  note at line 917 beside the KEYS panel, fact 21), F-267 (the corner
  lines in the same `paintOverlays` region). The F-204, F-205 and F-267
  edits are adjacent but distinct statements: F-267 extends the
  window and acquisition lines of the corner text, this story changes only
  the `"\nZoom " + vp.zoomLabel` suffix and the mode behind it, and F-205's
  scope note is one line above the KEYS section this story's row lands in.
  Whichever lands second rebases.
- `eslint.config.js`: F-202 (the worker block) and F-204 (the conditional
  browser-suite globals entry).
- `examples/folder-viewer/src/use.html`: F-202 and F-241 (plan line 369,
  the video route sentences) write sentences in it, and F-205 lists it for
  the section 7 search with no sentence change expected (fact 21). This
  story changes it not at all. **A note for F-202's designer:** the round 3
  review records that F-202's plan line about 947 says F-204 writes
  `use.html`, which is false and needs correcting in that plan. The line
  is in the truncated part of the visible copy, so it could not be re-read
  this session.
- `examples/folder-viewer/tests/`: F-202 adds `worker.mjs`, F-204 adds
  `one-to-one.mjs` and `dicomweb-upstream.mjs` and edits `dicomweb.mjs`
  for the extraction, F-205 adds `preferences.mjs` and amends
  `tests/preview.rs`, F-241 adds `video.mjs`, F-267 adds a new check and
  appends to `tests/preview.rs`. New files do not overlap, and the two
  edited files are serialised by the same rule.

This story's write set: `examples/folder-viewer/src/page.html` confined to
the zoom, layout, pan, key, tip and listener blocks named in the Approach,
plus the one activation-site drop statement of decision 15 in F-202's
remote block, and never `requestGrey`, `postPreview`, `paintGrey` or the
worker surface.
`examples/folder-viewer/src/more.html`, `README.md`, `bin/ocelli.sh`'s
additive arm and description lines, `eslint.config.js`'s conditional
browser-suite globals block, the new suite file, the new
`dicomweb-upstream.mjs` and the one moved-function edit to `dicomweb.mjs`,
and
`docs/lld/build-targets.md`. No shader, no ledger, no GPU or browser
exclusivity beyond the existing viewer-browser serial runs. The eslint
browser-suite globals block (`eslint.config.js`, the
`examples/folder-viewer/tests/browser/**/*.mjs` context) gains only the new
page names the suite calls, conditionally, with no rule disabled.

## Verification and evidence reuse

Focused checks, in order, reading every exit code from the command itself:

1. `npm run lint`, for the new suite file and the conditional globals entry.
2. `bin/ocelli.sh gate prose`, for `page.html`-adjacent tracked prose,
   `more.html`, `README.md`, this plan and the LLD edit.
3. `node examples/folder-viewer/tests/browser/one-to-one.mjs`, the new
   suite with its `--force-color-profile=srgb` launch argument, needing the
   browser install and the release viewer build that the
   `viewer-browser` gate already requires (`bin/ocelli.sh` lines 211 to
   213).
4. `bin/ocelli.sh gate viewer-browser`, the whole list including AC15's
   regression rows, on the final staged tree.

Evidence invalidation adds one entry: a change to the pinned colour
profile, the launch arguments or the PNG decode path invalidates the
exact-byte rows AC3, AC14 and AC19, because their bytes are defined only
under the pinned sRGB transform.

Final required profile: the feature profile through `/verify` on the final
staged integration tree, per the workflow. The timed subjects
`viewer.open`, `viewer.first_image`, `viewer.window_step`,
`viewer.scroll_rate` and `viewer.resident_memory` are covered by the
operator's answer of 2026-11-12, recorded at `864f3ed7` (fact 22): the
idle-host acquisition runs at sprint end, with GLM paused, beside the
consolidated `gate --sprint`. This story's path change sits inside
`drawGrey`'s call graph through `layoutViewport`, so that acquisition is
the instrument that covers it, and no separate timed run is scheduled or
claimed by this plan. The focused evidence above is complete without it.

Evidence invalidation: any later change to the grey reply contract, the
backing store size, the stamp inputs, the scale rule or the snap rule
invalidates the affected rows and requires the suite again. A final plan,
status or provider change invalidates prior evidence as the workflow
states. No baseline or tolerance changes. The suite's pixel rows use exact
byte equality against hand-computed values, which is a fixture assertion
under 27.2 R3 and not a tolerance under section 25.1, and it is justified
because the expected values come from the standard rather than from the
implementation. No new performance axis, so no new instrument.

## Boundary and tier

- wasm-bindgen: not touched. No `crates/` or `packages/` file is in the
  write set.
- Pixels across the boundary: no. The story adds no crossing of any kind.
  The existing local-process grey route is unchanged.
- Render-loop allocation: none. The layout path does style writes behind
  the stamp short-circuit, and the mode adds a comparison, a division and
  one rect read. No buffer is created, sized or grown.
- unsafe: none.
- Tier A (WebGPU): n/a. The mode is a display-scale property of the page's
  DOM and CSS, and the page's render path is unchanged on every tier.
- Tier B (WebGL2): n/a, for the same reason.
- Tier C (CPU): n/a, for the same reason. The page draws through its
  existing 2D canvas path today on all tiers, and nothing here gates on
  the resolved tier. There is no tier branch to get wrong, which is the
  honest form of the answer rather than an omitted row. D-07 is not
  engaged.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | The hand-computed display identity, cited to PS3.3 C.11.2.1.2.1 and its Note 4 as transcribed above: 8-bit stored values, Window Center 128, Window Width 256, VOI LUT Function LINEAR, no rescale, MONOCHROME2, display byte equal to stored value across 0 to 255, the byte taken through `display_index`'s round to nearest (colour.rs 518 via preview.rs 1531), which repairs the 95 of 256 f32 values whose linear-branch evaluation misses by one ulp, the bottom clamp bound exercised at the stored value 0, the linear branch exercised at the top boundary by fixture P's stored 255 at (127, 95), and the top clamp stated unreachable for 8-bit input. The pattern `(x * 29 + y * 57) mod 251` makes the stored value distinct along each row and along each column of every fixture whose longest side is at most 251, verified this session for P, B, C and D, and makes adjacent horizontal and vertical neighbours differ everywhere including fixture E, whose 2048-long rows repeat values because they exceed the 251-value cycle. It does not make the values distinct across a fixture, which no row consumes. Expected device tables: stored index to byte at one device pixel each, at ratios 1, 1.25, 1.5 and 2, permuted by the reachable rotations and flips, and under answer (a) the row subsample rule of AC19. The clamp cases' floors and targets are computed in the suite from the measured cell, so a layout change cannot silently invalidate them. The spacing-order citation for fixture D is the in-tree rule, `greyAspect` at page.html lines 2489 to 2493 with the PS3.3 10.7.1.3 order as `aspect.mjs` lines 3 to 8 state it | `examples/folder-viewer/tests/browser/one-to-one.mjs` |
| browser | Every AC row, at named selectors, through the real release viewer in headless Chromium at `deviceScaleFactor` 1, 1.25, 1.5, 2 and the four failing probes 1.38, 1.45, 1.54 and 1.57, launched with `--force-color-profile=srgb` so the exact-byte rows AC3, AC14 and AC19 read the same values on every host | `examples/folder-viewer/tests/browser/one-to-one.mjs`, registered in `bin/ocelli.sh` `viewer-browser` |
| browser, regression | AC15, the landed suites unchanged | the existing `viewer-browser` list |

There is no Rust and no package change, so there is no Rust unit or fixture
home for this story. The fixture rule of HLD 27.2 R3 is answered by the
suite's hand-computed table, written before the mode exists, per
implement-feature step 2. Mutation controls, each recorded red before the
green run. With the mode computing `1` instead of `1 / ratio`, AC1 and AC3
fail at ratio 2. With the origin snap removed while the scale rule stays,
AC2 fails on the odd-cell and odd-fixture cases while AC1 still passes, and
AC3 fails with it whenever the raster carries the half-pixel fraction rather
than snapping it away, which is why the pair is required and neither row
stands alone. With the line 3601 comparison left in place, AC10 fails at
the four probes. With `copyZoom` propagating the mode without the target's
own guards, AC7a's below-ratio or remote half fails under answer (a) and
AC7b's anisotropic half fails under answer (b). With the remote guard
removed, AC16's refusal half fails on the post-F-202 tree, and with the
activation-site drop removed, AC16's held-mode half fails. With the refusal
string never cleared, AC13's or AC16's no-stale-reason half fails. With the
clamp floor set to `identityScale` alone, AC20 fails on fixture E while AC6
still passes, which is why the pair is required.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no display-scale row and no
occurrence of F-204 or one-to-one
(`grep -in "display\|scale" docs/hld/B-parity-surface.md` prints nothing).
No row closes and no row is introduced. The More matrix row is the tracked
UI truth, not the HLD appendix, and this story updates it per the map
above.

## Deviations

None. No HLD rule is departed from. The register is untouched by this
plan. Should open question 4 resolve to answer (b), the departure from the
reference's anisotropic behaviour is stated in the More section as the
workflow requires, and it is a named capability gap in the tracked delivery
truth rather than a numeric divergence, so no `D-NN` row is written for it.

## LLD impact

- `docs/lld/build-targets.md`: the new browser suite paragraph, in the
  pattern of the F-245 paragraphs, naming the file, its registration
  immediately before `launcher.mjs`, and the conditional eslint globals
  entry.
- `docs/lld/benchmarks.md`: only if the sprint-end acquisition moves a
  band, which is the acquisition owner's record and not this story's
  claim.
- No other `docs/lld/*.md` has a home for the page's display scale. There
  is no viewer-page LLD file, and `complete-feature` step 9 adds none.

## User-facing documentation impact

- `README.md`: the toolbar group sentence at lines 396 to 405 gains the
  One-to-one button beside Fit, and the key list sentence at line 417
  gains the `\` key. The zoom sentence at lines 455 to 460 gains the
  mode's sentences: what it does, which source it covers, that pan keeps
  it, that a remote display in the cell returns the viewport to fit, and
  the refusal reasons including the remote frame's.
- `examples/folder-viewer/src/more.html`: the matrix row at line 156 moves
  to Implemented with truthful cells naming the source coverage, the
  viewer's local images, with the remote worker frame refused by name, and
  the One to one section at lines 524 to 537 is rewritten to name the
  shipped workflow, where the factor is computed, the refusal reasons, the
  tiny-frame floor, and the core route's remaining explicit-factor shape.
  Under the recommended answer to open question 4 the section says the
  rows are exact on non-square pixels and the columns are resampled, and
  under the refusing answer it says the mode needs square pixels and names
  the departure from the reference with its benefit.
- `examples/folder-viewer/src/use.html`: no change. It documents the
  library API, not the page's controls, and this story adds no library
  API.
- `examples/folder-viewer/src/page.html`: its own help panel, the KEYS
  row and the tip text, edited as part of the feature rather than as
  separate documentation.

Per implement-feature section 7, all three documents are then searched for
every name the story added or changed, including `zoomMode`, `One to
one`, `one-to-one` and `\`, and each stale sentence is fixed whether or
not this section listed it.

## Open questions

1. **The key binding.** Options: `\` beside the zoom keys on the US
   layout, the apostrophe `'`, or button only with no key.
   Recommendation: `\`, with the button present regardless, because every
   toolbar button carries a `data-key` tip today and the KEYS panel
   documents it. Affects AC1's trigger, the KEYS row and the README.
   Deferral cost: the control ships button-only and the binding lands
   later as a user-facing change.
2. **The core route.** Options: keep `@ocelli/core` on the explicit factor
   and name the page as where the factor is computed, or add a factor
   computation to the core camera route in this story. Recommendation:
   defer. The page is the story's deliverable, the core route has no
   showcase consumer, and `ocelli-wasm` and `ocelli-viewport` are F-202's
   write surface this sprint. Affects the More section wording and the
   matrix row's library cell. Deferral cost: the library cell keeps
   reading `Zoom building blocks` until a story with a consumer adds it,
   and the More text must say so plainly, which this plan's documentation
   section already does.
3. **Linked zoom propagation.** Options: the source's mode propagates, so
   a linked viewport enters one-to-one over its own image and cell, or
   the landed semantic of copying the zoom over each viewport's own fit
   is kept and the mode does not propagate. Recommendation: propagate the
   mode, by symmetry with how fit propagates today (page.html lines 7006
   to 7007) and because one-to-one, like fit, is a device-relative mode
   that is well defined per viewport, with the per-target guards of
   decision 9. Affects AC7a and AC7b. Deferral cost: none beyond the AC
   rows being written to the other answer before implementation starts.
4. **Anisotropic frames: match the floor or refuse.** Re-posed by the lead
   after design round 2, under the rule that cornerstone is the floor.
   The false premise is gone: the reference enters its scale mode on
   anisotropic images, rows 1:1 and columns nearest-resampled at
   `spacing[0] / spacing[1]` device pixels each, 0.714 and dropped on the
   synthetic. Options. (a) Match the floor. The mode enters on an
   anisotropic frame with the row axis at the identity, the corner reading
   `Zoom 1:1 rows`, More stating that columns are resampled on non-square
   pixels, and the acceptance rows AC17 to AC19. The departure from the
   tracked promise "nothing is resampled" is confined to the labelled
   axis and stated in the More text. (b) Refuse with the named reason
   `1:1 needs square pixels`, which is less capability than the
   reference. The departure and its benefit, the drawn image is never
   resampled on any axis while the mode holds, proved byte-exactly by
   AC3, would be stated in the More section as the workflow requires.
   Recommendation by the lead: (a). Affects AC13, AC17 to AC19, the scale
   rule, the clamp, the label, the corner text and the More rewrite.
   Deferral cost: none of the mode's guard, label or AC work can start,
   and the More row cannot read Implemented truthfully without the
   choice. Both answers are designed completely above so either can be
   applied without another design round.

Closed by the lead amendment, recorded in the decisions and not carried as
questions. Open question 5 of revision 2, F-202's worker surface sizing, is
closed by decision 15 and the adopted order. Open question 6 of revision 2,
where mode persistence lives, is closed by decision 14.

## Operator decision and lead amendment after design round 4, 2026-11-12

The operator answered open question 4 on 2026-11-12 with answer (a), match
the cornerstone3D 5.8.2 floor, recorded in `docs/sprints/CURRENT_SPRINT.md`.
An anisotropic frame enters the mode with stored rows one device row each
and columns nearest-resampled, the corner reads `Zoom 1:1 rows`, and the
More text says columns are resampled on non-square pixels. Every row, state
and path above marked as applying only under answer (b) is void and is not
built: AC7b, AC13's anisotropic refusal and its `1:1 needs square pixels`
reason, and the (b) cells of the state table. AC13's no-stale-reason check
stays, exercised on the remaining refusals. AC7a, AC17, AC18 and AC19 apply.

The lead decides open questions 1 to 3 as the plan recommends. The key is
`\`, with the button present regardless. The core route keeps the explicit
factor and the page computes it, with the More text saying so. The mode
propagates to linked viewports through each target's own guards.

The base moved to `sprint/s47` at `2a13dbf3`, which integrates F-267.
F-267 adds corner painting to `paintOverlays` in
`examples/folder-viewer/src/page.html` and registers `corners.mjs` after
`window.mjs` in the `viewer-browser` arm. Line citations to `page.html`
below its corner block shift by that change. The implementer re-reads
every cited line on the new base before editing and treats a moved line
as the same site, not as a contradiction.

## Lead amendment after design review round 4, 2026-11-12

This section is authoritative over every earlier section it touches. The
implementer builds what it says where an earlier sentence disagrees.

**Landing order, revised.** F-204 lands before F-202. F-202 is still in
design and is expected to carry into S48, and F-204 does not need anything
F-202 builds. The remote seam therefore moves to F-202: when F-202 shows a
remote frame in a cell, F-202 drops that viewport's held one-to-one mode to
fit through F-204's existing fit path, refuses the one-to-one control while
its remote source is active with the reason on its own remote status line,
and restores the local layout on release. F-204 builds no remote guard, no
`vp.remoteDisplayed` field and no remote case. AC16's remote rows, decision
15's remote branch, the remote cells of the state table, the remote target
of AC7a and every write-set and order entry that serialised F-204 after
F-202 are void. The coordinator records the matching requirement in F-202's
plan. F-267 is in the base, not a concurrent writer, and every entry naming
it as one is void.

**The below-ratio refusal gets a fixture and a row.** Add fixture F, 2 by 64
stored pixels, no Pixel Spacing, the same value rule as the others. It is
below the ratio at `deviceScaleFactor` 3 and suitable at 2. AC13 is replaced
by an answer (a) row, case `f-204-below-ratio`. At ratio 3 on fixture F,
pressing `\` shows `1:1 needs a frame at least the device pixel ratio big`
in the corner line while mode, scale, extent and the zoom label hold. A CDP
change to ratio 2 makes the predicate false and the corner shows the zoom
percent with no frame change. Back at ratio 3 with no new request, the
corner shows the percent and never the reason. Scrolling to fixture P and
back to F with no request still shows no reason. The named wrong
implementations are a reason string that is never cleared and an inverted
below-ratio comparison. AC7a's unsuitable linked target is fixture F at
ratio 3.

**The anisotropic identity is tested in both spacing orders.** Add fixture
D-prime, Pixel Spacing `0.5\0.7`, the same value rule. AC17 runs on D and
on D-prime. On D-prime at ratio 2 the mode gives 96 device rows and a
179.2 device pixel width, so an implementation dividing by the larger aspect
or by the product of both fails it.

**The ratio watcher.** Decision 12 registers with
`mql.addEventListener("change", handler, { once: true })` and re-arms on each
change. AC5's init-script wrapper counts only registrations whose `this.media`
contains `resolution`.

**The colour profile.** Playwright 1.62.1 already passes
`--force-color-profile=srgb` by default. The suite must not set
`ignoreDefaultArgs`, and the text claiming the suite adds the switch is
replaced by that statement.

**Open questions 1 to 4 are decided**, question 4 by the operator and 1 to 3
by the lead, as the previous amendment records. Every conditional on them
above is resolved to the decided answer.

**Citations on the new base.** `page.html` lines up to 3297 did not move.
Lines 3298 to 3676 moved by plus 10 and lines from 3678 by plus 23, with no
cited range straddling a hunk. In `bin/ocelli.sh` the `viewer-browser` arm
is 219 to 237 with `launcher.mjs` at 237, the comment 212 to 215, the lines
once at 211 to 213 now 216 to 218, and the arm runs thirteen suites. The
relayout function is 1787 to 1794 with `layoutDirty` at 1790. The refusal
string is written by the refusal site, and the single translate write site
writes the `panApplied` fields. AC20's illustration uses the same 1600 by
1000 cell as AC6. The rejected-alternatives rationale's example is the
below-ratio frame.

**Further to the above, after the confirmation review.** The remote seam's
removal also voids the user-facing documentation entries that describe it:
the README and More text say the mode covers local images, and the remote
wording is F-202's to write. The extraction of `startSyntheticDicomweb`
into `dicomweb-upstream.mjs` and every edit to `dicomweb.mjs` are void,
because their only consumer was the remote row, so F-204 writes neither
file. The below-ratio row gains a positive control at the boundary: on
fixture F at ratio 2, pressing `\` enters the mode with a 2 device pixel
wide extent, which fails a guard using `<=`, and this is the valid retry
that the refused-request rule names. Fixtures P and F are written as two
frames of one series of mixed sizes, so the move between them is a scroll
and not a series open. D-prime is 128 by 96 stored pixels. There are seven
fixture files, P, B, C, D, D-prime, E and F, and the distinctness sentence
and the browser Tests row cover all seven, with ratio 3 in the list of
ratios. In the 1600 by 1000 cell, AC20's illustrative floor is 0.0390625
and its extent 160 device pixels, which the suite computes from the
measured cell.

## Lead amendment after implementation review pass 1, 2026-11-12

This section is authoritative over decision 10 and every row it touches.

**The one-to-one mode draws into a device-sized backing, as cornerstone3D
5.8.2 does.** The review measured that a stored-size canvas scaled by CSS is
resampled on a ratio 2 display whenever a stored extent is odd, because the
box is a half CSS pixel (fixture B at 64.5 CSS px matched 2145 of 8385
bytes), while the corner reads `Zoom 1:1`. That is below the floor:
cornerstone sizes its backing as `Math.round(clientWidth * dpr)`
(`ContextPoolRenderingEngine.js` about 131 to 137) and maps texels at device
resolution. While the mode holds, the viewport's canvas element is sized in
whole CSS pixels to its cell, its backing store is that box times the device
pixel ratio, and the page writes stored pixels one to one into backing
pixels at an integer device offset that carries the centring, the pan and
the snap, with rotation and flips applied by index, never by a CSS
transform. Where the box times the ratio is not a whole number, the box is
chosen as the largest whole-CSS-pixel box whose product is a whole number
of device pixels, or the mode is refused by name for that cell size, and the
plan's tests say which. Outside the mode the existing stored-size canvas and
its CSS scaling are unchanged. The review's probe found this backing
byte-exact at ratios 1.25, 1.5 and 2 for 128 by 96 and 129 by 65.

**Every byte row is restored.** AC3 runs P and B at ratios 1, 1.25, 1.5 and
2 and the mid-pan, and AC19 runs its row count, row order and column
subsample on D and D-prime, each with an exact integer crop offset, never a
search window. AC2 asserts an observed painted origin, not a prediction.
Departures (a) to (c) of the implementation report are withdrawn.

**Truthful limit.** The suite runs under DevTools device scale emulation.
README and More say the mode is verified byte for byte under emulated
ratios 1, 1.25, 1.5 and 2 in the pinned Chromium, and the More row moves to
Implemented only when every restored byte row passes.

## Lead amendment after implementation review pass 2, 2026-11-13

The mode's box is sized up, never down: on each axis the smallest whole
multiple of the ratio's step that covers the cell, plus one step so the
alignment shift still covers it, clipped by the cell's existing
`overflow: hidden`. No part of the cell inside the drawn frame is left
unpainted, and the `1:1 needs a larger view` refusal, its case and its README
and More sentences are removed, matching cornerstone3D 5.8.2, which never
refuses for cell size. A flag records that the backing holds the mode buffer,
and leaving the mode by any path restores the stored-size backing whenever
that flag is set. The corner shows a refusal reason only while the condition
that produced it still holds.
