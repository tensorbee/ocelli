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
