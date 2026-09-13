# F-026 review, pass 1

**Reviewed**: fully staged 129-path working tree against
`35b7ca14afa1091d5668d1a4ccff767f29e5633f`
**Result**: 5 defects, 0 smells, 0 nitpicks

## Defects

### D1, the lossless-only adapter accepts quantized JPEG 2000

**Where**: `crates/ocelli-codec/src/jpeg2000.rs:165` and
`crates/ocelli-codec/src/jpeg2000.rs:254`

**What**: the owned main-header inspector reads SIZ and COD but ignores QCD.
Lossless-only validation therefore requires the reversible transform byte and
nothing that proves absence of quantization.

**Why it is wrong**: DICOM PS3.5 A.4.4 defines UID
`1.2.840.10008.1.2.4.90` as the reversible JPEG 2000 Part 1 mode with no
quantization. A reversible wavelet byte alone is not the full lossless process.
The approved plan states that `.90` permits only that process, but approach
step 3 narrows the check to COD and leaves the QCD half unenforced.

**Evidence**: a temporary integration probe used the public vendored encoder
to create a 64 by 64 constant unsigned 8-bit scalar-quantized codestream, then
changed its COD transform byte from irreversible to reversible. Calling
`Jpeg2000Decoder::lossless_only()` returned `Ok(())` and changed the caller
output. The probe's `assert!(result.is_err())` failed and
`bin/ocelli.sh test ocelli-codec --test jpeg2000
review_probe_lossless_refuses_scalar_quantization -- --nocapture` exited 101.
The probe was removed completely.

### D2, the published-source inventory is a mutable root of trust

**Where**: `scripts/pin_and_size_check.py:231-260`

**What**: the pins check reads every expected source digest from
`PACKAGE-INVENTORY.sha256` in the same vendored directory it is checking. No
constant or independently authenticated digest binds that inventory to the
published archive. Changing a Rust source file and its inventory row together
therefore passes.

**Why it is wrong**: the approved plan requires the exact archive contents to
stay mechanically asserted. D-20 calls the 74-file inventory immutable, and
`docs/SOURCE-POLICY.md` says the pins gate checks the archive provenance and
every inventory hash. A self-consistent rewritten inventory proves neither
archive identity nor unchanged source.

**Evidence**: an isolated copy of the vendor tree was changed by appending a
Rust comment to `src/lib.rs` and replacing that file's inventory digest. A
direct call to `check_ritk_vendor` returned `[]`. The review command used exit
97 as the sentinel for this fail-open and exited 97. The existing
`pins.ritk-inventory` catalogue probe deletes a file without changing the
inventory, so `bin/ocelli.sh gate guards` remained green and did not cover this
route.

### D3, both manifest exceptions permit unrelated edits

**Where**: `scripts/pin_and_size_check.py:257-260` and
`scripts/pin_and_size_check.py:279-289`

**What**: `Cargo.toml` and `Cargo.toml.orig` are wholly exempted from their
published hashes. Their only replacement check is that the parsed
`jpeg-decoder` dependency has `default-features = false`. Any unrelated
package, dependency, target, feature, or build change may coexist with that
boolean and pass.

**Why it is wrong**: the approved plan, D-20, patch provenance, and source
policy all say the vendor differs from the published crate only at the two
declared dependency entries. The guard does not enforce that boundary.

**Evidence**: an isolated vendor copy retained the required dependency flag
but changed the normalized manifest's package description.
`check_ritk_vendor` returned `[]`, and the fail-open sentinel command exited
97. The existing manifest test removes the required boolean, so it cannot
detect an additional edit that leaves the boolean present.

### D4, required JPEG 2000 refusal coverage does not hold the metadata boundary

**Where**: `crates/ocelli-codec/tests/jpeg2000.rs:90-147` and
`.claude/plans/F-026-design.md:165`

**What**: the permanent tests do not exercise the adapter's encapsulated OB
requirement. They also contain no direct cases for a non-one component count,
an invalid transform byte outside zero or one, a decoded sample outside its
stored range, or a wrong caller output length, despite the approved unit row
naming these refusal classes.

**Why it is wrong**: DICOM encapsulated Pixel Data uses OB, and the approved
plan says metadata, range, transform, and output-length mismatches are
permanent atomic refusals. The microscope rule treats a wrong implementation
that keeps its suite green as a defect because that suite would otherwise be
counted as coverage.

**Evidence**: temporarily removing
`desc.pixel_data_vr() != PixelDataVr::Ob` from `validate_descriptor` left the
entire `bin/ocelli.sh test ocelli-codec` command green, with 3 library tests,
5 JPEG 2000 integration tests, and every other codec test passing. The command
exited 0. The mutation was restored completely.

### D5, the approved test table is not completion-ready

**Where**: `.claude/plans/F-026-design.md:162-163`

**What**: the mandatory fixture row cites no DICOM section. The next row names
`crates/ocelli-codec/tests/jpeg2000_corpus.rs`, which does not exist. Its
conformance assertions are actually combined into
`crates/ocelli-codec/tests/jpeg2000.rs`.

**Why it is wrong**: `.claude/commands/complete-feature.md` requires every
fixture row to cite a DICOM section before completion. The design workflow
also requires the section behind each hand-computed pixel expectation. A test
location that does not exist is a false evidence claim.

**Evidence**: `rg -n '^\| fixture' .claude/plans/F-026-design.md` selects the
single uncited row. `test -e
crates/ocelli-codec/tests/jpeg2000_corpus.rs` is false, while the exact and
lossy conformance test is at `crates/ocelli-codec/tests/jpeg2000.rs:149`.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The current vendor bytes themselves are narrow and correct. The independently
  downloaded `ritk-codecs` 0.6.0 archive hashed to
  `5fb65755a819c6ba38bf8aaf146f4ccc23d2fe217c8161a61bf00517c9d1cce5`.
  `diff -ru` exited 1 only for the two declared `jpeg-decoder`
  `default-features = false` edits and the four added files
  `LICENSE-MIT`, `LICENSE-APACHE`, `PACKAGE-INVENTORY.sha256`, and
  `PATCH-PROVENANCE.md`. The tracked vendor contains 78 files and 2,044,376
  bytes.
- The two licence files were fetched independently from VCS revision
  `33497ccd55b44e004c0b8314a1bcc2e0fc9cb3ed`. Both `cmp` calls exited 0.
  Their SHA-256 digests are the two constants carried by the pins check.
- `bin/ocelli.sh gate pins` exited 0. Independent native and
  `wasm32-unknown-unknown` `cargo tree` searches found no resolved `rayon` or
  `rayon-core`. The workspace excludes exactly the vendor path and uses the
  exact local version. The untouched published package lock still contains
  upstream development graph entries, but those are not in either Ocelli
  resolved codec graph.
- No vendored Rust source contains `unsafe`. `bin/ocelli.sh gate unsafe`
  exited 0 over 159 repository files, `gate content` exited 0 with no patient
  data or build artefacts staged, `gate provenance` exited 0 over 707 files,
  and `gate prose` exited 0 over 265 files.
- SIZ extent, component precision and signedness, COD MCT and transform, exact
  EOC, one necessary NULL pad, genuine trailing data, MONOCHROME1 and
  MONOCHROME2, 8-bit and 16-bit containers, and signed and unsigned output were
  traced through the adapter and dependency. The four small fixtures include
  an odd logical codestream with exactly one physical pad byte. The corpus
  streams are even and end directly at EOC.
- The vendored decoder rejects COC and QCC main-header overrides and all
  tile-header coding overrides. That prevents an uninspected component or tile
  transform from replacing COD. It does not cure D1 because its QCD parser
  accepts scalar quantization with a reversible COD transform.
- The `Vec<f32>` result is length-checked before conversion. `exact_i64`
  rejects non-finite and fractional values. Stored-range checks precede all
  8-bit and little-endian 16-bit conversions. Caller output is copied only
  after every sample succeeds. Existing mismatch tests preserve the sentinel
  caller buffer.
- Fresh arithmetic mutation changed the signed 8-bit lower bound from
  `-limit` to `-limit + 1`. The permanent signed-domain fixture rejected its
  exact minimum and `bin/ocelli.sh test ocelli-codec --test jpeg2000
  lossless_decodes_hand_computed_eight_and_sixteen_bit_signed_domains` exited
  101 with `UnsupportedPixelFormat`. The mutation was restored.
- `bin/ocelli.sh test ocelli-codec --test jpeg2000` exited 0 with all 5 tests
  passing after every temporary probe was removed. `bin/ocelli.sh test
  ocelli-codec` and `bin/ocelli.sh clippy ocelli-codec` also exited 0 on the
  reviewed tree.
- `bin/ocelli.sh native` exited 0 through all 8 steps. The same production
  verifier executed `.90` exactness and `.91` divergence natively, then as
  plain wasm and `+simd128` wasm under Node. The resulting release modules are
  145,339 and 144,947 bytes.
- Independent pydicom extraction proved that the two tracked codestream
  fixtures are byte-identical to the manifest-backed synthetic DICOM frames at
  872 and 628 bytes. Their tracked independent decoded raws are also identical
  to pydicom and OpenJPEG output. The lossless raw equals the uncompressed
  12,288-byte truth.
- The native conformance test fixes the `.91` divergence at 418 of 6,144
  samples against uncompressed truth with signed sum 30 and maximum difference
  1. Against independent pydicom and OpenJPEG output it fixes 1,232 differing
  samples, signed sum 730, and maximum difference 1. No difference exceeds 1.
- `bin/ocelli.sh corpus` exited 0 with 92 verified, 0 missing, and 0
  mismatched. `bin/ocelli.sh gate bench` exited 0 with 12 subjects, 3 baseline
  entries, 27 Python tests, and 87 Node tests where the one declared browser
  test was skipped. The separate JPEG 2000 runner independently produced
  0.8049 ms and checksum 1,563,934, inside its recorded host-class baseline.
- F-004 is `done` and now has its named permanent runner. On this review
  surface the production probe reported `NoAdapter` and no fill rate. The
  runner refused to publish a value and exited 1, which is its documented
  honest behavior on a machine with no suitable adapter rather than a
  fabricated benchmark.
- `bin/ocelli.sh gate guards` exited 0 after 337 refusal probes went red, 41
  accept probes stayed green, and 50 controls stayed green. This includes the
  two current ritk probes, but D2 and D3 demonstrate the missing adversarial
  forms.
- `bin/ocelli.sh gate --floor` exited 0. The reviewed feature tree has no
  unstaged difference after all probes and mutations were removed.
