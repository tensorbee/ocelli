# F-239 palette validation hoist, independent source review pass 1

Reviewer: Codex `/root/f225_review`. Author: root Codex. The reviewer did not
author this product change or its lead ruling. GLM discovered the repeated
validation in its separate source investigation. Its first report is still
running on the original source and was neither interrupted nor steered.

Result: 0 defects, 0 smells, 0 notes in this bounded source review. This is
not build, fixture, benchmark or whole-feature acceptance.

## Identity and scope

Review target is `/private/tmp/ocelli-s46-palette-validation-hoist`, working
source delta from `cbd4df12`. Exactly three paths changed: folder colour draw,
`RasterColor`'s trait declarations and F-239's lead ruling. Exact base tree
and SHA-256 for all three files are in `root-hoist-review-identity.json`.
No source or index was changed by the reviewer. Root owns dynamic checks.

## Borrow and public API

`AcceptedColor` already derives Copy and Clone and contains only
`&ColorTransform`. Its field is private. Adding the same traits to the closed
`RasterColor` enum copies either that immutable borrow or the Unsupported
discriminant. It does not clone a transform, palette, ICC table or allocation.
It introduces no new constructor that can bypass `supported`.

The borrow remains tied to the input transform's lifetime. Within draw the
recipe is shared, while scratch and output are separate mutable arguments.
No transform mutation occurs between bands. Copying the token cannot extend
its lifetime, and no interior mutation was found in the owned transform or
palette fields. The change therefore carries the same accepted transform
through every band of one draw without a cross-frame cache or stale token.

## Consumers and arithmetic

The folder draw's request checks, window construction, output sizing,
rows-per-band, unpacking, slice extents and colour conversion are unchanged.
Only transform acceptance moves from inside the band loop to immediately
before it. Every call still passes the same transform, window, stored band,
width, scratch and output slice into `colour_bytes_into`.

This covers raw or decoder-converted RGB, both planar layouts, supported YBR,
palette with its depth fallback, and ICC-attached transforms. The colour
branches remain in `ColorTransform`. No index arithmetic, scaling, matrix,
ICC application, window formula or byte rounding changed. Pointer and ROI
consumers use `map_band` and remain unchanged. Monochrome `render_held` never
enters this folder draw. The other render consumers still take the same enum
variants by value. Unsupported remains rejected in both raster entry points.

## Refusal order and invalid frame reachability

The request's monochrome-only colour refusals and invalid window refusal
still precede output work. Output size and capacity failures and
`rows_per_band` still precede palette acceptance. Palette refusal now occurs
before first-band unpack instead of immediately after first-band unpack.
Both variants still refuse before any palette conversion.

This reordering can affect only a fabricated object combining a bad palette
acceptance with an independently invalid first sample band. Production
`HeldFrame` is built through `frame_parts` and `held_from_samples`.
`frame_parts` rejects zero pixel count and constructs validated dimensions,
bits and sample layout. `finish_decode` sizes the output before decoding and
refuses a zero decoded length. Palette channels and display depth are already
constructed through their validated private types. No file or HTTP route
constructs an arbitrary invalid `HeldFrame` or mutates its accepted recipe.
The draw function is crate-private and has one production caller.

A zero width is still refused by `rows_per_band` before acceptance. A zero
height is rejected before a production held frame exists. Manually fabricated
crate-internal invalid objects are outside that factory invariant. There is
no documented competing-error precedence contract requiring their old order.
This distinction was checked explicitly rather than claiming the reordering
is literally identical for every possible private struct literal.

## Evidence boundary

The existing end-to-end measurement establishes the slow palette workload.
The source establishes repeated acceptance once per band. Neither alone
proves the fraction of elapsed time consumed by that validation. The lead
ruling states that distinction and requires the same release benchmark after
the repair. No new threshold, tolerance, budget or baseline acceptance is
introduced. Source review found no need to change the proposed narrow patch.

Read-only source commands succeeded except one initial lookup for a
nonexistent `dimensions.rs`. It exited 1 and was corrected by locating
`ImageDimensions` in `image_plane.rs`. No check was claimed from that failed
read. No CPU-heavy job, native build, test, profile, benchmark, browser or GPU
was run for this review.
