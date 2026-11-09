# openjph-core 0.1.0 provenance

This directory contains the published `openjph-core` 0.1.0 package from
crates.io. The downloaded `.crate` archive has SHA-256 digest
`c8b96ed12b3d41623a771af4af8131abf353bc822b7a567c6ef3b35ab967a36d`.
Its `.cargo_vcs_info.json` identifies upstream Git commit
`7ed6d6d110d994ec740aacaa90a78b2e807c4c24` and path `openjph-core`.

`PACKAGE-INVENTORY.sha256` records every file in the unmodified published
package. No published file was removed. The upstream `LICENSE` from OpenJPH
tag 0.26.3 was added by F-217 and is pinned by the redistribution check.

## The F-239 patch, deviation D-31

Three source files carry a recorded patch so that the codestream's own inverse
multiple component transform runs on the decode path, before the DC level
shift and its clamp, and so that the inverse transform's coefficients are
ISO/IEC 15444-1 equation G-6's literals. Without it, a colour HTJ2K codestream
decodes each component as if it were monochrome, and no adapter can undo the
damage after the fact, because the clamp has already cut the chroma's extra
bit. The patch's own text adds no `unsafe`.

- `src/codestream/tile_comp.rs`: `perform_idwt` takes a
  `defer_level_shift` flag, the irreversible path can hold its unrounded
  sample-domain floats in `decoded_lines_f32`, and
  `finish_deferred_level_shift` rounds and shifts the deferred state. The
  shift itself is unchanged.
- `src/codestream/tile.rs`: `read_tile_data` defers the shift when the
  codestream's COD declares the transform and there are three components to
  apply it to, calls the crate's existing `gen_rct_backward` or
  `gen_ict_backward` across those components, choosing by the wavelet, and
  then finishes the deferred shift. ISO/IEC 15444-1 Annex G applies the
  forward transform after the DC level shift, so the inverse runs before the
  shift is undone.
- `src/transform/colour.rs`: `gen_ict_backward`'s four inverse coefficients
  are pinned to equation G-6's literals, 1.402, 0.34413, 0.71414 and 1.772,
  in place of the coefficients derived from the forward matrix. OpenJPEG
  applies the same literals, and the forward path is untouched.
- `src/codestream/tile.rs` also carries a `#[cfg(test)]` module at the
  bounds the new caller keeps and on the coefficients the decode applies.
  `gen_rct_backward` is published code the patch does not change, and the
  `unsafe` pointer loop inside it, at `src/transform/colour.rs`, had no
  caller on any decode path before this patch. The loop is sound under the
  caller's `repeat = width`, the minimum of the three row lengths, which
  never exceeds any of the six line buffers it reads from and writes to: the
  three input lines hold at least `width` samples by the minimum and the
  three output lines are allocated at exactly `width`. A test at the equal
  bound transforms every sample, and a test with a shorter third row caps
  the repeat there and asserts the longer tails untouched. A third test
  pins `gen_ict_backward` to the F-239 plan's AC25 hand values at a 1e-4
  tolerance, which a coefficient swapped in from a different matrix cannot
  pass and which the corpus rows cannot carry because an encoder's
  quantization moves them.

Their SHA-256 digests are:

- `src/transform/colour.rs`: `8d13e0db444ac9b9049b70dc5a65fe42b69568a3aeb717156fe12f7234cda829`
- `src/codestream/tile.rs`: `551f0f6f174c38e663bbff2f68573e602e7a7eff64b8195d64290e9853b59b0a`
- `src/codestream/tile_comp.rs`: `0585e14229f26b83c16fb8e1c7ab1aefb1edef42bdea0f31299cd01cd98c469c`

Every other published file is byte-identical to the archive, and the pins
gate refuses any unrecorded byte. The published suite's 151 lib tests pass
unmodified with the patch applied, and the patch adds three tests of its own
to `tile.rs`: 154 in total, measured 2026-11-07.

The encoder path is unchanged. `openjph-core` 0.1.0's encoder applies no
forward colour transform, which the F-239 fixtures rely on to push
already-transformed components through it, and which leaves the encoded
bytes of every monochrome row exactly what the published package wrote.
