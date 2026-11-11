# Build targets

**F-IDs that contributed:** F-002, F-004, F-005, F-007, F-008, F-016, F-017, F-021, F-022, F-023, F-026, F-039, F-050, F-101, F-X003, F-X008, F-110, F-198, F-233, F-231, F-239, F-240, F-238, F-244, F-268, F-266
**Last updated:** 2026-11-11

The wasm build pipeline, the size budget, and the invariants that keep the
core target-agnostic. This describes what the code does today.

## F-268: workspace docs floor and gate

The public workspace documentation builds with native/default scope. A named
`docs` floor gate and a Rust-job CI step invoke `bin/ocelli.sh gate docs`, which
spawns `cargo doc --workspace --no-deps --locked --keep-going` as a child with
`RUSTDOCFLAGS='-D rustdoc::private_intra_doc_links -D
rustdoc::broken_intra_doc_links'`, removing only the child's encoded variable,
while the caller's `CARGO_TARGET_DIR` is preserved. Lints are denied, never
suppressed. API-preserving comment fixes are the remedy. Permanent deep Cargo
probes use a two-member synthetic workspace whose encoded allow flags are
inherited, with the fault outside `default-members`, a fresh owned target per
run, a healthy control checked for real docs, and target cleanup after.

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

When the pkg is PRESENT, the build uses a decoded /@fs specifier during Rollup compilation and Rollup rewrites it into a bundled emitted chunk. The viewer build check refuses a PRESENT dist whose emitted files spell the temporary build base or the real checkout root, raw or URL-encoded, or carry the standalone /@fs marker. It bans no other absolute path. The real `ocelli_wasm_bg.wasm` carries Rust panic location strings with absolute cargo registry paths under the builder home directory, and the check does not refuse them. When ABSENT, the build keeps the encoded /@fs specifier as an external runtime import. Dev uses the encoded URL in both states. Dev fs.allow is the decoded checkout root. A rejected import or default-init failure leaves core not built.

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
`publishCpuInteraction`. F-198 adds `resolvedTier`. F-231 adds the session
frame cache's seven, `set_frame_budget`, `commit_cached_frame`,
`frame_resident`, `show_resident`, `release_series`, `request_frame_pressure`
and `content_id`, every one in `session.rs`'s `mod bind` and returning a
`u32` or, for `content_id`, an `Option<u64>` that is `undefined` on a
refusal. There is no `ring_ptr` and
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

Fifteen steps, each exit code read from the command itself.

| Step | What it proves |
|------|----------------|
| 1 | `cargo build -p ocelli-native --bins`. The two entry points **link**, not merely type-check |
| 2 | `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown`. Every crate the table marks `wasm: yes` builds for wasm32 |
| 3 | `cargo check --workspace --all-targets`. Every crate the table marks `native: yes` builds natively, tests included |
| 4 | `scripts/target_feature_check.py`. Resolved features agree across the two targets, or the difference is declared |
| 5 | `scripts/shipped_native_check.py`. The shipped native graph, `ocelli-native` with no default features, resolves no `objc2` package on either Apple target, over normal and build edges. F-238's AC1 |
| 6 | `cargo build -p ocelli-native --bins --release --no-default-features`, then the desktop binary it produced is run once and the step refuses unless it prints a tier, `Tier Cpu, decided by NoAdapter` on an Apple host. The shipped configuration links and starts. F-238's AC4 |
| 7 | `scripts/tests/test_run_codec_wasm.mjs`. The Node executor refuses wrong arity and a module without the production entry point |
| 8 | The production JPEG 2000 proof executes natively against exact and lossy synthetic truth |
| 9 | The same source builds as plain and `+simd128` release wasm modules. F-233 passes the flag to every crate through `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS` on the SIMD build only, so no build script or proc macro sees it and the plain cache is untouched |
| 10 | `scripts/simd_variant_check.py` decodes the two linked `verify_jpeg2000.wasm` modules and refuses a SIMD module holding no SIMD instruction the plain module lacks, a plain module holding any, and a module that is missing or does not decode. The OK line names every SIMD opcode found, with its count |
| 11 | Node executes both wasm modules. This is execution evidence, not merely a successful compile |
| 12 | The production HTJ2K proof executes natively against the same truth |
| 13 | The same source builds as plain and `+simd128` release wasm modules, the flag reaching every crate |
| 14 | `scripts/simd_variant_check.py` again, over the two `verify_htj2k.wasm` modules |
| 15 | Node executes both HTJ2K wasm modules |

Steps 8 to 11 are F-026's codec-specific execution proof with F-233's module
check, and steps 12 to 15 are the same shape for HTJ2K, the standing
version of F-X013's claim (D-22). `gate wasm` builds
the render module and the decode module of `ocelli-wasm`. It does not
exercise `ocelli-codec`.

### What the SIMD variant contains, measured once

F-233 measured the SIMD variant on 2026-11-07 with rustc 1.97.1. These are
recorded facts, not standing checks, and steps 10 and 14 assert only the module
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
difference, and nothing asserts them on each run. What pins the build shape
is structural: `build_shape_problems` in `scripts/simd_variant_check.py`
reads the native arm and refuses a SIMD variant step that does not pass
`+simd128` through `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS`, and any
step passing a target feature after `cargo rustc --`.
`scripts/tests/test_simd_variant_check.py` applies it to the tracked
`bin/ocelli.sh` in `gate native`, and probe
`simd-variant.cargo-rustc-shape` drives it red in `gate guards`. It reads the
runner's spelling, so it catches the pre-F-233 shape, measured at four
refusals over that commit's `bin/ocelli.sh`, and says nothing about a build
that reaches the crates some other way. In the fourth row cargo
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

### The shipped native build, and deviation D-33

The shipped native archive is `ocelli-native` built with
`--no-default-features`. Its `metal` feature forwards to
`ocelli-render/metal`, which forwards to `wgpu/metal`, so omitting the
default omits Metal and, with it, every objc2 crate wgpu-hal's Metal backend
reaches. The arrangement exists because the objc2 crates are generated from
Apple SDK headers and the upstream `LICENSE.md` flags the redistribution
question, which is the operator's answer of 2026-11-06 that F-238 implements.
Development restores Metal in four places: the native dev-dependency tables
of `ocelli-render` and `ocelli-compute`, `ocelli-native`'s own default
feature, the hard `metal` in `ocelli-wasm`'s and `tools/oracle`'s native
tables, and `ocelli-native`'s own dev-dependency on `wgpu`, which keeps
`cargo test -p ocelli-native --no-default-features` on Metal. The forwarding
feature is deliberately absent from
`ocelli-render`'s default set, because a default is target-blind: a forwarded
`wgpu/metal` on wasm32 would put the feature name on the wasm32 workspace
resolve and fail the decode-graph guard. For the same reason
`ocelli-native`'s dependency on `ocelli-render` is target-gated, so its
default feature is inert on the wasm32 resolve of a crate that is never
compiled there.

**The two measurements F-238 took, and the outcome of each.** Both were taken
on 2026-11-07 against wgpu 30.0.1's own manifests, on this tree, with
`cargo tree -p ocelli-native --no-default-features -e normal`:

1. **Per target, `metal` off and `gles` on.** Zero objc2 packages on
   `aarch64-apple-darwin` and on `x86_64-apple-darwin`. The reason is in the
   pinned manifests: `wgpu/gles` reaches `wgpu-core/gles`, which names only
   `wgpu-core-deps-windows-linux-android` and `wgpu-core-deps-emscripten`,
   and neither crate is a dependency on Apple. `wgpu-hal`'s own `gles`, the
   feature that names `dep:objc2`, is reached on Apple only through `angle`,
   which D-27 does not name. So `gles` stays in the shipped set, and native
   GL does not link on macOS in any build, which is the "otherwise
   unavailable" branch of the design's tier table. `gate gpu` and the
   development tier are unchanged.
2. **With no platform filter, `gles` off and `vulkan` on.** The objc2 family
   is present. `wgpu-hal`'s `vulkan` names `dep:raw-window-metal`, and
   `wgpu-core-deps-windows-linux-android/vulkan` activates `wgpu-hal/vulkan`
   in the union resolve because that crate is present for some target in the
   union even though it is absent from every Apple graph. A no-filter
   resolve therefore cannot describe the shipped graph for any one target.

The second measurement is why `THIRD_PARTY_LICENSES`' native closure is the
union of per-target shipped graphs over the five desktop triples recorded in
`scripts/licence_check.py`, and not one unfiltered resolve. The union over
explicit triples names exactly what each shipped binary links: the Apple
graphs hold no objc2 crate, and the Linux and Windows walks exclude
Apple-gated dependencies by target filter. The regenerated file holds 105
third-party crates where the development graph held 129, and the dropped
carriers are the objc2 family with `block2`, `dispatch2`,
`raw-window-metal` and `wgpu-core-deps-apple`, the Apple-gated carrier
wgpu-core's `metal` feature activates, plus crates only the emscripten,
android and union-only legs ever reached: `ndk-sys`, `jni-sys`,
`jni-sys-macros`, `android_system_properties`,
`wgpu-core-deps-emscripten`, `crunchy`, `redox_syscall`, `winapi-util`,
`windows-sys`, `portable-atomic` and `portable-atomic-util`. None of them is
compiled into anything that ships. The objc2 family's pinned texts and
fallback reasons stay in `ci/notices/sources.toml`, where `verify-upstream`
still checks them.

**A build with no backend compiled cannot create an instance.** wgpu 30.0.1's
`Instance::new` panics when the platform-filtered set of compiled backends is
empty, and on macOS that set is empty for the shipped build: the `metal` cfg
needs its feature, the `gles` cfg needs `angle` and the `vulkan` cfg needs
`vulkan-portability`, and none is enabled. Measured before the guard existed,
the shipped release binary panicked at startup. `probe.rs` now consults
`wgpu::Instance::enabled_backend_features()`, the same set the panic's own
check consults, before creating any instance, and resolves the honest
unprobed answer there: no adapter seen, tier C, recorded as `NoAdapter`. The
same guard sits in front of both GL entry points, where it changes nothing
today: on macOS even a development build reaches GL only through `angle`, and
on wasm32 the `webgl` feature always fires. AC4 is the shipped binary's own
report, pasted in the F-238 record: `Tier Cpu, decided by NoAdapter`, exit 0.

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

F-239's library half measured both modules on 2026-11-07 with
`bin/ocelli.sh gate wasm`, neither budget file edited and `--accept-size` not
run. On its base, `sprint/s46` at 699a5148, the render module was 2,179,477
bytes and the decode module 16,455. On its tip, 01bc1342, they were 2,179,981
and 16,455. The render module grew by 504 bytes and stays inside the
2,258,755 byte ceiling. Re-measured after review pass 1's fixes and ruling
R1's fallback, the render module is again 2,179,981 bytes, not byte-identical
to 01bc1342's but the same size, and the decode module is byte-identical to
base. `cargo tree -p
ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none` on the
tip reaches `ocelli-pixel` and `ocelli-render` and no `ocelli-codec`,
`ocelli-dicom`, `openjph-core` or `ritk-codecs`, and the same with
`--no-default-features --features decode-worker` reaches `ocelli-pixel`,
`ocelli-cache` and `ocelli-core` and no other workspace crate.

The 504 bytes, attributed with `twiggy diff` over the two trees built
unstripped for `wasm32-unknown-unknown` release: `ColourWindow::apply`,
`ColourWindow::apply_in_place` and the window closure, which ship because
`raster_rgb_into` reaches them through the shared `map_colour` step even at the
skipped initial window, the sample scale inside `ColorTransform::pixel` and
`stored_colour_is_a_display_byte`, the scaled `channel_entries` iterators that
`RasterColor::supported` walks, and the split `rgb_bytes` and `map_colour`
steps. F-239's plan expected fat LTO to remove `ColourWindow`, and it does not.
`colour_bytes_into` itself is not in the module, because no `ocelli-wasm`
export calls it.

F-231 measured the render artefact at 2,202,907 bytes on 2026-11-07, against
the ceiling of 2,258,755, which is the 2,151,196 byte baseline and its 5
percent. The last recorded measurement was 2,179,402 bytes, at F-233's
integration, so the module grew 23,505 bytes. F-231 is the session frame
cache entering the shipped module: an `ImageCache<CommittedFrame>`, its
`BTreeMap` operations over `FrameKey`, the FNV-1a fold, F-034's event
encoders and the seven exports. The growth is not split among them, and
F-224, which changed `ocelli-cache` between the two measurements, was not
measured on its own. The decode module stayed at 16,455 bytes. Neither
budget file was edited.

**That number is not an answer to Appendix A gate A4 and must not be read as
one.** A4 asks whether binary size and cold start land within budget and
estimates 3 to 8 MB uncompressed with Naga dominating. 2,151,196 bytes is the
F-050 measurement. Cold start was not remeasured. A4 stays open.

F-240 measured both modules on 2026-11-07 with `bin/ocelli.sh gate wasm`,
neither budget file edited and `--accept-size` not run. On its base,
`sprint/s46` at 6163b82f, the render module was 2,179,981 bytes and the decode
module 16,455. Re-measured after review pass 2, they are 2,182,144 and
16,455: the render module grew by 2,163 bytes over base, about 0.099
percent, and stays inside the 2,258,755 byte ceiling, and the decode module
is again 16,455 bytes, the size it already had at base. `cargo tree -p
ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none` on the
tip reaches `ocelli-pixel`, where `icc.rs` lands, and `ocelli-render`, and
neither `ocelli-dicom` nor the folder viewer, and the same with
`--no-default-features --features decode-worker` reaches `ocelli-pixel`,
`ocelli-cache` and `ocelli-core` and no other workspace crate.

**The growth, attributed with `twiggy diff` over the two trees built
unstripped for `wasm32-unknown-unknown` release, the base `6163b82f` and
`d3fef64f` "F-240, matrix and tone-curve ICC profiles in ocelli-pixel,
library part" as pass 1 reviewed it, when the growth measured 2,067
bytes:** `ColorTransform::map_into`
and the ICC branches of its `pixel` match, `icc::ToneCurve::evaluate`,
`icc::IccTransform::apply`, `icc::channel_value`, `icc::encoding_range` and
`icc::encode_srgb`, the glam `powf` and `floor` the encoding and the
interpolation use, the drop glue the new `icc` field adds to
`ColorTransform`'s own, and the `Vec<f32>` helpers the tables share with the
palettes. The further 96 bytes the pass 1 remediation `72a75882` "F-240,
review pass 1 remediation in the library" added, three linear
clamps in `apply`, were not re-attributed: the base tree was not rebuilt for
a second `twiggy diff`. **`IccProfile::parse` and every tag reader are
absent from the module**, which is the fat LTO removal the plan expected: no
`ocelli-wasm` export calls them. What ships is the transform's arithmetic,
because `pixel`'s match on the attached `Option` is reachable through
`map_into` whatever a caller attaches, and the field's lifecycle ships with
`ColorTransform`'s own.

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

The module exports thirty-two functions of ours, counted from the
generated `ocelli_wasm.d.ts` on the F-231 build. F-050 added `snapshot`,
`snapshot_error` and `restore_snapshot` to the twenty-one that were already
there, F-198 added `resolvedTier` and F-231 added seven. wasm-bindgen adds its own,
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

Native `ocelli-render` names the pinned default backends plus `webgl`, with
`metal` moved to a forwarding feature under D-33. wasm32 names `std`,
`parking_lot`, `wgsl`, `webgpu` and `webgl`, and drops `dx12`, `metal`, `gles`
and `vulkan`. `webgl` is not in wgpu 30.0.1's default set. `gles` is the
native GL backend rather than the browser one. Without `webgl` the crate
reaches WebGPU on wasm32 and cannot reach WebGL2 at all, so HLD section 7's
tier B could never resolve in a browser, and a tier the resolver can never
return is not a tier.

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

### S46 colour and ICC size evidence, 2026-11-08

F-239 AC29 and F-240 AC13 use the release wasm gate, with the shipping
`opt-level = "z"`, fat LTO, one codegen unit, abort and stripping profile.
All three actual gates returned 0. Neither size budget changed.

| Source | Render bytes | Gzip bytes, mtime 0 | Decode bytes | Decode gzip bytes |
|---|---:|---:|---:|---:|
| Original `5451cb7d`, tree `dd269895` | 2,179,477 | 860,541 | 16,455 | 7,663 |
| Colour stack tree `12797792` | 2,203,411 | 871,401 | 16,455 | 7,663 |
| Combined stack tree `3ecb1239` | 2,205,648 | 872,915 | 16,455 | 7,663 |

The render increase is 23,934 bytes from original to colour stack and
2,237 bytes from colour to combined stack, or 26,171 bytes overall. The
corresponding gzip changes are 10,860, 1,514 and 12,374 bytes. Every decode
binary has SHA256
`07cf6db9ba05bd09fb94b9e8be04c390e0ae80a857c7c4ebc19c74fab1ee7aad`.

These are stack comparisons. The first interval also adds F-231's frame
cache and session roots. The second includes F-227 and F-238 changes as well
as F-240. The full interval is not an isolated causal measurement of ICC.

#### Symbols from executable-identical copies

A private analysis build preserved the shipping dependency profile and
changed stripping only on the final crate with
`cargo rustc --release --lib -p ocelli-wasm --target wasm32-unknown-unknown -- -C strip=none`.
Its input retained names and bindgen descriptors, removed DWARF, producers
and target-feature custom sections, and omitted bindgen-sensitive function
names. It used pinned bindgen 0.2.127 and Binaryen 125 with the shipping
wasm-opt flags plus name retention. Every noncustom section of each result
was byte-identical to its corresponding shipping binary before its symbols
were used. Added analysis metadata is excluded from all size figures. The original shipping
`producers` custom section is 62 encoded bytes, identical in all three builds,
and remains included in the shipping totals.

The colour helpers were not all removed as the plan predicted. The actual
retaining paths are `publishCpuColor`, `publishCpuCanvas` and
`publishCpuInteraction` through `Session::draw_committed_target`.
`ColourWindow::apply`'s closure has 60 encoded code-body bytes and
`DisplayScale::apply` has 41 in the colour stack. The same session draw
function grows from 6,070 to 6,493 bytes. Its inlined changes are not charged
again to standalone helpers.

The seven F-231 cache exports account for 11,424 encoded code-body bytes
in the colour and combined stacks, and are absent from the original:
`commit_cached_frame` 4,467, `release_series` 2,166, `show_resident` 1,477,
`set_frame_budget` 1,106, `content_id` 1,063, `frame_resident` 865 and
`request_frame_pressure` 280. Their session and cache support also changes.
This is why the original-to-colour total cannot be labelled colour cost.

ICC parsing is absent from the named executable, and no wasm root calls
`IccProfile::parse`. ICC application helpers remain through the existing
CPU colour path: `icc::channel_value` 779, `IccTransform::apply` 395,
`icc::encode_srgb` 48, `icc::powf` 12 and ICC drop glue 41 code-body bytes,
1,275 in total. The combined session draw body grows by 648 bytes.
Other changes include `probe::detect` growing by 60 and `caps::classify`
by 14 with F-238's evidence field. The full symbol table records both gains
and reductions. For example, a separate libm power function appears while
a Naga evaluator closure shrinks, so adding every positive symbol alone
would overstate the increase.

#### Exact accounting

The following deltas include each section's encoded header and payload.
All other sections are unchanged.

| Section | Original to colour | Colour to combined |
|---|---:|---:|
| Types | 27 | 8 |
| Function declarations | 68 | 10 |
| Exports | 136 | 0 |
| Code | 22,141 | 2,219 |
| Data | 1,562 | 0 |
| Total | 23,934 | 2,237 |

The 2,219 combined code bytes balance as 1,275 ICC helper bytes, 648 in
session/cache names, 162 in colour/CPU names, 129 in other Ocelli names and
5 in other named functions. These groups classify symbols by name, rather
than claiming a separate causal cost for every story. Encoded body sizes
exclude function declarations. Twiggy shallow figures include an additional
one-byte declaration for the six ICC helper functions and therefore report
1,281 for those same helpers.

The exact approved graph commands were rerun for both render and decode on
all three sources, each returning 0:

```sh
bin/ocelli.sh cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none
bin/ocelli.sh cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none --no-default-features --features decode-worker
```

Both graphs reach `ocelli-pixel`. Neither reaches `ocelli-dicom`,
`ocelli-codec`, the folder viewer, `openjph-core` or `ritk-codecs`. The
unchanged decode binary and these graph results exclude a newly linked
DICOM reader or compressed codec as the explanation for the render increase.

The first private analysis attempts are retained as failed equivalence
experiments. Keeping DWARF caused Binaryen to fail, while overriding the
whole Cargo profile changed the dependency build and produced different
code. Neither is a product failure or a shipping size measurement. Earlier
reachability receipts without `--target` described native configuration and
are superseded by the six wasm32 receipts above.

Evidence: `wasm-size-attribution.json`, the three
`wasm-named-v6-*/attribution.json` files, per-section SHA256 identities,
complete twiggy listings and retaining paths under the local S46 evidence
directory. The analysis changes no production source or shipping artifact.

### F-244 isolated release size

The empty-Basic-Offset-Table parser patch was also built in isolation. Its
base commit is `3da2ea76e6884a796b7b5c531391747df3e77078`, tree
`5efae178b7aa1a167e1fd251850fdf260ad818a6`. The reviewed tip tree is
`3aec4ae5e5967354a2023ab2a27d088608b5bae7`. Both exact sources ran
`bin/ocelli.sh gate wasm` with private targets and two build jobs.

| Module | Base bytes | Tip bytes | Raw delta | Deterministic gzip bytes, both |
| --- | ---: | ---: | ---: | ---: |
| Render | 2202907 | 2202907 | 0 | 871190 |
| Decode | 16455 | 16455 | 0 | 7663 |

The module files are byte-identical across this pair. Render SHA-256 is
`60cdc9766b4a1b257841fac7cb8415bdb7523e41f4c72dc33d28c3b25520da11`.
Decode SHA-256 is
`07cf6db9ba05bd09fb94b9e8be04c390e0ae80a857c7c4ebc19c74fab1ee7aad`.
Both prescribed `cargo tree` commands specify `wasm32-unknown-unknown`, with
and without `--no-default-features --features decode-worker`. All four
commands pass and show no DICOM crate. Both size-budget files are unchanged,
and no size acceptance command was run. This isolates F-244's size effect,
without attributing unrelated viewer-stack growth to its parser patch.

### Byte-store follow-up at source c1fe3037

The render Wasm is now 2,205,614 bytes, gzip 872,881, SHA256
`2065761b0acb229e3b3937f47c4c52b1c6ba9cf6264fa7877d906bcc50bbcfaf`.
Both totals are 34 bytes smaller than the earlier 3ecb1239 build. The same
named analysis method was repeated: default release dependency profiles,
`strip=none` on the final crate only, the pinned bindgen and optimizer flags,
and exact byte equality of every noncustom section against the shipping file.
The earlier three-source proofs were preserved.

Only the code section changes encoded size. Its 34-byte decrease is fully
accounted for by these encoded function bodies, including their length fields:

| Symbol | Before | After | Change |
|---|---:|---:|---:|
| `ocelli_render::cpu::byte_from_channel` | 182 | 124 | -58 |
| `ocelli_wasm::session::Session::draw_committed_target` | 7141 | 7165 | +24 |
| Total | | | -34 |

The byte store replaces the bounded search with the pinned ties-to-even
primitive, the shared exact-integer reader and checked byte conversion. Its
payload decreases by 57 bytes and its length field by one. The session draw
source is unchanged. Its 24-byte increase is compiled caller variation, not
a separate source addition or performance conclusion. Other named-symbol size
sums and the anonymous-function size sum have zero net change.

The function count remains 5074, and the normalized named-symbol set and all
47 export names and kinds are unchanged. Function reindexing changes some
section contents, so unchanged section sizes are not cross-version byte
identity. The header and original 62-byte producers section are unchanged.
The 34-byte gzip decrease is a whole-module compression observation, not an
additive gzip cost for either symbol.

Retained ICC helper bodies still total 1275 bytes. No new named parser or
codec symbol or export appears. Separate source reachability checks remain
authoritative. Decode Wasm remains byte-identical at 16,455 bytes, gzip 7663,
SHA256 `07cf6db9ba05bd09fb94b9e8be04c390e0ae80a857c7c4ebc19c74fab1ee7aad`.
No budget, shipping profile or acceptance baseline changed.

### Nearest-coefficient correction, source 421e7a2e

Release source `421e7a2e48db0fa6cb8d6cac4f988d477f0c3812` was compared
with the preserved `c1fe30375cdd8d8a0767ff7a941fe2fc4d80dd06` render artifact, SHA256 `2065761b0acb229e3b3937f47c4c52b1c6ba9cf6264fa7877d906bcc50bbcfaf`.
Exactly three data-section bytes changed: binary32 constants `b81a6f2d` to
`b81a6f2a`, `b90d1ed9` to `b90d1ef6`, and `b8c74e10` to `b8c74e0f`.
The complete code section and every other section are byte-identical.
These are stored constants, not instruction immediates.

Render size remains 2,205,614 bytes. Gzip with mtime 0 rises from 872,881
to 872,882 bytes. New render SHA256:
`e9f275dc62700c3b325acbef2283d72c180535c7eec37af59b5936ac316327f2`.
Decode remains byte-identical at 16,455 bytes, SHA256 `07cf6db9ba05bd09fb94b9e8be04c390e0ae80a857c7c4ebc19c74fab1ee7aad`.
Both size budgets are unchanged.

The release Wasm gate, explicit development boundary build, all 19 boundary
tests and CPU-canvas browser checks returned 0. Development output supplies
functional evidence, not release-size or performance evidence.

The evidence retains an initial checker assumption about instruction opcodes
and a sandbox-denied browser startup. Corrected data inspection and the
unchanged browser command with required OS permission passed. Those are
checker/setup diagnostics, not product failures. Source and physical index
remained unchanged throughout.
