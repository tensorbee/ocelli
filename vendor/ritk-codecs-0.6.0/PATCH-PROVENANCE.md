# ritk-codecs 0.6.0 provenance

This directory contains the published `ritk-codecs` 0.6.0 package from
crates.io. The downloaded `.crate` archive has SHA-256 digest
`5fb65755a819c6ba38bf8aaf146f4ccc23d2fe217c8161a61bf00517c9d1cce5`.
Its `.cargo_vcs_info.json` identifies upstream Git commit
`33497ccd55b44e004c0b8314a1bcc2e0fc9cb3ed` and path
`crates/ritk-codecs`.

`PACKAGE-INVENTORY.sha256` records every file in the unmodified published
package. No published file was removed. Ocelli changes the `jpeg-decoder`
dependency in `Cargo.toml` and `Cargo.toml.orig` to disable its default Rayon
feature, and patches four source files of the `jpeg_2000` module under F-239
and five source files and one published test of the `jpeg_ls` module under
F-243, recorded below. Every other file is unchanged.

## The manifest patch

Ocelli changes only the `jpeg-decoder` dependency in `Cargo.toml` and
`Cargo.toml.orig` to disable its default Rayon feature. Their digests are
recorded in `scripts/pin_and_size_check.py` beside the inventory.

## The F-239 patch, deviation D-31

Four source files carry a recorded patch so that a three-component JPEG 2000
Part 1 codestream decodes, with the codestream's own inverse multiple
component transform applied before the DC level shift is restored. The
published decoder refuses `Csiz != 1` and `MCT != 0` outright, and PS3.5
8.2.4 permits a JPEG 2000 codestream to carry three components whose
photometric names that transform. The patch's own text adds no `unsafe`.

- `src/jpeg_2000/packet/reader.rs`: the packet walk generalises to
  `num_components` components of one shared geometry, and
  `decode_tile_planes` returns every component's level-shifted samples. The
  inverse transform runs inside the decode, choosing the reversible RCT or
  the irreversible ICT by the wavelet, before the irreversible path's single
  rounding. `decode_tile_part` delegates with one component, so its own
  decode is unchanged and an error's text alone gained the component index.
  The file also carries a `#[cfg(test)]` unit test
  pinning `inverse_ict_9_7` to the F-239 plan's AC25 hand values at a 1e-4
  tolerance, which the corpus rows cannot carry because an encoder's
  quantization moves them.
- `src/jpeg_2000/image.rs`: `decode_j2k_fragment_colour` validates a
  three-component codestream against PS3.5 8.2.4 Table 8.2.4-1, decodes all
  three components per tile-part, and restores the DC level shift with the
  range limit the reference decoders apply.
- `src/jpeg_2000/mod.rs` and `src/lib.rs`: the new public entry point
  `decode_jpeg2000_fragment_colour` is exported beside the existing one.

Their SHA-256 digests are:

- `src/jpeg_2000/image.rs`: `8c7a1c8b1147369c60b11fdd3132339cf798b2f4c40327ac00ffd34ca3f30e44`
- `src/jpeg_2000/mod.rs`: `a3e14bca44ec66090df7a1d3a69da08e74f01f2b8cbcd769c9073c300bf7d52a`
- `src/jpeg_2000/packet/reader.rs`: `2ed1bf22d0607915048fcb807f6ccaa599c05eb29057a2b19c95ef73e611efc1`
- `src/lib.rs`: `accfac2f2cfe750a0b8df2f3409e00a46c98ed2eeeef338469a84ad997ed7def`

Every other published file is byte-identical to the archive, and the pins
gate refuses any unrecorded byte. F-239's patch adds one test of its own to
`reader.rs`, and the suite keeps the published refusal of a three-component
codestream through `decode_jpeg2000_fragment`, because the monochrome entry
point is unchanged.

The exact upstream `LICENSE-MIT` and `LICENSE-APACHE` files from the identified
commit are retained here. Ocelli selects the MIT alternative for redistribution.
Their SHA-256 digests are:

- `LICENSE-MIT`: `8ff8d4621b3a1aa9df7006e0eaeacd5dfb686dcaa328e86c692374fbf2c4874f`
- `LICENSE-APACHE`: `b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1`

## F-243, JPEG-LS run mode, context reset, end of scan and default thresholds

The published single-component JPEG-LS decoder returned `Ok` with wrong
samples on flat or banded monochrome frames and on frames encoded from real CT
slices, and never refused a scan that ran out of data. Every undamaged stream
of F-243's generated corpus that CharLS opens decodes with the patch to the
samples CharLS 2.4.4 gives. A damaged scan can read as a valid
scan of another image, which no decoder can detect, so no claim is made that
every damaged scan is refused. Each change below names
what it follows. The clauses of ITU-T T.87, which is ISO/IEC 14495-1, cited
here were checked against T.87 (06/1998) from itu.int, recorded in
`.claude/reviews/F-243-ac14-standard-check.md`, and the upstream clause
citations in the patched files were corrected against that text.
`scripts/pin_and_size_check.py` records each digest in `RITK_PATCHED_HASHES`
and refuses any other byte.

- `src/jpeg_ls/scan.rs`. The run-mode loop, T.87 F.1 step 18: a run hit
  adds `min(2^J[RUNindex], samples left on the line)`, and RUNindex rises only
  when that count is the full segment. The published loop raised it on the hit
  that only fills the end of a line, which desynchronised every later run. A
  miss whose remainder carries the run past the line is refused rather than
  clamped. A decoded prediction error outside the range an encoder reduces it
  to is refused, in regular mode and at a run interruption, where the published
  code wrapped or clamped it to some sample. A scan that runs out of
  entropy-coded data is refused at the end of the line where it ran out. The
  new `verify_scan_end` requires the bits after the last sample to be zero and
  at most 7, the rest of one byte, and EOI to follow, after any 0xFF fill
  bytes (T.87 D.2). A whole byte more is refused, because review pass 2
  measured damaged scans that read as a valid scan one byte shorter opening to
  wrong samples under a 15-bit allowance. CharLS 2.4.4 opens most streams with
  one 0x00 inserted before EOI, so that is a named gap. Of these rules, CharLS
  was measured to open some streams with an error outside the coded range and
  limited-length codes with extra prefix zeros, which Ocelli refuses because
  no encoder writes them.
  SHA-256 `5fa6512032ced5ec593394d65111ae296ecb3702dbdc1dfe178a27dacdd2c559`.
- `src/jpeg_ls/bitstream.rs`, the bit reader. It records that a read asked for
  more bits than the segment holds and where the terminating marker is, and
  reports how many bits are left before that marker, whether all are zero,
  and whether the marker after any fill bytes is EOI. The limited-length
  Golomb code's unary prefix must end with a 1 after `LIMIT - qbpp - 1` zeros.
  SHA-256 `49367ccb7467dd91b267a2dfe5a1f70f70d872bf28243fdbbf817e9a34a68cbb`.
- `src/jpeg_ls/decoder.rs`, one call to `verify_scan_end` after the scan. It is
  called here rather than inside `decode_scan` because a published test of
  `decode_scan` passes entropy-coded bytes with no marker after them.
  SHA-256 `d898917107a8d2575a04cc47ba53748785f04c1b661235f283ede6744f8a4b11`.
- `src/jpeg_ls/context/mod.rs`. The default thresholds follow C.2.4.1.1.1's
  `CLAMP(i, j, MAXVAL)`, which gives `j` when `i` is above MAXVAL or below
  `j`. The published function saturated at MAXVAL, which at P 8 and NEAR 34
  gives T3 255 where the standard gives 177. And at a context RESET B is
  halved with an arithmetic shift, which floors, as CharLS does. The published
  `/ 2` rounds an odd negative B toward zero, which moved the bias correction
  and desynchronised most realistic frames. `update_context` is shared by the
  decoder and the encoder, so both change.
  SHA-256 `7a39478fed96333a1654ce3f8ad4029866abe2e28db26262fc8618e54d586a95`.
- `src/jpeg_ls/encoder.rs`, the mirror of the run-mode change, A.7.1 code
  segment A.16: the hit that only fills the end of a line no longer raises
  RUNindex. Ocelli's shipped paths do not call this encoder. It is fixed so
  that the published round-trip suite pairs it with the fixed decoder rather
  than with the defect.
  SHA-256 `be209fabf20a8b54b53c6156eb4de9922464c5192e600f3a6b88da6d5aa4b988`.
- `src/jpeg_ls/tests/conformance.rs`, **an edit to a published test**, by the
  lead's ruling. `jpeg_ls_fragment_2x2_all_zero_decodes_correctly` builds a 2
  by 2 frame whose scan byte was `0xF8`: four hit bits fill the frame and the
  padding after them carries a 1. CharLS 2.4.4 refuses that stream, and so does
  the patched decoder, so the byte is now `0xF0`, the same four hit bits with
  zero padding. The test's assertion is unchanged.
  SHA-256 `aee89d4a0a49b21d8f6ebc4229e4458cc4c037300dad36babcb17f08f4acc512`.

The patch adds no `unsafe` and no dependency. F-243's patch changes no test
count: `cargo test --lib` over this package passes the same 320 published
tests before and after it, with the one published test edited as above, and
F-239's patch adds one test of its own, for 321 in total on the merged tree.
