# F-028 review, pass 1

**Reviewed**: working tree, `crates/ocelli-codec/src/jpegls.rs`,
`src/sample_convert.rs`, `src/jpeg2000.rs`, `src/lib.rs`,
`examples/decode_jpegls.rs`, `tests/jpegls.rs`,
`tests/fixtures/generate_jpegls.py` and the nine fixtures it writes,
`tools/bench/src/runners/decode_transfer_syntax_jpegls.mjs`,
`docs/spikes/A2-jpeg-ls.md`, `docs/sprints/BACKLOG.md`, and the removal of
`tools/spikes/a2-jpeg-ls/`
**Result**: 0 defects, 3 smells, 0 nitpicks. All three were found by probe and
all three were closed before this file was finished, so the record below is
written as found rather than as it ended.

## Defects

None.

## Smells, all closed in this pass

### S1, the codestream-side multi-component refusal was unreached

**What**: `validate_header`'s `header.components != 1 || header.interleave != 0`
could not fire from any test. The colour fixture used a descriptor with three
samples per pixel, which `validate_descriptor` refuses **before** the codestream
is read, so the header condition never ran.

**Why it matters**: it is the condition that protects against a **single-sample
descriptor paired with a multi-component codestream**, which is the file that
claims to be monochrome and is not. The descriptor check cannot cover it.

**Evidence**: inverting the condition to `components != 3 || interleave != 1`
left the suite green at 12 passed.

**Closed** by generating `fixtures/jpegls_rgb8_ilv1.jls`, a genuinely
multi-component frame: 16 by 16 8-bit RGB encoded line-interleaved, so SOF55
declares `Nf = 3` and SOS declares `ILV = 1`. Decoded against a matching
single-sample monochrome descriptor, every earlier check passes and the header
check is the one that fires. Re-probed: inverting it now fails 11 tests.

### S2, `validate_descriptor`'s conditions were not separately falsifiable

**What**: `validate_descriptor` is a disjunction, and the only fixture reaching
it violated three of its four conditions at once. Removing
`desc.samples_per_pixel() != 1` left the suite green, because the same
descriptor's `"RGB"` photometric interpretation caught it anyway.

**Why it matters**: a disjunction tested only by an input that trips several
arms reports coverage for arms that are doing nothing.

**Evidence**: replacing `desc.samples_per_pixel() != 1` with `false` left 12
passed. The same was true of `!matches!(desc.bits_allocated(), 8 | 16)`.

**Closed** by `each_descriptor_condition_is_refused_on_its_own`, where every row
violates exactly one condition: three samples with a **monochrome**
interpretation, a 32-bit container with everything else valid, and
`PALETTE COLOR` with one sample. Re-probed: each condition now fails a test.

### S3, the stray SOI and EOI guard was unreached

**What**: `inspect_headers` refuses a SOI or EOI where a marker segment is
expected, and nothing reached it.

**Evidence**: defeating the condition left the suite green at 14 passed.

**Closed, and the guard was kept rather than deleted**, because a case exists
where its absence changes the answer. The new fixture is a hand-built stream:
SOI, then a stray EOI whose length bytes claim a four-byte segment, then a
SOF55 and SOS that are individually well formed and agree with the descriptor,
then EOI. Without the guard the walk steps over the stray segment, reads the
following headers as the frame's own, accepts them, and hands the dependency
bytes that are not a scan, so a structurally invalid stream is reported as a
decoder failure instead. Re-probed: defeating the guard now fails that test.

## Nitpicks

None.

## Verified clean

- **Fifteen fixtures pass**, `bin/ocelli.sh clippy ocelli-codec` exits 0, and
  the whole crate is green across its eight targets.
- **Eleven probes, each observed red, each reverted**, with the source restored
  from a byte copy and the suite re-run green afterwards:

  | Probe | Result |
  |-------|--------|
  | NEAR mode check removed | 13 passed, 1 failed |
  | Rescale pin moved to slope 2 intercept 5 | 6 passed, 8 failed |
  | Codestream component check inverted | 3 passed, 11 failed |
  | NEAR read one byte late, landing on ILV | 12 passed, 2 failed |
  | SOF55 precision check removed | 13 passed, 1 failed |
  | Descriptor sample-count check removed | 13 passed, 1 failed |
  | Descriptor container-width check removed | 13 passed, 1 failed |
  | Dimension check removed | 13 passed, 1 failed |
  | SOF55 rows and columns swapped | 10 passed, 4 failed |
  | Odd-length pad acceptance deleted | 13 passed, 1 failed |
  | Stray SOI and EOI guard defeated | 14 passed, 1 failed |

- **The lossless anchor is encoder-independent and it holds.** The `.80` corpus
  row decodes byte-identically to `syntax/explicit_vr_le.dcm`'s Pixel Data. That
  reference's SHA-256 is
  `b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609`, which is
  **exactly the digest `docs/spikes/A2-jpeg-ls.md` recorded for `R`**, so the
  extraction is confirmed against an independently written record rather than
  against itself.
- **The near-lossless bound is asserted in both directions.** Zero samples
  exceed `NEAR = 3`, and the maximum absolute error **equals** 3. The second
  half matters: a decode that came back bit-identical to the reference would
  also satisfy "within the bound" while meaning the near-lossless path had not
  run.
- **The full stored range is proven, not sampled.** The 256 by 256 fixture
  carries all 65,536 unsigned 16-bit values exactly once, the decode is
  byte-identical to the constructed ramp, and the test additionally asserts that
  every distinct value appeared and none appeared twice. That is what makes it a
  statement about the domain rather than about 65,536 samples that might repeat.
- **The rescale pin is checkable from outside the crate.** Sample `n` is exactly
  `n`, which is only true under slope 1 and intercept 0. Moving the pin to slope
  2 and intercept 5 fails eight of fifteen tests, so HLD section 18's
  single-implementation rule is held by a test rather than by a comment.
- **SOF55's `Y` is rows and `X` is columns**, confirmed by reading the corpus
  fixture's own bytes: `P=16 Y=64 X=96 Nf=1`, against a corpus row that
  `scripts/corpus_synth.py` builds as 64 rows by 96 columns. Swapping them in
  the parser fails four tests.
- **`NEAR` is read past the component specifiers, not from the end of the
  segment.** The unit test pins the exact SOS payload `pyjpegls` 1.5.1 emits and
  includes a row where `ILV` is non-zero, so the correct index is separated from
  the plausible one. Reading one byte late fails two tests.
- **The two UIDs are not interchangeable**, asserted in both directions, and
  each still accepts its own codestream so the check refuses the wrong pair
  rather than everything.
- **Registration is atomic.** A collision on the second UID leaves the first
  `KnownUnavailable`, asserted directly. Deviation D-19.
- **Every failure leaves the caller's buffer byte-unchanged**, asserted with a
  surviving `0xa5` fill on every error path in the suite.
- **`sample_convert` is an extraction, not a new abstraction.** `convert_samples`,
  `append_sample` and `exact_i64` moved out of `jpeg2000.rs` unchanged, with
  their unit tests, and now have two call sites instead of one. The alternative
  was a byte-identical copy of the stored-domain boundary in each adapter.
- **No `as` cast, no `unsafe`, no `wasm-bindgen`.** `grep -n ' as '
  crates/ocelli-codec/src/jpegls.rs` finds none, which is the difference gate A2
  predicted between this route and the `ritk-codecs` harness it measured: the
  harness needed one cast because it converted `f32` by hand, and the shared
  `exact_i64` path does it without one.
- **`ocelli-codec` builds for `wasm32-unknown-unknown`** with the adapter in it,
  exit 0.
- **The wasm size budget does not move, and that is a fact rather than an
  omission.** `ocelli-wasm`'s only workspace dependency is `ocelli-core`, so
  `ocelli-codec` is not in the shipped module's graph at all and no rebaseline
  applies. Gate A2 predicted "roughly 40 KB" for the story that registers a
  decoder, and that cost arrives when the render and worker path pulls the codec
  crate in, not here. `bin/ocelli.sh gate wasm` is green unchanged.
- **The benchmark subject is real.** `decode.transfer_syntax.jpegls` already
  existed in `tools/bench/subjects.json` with `subject_story: F-028`, so this
  story supplies the runner the registry was waiting for rather than inventing a
  row. `bin/ocelli.sh gate bench` refused the runner while the backlog still
  said `pending`, which is the anti-fabrication rule working, and passes with
  the row at `in-progress`. Measured: median 0.1221 ms over the 64 by 96
  lossless corpus frame, range 0.1108 to 0.1928.
- **The A2 harness is removed and its answer file says so.**
  `tools/spikes/a2-jpeg-ls/` is deleted per the S09 design round.
  `docs/spikes/A2-jpeg-ls.md` keeps every measurement and gains a note that the
  rig its **Reproducing this** section names no longer exists, so the section
  describes a rig that existed rather than one a reader will fail to find.
