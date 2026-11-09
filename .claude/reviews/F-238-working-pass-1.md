# F-238 review, pass 1

**Reviewed**: `67342a7c..69f05876` on `work/f-238-xai` in `ocelli-wt-f-238`, HEAD `69f05876a469cb0f0ecd284fda93303e1584aa2b`, commits `c86e8734`, `258a1894` and `69f05876` only. Working tree clean apart from the untracked `.venv` and `tools/oracle/out`. F-229, F-232 and F-233 material on the branch was not judged.
**Author / reviewer**: GLM in opencode is the author, with four self-review cycles in `.claude/scratch/F-238-self-review-1.md` to `-4.md`. Reviewer is Claude Opus 5.5 as an independent subagent of the sprint lead. The reviewer wrote none of the reviewed material.
**Result**: 6 defects, 3 smells, 2 nitpicks

## Acceptance challenged

- **AC1, no objc2 crate in the shipped Apple graph.** Measured independently with `cargo tree --locked --offline -p ocelli-native --no-default-features --prefix none --format {p}` for seven Apple triples (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-apple-ios`, `x86_64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-apple-tvos`, `aarch64-apple-visionos`) under `-e normal` and `-e normal,build`. Zero `objc2*`, `block2`, `dispatch2` or `raw-window-metal` lines on all fourteen. The development graph (default features, `-e normal`) holds 13 objc2-family lines on each triple. With `-e features,normal,build` the shipped Apple graph resolves `wgpu-hal` with only `default` and `portable-atomic`, so no hal backend at all. Linux resolves `wgpu-hal` `vulkan` and `gles`, Windows `dx12`, `vulkan` and `gles`.
- **AC2, the check reads the resolved graph.** Mutations in a scratch copy with its own `CARGO_TARGET_DIR`. Red for an Apple-gated normal dependency on `objc2`, for `default = ["metal"]` on `ocelli-render`, for `vulkan-portability` and for `angle` on the render table. Green for `objc2` as a build-dependency of `ocelli-native` (S1 below) and for an iOS-only normal dependency (outside the archive, not a finding). The catalogue probe `shipped-native.metal-planted` went red with its control green.
- **AC3, development keeps Metal.** `target/debug/ocelli-desktop` from `bin/ocelli.sh native` step 1 resolved `Tier A, decided by Benchmark` on `"Apple M4 Max" on Metal` and links `Metal.framework`. `gate gpu` itself was not run, per the brief.
- **AC4, the shipped macOS binary reports tier C.** `target/release/ocelli-desktop` from step 6 printed `Tier Cpu, decided by NoAdapter`, `caps compute false, max_tex_3d 0, max_buffer 0`, exit 0. `OCELLI_TIER=a` gives `RefusedUnconstructible(A)` and still tier C. `ocelli-server` prints the same. `otool -L` shows only `libiconv` and `libSystem`, and `strings` finds zero `objc2` against 62 in the debug binary. Every wgpu instance creation in library code (`probe.rs:292`, `:524`, `:578`) sits behind the new guard. Tier C features on this build are the existing `TierCUnavailable` and `CpuRaster` paths, so nothing produces a different result.
- **AC5, notices.** `gate licences` green. `THIRD_PARTY_LICENSES` has 0 `objc2` strings, 105 `name:` lines, 103 unique names (see the 105 and 103 note below).
- **Composition.** Rejected the plausible wrong implementation of dropping `metal` and keeping `gles` by reading wgpu-hal 30.0.1's `gles` (names `dep:objc2`) against wgpu-core 30.0.1's `gles` (names only `wgpu-core-deps-windows-linux-android/gles` and `wgpu-core-deps-emscripten/gles`, neither present on Apple). The lead's Q2 measurement is therefore that `gles` on Apple pulls in no objc2 crate, and `gles` stays. The shipped Apple tier is still C, because the `gles` cfg on Apple needs `angle` and the `vulkan` cfg needs `vulkan-portability` (`wgpu-30.0.1/build.rs:17` to `:35`).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh native` | HEAD `69f05876` | now | 0, fifteen steps, step 5 OK on both Apple triples |
| `bin/ocelli.sh gate native` | same | now | 0, plus 25 simd and 7 shipped-check unit tests OK |
| `bin/ocelli.sh gate licences` | same | now | 0, `OK: licence check (rust, npm, python, notices, holder, binaries)`, 73 tests OK |
| `bin/ocelli.sh gate guards` | same | now | 0, 445 refusal probes red, 109 controls green |
| `guard_probe.py --profile deep --only` for `shipped-native.metal-planted`, `decode-graph.shipped-table-drops`, `decode-graph.shipped-forwarding-absent` | same | now | each red for its declared reason, control green |
| `deviation_check.py`, `prose_check.py`, `decode_graph_check.py` | same | now | 0, 33 deviations, voice clean |
| `cargo tree` objc2 census, 7 Apple triples by 4 edge sets plus dev default | same | now | shipped 0 on normal and normal,build, dev 13 lines |
| `cargo tree -e features,normal,build -i wgpu-hal/wgpu-core/wgpu` on 4 triples | same | now | Apple hal has no backend feature, Linux vulkan and gles, Windows dx12, vulkan and gles |
| Shipped and dev `ocelli-desktop`, shipped `ocelli-server`, `otool -L`, `strings` | binaries from the native run above | now | tier C and tier A as above |
| Scratch mutations M1 to M6 on manifests | scratch copy of HEAD | now | see AC2 |
| Scratch mutations S1 to S5 on `shipped_native_check.py` | scratch copy | now | regex, exit-code and target-pair mutations red in the unit suite, `--no-default-features` dropped red on the real graph, wrong `-p` package green in both and caught only by the deep probe |
| `probe::tests::a_development_build_compiles_at_least_one_backend` with `metal` removed from render dev-deps | scratch | now | FAILED, then ok after restore |
| Detect guard disabled (`if false && ...`) | scratch | now | shipped binary panics at `wgpu-30.0.1/src/api/instance.rs:65`, shipped check, `ocelli-native` tests and `ocelli-render` lib tests all green |
| Per-target closure over 9 triples | HEAD | now | `aarch64-pc-windows-msvc`, `i686-pc-windows-msvc` and `x86_64-unknown-freebsd` add nothing to the five, Android adds only the android crates |
| GLM self-review receipts | `.claude/scratch/F-238-self-review-*.md` | prior, not relied on | read for the catch figure only |

## Defects

### D1, a false "because" for the development tier A

**Where**: `docs/lld/tier-resolution.md:360` to `:365`
**What**: "The same crate built with its default features resolves `Tier A` ... because the native dev-dependency tables restore `metal` for every build the test suite runs in". The binary built with default features is not a test build. Dev-dependencies are never used when building a `bin` target, and the binary gets Metal from `ocelli-native`'s `default = ["metal"]` through `ocelli-render/metal`.
**Why it is wrong**: microscope section 3, a false factual claim. The sentence names the wrong mechanism for AC3's own observation, so a maintainer who deletes `ocelli-native`'s default would read this sentence as saying tier A survives.
**Evidence**: the dev binary run above is `cargo build -p ocelli-native --bins` with no test target, and `crates/ocelli-native/Cargo.toml:49` to `:50` is the only path to `wgpu/metal` in that build.
**Scope and regression**: prose only, present on HEAD. Name `ocelli-native`'s default feature for the binary, and keep the dev-dependency clause for the test suite.

### D2, the new section splits an existing subsection and orphans its last paragraph

**Where**: `docs/lld/tier-resolution.md:336` and `:367`
**What**: the new `## A build with no backend compiled resolves tier C and says so` is inserted inside `### The override's named user`, before that subsection's final paragraph. "In a browser the value comes from the shell instead" now sits under the no-backend heading, where "the value" has no antecedent. It was the `OCELLI_TIER` override value.
**Why it is wrong**: the moved sentence reads as false in its new place, which is a defect under microscope section 3.
**Evidence**: `sed -n 300,380p docs/lld/tier-resolution.md`.
**Scope and regression**: prose only. Move the new section after line 370 or move the browser paragraph back above it.

### D3, the measuring host is not the host `ci/tier-thresholds.json` records

**Where**: `docs/lld/tier-resolution.md:349`
**What**: "Measured on the macOS host of `ci/tier-thresholds.json`". That file's only measurement records `"adapter": "Apple M5 Max"`. This host reports `Apple M4 Max` (`sysctl machdep.cpu.brand_string`, and the dev tier report the paragraph itself quotes).
**Why it is wrong**: a checkable identity claim that does not hold.
**Evidence**: `sed -n 15,35p ci/tier-thresholds.json`.
**Scope and regression**: prose only. Name the machine directly instead of the thresholds file.

### D4, "three places" leaves out a fourth place where development restores Metal

**Where**: `docs/hld/DEVIATIONS.md:50` (D-33), `docs/lld/build-targets.md:314`, `crates/ocelli-render/Cargo.toml:47`
**What**: all three name exactly three places, the two crates' dev-dependency tables, `ocelli-native`'s default, and the hard `metal` in `ocelli-wasm` and `tools/oracle`. `crates/ocelli-native/Cargo.toml:59` is a fourth: `ocelli-native`'s own dev-dependency on `wgpu` names `metal`, so `cargo test -p ocelli-native --no-default-features` keeps Metal.
**Why it is wrong**: an enumeration given as complete that is not, written into the deviation register.
**Evidence**: `grep -n wgpu crates/ocelli-native/Cargo.toml`.
**Scope and regression**: prose only. Add the fourth place, or drop the count.

### D5, `ocelli-compute` is described as part of the shipped build, and nothing depends on it

**Where**: `scripts/decode_graph_check.py:122` to `:123`, also `:98` to `:99`, and the D-33 clause "`ocelli-render` and `ocelli-compute` forward `metal` to wgpu through a cargo feature that this build omits"
**What**: "The renderer and the compute crate are the two the shipped native archive is built from". `cargo tree -i ocelli-compute --workspace -e all` shows no dependent at all, and the shipped `ocelli-native` graph names `ocelli-render`, `-cache`, `-pixel`, `-core`, `-seg`, `-viewport` and `-volume`, but not `ocelli-compute`.
**Why it is wrong**: a false structural claim in the guard that justifies holding compute to the shipped rule.
**Evidence**: both commands above, on HEAD.
**Scope and regression**: prose only. Say that compute shares the renderer's device and is held to the same rule for when it joins a shipped graph, which is what `crates/ocelli-compute/Cargo.toml` already says.

### D6, `wgpu-core-deps-apple` is attributed to the wrong leg

**Where**: `docs/lld/build-targets.md:351` to `:357`
**What**: the dropped carriers list puts `wgpu-core-deps-apple` among "crates only the emscripten, android and union-only legs ever reached". It is an optional, `cfg(target_vendor = "apple")` dependency of wgpu-core, enabled by `metal`, and the development `aarch64-apple-darwin` graph resolves it. It left the notices for the same reason the objc2 family did, because `metal` is off.
**Why it is wrong**: a false sentence about why a crate left the shipped notices.
**Evidence**: `cargo tree -p ocelli-native -e normal,build --target aarch64-apple-darwin` (default features) prints `wgpu-core-deps-apple v30.0.1`, and the same with `--no-default-features` does not. `wgpu-core-30.0.1/Cargo.toml` marks it `optional = true` under the Apple target.
**Scope and regression**: prose only. Move it next to `block2`, `dispatch2` and `raw-window-metal`.

## Smells

### S1, the shipped check ignores build edges while its sibling walk counts them

**Where**: `scripts/shipped_native_check.py:46` to `:50` (`-e normal`), against `scripts/licence_check.py` `NATIVE_TREE_COMMAND` (`-e normal,build`)
**What**: the OK line says "compiles no objc2 crate" and the guard's `refuses` says "An objc2 package in the resolved graph". Planting `objc2 = "=0.6.4"` under `[build-dependencies]` of `ocelli-native`, or Apple-gated build-dependencies, left `shipped_native_check.py` at exit 0. The same plant took the licence closure from 105 to 107 with `objc2` and `objc2-encode`, so `gate licences` goes red today. But the documented repair for that red is `licence_check.py --write binaries`, and after it `gate native` is the only objc2 refusal and it stays green.
**Why it matters**: the story row says the shipped build "compiles no objc2 crate", and a build-dependency is compiled. The two walks of "the shipped graph" disagree on edge kinds, and the docstring gives a reason only for leaving out dev edges.
**Smallest regression**: use `-e normal,build` in `COMMAND` and add a probe that plants a build-dependency. If build edges are excluded on purpose because they do not ship, say so in the docstring and narrow the OK line.

### S2, AC4's guard is executed by nothing that runs on its own

**Where**: `crates/ocelli-render/src/probe.rs:301` to `:307`, `bin/ocelli.sh:632` to `:640` (step 6)
**What**: with the guard disabled in a scratch copy the shipped binary panics at `wgpu-30.0.1/src/api/instance.rs:65`, and the shipped check, `ocelli-native` tests and `ocelli-render` lib tests all stay green. Step 6 links the binary and never runs it, although its comment says "the binary it produces is the one whose tier report AC4 reads: it resolves tier C and prints the evidence rather than panicking". No gate, script or workflow runs `ocelli-desktop` or `ocelli-server` (`grep` over `bin/ocelli.sh`, `scripts/*.py`, `ci`, `.github`). The new unit test only proves development has a backend, which is the inverse case.
**Why it matters**: microscope section 4. A later instance-creation site in `ocelli-render`, or a guard edit, restores a startup panic in the shipped macOS build with every gate green.
**Smallest regression**: after step 6, run `target/release/ocelli-desktop` and require exit 0 and a `Tier ` line. On an Apple host also require `Tier Cpu, decided by NoAdapter`.

### S3, `ocelli-compute`'s `metal` feature has no named user, and the guard now requires it

**Where**: `crates/ocelli-compute/Cargo.toml:45` to `:47`, `scripts/decode_graph_check.py:555` to `:556`
**What**: the manifest says "Nothing enables it today". `AGENTS.md:122` is "No feature flag without a named user". `check_trimmed(..., shipped=True)` on compute refuses the manifest if the flag is removed, so the guard locks in a flag the rule forbids.
**Why it matters**: the design scoped compute in because it shares the device, which is a reason to drop `metal` from its normal table, not to add a forwarding feature ahead of a consumer. When a consumer arrives it can add the flag then.
**Smallest regression**: remove compute's `[features] metal` and judge compute's table against the shipped set without the forwarding requirement, or record the lead's ruling that the flag's named user is a future one.

## Nitpicks

- **N1.** The shipped tier report says `decided by NoAdapter` and `adapters 0 seen`, which is the same text a machine with backends but no GPU prints. A reason naming "no backend compiled" would save a macOS user from debugging their hardware. D-33 documents it, so this does not block.
- **N2.** D-33's "because `gles` is only reached there through `angle`" gives one of the three conditions for an empty Apple backend set. `build-targets.md` gives all three (`metal`, `angle`, `vulkan-portability`).

## Verified clean

- wgpu API and feature names against the pinned registry source: `Instance::enabled_backend_features` is `pub const fn` at `wgpu-30.0.1/src/api/instance.rs:108`, platform-filtered through the build.rs cfg aliases. `Instance::new` panics on an empty set at `:65`. `new_instance_with_webgpu_detection` forwards to `Instance::new`. Feature names `metal`, `gles`, `vulkan`, `dx12`, `angle`, `vulkan-portability` checked in wgpu, wgpu-core, wgpu-core-deps-windows-linux-android and wgpu-hal 30.0.1 manifests.
- D-33's per-platform sentence, executed: Linux keeps Vulkan and native GL, Windows keeps D3D12, Vulkan and native GL, from the resolved wgpu-hal features per triple and the cfg aliases. D-33's measurement clause holds: within the D-27 set only `metal` reaches objc2 on Apple, and `vulkan` reaching `raw-window-metal` in a no-filter resolve matches wgpu-hal's `vulkan` feature list.
- The workspace is `resolver = "3"`, so the dev-dependency tables do not unify into `cargo build -p ocelli-native --no-default-features`, which the step 6 binary's link list confirms.
- `probe.rs` GL guards: `resolve_webgl` and `resolve_native_gl` check `Backends::GL`, which `enabled_backend_features` sets for `any(gles, webgl)`, and `webgl` is wasm-only. Comments there are accurate.
- The licence closure change: five triples, union of per-target shipped graphs, records from an unfiltered metadata document. Three more desktop triples add nothing. `test_the_archive_target_list_is_the_five_desktop_triples` pins the list.
- **GLM's 105 and 103 note is correct.** `THIRD_PARTY_LICENSES` has 105 `name:` lines with `hashbrown` (0.16.1, 0.17.1) and `syn` (2.0.119, 3.0.4) each listed twice, so 103 unique names. The closure has 105 (name, version) pairs. The parent file had 129 entries with the same two duplicates plus `jni-sys` (0.3.1, 0.4.1), which this story drops correctly because only Android reaches it. Not a defect.
- Step renumbering: every reference to the old steps in `bin/ocelli.sh`, `docs/SOURCE-POLICY.md`, `docs/lld/codecs.md`, `docs/lld/build-targets.md`, D-22, `scripts/guards/catalogue.py` and `scripts/tests/test_simd_variant_check.py` points at the right step under the fifteen-step numbering.
- README line 475 to 477, true apart from S1's edge-kind caveat. `use.html` and `more.html` make no native backend or macOS claim, and their tier C sentences describe existing behaviour.
- The test `a_development_build_compiles_at_least_one_backend` goes red when `metal` is removed from render's dev-dependencies, as claimed.
- No `unsafe`, no `wasm-bindgen` outside `ocelli-wasm`, no pixel arithmetic, no new dependency, no version change in the reviewed commits.

## Self-review catch figure

9 of the 11 findings were within reach of GLM's own self-review notes. All 9 are blocking.

- D1, D3 and D6 are sentences the notes record as executed (cycle 2 "The same crate built with its default features resolves Tier A: measured", cycle 1 "pasted reports match the real binary", cycle 3 "23 of 23 present").
- D2, D4 and D5 are in files the notes record as reviewed sentence by sentence, and each falls to one `sed` or `cargo tree`.
- S1 sits beside cycle 1's own mutation "build edges dropped from the native walk", made in the sibling walk only. S2 is written into the test docstring as known ("the true branch is observed by running the shipped binary, not by a test") and never raised as a finding. S3 is written into the manifest as "Nothing enables it today".
- N1 and N2 are judgement calls a self-review would not be expected to raise.

## Limitations and handoff

- `gate gpu` was not run, per the brief. AC3 rests on the dev desktop binary's tier report only, not on the `#[ignore]` GPU suite.
- No Intel Mac was available, so `x86_64-apple-darwin` is resolved by `cargo tree` and never linked or run.
- Linux and Windows shipped binaries were not built or run. The per-platform tier sentence is taken from the resolved features and the cfg aliases, not from execution.
- The scratch copy and its two target directories were deleted with `shutil.rmtree`. Writing run logs into the shared scratchpad overwrote earlier files there named `native.log`, `gate-native.log`, `gate-licences.log` and `gate-guards.log`, and `closure.py` and `meta.err` there are this pass's.
- Next owner: the implementer remediates D1 to D6 and S1 to S3, then a fresh independent pass 2 reviews the changed material. This is a bounded review of F-238's three commits, not acceptance and not sprint readiness.
