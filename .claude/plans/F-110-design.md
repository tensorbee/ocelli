# F-110, binary size budget

**Status**: approved
**Epic ref**: E18.1
**Sprint**: S40
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/12-workspace-and-build.md` section 15.2 gives this release
profile.

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

The same section pins `wgpu = "=30.0.1"`. It does not list wgpu features.
D-14 already adds `webgl` because the defaults cannot reach browser WebGL2.

`docs/hld/A-spike-gates.md` gate A4 says binary size and cold start are
estimated at 3 to 8 MB uncompressed before tuning, with Naga dominating,
and that the figure was unmeasured when the gate was written.

`docs/hld/23-performance-rules.md` section 26 says to measure with the
benchmark harness before optimising. This story measures the release wasm
artefact. It does not tune from a guess.

## What the specification does not cover

The HLD does not say which Naga frontends to drop, and it does not say
how a split build is packaged.

Those gaps are local decisions. They are not HLD formulas.

The pinned wgpu 30.0.1 manifest defaults are `std`, `parking_lot`,
`dx12`, `metal`, `gles`, `vulkan`, `wgsl` and `webgpu`. `glsl` and
`spirv` are already off. On wasm32 the four native backends are dropped.
That is D-27. `webgl` and `wgsl` stay on the render module.

The split is a second build of `ocelli-wasm`. The feature
`decode-worker` is not the default. It does not depend on
`ocelli-render`, `ocelli-viewport` or `wgpu`. The default feature
`render-worker` keeps today's render module. The named user of
`decode-worker` is `bin/ocelli.sh wasm-decode`.

wasm-opt already runs. This story does not set it to false and does not
change its flag list.

Cold start stays with F-111. This story does not remeasure it and does
not close gate A4.

## Problem and reproducer

One wasm module currently links the renderer, so every decode worker
carries Naga. `cargo tree -p ocelli-wasm --target wasm32-unknown-unknown`
shows `wgpu`. A decode build that still shows `wgpu` or `naga` has not
split.

## Scope, API and compatibility

In scope: the wasm32 feature trim in D-27, the `decode-worker` feature,
the second build command, a graph check, and a recorded decode-module
size.

Out of scope: a new JavaScript API, a tolerance edit, a move of the
codec crates, and a cold-start number.

The default module keeps its current exports. The decode module exports
`ocelli_version`, `install_panic_hook`, `panic_record_ptr` and
`panic_record_len`. It does not export `attachOffscreenCanvas`.

`ci/wasm-size-budget.json` changes only when a release measurement of
the render module leaves the recorded 5 percent band. The cause is
written then. The tolerance field stays 0.05. A move inside the band
does not edit that file.

The decode size is a new file, `ci/wasm-decode-size.json`, written from
the measurement. The plan does not invent that byte count.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `bin/ocelli.sh wasm` | default `render-worker` | release wasm-pack | render module, wasm-opt on |
| `bin/ocelli.sh wasm-decode` | `decode-worker` | release wasm-pack, no default features | module whose graph has no wgpu |
| `scripts/pin_and_size_check.py --with-size` | both size files | after the builds | pass, or a declared mismatch |

## State transitions and invariants

This story does not keep session state. The invariant is the graph. The
render build links wgpu. The decode build does not. wasm-opt stays a
flag list.

## Ownership, commit and recovery

Not applicable. The builds do not publish and do not keep a device.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Decode graph on wasm32 has no wgpu and no naga | `decode_graph_check.py` | A decode build that still links the renderer | implementation |
| AC2 | Default graph on wasm32 still has wgpu | the same check | Dropping the renderer from the shipping module | implementation |
| AC3 | wasm-opt is a list and is not false | Cargo.toml read by the check | `wasm-opt = false` | implementation |
| AC4 | wasm32 wgpu features are the D-27 set | manifest read | Native backends left on the browser module | implementation |
| AC5 | ocelli-render native wgpu includes webgl and the default backends. ocelli-wasm and ocelli-compute native include the default backends | manifest read | A trim that removes tier B from the renderer or a desktop backend | implementation |
| AC6 | Decode size file matches the built decode module within 5 percent | `--with-size` after `wasm-decode` | A guessed byte count | integrated |
| AC7 | Render budget moves only with a cause when the measurement leaves the band | existing size check | Editing the tolerance to go green | integrated |
| AC8 | Decode bindings export the panic surface and do not name attachOffscreenCanvas | `--with-size` reads the decode `.d.ts` | A decode module that still exports the canvas | integrated |

## Approach

`ocelli-wasm` gains `default = ["render-worker"]`. `render-worker`
enables optional `ocelli-render`, `ocelli-viewport` and `wgpu`.
`decode-worker` enables none of them. `canvas` and `session` compile
only with `render-worker`.

On wasm32, `ocelli-render`, `ocelli-wasm` and `ocelli-compute` set
`default-features = false` and the D-27 feature list. The workspace
entry sets `default-features = false` too, because Cargo rejects the
member switch otherwise. Each native dependency line names the pinned
default backends. `ocelli-render` also names `webgl`. `ocelli-oracle`
and the `ocelli-native` dev-dependency use a native target table. The
oracle also has a wasm32 table with the D-27 set, because the
cross-target proof checks that crate for wasm32. A shared oracle line
would turn the native backends on in that resolve.

`bin/ocelli.sh wasm-decode` runs wasm-pack with `--no-default-features
--features decode-worker` into `pkg-decode`. That directory stays
gitignored like `pkg`.

`scripts/decode_graph_check.py` checks the manifests and both graphs.
`scripts/pin_and_size_check.py --with-size` checks the decode size file
and the decode bindings. The wasm gate runs the graph check, both
packs, then `--with-size`. The floor pins gate without `--with-size`
does not build wasm.

## Dependencies, write ownership and resources

Depends on F-042, which is done. This story writes the wasm manifests,
`bin/ocelli.sh`, the size checker, `ci/wasm-decode-size.json` and
`docs/hld/DEVIATIONS.md`. It does not share those files with F-112,
F-113 or F-114. The wasm build is exclusive with every other cargo
process.

## Verification and evidence reuse

Focused check: `cargo check -p ocelli-wasm --no-default-features
--features decode-worker --target wasm32-unknown-unknown`, then the
graph script, then `cargo check -p ocelli-render --target
wasm32-unknown-unknown`.

The feature profile is `bin/ocelli.sh gate --floor` and then
`bin/ocelli.sh gate corpus`. The wasm gate builds the render module.
`wasm-decode` is run before that gate so the decode size file matches
the tree. A later edit to a crate in the decode graph invalidates the
size file.

## Boundary and tier

- wasm-bindgen: `ocelli-wasm` only
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a for the decode module. The render module keeps `webgl`
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| graph | decode build has no wgpu or naga | `scripts/decode_graph_check.py` |
| manifest | wasm-opt stays on and D-27 holds | the same script |
| size | recorded decode bytes match the artefact | `pin_and_size_check.py --with-size` |

No pixel fixture. This story does not compute a sample.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no binary-size row.

## Deviations

**D-27.** The wasm32 feature set above. Cargo rejects a member
`default-features = false` unless the workspace entry sets it too, so
the workspace line is `{ version = "=30.0.1", default-features = false }`
and each native crate names the default backends. The version pin is
unchanged. The row is in `docs/hld/DEVIATIONS.md`.

## LLD impact

- `docs/lld/build-targets.md`
- `docs/lld/tier-resolution.md`
- `docs/lld/guards.md`
- `docs/lld/gpu-ownership.md`

## Open questions

None. The gaps in the previous section are decided.
