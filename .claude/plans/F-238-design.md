# F-238, Native build without wgpu's Metal backend

**Status**: draft
**Epic ref**: E19.7
**Sprint**: S47
**Estimate**: 2w

Story row, `docs/sprints/BACKLOG.md`: "Native build without wgpu's Metal backend: the shipped native build compiles no objc2 crate, proven by the resolved dependency graph for every Apple target, while development builds keep Metal for the tier A gate". Layer Build, depends on F-220 and F-233, milestone M20.

`allocation.json` notes: "Found by F-220's first review reading the objc2 LICENSE.md. Answered by the operator on 2026-11-06: a native build without wgpu's Metal backend, a story in S47."

## Normative source, transcribed

The HLD has no native backend choice. `grep -n 'Metal\|Backends::' docs/hld/*.md` finds nothing. The sources are:

- `docs/hld/12-workspace-and-build.md:76`: `wgpu = "=30.0.1" # pin EXACTLY - breaking changes ~quarterly`.
- `docs/hld/05-rendering.md:20`, the tiers: Tier A is WebGPU and Tier B is WebGL2 through wgpu's downlevel profile.
- D-14, `docs/hld/DEVIATIONS.md`: "wgpu 30.0.1's default feature set is `std, parking_lot, dx12, metal, gles, vulkan, wgsl, webgpu`, read from the pinned crate's own manifest."
- D-27, same file: "A native build names `std`, `parking_lot`, `dx12`, `metal`, `gles`, `vulkan`, `wgsl` and `webgpu`, and `ocelli-render` also names `webgl` ... `dx12`, `metal`, `gles` and `vulkan` are native backends."

### The operator's answer, `docs/sprints/CURRENT_SPRINT.md`

"F-220's first review read the upstream objc2 `LICENSE.md`, which flags that redistributing these crates, generated from Apple SDK headers, under the Xcode licence is unclear. They reach the native binaries through wgpu's Metal backend. ... **Answered by the operator on 2026-11-06:** a native build without wgpu's Metal backend, so the objc2 crates are not shipped. A story in S47."

### `docs/RELEASE.md`, what ships natively

"`ocelli-native` is `publish = false` for as long as it stays stubbed ... The milestone that ships them lifts the flag and adds it to the crates.io list above. That milestone also ships a native archive carrying `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `THIRD_PARTY_NOTICES` and `THIRD_PARTY_LICENSES`."

## Facts measured for this plan

Read on 2026-11-06 on `sprint/s46` at `1ba258e`.

1. The native wgpu feature list `["std", "parking_lot", "dx12", "metal", "gles", "vulkan", "wgsl", "webgpu"]` is named by `crates/ocelli-render/Cargo.toml:42` (with `webgl`), `crates/ocelli-compute/Cargo.toml:29`, `crates/ocelli-wasm/Cargo.toml:124`, `crates/ocelli-native/Cargo.toml` dev-dependencies, and `tools/oracle/Cargo.toml:37`. The workspace pin is `wgpu = { version = "=30.0.1", default-features = false }`.
2. `Cargo.lock` holds objc2 0.6.4, objc2-encode 4.1.0, and objc2-core-foundation, objc2-core-graphics, objc2-foundation, objc2-io-surface, objc2-metal and objc2-quartz-core at 0.3.2.
3. In the pinned manifests, wgpu's `metal = ["wgpu-core?/metal"]`, wgpu-core's `metal = ["wgpu-core-deps-apple/metal"]`, and wgpu-core-deps-apple's `metal = ["wgpu-hal/metal"]`. wgpu-hal 30.0.1's `metal` names `dep:objc2` and the five objc2 framework crates, and its `gles` also names `dep:objc2`. Which objc2 crates survive with `metal` off and `gles` on was not measured.
4. `crates/ocelli-render/src/probe.rs` creates the instance with `InstanceDescriptor::new_without_display_handle()`, whose backends are the compiled-in default, and enumerates `Backends::all()` on native. The GL paths set `Backends::GL`.
5. The tier A GPU tests run under `bin/ocelli.sh gate gpu` on the operator's Apple M4 Max through Metal, which the F-227 reviews used.
6. `ocelli-native` builds `ocelli-desktop` and `ocelli-server`, depends on `ocelli-render` only, and is `publish = false`.

## What the specification does not cover

Which build ships without Metal, what a macOS user of that build gets, and whether that departs from D-27 or the tier model.

## Problem and reproducer

```bash
cargo tree --offline -i objc2@0.6.4 --target aarch64-apple-darwin -e normal
```

reaches, through `wgpu-hal`, `ocelli-render`, `ocelli-compute`, `ocelli-native`, `ocelli-oracle`, `ocelli-wasm`, `ocelli-folder-viewer` and `ocelli-example-server`, measured with `--offline`. A native archive built today would ship the objc2 crates.

## Scope, API and compatibility

In scope: a feature on `ocelli-render` (and `ocelli-compute`, which shares the device) that removes `metal`, the native release build that uses it, a check over the resolved graph for that build, and the notices that follow. Non-goals: the wasm module, which compiles no wgpu native backend, and development builds, which keep Metal so `gate gpu` keeps a tier A adapter on macOS.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| shipped native archive | the Metal-free feature set | release, every target in the archive | binaries with no objc2 code |
| `gate gpu`, development | today's features | macOS M4 Max | tier A through Metal, unchanged |
| tier resolution on a Metal-free macOS build | `probe.rs` | `Backends::all()` over what is compiled | tier B through GL if a GL backend links, otherwise tier C, reported |
| `THIRD_PARTY_LICENSES` for the native archive | F-220's generator | the shipped graph | no objc2 entry |

## State transitions and invariants

Not applicable. The invariant: the shipped native graph contains no `objc2*` package on any Apple target.

## Ownership, commit and recovery

Not applicable.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The shipped build's resolved graph has no `objc2*` package for `aarch64-apple-darwin` and `x86_64-apple-darwin` | a gate step over `cargo tree` for that feature set, refusing any objc2 name | Dropping `metal` and leaving `gles`, which fact 3 shows still names `objc2` | Implementation |
| AC2 | The check refuses a planted `metal` in the shipped feature set | probe with a control | A check that reads a feature list rather than the resolved graph | guards |
| AC3 | Development is unchanged. `gate gpu` on the M4 Max resolves tier A through Metal | `gate gpu` | Metal removed from every build | Implementation |
| AC4 | A Metal-free macOS binary resolves a tier and reports it, never a blank canvas | `ocelli-desktop` run printing its tier report | A build that fails to start | Implementation |
| AC5 | The native archive's `THIRD_PARTY_LICENSES` lists no objc2 crate and F-220's gate passes | `gate licences`, which runs `scripts/licence_check.py` | Notices generated from the development graph | Implementation |

## Approach

Depends on Q1 and Q2.

## Dependencies, write ownership and resources

Declared: F-220, which owns the notices generator, and F-233, which writes the `native)` arm of `bin/ocelli.sh` first. Writes the five manifests of fact 1, `bin/ocelli.sh`, the notices inputs, the guard catalogue, budget and runbook, `docs/hld/DEVIATIONS.md` if Q3 needs a row, and `docs/lld/build-targets.md`.

## Verification and evidence reuse

```bash
bin/ocelli.sh gate native guards
bin/ocelli.sh gate gpu
```

AC3 and AC4 need the macOS host.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): unavailable natively on macOS in the shipped build, reported by tier resolution. Full on other native targets through Vulkan or D3D12. Full in development builds
- Tier B (WebGL2): native GL on macOS only if Q2's choice links one, otherwise unavailable and reported
- Tier C (CPU): full, the macOS fallback of the shipped build

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| guard probe | AC2 | `scripts/guards/catalogue.py` |
| integration | AC1, AC5 | `gate native`, `gate licences` |
| host | AC3, AC4 | macOS M4 Max |

## Parity surface covered

None.

## Deviations

Likely one new row, numbered when this plan is approved: the shipped native build departs from D-27's native feature list, and on macOS it resolves without a Metal tier A. Named here under Q3, not yet in the register.

## LLD impact

`docs/lld/build-targets.md` and `docs/lld/tier-resolution.md`, the shipped feature set and the macOS tier.

## User-facing documentation impact

`README.md`'s third-party paragraph near line 406 and `examples/folder-viewer/src/more.html` if it describes native backends. Checked at approval.

## Open questions

- **Q1, which build is Metal-free.** Affects AC1, AC3. Options: **O1, recommended**, a cargo feature, default on, that the shipped native archive turns off, so development and `gate gpu` keep Metal. O2, Metal removed from every native build, which loses tier A on the macOS development host (fact 5). O3, ship no macOS native archive. Owner: the lead in the S47 design round.
- **Q2, `gles` on Apple.** Fact 3 shows wgpu-hal's `gles` names `objc2`. Options: drop `gles` from the Apple shipped build as well, leaving tier C on macOS, or measure that `gles` on Apple compiles only the base `objc2` crate and decide whether that crate carries the same redistribution question. Owner: the lead, after a measurement in this story.
- **Q3, the deviation row.** Whether the shipped feature set and the macOS tier outcome need a new row beside D-27. Owner: the lead at approval.
