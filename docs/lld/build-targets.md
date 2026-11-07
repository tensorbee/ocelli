# Build targets

**F-IDs that contributed:** F-002, F-004, F-005, F-007, F-008, F-016, F-017, F-021, F-022, F-023, F-026, F-039, F-050, F-101, F-X003, F-X008, F-110, F-198, F-233
**Last updated:** 2026-11-06

The wasm build pipeline, the size budget, and the invariants that keep the
core target-agnostic. This describes what the code does today.

## The two targets, and the two crates that are not shared

HLD section 4's crate table has a `wasm` and a `native` column. Eleven crates
read `yes` in both. Two do not, and they are the whole content of the
shared-core promise.

| Crate | wasm | native |
|-------|------|--------|
| `ocelli-wasm` | yes | no |
| `ocelli-native` | no | yes |
| everything else | yes | yes |

### The two `no` cells are not symmetric, and the asymmetry is deliberate

**`ocelli-native` is `wasm: no` and that is enforced.** Its `lib.rs` carries a
`#[cfg(target_arch = "wasm32")] compile_error!` naming section 4 as the source.
Before F-007 the cell was not enforceable at all: `cargo check -p ocelli-native
--target wasm32-unknown-unknown` **succeeded**, because the crate is a stub
with no native-only dependency and nothing declared it native-only. A guard
asserting "it does not build for wasm32" would have been asserting something
untrue. F-007 made it true instead of asserting it.

**`ocelli-wasm` is `native: no` and that is NOT enforced, on purpose.** The
crate compiles natively today, because `wasm-bindgen` is target-gated, and
`cargo test -p ocelli-wasm` depends on that. The cell means the crate is not
*shipped* natively. Its native compilation is what lets the boundary's logic be
unit-tested without a browser, which the project needs, so a `compile_error`
here would cost real coverage to enforce a claim the table is not making.

## The wasm pipeline

```bash
bin/ocelli.sh wasm            # render worker, release
bin/ocelli.sh wasm-decode     # decode worker, release, no wgpu
bin/ocelli.sh gate wasm       # graph check, both builds, then the size check
```

`bin/ocelli.sh wasm` runs `wasm-pack build crates/ocelli-wasm --target web
--out-dir pkg`. Release is the default and is not a convenience. HLD section
15.2's profile is `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`,
`panic = "abort"` and `strip = true`, all of which apply to release only, so a
dev-profile size measurement is not a smaller version of the same number. It
is a different number.

`crates/ocelli-wasm/pkg/` is generated and gitignored.

`bin/ocelli.sh wasm-decode` builds the same crate with
`--no-default-features --features decode-worker` into
`crates/ocelli-wasm/pkg-decode/`. That directory is gitignored. The decode
module does not depend on `ocelli-render`, `ocelli-viewport` or `wgpu`.
`eslint.config.js` ignores `**/pkg/**` and `**/pkg-decode/**`. Both are
generated wasm-bindgen glue, and the floor lint walks the tree.
`ci/wasm-decode-size.json` records its measured size and is not the render
budget. The default feature is `render-worker`. `canvas` and `session`
compile only with that feature.

### The generated package carries both licence grants

`crates/ocelli-wasm/` has three tracked relative symlinks to the repository
licence files: `LICENSE`, `LICENSE-MIT` and `LICENSE-APACHE`. Keeping the links
relative makes the crate self-contained in a clone and gives wasm-pack the
package-local inputs it expects without copying legal text into a second
tracked location. The generated `pkg/` contains regular copies.

The workspace declares `MIT OR Apache-2.0`, so a release package needs both
grants. `scripts/pin_and_size_check.py --with-size` requires generated
`LICENSE-MIT` and `LICENSE-APACHE` to be regular files and to be byte-identical
to the repository originals. A symlink is refused even when following it finds
the right bytes, because the publish directory must contain the grant itself.
`bin/ocelli.sh gate wasm` runs that check after the release build and runs its
focused unit suite. The guard catalogue separately probes an absent Apache
grant, so a package can no longer pass because its wasm size alone is healthy.

### wasm-opt runs, and is not disabled

`crates/ocelli-wasm/Cargo.toml` carries
`[package.metadata.wasm-pack.profile.release]` with an explicit `wasm-opt`
flag list. The flags are needed because rustc for `wasm32-unknown-unknown`
enables several WebAssembly proposals by default and the `wasm-opt` that
wasm-pack downloads validates without them. A stock build fails with
`Bulk memory operations require bulk memory [--enable-bulk-memory]`, measured
on rustc 1.97.1 and wasm-pack 0.15.0.

**The other documented fix is `wasm-opt = false` and it is the wrong one
here.** Turning the optimiser off produces a green build and a larger
artefact, so the number the budget records stops describing what would ship.
That is the same defect class as widening a tolerance to make a test pass.

### The exported surface

The panic probe table below still names the six exports from F-005 and F-039.
F-101 adds `alloc`, `bulk_ticket`, `release_bulk`, `apply_commands`,
`commit_frame`, `configure_stack`, `commit_selected_frame`, `copy_state`,
`drain_events` and `ring_capacity` on wasm32, plus the CPU canvas set
`bindCpuCanvas`, `resizeCpuCanvas`, `publishCpuCanvas`, `publishCpuColor` and
`publishCpuInteraction`. F-198 adds `resolvedTier`. There is no `ring_ptr` and
no pixel getter. A pointer would be a view over linear memory, and the drain
export copies bytes out instead. `coreAvailable()` stays false until `loadCore`
resolves true. The render size budget is `ci/wasm-size-budget.json`. F-110 did not
edit it. The measurement on 2026-11-03 stayed inside the recorded band.

`ocelli_version` exists for measurement rather than for features. Fat LTO with `strip =
true` lets the linker discard anything unreachable, so a module with no
exported root would measure the size of nothing. It also meets a real seam:
`packages/core/src/index.ts` carries a `VERSION` constant and a
`coreAvailable()` that stays false until `loadCore` resolves true. `ocelli_version`
returns `CARGO_PKG_VERSION`, the same `0.1.0` text as `VERSION`. `resolvedTier`
reports the tier `resolve` kept and does not substitute the CPU canvas path.

Its test asserts the literal `"0.1.0"` rather than `env!("CARGO_PKG_VERSION")`.
The obvious form restates the function body, so it passes whatever the body
returns, which is HLD 27.2 R2's failure exactly. `/release` updates the
literal with the version bump.

## The cross-target proof

```bash
bin/ocelli.sh native           # the proof
bin/ocelli.sh gate native      # the same thing, as a gate, in the floor
```

Thirteen steps, each exit code read from the command itself.

| Step | What it proves |
|------|----------------|
| 1 | `cargo build -p ocelli-native --bins`. The two entry points **link**, not merely type-check |
| 2 | `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown`. Every crate the table marks `wasm: yes` builds for wasm32 |
| 3 | `cargo check --workspace --all-targets`. Every crate the table marks `native: yes` builds natively, tests included |
| 4 | `scripts/target_feature_check.py`. Resolved features agree across the two targets, or the difference is declared |
| 5 | `scripts/tests/test_run_codec_wasm.mjs`. The Node executor refuses wrong arity and a module without the production entry point |
| 6 | The production JPEG 2000 proof executes natively against exact and lossy synthetic truth |
| 7 | The same source builds as plain and `+simd128` release wasm modules. F-233 passes the flag to every crate through `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS` on the SIMD build only, so no build script or proc macro sees it and the plain cache is untouched |
| 8 | `scripts/simd_variant_check.py` decodes the two linked `verify_jpeg2000.wasm` modules and refuses a SIMD module holding no SIMD instruction the plain module lacks, a plain module holding any, and a module that is missing or does not decode. The OK line names every SIMD opcode found, with its count |
| 9 | Node executes both wasm modules. This is execution evidence, not merely a successful compile |
| 10 | The production HTJ2K proof executes natively against the same truth |
| 11 | The same source builds as plain and `+simd128` release wasm modules, the flag reaching every crate |
| 12 | `scripts/simd_variant_check.py` again, over the two `verify_htj2k.wasm` modules |
| 13 | Node executes both HTJ2K wasm modules |

Steps 6 to 9 are F-026's codec-specific execution proof with F-233's module
check, and steps 10 to 13 are the same shape for HTJ2K, the standing
version of F-X013's claim (D-22). `gate wasm` builds
the render module and the decode module of `ocelli-wasm`. It does not
exercise `ocelli-codec`.

### What the SIMD variant contains, measured once

F-233 measured the SIMD variant on 2026-11-07 with rustc 1.97.1. These are
recorded facts, not standing checks, and steps 8 and 12 assert only the module
property in the table above. Scratch target directories throughout, built with
the arm's own commands.

**The flag reaches each crate's `cfg`.** Six crates in `ocelli-codec`'s wasm32
closure have `cfg(target_feature = "simd128")` code, found by `grep -rln
simd128` over each package's `src`. For each, the first command prints
`target_feature="simd128"` under the SIMD variable and not without it:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C target-feature=+simd128" \
  cargo rustc -p <crate> --lib --release --target wasm32-unknown-unknown \
  --target-dir <scratch> -- --print cfg | grep simd128
cargo rustc -p <crate> --lib --release --target wasm32-unknown-unknown \
  --target-dir <scratch> -- --print cfg | grep simd128
```

**The gated code is compiled in.** Read from each crate's `.d` dep-info file
and from the bitcode member `/0` and `lib.rmeta` of its rlib, plain against
SIMD:

| Crate | Plain / SIMD |
|-------|--------------|
| glam 0.30.10 | dep-info lists 14 `wasm32` source files under SIMD only and 13 `scalar` files under plain only |
| jpeg-decoder 0.3.2 | bitcode names `arch::wasm::color_convert_line_ycbcr` 0 / 5 times and `arch::wasm::dequantize_and_idct_block_8x8` 0 / 4 |
| jxl-grid 0.6.2 | `lib.rmeta` holds `v128`, the `SimdVector for v128` impl, 0 / 3 times |
| jxl-modular 0.11.3 | bitcode names `inverse_h_i16_wasm32_simd128` 0 / 104 times |
| jxl-render 0.12.4 | dep-info lists 5 `wasm32` source files under SIMD only, bitcode names `epf_row_wasm32_simd128` 0 / 57 times |
| memchr 2.8.3 | dep-info lists 5 `arch/wasm32` source files under SIMD only |

**None of that code reaches either module.** The release profile is `lto =
"fat"`, so each rlib carries bitcode and code generation runs once, at the
example's link, under the example's flags. Disassembled with
`scripts/simd_variant_check.py`, cross-checked with binaryen `wasm-dis
--all-features` version 125, which is not a repository dependency:

| Module | SIMD instructions | Executed by the proof |
|--------|-------------------|-----------------------|
| `verify_jpeg2000.wasm`, plain | none | none |
| `verify_jpeg2000.wasm`, SIMD | 156: `v128.load` 67, `v128.store` 62, `v128.const` 9, `i8x16.eq` 8, `i8x16.all_true` 8, `v128.load64_lane` 1, `v128.load64_zero` 1 | 14: `v128.load` 7, `v128.store` 7, in `ritk_codecs::jpeg_2000::image::decode_j2k_fragment` and in core and alloc generics instantiated for ritk-codecs types |
| `verify_htj2k.wasm`, plain | none | none |
| `verify_htj2k.wasm`, SIMD | 43: `v128.load` 18, `v128.store` 23, `v128.const` 2. 30 in openjph-core functions, 5 in `ocelli_codec`'s `Htj2kDecoder::decode`, 6 in std and 2 in dlmalloc | 25: `v128.store` 14, `v128.load` 9, `v128.const` 2. 20 in openjph-core, in `Tile::read_tile_data`, `Codestream::read_headers`, `Subband::new`, `rev53_atk` and `irv97_inverse_1d`, and 5 in `Htj2kDecoder::decode` |

No lane arithmetic appears in either module. The only lane-wise operations are
`i8x16.eq` and `i8x16.all_true` in the JPEG 2000 module, and neither executes.
No function of glam, jpeg-decoder, memchr or the three jxl crates holds a SIMD
instruction. The function attribution comes from a scratch build with
`CARGO_PROFILE_RELEASE_STRIP=false`, whose SIMD inventory equals the stripped
build's. The execution column comes from a scratch rewrite that inserts
`unreachable` after one SIMD instruction at a time and runs the module under
Node: a trap means that instruction executed. The same rewrite with no trap
inserted is byte-identical to its input and completes.

**What the check refuses and what it passes**, through the real arm in a
scratch copy of the tree or with the arm's commands:

| Build | SIMD module | Step 8 or 12 | rlib comparison it replaced |
|-------|-------------|--------------|-----------------------------|
| A `RUSTC_WRAPPER` strips `+simd128` from every crate | byte-identical to plain | refused | passed |
| `-C target-feature=+bulk-memory` in place of `+simd128` | no SIMD instruction | refused | passed |
| The same wrapper strips the flag from every crate but the example | byte-identical to the pre-F-233 `cargo rustc --` module | passes | passed |
| A reused HTJ2K SIMD directory: a no-flag build, the SIMD build, the no-flag build again with nothing recompiled | the third build's module is byte-identical to plain | refused | passed |

The third row is not refused. The pre-F-233 module and the F-233 module differ
in bytes and hold the same SIMD inventory, so no check over the linked module
separates them. The per-crate `cfg` facts above are the only measured
difference, and nothing asserts them on each run. In the fourth row cargo
links the fresh unit's module into `examples/` again, so the check reads the
build that just ran.

**Step 2 deliberately omits `--all-targets` and step 3 keeps it.** For wasm32
that flag pulls in dev-dependencies, and `proptest` reaches `wait-timeout`,
which does not compile for wasm32 and is not meant to. What ships to a browser
is the lib. Running the test suite under wasm32 needs `wasm-bindgen-test` and a
browser runner, which is the oracle's and F-101's ground. A native build does
run its tests, so step 3 has to compile them.

### The two entry points

`crates/ocelli-native/src/bin/ocelli-desktop.rs` and `ocelli-server.rs`. Both
print `entry_point_banner()`, which names the binary and the four extension
points of HLD section 13 that Phase 2 and Phase 3 will fill, and then, since
F-004, the tier they resolved to and the evidence for it.

They are two binaries rather than one with a subcommand because section 13
names two entry points, and because the server one is what the render-target
trait exists to serve. Collapsing them would make the first Phase 3 story a
split rather than a fill.

### Step 4, and why a build proof is not enough on its own

A build proof catches a target that stops compiling. It does not catch **both
targets compiling while one quietly resolved a different feature set**, which is
the sprint's stated false-portability defect and the more dangerous half,
because nothing goes red and the difference is in the artefact rather than in
the log.

`scripts/target_feature_check.py` makes **one** claim: every dependency this
workspace declares directly, meaning the entries in
`[workspace.dependencies]`, resolves the same features on both targets. A
difference there is our decision and we have to say so, in
`ci/target-feature-baseline.json`. `wgpu` is that exception under D-27.
wasm32 resolves `std`, `parking_lot`, `wgsl`, `webgpu` and `webgl`. The host
resolve also names `dx12`, `metal`, `gles` and `vulkan`. The version pin is
still `=30.0.1`. The `ocelli-wasm` package graph is the wasm32 set. A shared
`wgpu` line on `ocelli-oracle` would put the native backends into the wasm32
workspace resolve, so that line is target-gated too.

**It was broader in F-007 and F-008 narrowed it. That was a correction, not a
retreat.** The first version compared the whole transitive closure, which was
easy to believe when the only dependency was `glam`. Activating wgpu produced
42 findings, 32 of them packages present on one target only, and every one of
them legitimate. Worse, **most were specific to the machine**: `objc2-metal`
and `raw-window-metal` are macOS host-only, where a Linux runner reports `ash`
and `gpu-alloc`. A baseline listing them would have been correct on one laptop
and red in CI, and the fix for a red CI would have been to re-declare it. That
is tolerance-tuning wearing a different hat, and it is the exact thing
`docs/hld/22-testing-and-tolerance.md` section 25.1 says destroys a suite.

The transitive closure of a cross-platform GPU library differs per target by
design. That is wgpu doing its job, and asserting otherwise measures the
dependency rather than this project.

**A parsing bug went with it.** `cargo tree` marks an already-printed subtree
with a trailing ` (*)`, which lands after the `--format` string and so arrives
inside the feature field, turning `default` into `default (*)`. It produced
four phantom differences between a package and itself. Stripped before the
split.

**Step 4 does not assert section 4's crate table**, and an earlier draft tried
to. `cargo tree` lists every workspace member whatever target it is given,
because nothing in a manifest restricts a member to a target, so the assertion
reported `ocelli-native` present under wasm32 and could not tell that from a
real violation. The table is enforced where it can be: the `compile_error!` in
`ocelli-native`, and steps 1 to 3 building each target for real.

### The DICOM parser is shared and uses `std`

`ocelli-dicom` builds in step 2 for `wasm32-unknown-unknown` and in step 3 for
the native host. Step 4 compares its direct dicom-rs feature selection across
both targets. The selected 0.10 components are `dicom-core`, `dicom-object`,
`dicom-encoding`, `dicom-parser`, and `dicom-transfer-syntax-registry`, all with
defaults disabled. F-017 added the direct `dicom-core` edge for VR-aware
lossless metadata projection. Only whole-data-set deflate is enabled. Pixel
codec features remain outside F-016. A direct `flate2` dependency uses the
Rust backend on both targets and exposes the consumed stream length needed for
strict PS3.5 A.5 padding validation.

F-021 adds the workspace `serde` and `serde_json` dependencies directly to
`ocelli-dicom` for its pure DICOM JSON response parser and duplicate-detecting
visitor. The same dependencies and parser compile for both targets. Browser
fetch and authentication remain in TypeScript, and `wasm-bindgen` remains
absent from `ocelli-dicom`.

The same story uses `uriparse` to validate absolute and relative RFC 3986
Content-Location references in multipart resource headers. It is shared Rust
with no browser binding and is compiled in both target legs.

Shared does not mean no-std. The dicom-rs parser, dictionary and object graph
require `std`, so D-18 removes `ocelli-dicom` from the repository's explicit
no-std set. The `nostd` gate checks the remaining seven crates and the guard
census binds that exact set to a reviewed digest.

### The codec registry is shared and uses `std`

`ocelli-codec` builds unchanged in the native and wasm legs of the cross-target
proof. HLD section 21 prescribes `std::collections::HashMap` and
`std::sync::Arc` for its explicit runtime registry. F-023 therefore removes the
scaffold's self-selected `no_std` declaration. This is not a target split and
adds no browser binding. The same registry implementation compiles for both
targets, and `wasm-bindgen` remains declared only by `ocelli-wasm`.

The crate allocates its known-UID set and decoder map during setup. Exact UID
lookup and decode dispatch borrow that state, and decoded bytes are written to
the caller's buffer. No codec dependency is active yet.

### NIfTI ingest is shared and has no I/O dependency

F-022 adds a direct NIfTI-1.1 parser to `ocelli-dicom`. The parser accepts an
in-memory byte slice and uses only `ocelli-core` coordinate types plus the
existing workspace `glam` matrix implementation. It adds no NIfTI parser,
compression, filesystem, browser, or native-only dependency.

The same code validates headers, payload ranges, and RAS-to-LPS geometry on
native and `wasm32-unknown-unknown`. The shipped result contains only a
`Transform<Index, World>` after conversion to DICOM LPS. `wasm-bindgen`
remains absent from `ocelli-dicom`.

## The size budget

`ci/wasm-size-budget.json` holds a recorded measurement and a 5% tolerance.
`scripts/pin_and_size_check.py --with-size` compares the built module against
it. The first run records and passes, because a budget invented before the
first measurement would be either meaningless or immediately wrong.

**First measurement: 14,104 bytes**, on 2026-09-04.

**Current baseline: 2,151,196 bytes**, re-baselined by F-050 on 2026-10-27.
`bin/ocelli.sh wasm` measured `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` at
that size, 2.05 MiB. The previous recorded baseline was 1,999,061 bytes, from
F-039 on 2026-10-18. The delta against that recorded number is 152,135 bytes.
`ci/wasm-size-budget.json` carries the same from, to and cause. F-039's own
record stays in that file under `previous`.

**What the delta is** is `serde_json` entering the shipped module. F-050 stores
the camera inputs, the display area and the presentation as JSON. The operator
chose that record on 2026-10-27. F-039 remains the first edge from
`ocelli-wasm` to `ocelli-render`, which pulled in wgpu and Naga and moved the
budget from 16,388 bytes to 1,999,061.

F-110 measured the same render artefact at 2,171,133 bytes on 2026-11-03.
That is inside the 5 percent ceiling of the 2,151,196 byte baseline, so the
budget file was not edited and the tolerance stayed 0.05. The decode-worker
module measured 16,455 bytes the same day and is recorded in
`ci/wasm-decode-size.json`. That file is not the render budget. The decode
module does not link wgpu or Naga. Cold start was not remeasured, so this
story does not close gate A4.

**That number is not an answer to Appendix A gate A4 and must not be read as
one.** A4 asks whether binary size and cold start land within budget and
estimates 3 to 8 MB uncompressed with Naga dominating. 2,151,196 bytes is the
F-050 measurement. Cold start was not remeasured. A4 stays open.

### The other half of A4, cold start

A4 names two figures and the paragraphs above are only the first. **Cold start
was never measured at all until F-006**, which recorded **2.3 ms** on
2026-09-05 for the release artefact: fetch, `WebAssembly.compile`, instantiate
and the first `ocelli_version()` call, timed in headless Chromium from the page
rather than from inside the module, so `crates/` gained not one line. The figure
and everything about how it was taken are in `ci/bench-baseline.json`, and the
design is `docs/lld/benchmarks.md`.

**The same caveat applies to it, and it is the reason both halves live in one
place rather than in two documents.** F-039 changed the module, and F-050
changed it again. The 2.3 ms figure was not remeasured, so it still describes
the module from before wgpu and Naga were reached. **A4 stays open**, and
2.3 ms remains a regression baseline until a later story measures cold start
again.

**Re-baselining is expected, repeatedly, for the whole build-out phase.** A 5%
tolerance on a 14 KB module is blown by the first story that adds anything
real. That is the mechanism working rather than failing: during buildup the
gate does not mean "you exceeded a budget", it means **"the module changed
size and the change was not declared"**. `--accept-size` is the declaration,
and the design plan that used it says why. The gate only starts meaning the
other thing once the module is feature-complete.

## The exported surface, and the panic probe

The module exports twenty-four functions of ours. F-050 added `snapshot`,
`snapshot_error` and `restore_snapshot` to the twenty-one that were already
there. wasm-bindgen adds its own,
and `memory`, which are the glue's and not this table's. The table below names
the six that predate F-101, which are the measurement root, the panic probe and
the two render-target calls. The F-101 boundary and CPU canvas exports are
listed in the section above. The render module still exports
`attachOffscreenCanvas` and `resizeOffscreenCanvas`. The decode-worker
artefact does not. A decode worker does not load this module.

| Export | Called |
|--------|--------|
| `ocelli_version()` | Any time. F-002's measurement root |
| `install_panic_hook()` | Once, by a worker, before any other call |
| `panic_record_ptr()` | Once, straight after instantiation. Cached |
| `panic_record_len()` | Once, straight after instantiation. Cached |
| `attachOffscreenCanvas(viewport, canvas, cssWidth, cssHeight, dpr)` | A render worker, when a canvas is transferred. F-039 |
| `resizeOffscreenCanvas(viewport, canvas, cssWidth, cssHeight, dpr)` | The same worker, when CSS size or device pixel ratio changes. F-039 |

`install_panic_hook`, `panic_record_ptr` and `panic_record_len` are F-005's,
and `docs/lld/errors.md` is where they are specified. The property that
matters for those three is that **none of them is called after a trap**, so
the panic path adds no post-trap requirement on the module.
`attachOffscreenCanvas` and `resizeOffscreenCanvas` are F-039.

**`wasm32-unknown-unknown` is `panic = "abort"` in every profile**, not only in
the release profile HLD section 15.2 sets it in, because it is the target's own
default. `rustc --print cfg --target wasm32-unknown-unknown` reports
`panic="abort"` with no profile involved. That is worth stating here because
this file describes the build, and the obvious reading of section 15.2 is that
a dev wasm build unwinds. It does not.

`bin/ocelli.sh gate panic` builds a **second** module, carrying the
`panic-probe` cargo feature, into `crates/ocelli-wasm/target/panic-probe`,
which is gitignored. That feature adds one export that panics on purpose, and
the shipped artefact does not carry it, so the module `bin/ocelli.sh wasm`
measures has no way to be asked to panic. `AGENTS.md` forbids a feature flag
without a named user, and the named user is `scripts/panic_probe.mjs`.

## The isolation invariant

`ci/check-bindgen-isolation.sh` enforces decision D2, wasm-bindgen in exactly
one crate. It has three parts.

1. HLD section 15.3's loop verbatim, over the **host** target.
2. The same loop over **wasm32-unknown-unknown**, added by F-002.
3. A source grep for `wasm_bindgen`, which catches a dev-dependency or a
   re-export that `cargo tree -e normal` does not see.

**Part 2 is not redundant and the reason is worth keeping.** `cargo tree`
filters to the host platform when no `--target` is given, so part 1 cannot see
a dependency declared under `[target.'cfg(target_arch = "wasm32")'.dependencies]`.
That is precisely the form `ocelli-wasm` itself uses, so it is the form a
second crate would most plausibly copy. Measured while adding it: with
`wasm-bindgen` added to `ocelli-core` under a wasm32 target gate, part 1
reports zero hits and would have passed, and part 2 fails.

**The target does not need to be installed**, because `cargo tree` resolves
cfg rather than compiling. That was verified against an uninstalled triple
rather than assumed, and it matters: the CI `guards` job installs no extra
target, so requiring one would fail that job for no reason.

The triple is still checked against `rustc --print target-list`, and that
check is not decoration. `cargo tree --target <typo>` errors, the error goes
to `/dev/null`, and `grep -q` then finds nothing in an empty stream, so a
misspelled triple would report a clean pass over zero crates. "The check could
not run" and "the check ran and was happy" must not look the same.

## The exact pins

`scripts/pin_and_size_check.py` refuses a range form for:

| Crate | Why a range is refused |
|-------|------------------------|
| `wgpu` | HLD section 15.2. Agents reliably emit wgpu 0.19-era pipeline code, and a range lets that compile against something subtly different from what the shader expects |
| `wasm-bindgen` | `wasm-pack` runs a CLI whose version must match the crate version. A range lets the two drift, and the mismatch reads as a build break rather than as a resolution change |

### `ocelli-render` names `webgl`, and that is deviation D-14

The workspace entry is `{ version = "=30.0.1", default-features = false }`.
That switch is D-27. Cargo rejects a member `default-features = false` while
the workspace entry leaves the defaults on. The version pin is still
`=30.0.1`.

Native `ocelli-render` names the pinned default backends plus `webgl`.
wasm32 names `std`, `parking_lot`, `wgsl`, `webgpu` and `webgl`, and drops
`dx12`, `metal`, `gles` and `vulkan`. `webgl` is not in wgpu 30.0.1's default
set. `gles` is the native GL backend rather than the browser one. Without
`webgl` the crate reaches WebGPU on wasm32 and cannot reach WebGL2 at all, so
HLD section 7's tier B could never resolve in a browser, and a tier the
resolver can never return is not a tier.

**The two targets do not resolve the same wgpu features, and step 4 says so.**
`ci/target-feature-baseline.json` names `wgpu`. The packages `webgl` pulls in
on wasm32 stay inside wgpu's own target tables, so the host tree does not
reach `wasm-bindgen`. `ci/check-bindgen-isolation.sh` part 1 is unaffected for
that reason.

**Measured cost of reaching wgpu from the wasm module is the F-039 delta.**
`ocelli-wasm` depends on `ocelli-render` and on `wgpu`, so the release module
reaches wgpu and Naga. F-039 measured 1,999,061 bytes against the recorded
baseline of 16,388. The parent commit was not rebuilt. Device
creation stays in `ocelli-render`. `ocelli-wasm` names
`SurfaceTarget::OffscreenCanvas` because D-12 keeps that browser type out of
`ocelli-render`. `web-sys` is a dependency of `ocelli-wasm` only.

### `ocelli-native` now depends on `ocelli-render`

F-004 gives the two entry points a real job: read `OCELLI_TIER`, resolve a
tier, and print the evidence. That is the operator override's named user, and
`AGENTS.md` forbids a flag without one.

The cost is that **step 1 of the cross-target proof starts linking wgpu**,
which is a real build-time increase to a gate in the floor. It is accepted
because the alternative is shipping an override nobody can observe. See
[tier-resolution.md](tier-resolution.md).

## Known gaps

- **Step 4 names `wgpu`.** It compares the dependencies this workspace
  declares directly, not the workspace's own crates. The other direct
  dependencies still resolve the same features on both targets. `allowed` in
  `ci/target-feature-baseline.json` names `wgpu` under D-27. Removing that
  key makes the check fail. The names come from `[workspace.dependencies]`.
  The `checked_dependencies` array in that file is not read by the check.
