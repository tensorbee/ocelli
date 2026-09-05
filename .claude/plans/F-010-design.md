# F-010, Headless cornerstone3D reference renderer

**Status**: approved
**Epic ref**: E2.2
**Sprint**: S02
**Estimate**: 4w

## Normative source, transcribed

_Transcriptions below are verbatim except for one normalisation: a prose
semicolon in the source is written as a comma, and an em-dash as a hyphen,
because `scripts/prose_check.py` covers `.claude/plans/` and `docs/hld/` is
exempt. No word is changed. Where the exact bytes matter, the tracked
Markdown under `docs/hld/` wins._

### `docs/hld/08-validation-architecture.md`, section 11, verbatim

> Cornerstone3D is a correct reference implementation that can render any
> series you own. The harness pushes the same study through both stacks and
> compares frames within a written per-modality tolerance, with metadata
> diffed alongside pixels because a wrong rescale slope can still produce a
> plausible image.
>
> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture. In production, shadow mode renders both libraries and
> alerts on divergence - the oracle running against real clinical traffic, and
> the same corpus a regulatory submission would want to see.

### `docs/hld/25-first-ten-files.md`, section 28, entry 4, verbatim

> | 4 | tools/oracle/ | The differential harness. Nothing else should start
> before this works |

and the framing sentence, verbatim:

> In this order. The goal of the first two weeks is to diff one windowed 2D
> image against cornerstone3D - everything below serves that.

### `docs/hld/22-testing-and-tolerance.md`, section 25.1, verbatim

> Write it down once and hold it. Tuning tolerance per failure is how a suite
> stops meaning anything.
>
> - **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference <= 1
>   LSB on at least 99.9% of pixels, zero pixels differing by more than 2.
>
> - **Colour and ultrasound:** perceptual difference below a stated threshold,
>   because chroma subsampling and YBR conversion legitimately differ.
>
> - **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
>   a quarter pixel.
>
> - A tolerance change is a pull request with a rationale, reviewed like code.

**This story sets no tolerance and compares nothing.** The policy is
transcribed because it is what F-011 will apply to this story's output, and
because it says what the output has to carry to be usable at all: a pixel
class per row, and metadata alongside pixels.

### `docs/hld/11-decision-log.md`, decision D14, and D7

D7 is the oracle-before-port-code decision. D14, from `CLAUDE.md`'s standing
table, verbatim:

> | D14 | Claim MEASURED divergence, never bit-exact reproducibility. |

### `docs/hld/B-parity-surface.md`, Appendix B, verbatim

> Measured from cornerstone3D v5.8.9 source. The accompanying backlog maps
> each of these to the story that covers it.

### `docs/hld/DEVIATIONS.md`, D-05, verbatim

> The corpus lives under ignored `corpus/data`, with a committed manifest of
> per-case checksums and metadata. Operator constraint. A TCIA-derived corpus
> is large and its redistribution terms are not ours to assume. The manifest
> makes the corpus verifiable without being present.

### `docs/sprints/CURRENT_SPRINT.md`, the defect class and what done means, verbatim

> The dangerous oracle defect is false reference output. A headless page can
> start, load a test runner and exit successfully without decoding every
> corpus row, presenting a frame or reading back the rendered pixels. F-010 is
> complete only when failures at each of those boundaries are visible and the
> output is tied to the pinned cornerstone3D version and manifest digest.

> - F-010 renders every applicable corpus row through the pinned reference
>   stack in a headless browser and emits deterministic reference pixels or a
>   precise failure. It does not yet compare Ocelli output, which belongs to
>   F-011.

### `.claude/WORKFLOW.md`, the bootstrap exception, verbatim

> **Bootstrap exception.** S01 builds the corpus the oracle consumes, and
> F-010 in S02 builds the oracle itself. [...] The exception cannot apply once
> F-010 moves from pending.

## What the specification does not cover

Section 11 is five sentences and it is the entire normative text on the
oracle. It names cornerstone3D as the reference and says frames and metadata
are compared. It does not say:

1. **Which cornerstone3D version is installable.** Appendix B says v5.8.9.
   That version does not exist. See the open questions, this is blocking.
2. **How the reference is driven.** The allocation note says
   "Playwright-driven", which is a repository decision and not an HLD one.
3. **What a reference frame is, as bytes.** Nothing says RGBA, PNG, bit depth,
   canvas size or colour space.
4. **What the render parameters are.** A rendered frame is a function of the
   window, the camera, the canvas size and the interpolation, and none of
   those is in the DICOM file. Two runs that disagree on any of them produce
   two correct frames that differ, which would read as a port defect in F-011.
5. **Where the output lives.** A rendered frame of a real corpus case is a
   picture of patient data, and `corpus/README.md`'s real layer is marked
   `burned-in-unchecked` on all 44 real rows.
6. **What "applicable" means.** The corpus carries 91 rows across 16 transfer
   syntaxes. cornerstone3D does not decode all 16.

## Approach

The story has one job and one failure mode. The job is to produce, for every
applicable corpus row, a reference frame plus the metadata that explains it.
The failure mode is producing nothing while reporting success, and every
decision below is shaped by that.

**1. `tools/oracle` becomes a Node workspace beside the existing Rust crate.**

`tools/oracle/package.json` pins `@cornerstonejs/core`,
`@cornerstonejs/tools` and `@cornerstonejs/dicom-image-loader` at exactly
`5.8.2`, and Playwright at exactly `1.62.1`, with no caret and no tilde, for
section 15.2's stated reason applied to the reference rather than to wgpu: an
oracle that drifts is not an oracle. **The version is 5.8.2 and not Appendix
B's v5.8.9, which does not exist.** That is deviation D-11. The existing
`tools/oracle/Cargo.toml` crate stays as it is. It is the Rust side that
F-011 onwards will use to run Ocelli, and this story does not touch it.

`tools/oracle/node_modules` and `tools/oracle/out` are already ignored, and
`bin/ocelli.sh oracle` already refuses to run when `node_modules` is absent
and names this story. That refusal path stays and gains a second half: present
but not installed at the pinned version is also a refusal, not a warning.

**2. Playwright drives headless Chromium on SwiftShader, and the render page
is static.**

The browser launches with software rasterisation forced, so the reference is
reproducible across machines rather than a property of one developer's GPU.
The adapter string is read from the page through `WEBGL_debug_renderer_info`
and recorded in `run.json`, so the choice is visible in every output rather
than implied by a launch flag nobody reads. D14's counter-argument, that the
divergence we publish should be the one a user would see, is real and is
answered in F-011: the reference is held still here, and the measured
divergence F-011 publishes is Ocelli's own render on real hardware against
this stable reference.

`tools/oracle/page/index.html` loads cornerstone3D from the installed package
and exposes one function to the driver: render a row and return the frame.
The driver `tools/oracle/run.mjs` reads `corpus/manifest.tsv`, resolves the
bytes under `corpus/data`, and for each applicable row passes the file into
the page, waits for the render, and reads the pixels back.

Bytes go in through Playwright's binding as an `ArrayBuffer`, not through a
file URL, so the page needs no server with filesystem access to the corpus.

**3. The render parameters are declared, not defaulted.**

`tools/oracle/render-params.json` is a committed file giving the canvas size,
the camera, the interpolation mode and the VOI source for every row, keyed by
the manifest's category. It is committed because it is the thing that makes
two runs comparable, and because F-011's tolerance policy is meaningless
without it.

The VOI comes from the file where the file has one and from a per-modality
default where it does not, and the default is written in that file rather than
inherited from whatever cornerstone3D happens to do. The chosen values are
recorded per row in the output sidecar, so a frame always carries the
parameters that produced it.

**4. Four boundaries, four explicit failures.**

The sprint names the defect precisely, so each boundary gets an assertion that
fails loudly rather than a code path that continues quietly.

- **Row reached.** The driver counts applicable rows from the manifest before
  it starts and asserts at the end that it attempted every one. A row that was
  never attempted is a failure, not an absence.
- **Decoded.** cornerstone3D's image load either resolves or rejects, and a
  rejection is recorded with the transfer syntax and the error, then fails the
  run unless the row is on the declared unsupported list of step 6.
- **Presented.** The render is awaited on cornerstone3D's own image-rendered
  event, with a timeout. A timeout is a failure. Waiting on a fixed sleep is
  the shape this defect class takes and it is not used.
- **Read back.** The pixels come back and are checked for degeneracy: a frame
  that is entirely one value fails, unless the row's entry in
  `render-params.json` declares it should be. A blank canvas reads back
  perfectly and hashes stably, which is exactly why it needs its own check.

**5. Output, and none of it is committed.**

Per row, into ignored `tools/oracle/out/`:

- `<row>.png`, the reference frame, and `<row>.raw`, the RGBA bytes the PNG
  encoder never touched. F-011 compares against the raw bytes. The PNG exists
  for a human looking at a divergence.
- `<row>.json`, the sidecar: the manifest row's own fields, the render
  parameters used, the DICOM attributes that drove the render (rescale slope
  and intercept, window centre and width, VOI function, photometric
  interpretation, bits stored, high bit, pixel representation, pixel spacing),
  the cornerstone3D version, the browser build, and the sha256 of the raw
  bytes.

And once per run, into ignored output as well, `run.json`: the manifest's own
sha256, the pinned cornerstone3D version, the Playwright and Chromium
versions, the host and adapter string, the per-row digests, and the counts at
each of the four boundaries.

**Nothing under `tools/oracle/out/` is committed, and the reason is the
project's first rule rather than tidiness.** A reference frame of a real
corpus row is a rendered picture of patient data, and every real row is
`burned-in-unchecked`. The pre-commit hook refuses staged DICOM by magic
bytes, and it would not refuse a PNG. The directory is already in
`.gitignore`, and `scripts/staged_content_check.py` gains the same refusal for
anything under an oracle output path so that the guard is a mechanism and not
a habit.

**6. Applicability is discovered and recorded, never silently skipped.**

Every row is attempted. A row cornerstone3D cannot decode produces a named
entry in `tools/oracle/unsupported.json` giving the transfer syntax, the
error, and the cornerstone3D version that produced it. That file is committed,
because it is a fact about the reference implementation and it is exactly what
Appendix A gates A1 and A2 need. A row failing for a reason not in that file
fails the run.

This is the difference between an oracle that covers 30 rows and knows it, and
one that covers 30 rows and reports 91.

**7. Determinism is measured, not claimed.**

The run is executed twice in the same invocation of the gate and the two sets
of digests must match. That is the claim this story can actually support:
stable output on one machine and one browser build. It is not a claim of
cross-machine reproducibility, and D14 is why the stronger claim is not made.
The digests in `run.json` carry the host and adapter string so a cross-machine
difference is attributable rather than mysterious.

**8. The gate, and the skip that ends here.**

`bin/ocelli.sh oracle` runs the driver. The `oracle` gate stops being skipped
the moment F-010 leaves `pending`, which is what `s01_pre_oracle` in
`bin/ocelli.sh` already keys on and what the WORKFLOW text already says. The
function and its two conditions are removed rather than left in place with a
condition that can no longer be true, because a dead exception is a live
misreading.

`oracle` stays out of `--floor`. D-04 is unchanged by this story.

**9. Stack viewports only.**

Section 28 says the goal of the first two weeks is to diff one windowed 2D
image, and F-011 is a pixel-diff comparator. So every reference frame in this
story is a stack render of one frame of one instance. The twenty series rows
in the corpus, ten uniform-spacing and ten non-uniform-spacing, are rendered
as stacks here, and the volume builder's refusal path that the non-uniform ten
exist to exercise is asked nothing by this story. That is a stated gap and not
an oversight: it belongs to the E2 stories that add volume reference renders,
and `docs/lld/oracle.md` records it so the next reader does not mistake
coverage of a row for coverage of what the row is for.

## Boundary and tier

- wasm-bindgen: not touched. This story is Node, TypeScript and a browser.
- Pixels across the boundary: no. There is no Ocelli boundary in this story at
  all. The reference stack is cornerstone3D and the pixels it produces go to a
  file, not into the core.
- Render-loop allocation: none. There is no Ocelli render loop here.
- unsafe: none.
- Tier A (WebGPU): n/a. cornerstone3D v5 renders through vtk.js on WebGL2,
  and this story runs the reference stack rather than Ocelli's. Ocelli's tiers
  are resolved by F-004 and exercised against this output by F-011.
- Tier B (WebGL2): n/a, same reason. The browser's own WebGL2 context is what
  the reference uses, and that is a property of the reference and not a tier
  declaration by Ocelli.
- Tier C (CPU): n/a, same reason.

The three rows are n/a rather than omitted, and the distinction matters more
here than anywhere else in the sprint: it would be easy to read this story as
declaring a tier because it renders. It does not. It runs somebody else's
renderer.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `browser` | Every applicable corpus row decodes, presents and reads back through the pinned cornerstone3D under headless Chromium | `tools/oracle/`, Playwright |
| `browser` | Each of the four boundary failures is observed red before the pass is claimed | `tools/oracle/tests/`, fault injection |
| `conformance` | An unsupported transfer syntax is recorded with a reason and does not read as coverage | `tools/oracle/unsupported.json`, asserted by the driver |
| `unit` | The manifest reader resolves rows, digests and applicability without a browser | `tools/oracle/tests/` under vitest |
| `fixture` | The sidecar's transcribed DICOM attributes match values read independently with pydicom, on at least one row per photometric interpretation and per pixel representation in the corpus | `tools/oracle/tests/`, against `scripts/corpus_synth.py` cases whose values are known by construction |

The `fixture` row is not optional and it is not ceremonial. Section 11 says
metadata is diffed alongside pixels "because a wrong rescale slope can still
produce a plausible image", so the sidecar's metadata is load-bearing output
of this story, and a sidecar that transcribes a rescale slope incorrectly
would make F-011 chase a pixel difference that is really a metadata bug. The
expected values come from PS3.3 through pydicom, per HLD 27.2 R2, and never
from what the harness itself printed.

**Fault injection is how the four boundary failures are observed.** Point a
row at a truncated file, at a syntax the loader rejects, at a page whose
render event never fires, and at a canvas that reads back uniform. Four
mutations, four reds, all recorded in the completion note.

## Parity surface covered

None yet, and that is worth stating rather than leaving blank. Appendix B's
`Covered by` column is described in the HLD but is not present in the tracked
import, and this story implements no parity feature. It builds the instrument
that will measure them.

## Deviations

**D-11**, approved in the consolidated design round and recorded in
`docs/hld/DEVIATIONS.md`:

> Appendix B says "Measured from cornerstone3D v5.8.9 source" and the parity
> target is stated as v5.8.9. The oracle pins `@cornerstonejs/core`,
> `@cornerstonejs/tools` and `@cornerstonejs/dicom-image-loader` at exactly
> `5.8.2`, because v5.8.9 does not exist.

`CLAUDE.md` and `README.md` both state the parity target as v5.8.9. This story
corrects both to 5.8.2 with a pointer to D-11, because a parity claim against
a version nobody can install is not a claim anyone can check.

## LLD impact

A new `docs/lld/oracle.md`: the harness layout, the pinned versions, the
render-parameter contract, the four boundary assertions, the output format
including the sidecar's fields, the applicability file, and the determinism
claim with its stated limit.

`docs/lld/corpus.md` gains a pointer to it, since the two are read together.

## Decisions taken in the design round

1. **Pin 5.8.2, and raise D-11.** v5.8.9 does not exist. `@cornerstonejs/core`
   has 1124 published versions and the highest is 5.8.2, checked against the
   npm registry on 2026-09-04, and the same holds for `@cornerstonejs/tools`
   and `@cornerstonejs/dicom-image-loader`. Appendix B's surface counts are
   left as the author measured them and `/parity` re-checks them against 5.8.2
   on its first run.
2. **SwiftShader, not a hardware adapter.** The reference is held still so it
   is the same artefact on every machine, and the adapter string is recorded
   in `run.json` so the choice is visible rather than implied. The measured
   divergence D14 commits to publishing is F-011's output, and it is measured
   against a reference that does not move.
3. **Stack viewports only**, per step 9. Volume reference renders belong to
   the later E2 stories, and the gap is recorded in the LLD rather than left
   to be inferred from an output directory.

## Open questions

None. All three were resolved above.
