# A1 follow-up, price the HTJ2K decoder route

**Story**: F-X013, after Appendix A gate A1 failed in F-X006.
**Candidate**: `openjph-core` 0.1.0, route F3.
**Outcome**: **Recommend F3 for the E2.6 production design, with conditions.**

This story does not add a decoder to Ocelli. It measures the one candidate
that can use the same Rust implementation on browser, desktop and server
targets. The production decision remains with E2.6 and requires a deviation
from HLD section 15.2, which names `openjp2`.

## Decision

`openjph-core` 0.1.0 decodes all three HTJ2K transfer syntaxes on native and
`wasm32-unknown-unknown`. Native, plain wasm and wasm built with `+simd128`
produce identical samples for every row. The two reversible syntaxes reproduce
the independent synthetic ramp exactly. The irreversible syntax differs from
OpenJPH 0.31.0 at 41 of 6,144 samples, always by one, while remaining identical
across the candidate's three builds.

That satisfies the plan's F3 path. The irreversible difference is a D14
measured divergence, not a bit-exact claim. F3 is recommended as the route for
E2.6 to design and harden. It is not ready to register directly because its
public decode API allocates, its audit surface is large, its package omits the
licence text, and it has one release and one owner.

| Axis | Result | Consequence |
|------|--------|-------------|
| `.201` lossless | Exact against the synthetic ramp and OpenJPH on all three builds | Pass |
| `.202` lossless RPCL | Exact against the synthetic ramp and OpenJPH on all three builds | Pass |
| `.203` irreversible | Candidate builds agree. 41 samples differ from OpenJPH by one | Accept as a recorded D14 divergence |
| Native target | Builds and decodes on `aarch64-apple-darwin` | Pass |
| wasm target | Plain and `+simd128` builds decode all rows | Pass |
| Source provenance | Registry says BSD-2-Clause. Package has no licence file or repository URL | Conditional production risk |
| Caller-owned output | Not supported. `pull()` returns a new `Vec<i32>` | E2.6 needs an upstream API or a maintained adaptation |
| Maintenance | One release, one owner, no repository or issue tracker in metadata | Pin exactly and require a fork plan |

## Provenance before source inspection

The crates.io archive was inspected before any Rust implementation was read.
Its SHA-256 is
`c8b96ed12b3d41623a771af4af8131abf353bc822b7a567c6ef3b35ab967a36d`,
matching the official crates.io API. The archive contains no root licence or
notice file. Both its normalized manifest and registry metadata declare
`BSD-2-Clause`, which permits derivative works. The publisher is Michael
Knopke, crates.io login `knopkem`, with a GitHub-matching identity. Packaged
VCS metadata names commit
`7ed6d6d110d994ec740aacaa90a78b2e807c4c24` and path `openjph-core`.

Crates.io reports no homepage, documentation URL, repository or issue tracker.
The release was published on 2026-03-20. At measurement time it was the only
version, was not yanked, and had 3,703 downloads. The plan recorded 3,643,
which was correct when written and had risen by measurement time.

`docs/SOURCE-POLICY.md` records the three-question assessment. Dependence is
permitted for this spike because platform licence metadata is present. Before
production redistribution, E2.6 must obtain the complete BSD notice and
copyright material that the package does not carry.

## Method

The repository's common extraction tooling reads only manifest-backed corpus
rows. Every decode is canonicalised to 12,288 little-endian bytes, 6,144
unsigned 16-bit samples, 64 rows by 96 columns. The exact comparator is
`tools/spikes/common/compare.mjs`.

| Symbol | Source |
|--------|--------|
| `R` | Pixel Data from the synthetic uncompressed reference |
| `D_native` | `openjph-core` 0.1.0 on `aarch64-apple-darwin` |
| `D_wasm` | the same crate on `wasm32-unknown-unknown` |
| `D_simd` | the same wasm target with `-C target-feature=+simd128` |
| `D_ojph` | `ojph_expand` 0.31.0 converted from 16-bit PGM |

`R` is an independent exact anchor for `.201` and `.202` because all three
compressed cases were encoded from the synthetic ramp. It is not an exact
anchor for irreversible `.203`. `D_ojph` is useful for measuring `.203`, but
F3 describes itself as a port of OpenJPH v0.26.3, so agreement or near
agreement between them is not independent algorithmic evidence.

The conformance driver was written before the Rust candidate harness. Its
first run exited 1 because the native candidate binary did not exist. The
shared comparator suite then passed its five checks, including the deliberately
changed sample. After implementation, `COLS` was temporarily changed from 96
to 95. The native build succeeded and the driver exited 1 with raw geometry
error 3. The constant was restored and the driver returned to green.

The driver pins the digest of every input codestream, the independent
reference, every candidate output and every OpenJPH output. For `.203` it also
requires the exact measured divergence below, including the count, first
difference, maximum absolute difference and complete magnitude histogram. As
a red check, the `.203` route was deliberately changed to decode the `.201`
codestream in all three builds. The driver exited 1 with candidate digest and
divergence mismatches, then returned to exit 0 after the route was restored.

## Raw correctness results

The synthetic ramp digest is:

```text
b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
```

For `.201` and `.202`, `R`, `D_native`, `D_wasm`, `D_simd` and `D_ojph` all
have that digest.

For `.203`:

```text
D_native = D_wasm = D_simd
ce4a2bb9d75b897292a4e9e9e7455447e976f5ff1e18b4ecb3219bb17d4d062c

D_ojph
41a94cc4db2b871e16e50c738512413db800657b0dbeab7872415504be87c138

differing samples: 41 of 6144
first difference: index 69, candidate=756, OpenJPH=757
maximum absolute difference: 1
histogram by magnitude: 1:41
```

This is enforced without a tolerance change. It is the measured output of the
irreversible path and the candidate is identical to itself across targets.

## Build, binary and toolchain surface

All builds use Rust 1.97.1. The native target is
`aarch64-apple-darwin`. The browser target is
`wasm32-unknown-unknown`. The release profile is the HLD section 15.2 profile:
`opt-level = "z"`, fat LTO, one codegen unit, aborting panic and stripped
symbols.

```text
native spike executable                 352,544 bytes
plain wasm cdylib                       110,724 bytes
wasm cdylib with +simd128               110,575 bytes
```

These are whole-spike sizes and include the three embedded codestreams, so
they are not an incremental production size claim. The two wasm artefacts have
different hashes and differ by 149 bytes. Their decoded samples are identical.
The crate has SIMD implementations for x86_64 and aarch64, but no wasm32 or
`simd128` implementation. The wasm size change therefore comes from compiler
target-feature code generation, not a wasm SIMD backend in the crate.

The runtime dependency surface is `openjph-core` 0.1.0 and `thiserror` 2.0.20.
The remaining locked packages are the proc-macro build dependencies of
`thiserror`: `thiserror-impl`, `proc-macro2`, `quote`, `syn` and
`unicode-ident`. No C or C++ compiler, CMake, JavaScript package, wasm-bindgen,
system library or wasm import is required.

## Allocation and audit surface

The public API does not meet HLD section 21's caller-owned output contract.
`create()` materialises decoded image rows internally. Each `pull(0)` clones
one stored row into a fresh `Vec<i32>`. For this 64-row fixture the public API
therefore forces at least 64 row allocations. The spike then needs one
12,288-byte output allocation, giving a lower bound of 65 allocations after
and around the pull boundary, in addition to the decoder's internal tile,
subband, coefficient and row allocations. There is no API that decodes into a
caller-provided slice.

The published crate contains 22,504 Rust code lines in 38 files. A source
audit found 104 `unsafe fn`, `unsafe impl` or `unsafe { ... }` constructs in
nine files. The scalar memory, wavelet and colour paths account for 50 of
those constructs and are relevant to wasm. Architecture-specific SIMD files
hold the rest. Ocelli's spike adds no unsafe code, but production adoption
would make this dependency a substantial safety audit surface.

## Rejected and conditional routes

- **F1, patched `openjp2`** remains rejected. F-X006 found undefined behavior
  around null deallocation, a wasm allocator shim and an upstream release
  dependency. A local shipping patch would retain the C build the failed gate
  was trying to avoid.
- **F2, `@cornerstonejs/codec-openjph`** remains a browser-only fallback. It
  would add a second wasm module and a codec-byte copy, while desktop and
  server would still need another decoder. F3 makes that split unnecessary.
- **Unavailable** remains the safe behavior until E2.6 lands a production
  decoder. This evidence does not register F3 or change the parity count.

## Exact follow-up for E2.6

E2.6 must design a production HTJ2K decoder around an exact pin of
`openjph-core` 0.1.0 or a reviewed successor. Before activation it must:

1. raise the section 15.2 deviation from `openjp2`
2. obtain and retain complete BSD notice and copyright material
3. decide whether to upstream a caller-owned decode API or maintain a bounded
   adaptation that removes the per-row clone contract
4. audit the dependency's relevant unsafe paths and malformed-input behavior
5. add standing conformance cases for `.201`, `.202` and `.203` on native and
   wasm, including the `.203` divergence measured here
6. measure incremental wasm size in the real production crate
7. keep HTJ2K `Unavailable` on every rendering tier until all of those gates
   pass

## Commands measured

```text
node --test tools/spikes/common/tests/compare_test.mjs       exit 0
uv run tools/spikes/common/extract.py                       exit 0
cargo generate-lockfile --offline                           exit 0
cargo build --release --offline                             exit 0
CARGO_TARGET_DIR=target/wasm-plain cargo build ...          exit 0
CARGO_TARGET_DIR=target/wasm-simd RUSTFLAGS=... cargo ...   exit 0
node tools/spikes/x013-htj2k-route/run.mjs                  exit 0
```
