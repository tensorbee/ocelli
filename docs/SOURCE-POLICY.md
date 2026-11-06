# Source policy

**This file is the source-provenance policy and it is normative.**
`scripts/source_provenance_check.py` enforces it over every tracked text file
and every manifest, with no allowlist.

HLD Appendix A gate A6 requires this policy to be agreed in writing **before
any agent touches any repository**. It originated as HLD Appendix C.2.1 and was
moved here when Appendix C was taken out of the repository, because the
appendix around it is commercial analysis and this is a legal and engineering
constraint. Nothing normative was lost in the move, and the table below is
C.2.1's, unchanged.

Why the bar is higher here than in an ordinary project:

> Translating source into Rust is a translation, which is an exclusive right of
> the copyright holder, so a copyleft licence blocks **reading**, not merely
> depending. Agent-assisted development sharpens this: exposure cannot be shown
> to be absent after the fact, which weakens any clean-room position.

## What we may read, and what we may depend on

| Project | Licence | Read? | Depend? |
|---------|---------|-------|---------|
| cornerstone3D | MIT | yes | yes |
| dicom-rs, wgpu | MIT / Apache-2.0 | yes | yes |
| VTK | BSD-3 | yes | yes |
| ITK, elastix | Apache-2.0 | yes | yes |
| DCMTK | BSD-style (OFFIS) | yes | yes |
| OpenJPEG, CharLS, OpenJPH | BSD-2 / BSD-3 | yes | yes |
| BlueLight, dicom-microscopy-viewer | MIT | yes | yes |
| NiiVue | BSD-2 | yes | yes |
| Neuroglancer | Apache-2.0 | yes | yes |
| **dwv** | GPL-3.0 | **NO** | no |
| **Horos** | LGPL-3 with a linked AGPL-3 component (Grok) | **NO** | no |
| **Grok JPEG 2000** | AGPL-3 | **NO** | no |

**Three entries are read-blocked: dwv, Horos and Grok**, as the table's "Read?"
column says and as `scripts/source_provenance_check.py` enforces. None may be
opened by a person or an agent on this project.
Where their ideas are worth having, and dwv's annotations-as-DICOM-SR certainly
is, take them from the standard, which is where those projects took them from.
Grok is blocked to read as well as to depend on because Horos links it, and an
AGPL component in a browser-delivered product would trigger network-use
disclosure, which is close to a worst case.

## Third-party obligations

Track them from day one. Bridging to `@cornerstonejs/codec-charls` is fine and
so are OpenJPEG, CharLS and OpenJPH, all permissive. Grok is not. Attribution
files are cheap to maintain incrementally and expensive to reconstruct.

## Which product names appear here, and why

Three categories, and only one of them is redacted.

**Commercial competitor names are removed.** They are competitive intelligence
rather than engineering. The source material was sanitized before the tracked
Markdown became authoritative. Each replacement keeps the point the sentence
was making, because a vendor's published memory constant is evidence for a
design decision and the evidence survives without the name.

**Read-blocked projects are named, deliberately.** A policy whose function is to
say what must not be opened cannot do that without naming them. HLD gate A6
requires this agreed in writing, and an unnamed prohibition is not one.

**Dependencies are named, necessarily.** `Cargo.toml` declares several by name,
the codec registry is specified in terms of its decoders, and two of the six
spike gates are questions about specific libraries. Removing them would stop
the documents describing the software.

## The rule for anything not yet listed

**No licence is not the same as permissive.** A repository with no LICENSE
file, and no `license` field in its metadata, is **all rights reserved** by
default under the Berne Convention. It may be read as a person reads any
published page. It may not be copied into this repository, in whole or in
part, and its text may not be used as the base for a file here.

Before taking anything from a source not in the table above, check three
things and record the answer:

1. Is there a LICENSE, LICENCE, COPYING or NOTICE file at the repository root?
2. Does the hosting platform's metadata report a licence?
3. Does the licence permit the specific use, which for source is usually
   **derivative works**, not merely use?

If 1 and 2 are both absent, the answer is no.

## What a dependency is

The rule covers dependencies declared in tracked manifests and lockfiles, plus
libraries those dependencies bundle. It does not cover tools that are
downloaded or installed on a developer or CI host and never redistributed.
Playwright browsers, ffmpeg, git, bash and CI images are in that second group.

## Extensions to the table

A source assessed after this policy was written gets a row here, in the same
form as the table above, with the date it was decided.

| Source | Licence | Read? | Depend? | Decided |
|--------|---------|-------|---------|---------|
| TCIA collection CMB-MML | CC BY 4.0 | yes | yes | 2026-09-04 |
| TCIA collection EAY131 | CC BY 4.0 | yes | yes | 2026-09-04 |
| TCIA collection VAREPOP-APOLLO | CC BY 4.0 | yes | yes | 2026-09-04 |
| TCIA collection CMB-CRC | CC BY 4.0 | yes | yes | 2026-09-04 |
| pydicom bundled test files | not stated | yes | **NO** | 2026-09-04 |
| openjph-core 0.1.0 | BSD-2-Clause in registry metadata | yes | yes | 2026-09-06 |
| flate2 | MIT OR Apache-2.0 in registry metadata | yes | yes | 2026-09-07 |
| oxideav-mjpeg 0.1.8 | MIT, packaged licence and repository metadata | yes | yes | 2026-09-10 |
| oxideav-core 0.1.35 | MIT, packaged licence and repository metadata | yes | yes | 2026-09-10 |
| ritk-codecs 0.6.0 | MIT OR Apache-2.0 in registry metadata | yes | yes | 2026-09-10 |
| openjph-core 0.1.0, production use | BSD-2-Clause in registry metadata, no packaged notice | yes | yes, not shippable | 2026-09-13 |
| openjph-core 0.1.0, notice obtained | BSD-2-Clause, upstream LICENSE at tag 0.26.3 | yes | yes | 2026-11-05 |
| imagecodecs 2026.10.10 | BSD-3-Clause | yes | yes, tooling only | 2026-11-05 |
| github/choosealicense.com | CC BY 3.0 for the copied content | yes | yes, terms template | 2026-11-05 |
| apache.org | Apache-2.0 | yes | yes, published licence text | 2026-11-05 |
| madler/zlib | zlib terms, GitHub reports no SPDX id | yes | yes, terms template | 2026-11-05 |
| rust-docs | Apache-2.0 or MIT, toolchain notice | yes | yes, pinned HTML | 2026-11-05 |
| madsmtm/objc2 | MIT or Zlib OR Apache-2.0 OR MIT, GitHub reports no SPDX id | yes | yes, pinned LICENSE.md and templates | 2026-11-05 |
| brendanzab/gl-rs | Apache-2.0 | yes | yes, pinned licence files | 2026-11-05 |
| jni-rs/jni-sys | Apache-2.0 on GitHub, MIT file pinned | yes | yes, pinned licence file | 2026-11-05 |
| rust-mobile/ndk | Apache-2.0 on GitHub, MIT file pinned | yes | yes, pinned licence file | 2026-11-05 |
| aclysma/profiling | Apache-2.0 on GitHub, MIT file pinned | yes | yes, pinned licence file | 2026-11-05 |
| gfx-rs/rspirv | Apache-2.0 | yes | yes, pinned licence file | 2026-11-05 |
| spdx/license-list-data | not stated | yes | **NO** | 2026-11-05 |
| jxl-oxide 0.12.6 and its twelve jxl-* subcrates | MIT OR Apache-2.0 in each package's registry metadata, packaged licence files in every one | yes | yes | 2026-11-05 |
| brotli-decompressor 5.0.3 | BSD-3-Clause/MIT in registry metadata (the legacy `/` separator), packaged licence file | yes | yes | 2026-11-05 |
| alloc-no-stdlib 2.0.4 and alloc-stdlib 0.2.4 | BSD-3-Clause in registry metadata, alloc-stdlib ships no licence file | yes | yes, not shippable | 2026-11-05 |

### Shipped binary notice sources

F-220 copies licence texts into `ci/notices/` and names each source in
`ci/notices/sources.toml`. The rows above use those names. The date is
2026-11-05.

1. **Is there a licence file?** github/choosealicense.com has `LICENSE.md` at
   the repository root for its source code and `_licenses/mit.txt` on the
   gh-pages branch. The copied content is `_licenses/mit.txt`. The MIT
   template follows the terms in that file and replaces the copyright line
   with one marker. The content of the project is Creative Commons
   Attribution 3.0 Unported. The source code is MIT. Attribution names
   GitHub, the title choosealicense.com, the file `_licenses/mit.txt`, the
   URI https://github.com/github/choosealicense.com/blob/gh-pages/_licenses/mit.txt
   and the licence URI https://creativecommons.org/licenses/by/3.0/. The
   change is the replacement of the copyright line. apache.org publishes
   `https://www.apache.org/licenses/LICENSE-2.0.txt`. The Apache-2.0 pin is
   that text. madler/zlib tag v1.3.1 has `LICENSE`. The Zlib template follows
   its terms and omits that project's copyright line and sign-off. It carries
   one marker. rust-docs installs `share/doc/rust/COPYRIGHT-library.html` for
   the recorded toolchain. The pin is that file. madsmtm/objc2 at each locked
   crate's published commit has `LICENSE.md` and no licence text. The pin for
   each crate is `LICENSE.md` at that commit. The notice then uses the
   elected template because `LICENSE.md` names no holder and holds no terms.
   The four MIT-only crates also reproduce the repository root `LICENSE.txt`
   from the parent of commit `cfb19922`, which names Copyright (c) Steven
   Sheldon. brendanzab/gl-rs has `LICENSE` at the commits recorded for
   gl_generator and khronos_api. jni-rs/jni-sys has `LICENSE-MIT` at the
   commit recorded for jni-sys-macros. rust-mobile/ndk has `LICENSE-MIT` at
   the commit recorded for ndk-sys. aclysma/profiling has `LICENSE-MIT` at
   the commit recorded for profiling. gfx-rs/rspirv has `LICENSE` at the
   commit recorded for spirv. spdx/license-list-data tag v3.29.0 has no
   `LICENSE`, `LICENCE`, `COPYING` or `NOTICE` at the repository root.
2. **Does the hosting platform's metadata report a licence?** GitHub reports
   MIT for the source code of github/choosealicense.com. The README of that
   project states that its content, including `_licenses/mit.txt`, is
   Creative Commons Attribution 3.0 Unported. GitHub reports Apache-2.0 for
   brendanzab/gl-rs, jni-rs/jni-sys, rust-mobile/ndk, aclysma/profiling and
   gfx-rs/rspirv. It reports no SPDX id for madsmtm/objc2 and madler/zlib.
   apache.org is the Apache Software Foundation's published text and is not a
   GitHub repository. rust-docs is a rustup component. The HTML it installs
   says the standard library is dual-licensed under Apache 2.0 and MIT terms.
   GitHub reports no licence for spdx/license-list-data.
3. **Does the licence permit the use?** The use is copying a licence text
   into the shipped notice. For the choosealicense content the use is copying
   `_licenses/mit.txt` and replacing the copyright line with one marker.
   Creative Commons Attribution 3.0 Unported permits that adaptation when the
   attribution in the answer above is kept, including the licence URI and a
   note of the change. For the MIT and Zlib templates the use is copying the
   terms and filling one copyright line. Apache-2.0 is copied as published
   and its appendix line is not filled. MIT, Apache-2.0 and the zlib terms
   permit that copying with the notice kept. The crate pins named above are
   the notices for those crates. `LICENSE.md` from madsmtm/objc2 is the
   repository's statement at the published commit and is reproduced with the
   elected template. spdx/license-list-data has no licence file and GitHub
   reports no licence, so nothing from it was copied.

### S08 codec dependencies

`oxideav-mjpeg` 0.1.8 carries an MIT `LICENSE` file in both its package and
repository. Its package metadata names the same public repository and licence.
MIT permits derivative works and redistribution with the notice retained.
The published standalone decoder entry point is not public in this version, so
F-024 enables only its `registry` feature in addition to disabling defaults.
That route resolves `oxideav-core` 0.1.35, which carries the same MIT licence
evidence. An audit of the published source found no unsafe code in
`oxideav-mjpeg` and eleven unsafe sites in `oxideav-core` arena support, four
unsafe implementations and seven unsafe blocks. The audit matches whole-word
`unsafe` over every `*.rs` file below each exact published package's `src/`
directory. Those are external dependency sites and do not change the
repository's unsafe allowlist.

`ritk-codecs` 0.6.0 is vendored from the crates.io archive whose SHA-256 is
`5fb65755a819c6ba38bf8aaf146f4ccc23d2fe217c8161a61bf00517c9d1cce5`.
Its package metadata names VCS revision
`33497ccd55b44e004c0b8314a1bcc2e0fc9cb3ed`. The archive's exact 74-file
inventory is retained. No published file is removed and no Rust source is
changed. Only `Cargo.toml` and `Cargo.toml.orig` disable `jpeg-decoder` default
features so the native and wasm graphs contain no Rayon. Exact upstream MIT
and Apache-2.0 texts from that revision are retained with the package. Ocelli
selects the MIT alternative for redistribution. The pins gate checks every
inventory hash and binds the complete 74-row inventory to an immutable digest
outside the vendor tree. It binds the complete bytes of both patched manifests
so unrelated edits cannot accompany the sole declared default-feature patch.
It also binds the complete `PATCH-PROVENANCE.md` bytes to the external
SHA-256 constant
`3161785b1cbf76437756c68d13ba6b5d53dfc26f88f0dd285d2af0e395aedeff`,
so the human-readable patch account cannot gain an undeclared claim.
It also checks both licence hashes, the archive and VCS provenance, workspace
exclusion, exact path and version, and both no-Rayon graphs.

### openjph-core 0.1.0, and the missing packaged licence text

F-X013 checked the crates.io package before reading its implementation. The
archive checksum is
`c8b96ed12b3d41623a771af4af8131abf353bc822b7a567c6ef3b35ab967a36d`,
which matches the crates.io API. Packaged VCS metadata names commit
`7ed6d6d110d994ec740aacaa90a78b2e807c4c24` under path `openjph-core`.

1. **Is there a licence file?** No. The published archive has no `LICENSE`,
   `LICENCE`, `COPYING` or `NOTICE`, and crates.io reports no repository URL
   where a root file could be checked.
2. **Does the platform metadata report a licence?** Yes. Both the normalized
   packaged manifest and crates.io report the SPDX expression
   `BSD-2-Clause`. The crates.io publisher is Michael Knopke, login `knopkem`,
   whose crates.io identity matches GitHub. The package says it is a pure Rust
   port of OpenJPH v0.26.3, which is already in the policy's permitted table.
3. **Does the licence permit the use?** Yes. BSD-2-Clause permits use,
   modification and redistribution with its notice conditions retained.

Question 2 is present, so the policy's rule for two absent provenance sources
does not refuse the package. F-217 placed the upstream notice at
`vendor/openjph-core-0.1.0/LICENSE`. The redistribution check passes.

**F-027 took this package into production use and the risk is now held by a
gate rather than by this paragraph.** The three answers above were re-verified
independently rather than copied: the archive fetched on 2026-09-13 has SHA-256
`c8b96ed12b3d41623a771af4af8131abf353bc822b7a567c6ef3b35ab967a36d`, matching
the recorded value, the crates.io API reports `BSD-2-Clause`, not yanked, and
4,863 downloads against F-X013's 3,703, and the archive's fifty files contain
no `LICENSE`, `LICENCE`, `COPYING`, `NOTICE` or `README`.

`python3 scripts/pin_and_size_check.py --require-redistribution` refuses while
that material is absent, and `/release` step 5 runs it. **No development
profile does**, because `--floor`, `--sprint` and `--all` gate development and
this gates publication, which is the difference between depending on a package
and shipping it. The refusal names the file that closes it. F-217 placed the upstream OpenJPH
LICENSE from tag 0.26.3 at `vendor/openjph-core-0.1.0/LICENSE` and copied those
bytes into `THIRD_PARTY_NOTICES`. `python3 scripts/pin_and_size_check.py
--require-redistribution` now passes. Probe `pins.openjph-notice` deletes that
file and must refuse. Probe `pins.openjph-notice-hash` appends a newline and
must refuse on the recorded hash. The text referring this condition to "E2.6"
was a misnumbering in
F-X013's answer file: E2.6 is F-014, the quirk-capture workflow, and the HTJ2K
production story is E4.5, F-027.

### openjph-core 0.1.0, the unsafe audit

The published package is 46 Rust files. Counted with
`scripts/unsafe_allowlist_check.py`'s own scanner, which strips comments and
string literals before matching the keyword, it contains **104** `unsafe`
constructs in nine files:

| File | Constructs | Reachable on wasm32 |
|------|-----------|---------------------|
| `src/transform/wavelet.rs` | 28 | yes |
| `src/transform/simd/x86.rs` | 16 | no, `x86_64` only |
| `src/transform/colour.rs` | 14 | yes |
| `src/transform/simd/neon_colour.rs` | 10 | no, `aarch64` only |
| `src/transform/simd/x86_colour.rs` | 10 | no, `x86_64` only |
| `src/mem.rs` | 8 | yes |
| `src/transform/simd/neon.rs` | 8 | no, `aarch64` only |
| `src/coding/simd/neon.rs` | 6 | no, `aarch64` only |
| `src/coding/simd/x86.rs` | 4 | no, `x86_64` only |

**Fifty of the 104 are on paths a browser build reaches**, the scalar memory,
wavelet and colour code. The package has no wasm32 or `simd128` implementation,
so the architecture files are compiled out there. This reproduces F-X013's audit
exactly, including its split, and it was re-counted rather than quoted.

**Ocelli adds no unsafe of its own**, and this vendored surface is now visible
rather than invisible: `scripts/unsafe_allowlist_check.py` records the per
package count, refuses a vendored package with no record, refuses one whose
count moves, and prints both in its OK line. Before F-027 a dependency's unsafe
was unreachable by that guard whether vendored or not, so the guard's stated
purpose, that a device-submission reviewer reads two files, was weaker than it
read. It is now qualified by a number the build holds.

### jxl-oxide 0.12.6, the unsafe audit

F-229 adds the JPEG XL decode closure. The audited set is what
`cargo tree -p ocelli-codec -e normal` shows this story added against base
commit `4165d1e8`, the sprint's merge base before this branch integrated, read
from `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f` at the exact
`Cargo.lock` versions. It is twenty packages. Thirteen are `jxl-*`.
`brotli-decompressor` 5.0.3 arrives with `alloc-no-stdlib` 2.0.4 and
`alloc-stdlib` 0.2.4. `tracing` 0.1.44 arrives with `tracing-core` 0.1.36,
`once_cell` 1.21.4 and `pin-project-lite` 0.2.17 because the `jxl-*`
subcrates depend on tracing directly, every one of them but
`jxl-oxide-common`, which reaches it through `jxl-bitstream`, and those four
were already locked at the same versions for other workspace members, so they
are new to this closure without being new to the lock.

Counted with `scripts/unsafe_allowlist_check.py`'s own scanner, the module's
`strip_noise` and its whole-word `unsafe` regex, which strips comments and
string literals before matching the keyword, string continuations included,
over every `*.rs` file below each package's `src/` directory. The scanner's
string pattern was fixed in this story: without the continuation-aware rule it
re-paired quotes across a literal that ends a line with a backslash and
blanked the code after it, which hid one construct each in `tracing-core`'s
`callsite.rs` and `metadata.rs`. The twenty packages hold 207 Rust files
there. **534 `unsafe` constructs sit in 70 of them:**

| Package | File | Constructs | Reachable on wasm32 |
|---------|------|-----------|---------------------|
| alloc-no-stdlib 2.0.4 | `src/lib.rs` | 6 | yes |
| alloc-no-stdlib 2.0.4 | `src/bin/example.rs` | 1 | no, binary target |
| alloc-no-stdlib 2.0.4 | `src/bin/tests.rs` | 1 | no, binary target |
| alloc-stdlib 0.2.4 | `src/heap_alloc.rs` | 1 | no, `cfg(feature = "unsafe")` off |
| alloc-stdlib 0.2.4 | `src/bin/example.rs` | 1 | no, binary target |
| brotli-decompressor 5.0.3 | `src/ffi/mod.rs` | 23 | no, `ffi-api` off |
| brotli-decompressor 5.0.3 | `src/ffi/alloc_util.rs` | 18 | no, `ffi-api` off |
| brotli-decompressor 5.0.3 | `src/memory.rs` | 9 | no, `unsafe` feature off |
| brotli-decompressor 5.0.3 | `src/bin/brotli-decompressor.rs` | 6 | no, binary target |
| brotli-decompressor 5.0.3 | `src/decode.rs` | 1 | no, `unsafe` feature off |
| brotli-decompressor 5.0.3 | `src/ffi/interface.rs` | 1 | no, `ffi-api` off |
| jxl-bitstream 1.1.0 | `src/bitstream.rs` | 1 | yes |
| jxl-coding 1.0.1 | `src/ans.rs` | 2 | yes |
| jxl-color 0.11.0 | `src/tf/pq.rs` | 22 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/convert/tone_map.rs` | 20 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/fastmath/powf.rs` | 12 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/convert/gamut_map.rs` | 7 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/fastmath/rational_poly.rs` | 4 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/gamut.rs` | 4 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/tf/bt709.rs` | 4 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/tf/rec2408.rs` | 4 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/tf/srgb.rs` | 4 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/tf.rs` | 4 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/xyb.rs` | 4 | no, `x86_64` and `aarch64` regions only |
| jxl-color 0.11.0 | `src/ycbcr.rs` | 4 | no, `x86_64` and `aarch64` regions only |
| jxl-grid 0.6.2 | `src/simd.rs` | 80 | 15 yes, 35 `x86_64` only, 15 `aarch64` only, 15 `+simd128` only |
| jxl-grid 0.6.2 | `src/mutable_subgrid.rs` | 18 | yes |
| jxl-grid 0.6.2 | `src/shared_subgrid.rs` | 9 | yes |
| jxl-modular 0.11.3 | `src/transform/squeeze.rs` | 40 | 10 `+simd128` only, 24 `x86_64` only, 6 `aarch64` only |
| jxl-modular 0.11.3 | `src/transform/rct.rs` | 12 | 4 yes, 4 `x86_64` only, 4 `aarch64` only |
| jxl-oxide 0.12.6 | `src/fb.rs` | 2 | yes |
| jxl-render 0.12.4 | `src/vardct/x86_64/dct/mod.rs` | 29 | no, `x86_64` only |
| jxl-render 0.12.4 | `src/vardct/aarch64/dct.rs` | 16 | no, `aarch64` only |
| jxl-render 0.12.4 | `src/vardct/wasm32/dct.rs` | 15 | wasm32 with `+simd128` only |
| jxl-render 0.12.4 | `src/vardct/x86_64/transform.rs` | 7 | no, `x86_64` only |
| jxl-render 0.12.4 | `src/vardct/aarch64/transform.rs` | 6 | no, `aarch64` only |
| jxl-render 0.12.4 | `src/vardct/wasm32/transform.rs` | 6 | wasm32 with `+simd128` only |
| jxl-render 0.12.4 | `src/vardct/transform_common.rs` | 5 | yes |
| jxl-render 0.12.4 | `src/filter/gabor.rs` | 4 | yes |
| jxl-render 0.12.4 | `src/filter/epf.rs` | 3 | yes |
| jxl-render 0.12.4 | `src/filter/impls/aarch64/epf.rs` | 3 | no, `aarch64` only |
| jxl-render 0.12.4 | `src/filter/impls/aarch64.rs` | 3 | no, `aarch64` only |
| jxl-render 0.12.4 | `src/filter/impls/wasm32/epf.rs` | 3 | wasm32 with `+simd128` only |
| jxl-render 0.12.4 | `src/filter/impls/x86_64/epf_sse41.rs` | 3 | no, `x86_64` only |
| jxl-render 0.12.4 | `src/filter/impls/x86_64.rs` | 3 | no, `x86_64` only |
| jxl-render 0.12.4 | `src/filter/impls/aarch64/gabor.rs` | 1 | no, `aarch64` only |
| jxl-render 0.12.4 | `src/filter/impls/generic.rs` | 1 | yes, the plain wasm32 fallback arm |
| jxl-render 0.12.4 | `src/filter/impls/wasm32.rs` | 1 | wasm32 with `+simd128` only |
| jxl-render 0.12.4 | `src/filter/impls/x86_64/gabor_avx2.rs` | 1 | no, `x86_64` only |
| jxl-render 0.12.4 | `src/vardct/aarch64/mod.rs` | 2 | no, `aarch64` only |
| jxl-render 0.12.4 | `src/vardct/x86_64/mod.rs` | 2 | no, `x86_64` only |
| jxl-render 0.12.4 | `src/vardct/generic/transform.rs` | 1 | yes, the plain wasm32 fallback arm |
| jxl-vardct 0.11.1 | `src/hf_pass.rs` | 2 | yes |
| jxl-vardct 0.11.1 | `src/dct_select.rs` | 1 | yes |
| once_cell 1.21.4 | `src/race.rs` | 14 | yes, the `race` feature is on |
| once_cell 1.21.4 | `src/lib.rs` | 11 | yes |
| once_cell 1.21.4 | `src/imp_std.rs` | 10 | 8 yes, 2 `#[cfg(test)]` module |
| once_cell 1.21.4 | `src/imp_pl.rs` | 9 | no, `parking_lot` off |
| once_cell 1.21.4 | `src/imp_cs.rs` | 3 | no, `critical-section` off |
| pin-project-lite 0.2.17 | `src/lib.rs` | 9 | yes, bodies of the generated-code macro |
| tracing 0.1.44 | `src/instrument.rs` | 5 | yes |
| tracing 0.1.44 | `src/lib.rs` | 1 | yes |
| tracing 0.1.44 | `src/span.rs` | 1 | yes |
| tracing-core 0.1.36 | `src/subscriber.rs` | 9 | yes |
| tracing-core 0.1.36 | `src/spin/once.rs` | 7 | no, `not(feature = "std")` module |
| tracing-core 0.1.36 | `src/spin/mutex.rs` | 4 | no, `not(feature = "std")` module |
| tracing-core 0.1.36 | `src/dispatcher.rs` | 3 | yes |
| tracing-core 0.1.36 | `src/callsite.rs` | 2 | yes |
| tracing-core 0.1.36 | `src/metadata.rs` | 2 | 1 yes in release, `not(debug_assertions)`, 1 no, `#[cfg(test)]` module |
| tracing-core 0.1.36 | `src/lazy.rs` | 1 | no, `not(feature = "std")` module |

**138 of the 534 compile into a plain wasm32 build**: the grid and allocator
support, the entropy coder, the framebuffer, the shared filter and VarDCT
helpers, and the tracing and once_cell support the parse itself calls.
**Fifty more compile only when the build enables `+simd128`**, the wasm32 SIMD
implementations of `jxl-grid`, `jxl-modular` and `jxl-render`. The shipped
`bin/ocelli.sh wasm` module enables neither half, and no shipped module
carries any of this closure today: neither `ocelli-wasm` nor `ocelli-native`
reaches `ocelli-codec`, which
`cargo tree -p ocelli-wasm -i ocelli-codec --target all` and the same command
for `ocelli-native` both answer as a package that matches nothing in their
graphs, and the reverse tree names `ocelli-folder-viewer` and
`ocelli-oracle`. The other **346 are compiled out
of a wasm32 build**: the `x86_64` and `aarch64` specialisations,
`brotli-decompressor`'s ffi and `unsafe`-feature code, `alloc-stdlib`'s unsafe
allocator, the four `src/bin` files with their nine constructs, `once_cell`'s
`critical-section` and `parking_lot` backends and its `#[cfg(test)]` module,
and `tracing-core`'s no-std `spin` and `lazy` code and its own `#[cfg(test)]`
module.

Each reachability was decided by reading every `#[cfg(...)]` attribute on the
item that holds the construct, on its module declaration and on every parent
module up to the crate root, against the resolved feature set of
`cargo tree -p ocelli-codec -e features`. In this build the `jxl-*` crates
resolve with their own defaults and without `jxl-oxide`'s `rayon`, which the
workspace pin turns off. `tracing` resolves with `std`, `tracing-core` with
`std` and `once_cell`, `once_cell` with `default`, `race`, `alloc` and `std`,
and `brotli-decompressor` with `default`, `std` and `alloc-stdlib`. A target
other than `x86_64` or `aarch64` compiles no architecture region, and wasm32
compiles a `target_family = "wasm"` region only with
`target_feature = "simd128"`. A dependency's binary targets are never built,
so the `src/bin` files are unreachable however they are gated. `jxl-color` is
the whole-package case the openjph audit had per file: all of its unsafe sits
in `x86_64` or `aarch64` regions, the `x86_64` and `aarch64` arms behind
runtime `is_x86_feature_detected!` and `is_aarch64_feature_detected!`
dispatch except the SSE2 baseline arms, which need no detection because
`x86_64` implies SSE2, so none of it is compiled on wasm32. **No file was
left undecided.** Every construct sits either in an ungated item, in a region
whose attribute this paragraph quotes, or in one of the counted exceptions
the column names. The five mixed files, `jxl-grid`'s `simd.rs`,
`jxl-modular`'s `squeeze.rs` and `rct.rs`, `once_cell`'s `imp_std.rs` and
`tracing-core`'s `metadata.rs`, were counted region by region against the
stripped source, with the region boundaries at the cfg attributes, and the
parts sum to each file's total.

**Ocelli adds no unsafe of its own and the repository's allowlist is
unchanged.** This closure resolves from the registry rather than the vendor
tree, so the per-package ratchet in `scripts/unsafe_allowlist_check.py`, which
records `ritk-codecs-0.6.0` and `openjph-core-0.1.0`, does not see it, and the
record here is the audit surface. The versions are held the way D-28 records
them: `=0.12.6`, `=0.12.4`, `=1.1.0` and `=1.0.0` pinned in the workspace
manifest with default features off, the rest locked in `Cargo.lock`, and the
pins gate's no-Rayon graphs over both targets.

The malformed-input record is AC16's test family in
`crates/ocelli-codec/tests/jpegxl.rs`, taken as the tests are. The scan is
`malformed_jpegxl_never_panics`. For each of its fourteen fixtures it applies
every truncation at every byte offset and 2,000 seeded single-bit flips, and
requires every input to return `Ok` with a full write, proved by two sentinel
buffers that must agree, or `Err` with `out` untouched, and never a panic.
Panics are caught, recorded with their site and reported together, and a panic
on any case stops the story (decision 12). `.111` is not scanned because it
never reaches `jxl-oxide`, and the recompression fixture is scanned as
`.112`.

| Malformed input | What it exercises | Asserted outcome |
|-----------------|-------------------|------------------|
| `jpegxl_lossless_16.jxl` as `.110` | the 16-bit monochrome lossless parse and its ramp write | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_lossless_12.jxl` as `.110` | the 12-bit stored-value parse | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_lossless_rgb8.jxl` as `.110` | the colour modular parse | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_lossy_12.jxl` as `.112` | the VarDCT lossy render and the quantiser | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_orient6.jxl` as `.112` | the orientation refusal | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_two_frames.jxl` as `.112` | the one-keyframe rule | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_alpha.jxl` as `.112` | the extra-channel refusal | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_icc.jxl` as `.112` | the ICC pre-check on a crafted stream | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_icc_compressed.jxl` as `.112` | the pre-check on the real libjxl profile stream | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_pq.jxl` as `.112` | the PQ transfer function refusal | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_float.jxl` as `.112` | the float-sample refusal | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_container.jxl` as `.112` | container extraction | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_modular_xyb.jxl` as `.112` | lossy modular with an XYB colour encoding | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| `jpegxl_recompression.jxl` as `.112` | the codestream parse with reconstruction data present, the `jbrd` box stripped | `Ok` with a full write or `Err` with `out` unchanged, never a panic |
| the `.111` fixture intact, a bit flip of its byte 100, and its one-byte prefix | `jpegxl_recompression_is_unavailable`, that `.111` is refused before any byte reaches `jxl-oxide` | `KnownUnavailable`, `out` untouched, never a panic |
| a crafted 26-byte codestream declaring `1 << 28` encoded ICC bytes, the same in an exact-size `jxlc` box, and the same split over `jxlp` boxes | `declared_icc_refuses_unsupported_before_any_parse`, the pre-check refusing the ICC flag before any parse that could grow | `UnsupportedPixelFormat` under both UIDs |
| the same crafted stream inside a `jxlc` box whose declared size runs 1,000 bytes past the input | `declared_icc_refuses_unsupported_before_any_parse`, the walk refusing an oversized box before the pre-check runs | `InvalidCodestream` |
| `jpegxl_icc_compressed.jxl`, the real libjxl 0.12.0 stream with an embedded Display P3 profile | `libjxl_icc_profile_refuses_unavailable`, a valid ICC stream reported unavailable rather than corrupt | `UnsupportedPixelFormat` under both UIDs |
| an oversized `jxlc` box whose codestream stops five bytes into the header | `container_header_cut_short_refuses_before_jxl_oxide`, failing closed before the dependency sees a byte | `InvalidCodestream` |
| a container with no codestream, two `jxlc` boxes, a `jxlc` beside `jxlp` boxes, out-of-order `jxlp` indexes, a `jxlp` after the final one, and a final `jxlp` with its last flag cleared | `containers_that_break_the_extraction_rules_refuse`, the extraction rules refusing before the decode | `InvalidCodestream` for each |
| `jxlp` indexes 0 then 5 with the second flagged last, and a single `jxlp` carrying index 1 | `jxlp_index_order_refuses_out_of_order_indexes`, the index rule alone refusing complete codestreams | `InvalidCodestream` |
| a `jxlc` followed by an empty last-flagged `jxlp` | `jxlp_after_a_jxlc_refuses` | `InvalidCodestream` |
| two last-flagged `jxlp` boxes indexed 0 then 1 | `jxlp_after_the_final_jxlp_refuses` | `InvalidCodestream` |
| a last-flagged `jxlp` followed by a `jxlc` | `jxlc_after_a_final_jxlp_refuses` | `InvalidCodestream` |
| two `jxlc` boxes each carrying half the codestream | `two_jxlc_boxes_refuse`, the one-`jxlc` rule | `InvalidCodestream` |
| nine junk bytes after a container, forty junk bytes after two different containers, a `jxlc` declaring 1,000 bytes more than it carries, and an oversized `Exif` box after a `jxlc` | `boxes_running_past_the_input_refuse`, a box size running past the input refused rather than shortened | `InvalidCodestream` for each |
| the valid container fixture plus one `0x00` byte | `container_trailing_byte_is_trailing_data`, the even-length trailing rule | the unpadded container decodes to the ramp, the padded one refuses `TrailingData` |
| a container with a `brob` box carrying invalid Brotli beside the codestream, and one with a `jbrd` box carrying neither header nor Brotli | `brob_box_beside_a_codestream_is_ignored` and `jbrd_box_beside_a_codestream_is_ignored`, that no auxiliary box is ever inflated or parsed | both decode to the reference bytes |
| a header-only codestream declaring 256 extra channels with 1,071-byte names | `extra_channel_header_refuses_before_the_reparse`, the pre-check's extra-channel refusal ahead of the re-parse | `UnsupportedPixelFormat`, `out` unchanged |
| an eight-byte header-only codestream declaring 65535 by 65535 | `oversized_header_refuses_before_render`, the size check preceding any allocation | `FrameMismatch` |
| a bit flip that declares 15 bits per sample over the 16-bit ramp, against a descriptor of Bits Stored 15 | `samples_above_declared_bits_stored_refuse`, the range check preceding any write | `UnsupportedPixelFormat` |
| rewritten headers declaring an XYB and an unknown colour encoding | `codestream_colour_space_other_than_rgb_or_grey_refuses`, only RGB or greyscale colour spaces decoding | `UnsupportedPixelFormat` under `.110` and `.112` |

### The four TCIA collections, against the three questions

These are the real layer of the golden corpus (F-009, deviation D-05). They are
data rather than source, so "depend" here means "may a case be redistributed
and cited", which is what `corpus/manifest.tsv` records per row.

1. **Is there a licence file?** Yes. Every series archive TCIA serves carries a
   `LICENSE` file naming the collection and its licence.
2. **Does the platform's metadata report a licence?** Yes for every series
   taken. The NBIA REST service returns `LicenseName` and `LicenseURI` per
   series, and for each of the four `SeriesInstanceUID` values recorded in
   `corpus/README.md` it returns the Creative Commons Attribution 4.0
   International License at `https://creativecommons.org/licenses/by/4.0/`.
   Both sources were checked for those four and they agree.

   **The claim is per series and not per collection, because for one
   collection it would be false per collection.** CMB-MML (1,156 series),
   VAREPOP-APOLLO (1,549) and CMB-CRC (2,537) report CC BY 4.0 on every
   series. EAY131 reports it on 14,494 of its 30,293 and returns null for both
   fields on the other 15,799, all of which are RTSTRUCT (14,395) or SEG
   (1,404) third-party analysis objects rather than acquired images. The image
   series carry the licence. Anyone extending the corpus into EAY131's
   derived objects has to answer question 2 again for those, and this note is
   here so they know to.
3. **Does the licence permit the use?** Yes. CC BY 4.0 permits redistribution
   and derivative works with attribution, and TCIA's data usage policy adds a
   citation requirement, which the manifest's `source` column satisfies by
   carrying each collection's DOI.

Collections under Attribution-NonCommercial were available and were **not**
taken. A non-commercial restriction on a corpus used to validate an engine
intended for commercial embedding is exactly the kind of term that is cheap to
avoid now and expensive to unwind later.

### The pydicom test files, and why they are refused

The pydicom project ships a large set of DICOM test files covering most of the
transfer syntaxes the codec registry claims, and it is the obvious shortcut for
anyone assembling this corpus. Its own `test_files/README.txt` says of them,
verbatim, "I believe there is no restriction on using any of these files in
this manner", and traces individual files to several upstream sources with
differing terms.

**A belief is not a grant.** Against question 3 above, the answer is not yes,
and `corpus/manifest.tsv` requires a per-case `licence_url` that someone could
act on, which cannot be written from that sentence. So the files are not used
as corpus cases. This is recorded rather than left as a silent absence, because
the next person assembling a corpus will find them first and deserves the
reason.

Reading them, and depending on pydicom itself, is a separate question and the
answer to both is yes. pydicom is MIT licensed, and this repository uses it as
tooling around the corpus rather than as a source of code or of fixtures.

## One defect worth naming, because it is everywhere

Reference material for DICOM windowing very commonly computes:

```python
# WRONG. Neither LINEAR nor LINEAR_EXACT. Do not copy this shape.
img_min = window_center - window_width // 2
img_max = window_center + window_width // 2
```

**This is neither `LINEAR` nor `LINEAR_EXACT`.** Against PS3.3 C.11.2.1.2 it
omits the `c - 0.5` and `w - 1` adjustment, and against C.11.2.1.3.2 it omits
the `+ 0.5` centring term, so it disagrees with both.

It is extremely common in tutorials, blog posts and example code, and it
survives because it produces a picture that looks entirely correct. It is a
live instance of the exact defect class `docs/hld/15-lut-chain.md` section 18.3
exists to catch: values wrong by a fraction of a level, invisible to a
screenshot review and immediate to a pixel diff.

**Any window and level implementation reaching this repository must match one
of the three formulas in section 18.2 exactly, and must be proved against the
four-row fixture in section 18.3 before the shader is written.** If a reference
you are consulting contains the shape above, it is not authoritative on the
LUT chain, whatever else it gets right.
