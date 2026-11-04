# Current sprint, S42

**Milestone**: M19, series open in the core and the viewer.
**Branch**: `sprint/s42`
**Opened**: 2026-11-04
**Goal**: Frame open from a Part 10 object, series assembly, overlay planes and display shutters, and stack display interpolation on tiers A, B and C.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-191 | E38.1 | Frame open from a Part 10 object: chain inputs including Modality and VOI LUT Sequences, colour selection, pixel padding PS3.3 C.7.5.1.1.2, Lossy Image Compression and Burned In Annotation flags, private creators | Rust | 5w | done |
| F-192 | E38.2 | Series assembly: order by projected position, split mixed series, temporal axis, cine timing with loop and bounce | Rust | 5w | done |
| F-193 | E38.3 | Overlay planes PS3.3 C.9.2 and display shutters PS3.3 C.7.6.11 and C.7.6.15 | Rust | 3w | done |
| F-194 | E38.4 | Stack display interpolation, nearest and linear, declared on tiers A, B and C | Rust | 3w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority.

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S42 / {print $2, $9}'
```

## What this sprint is

S41 is closed at tag `s41`. The review fixes that followed that close are
`bf3c9f3` on main. This sprint is the first of the three series-open sprints
inserted before the brick work. The brick story F-119 is S45. It is not
opened here.

The showcase is `examples/viewer-react`, which `npm run dev` serves. It
must display every behaviour this programme adds. A core function with no
control on that page is not done. `examples/folder-viewer` stays the
local folder page and is not a second product.

## What is carried in

- **The review commit has no verify trailer.** `bf3c9f3` recorded the
  post-S41 review fixes. Push of that commit alone would be refused until a
  later head carries comparison evidence.
- **D-22 still blocks publication.** Nothing in this sprint publishes.
- **The LUT chain already exists.** `ocelli-pixel::LutChain` is the only
  pixel formula. Frame open supplies its inputs. It does not map samples
  itself.

## The defect class this sprint is exposed to

A quietly wrong pixel. Padding that is applied as a second window, a colour
frame that is shown as grey, and a lossy frame that looks lossless are the
same class. The hand fixture cites PS3.3 C.7.5.1.1.2 for padding and the
existing section 18.3 rows for the window. A tolerance is not widened to
make one of them pass.

## What done means

Frame open returns the chain inputs, the colour selection, the padding mask,
and the lossy and burned-in flags for one Part 10 object. Private creators
are resolved for the tags that carry them. Series order uses the projected
image position. Cine timing, overlay planes, display shutters, and nearest
versus linear sampling each have a core answer. The example viewer shows
each of those controls on the page a person opens with `npm run dev`. A
tier that cannot do the requested sampling reports unavailable on that
same page.
