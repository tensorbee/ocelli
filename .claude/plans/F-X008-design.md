# F-X008, One parity-target version string, and a licence in the published wasm package

**Status**: approved
**Epic ref**: Y1.3
**Sprint**: S04
**Estimate**: 1w

## Normative source, transcribed

_Transcriptions preserve the words in the HLD, with em-dashes and prose
semicolons normalised because plans are prose-checked. The tracked HLD remains
authoritative._

### `docs/hld/01-purpose-and-scope.md`, section 1

> In scope for Phase 1: feature parity with cornerstone3D v5.8.9 - DICOM
> ingest and metadata, the pixel pipeline, stack and volume viewports, MPR and
> 3D rendering, segmentation, and the approximately sixty-three tools -
> delivered as a Rust core with a TypeScript shell.

### `docs/hld/08-validation-architecture.md`, section 11

> Cornerstone3D is a correct reference implementation that can render any
> series you own. The harness pushes the same study through both stacks and
> compares frames within a written per-modality tolerance, with metadata
> diffed alongside pixels because a wrong rescale slope can still produce a
> plausible image.

### `docs/hld/12-workspace-and-build.md`, section 15.2

```toml
[workspace.package]
edition = "2024"
rust-version = "1.85" # dicom-rs MSRV floor
```

### `docs/hld/DEVIATIONS.md`, D-11

> The oracle pins `@cornerstonejs/core`, `@cornerstonejs/tools` and
> `@cornerstonejs/dicom-image-loader` at exactly `5.8.2`.

> **v5.8.9 does not exist.** Checked against the npm registry on 2026-09-04:
> `@cornerstonejs/core` has 1124 published versions, the highest is 5.8.2, and
> the same holds for the other two packages. 5.8.2 is the highest published
> 5.8.x and therefore the nearest installable reference.

## What the specification does not cover

The HLD does not specify how prose and generated planning material obtain the
effective parity target after D-11. It also does not specify how `wasm-pack`
finds licence text for the published package.

## Approach

Treat the exact dependency pins in `tools/oracle/package.json` as the
executable authority for the target. Correct `.claude/commands/parity.md` and
the `PREAMBLE` in `scripts/gen_sprint_plan.py` to name 5.8.2 and D-11, then
regenerate `.agents/skills/parity/SKILL.md` and verify the sync. Extend the
existing oracle pin test to assert those two operational consumers contain
5.8.2. Its scan is deliberately limited to those two files. HLD text,
`docs/hld/DEVIATIONS.md`, `docs/RELEASE.md`, and oracle diagnostics may retain
5.8.9 when explaining why the unavailable target was replaced.

Add tracked relative symlinks `crates/ocelli-wasm/LICENSE`,
`crates/ocelli-wasm/LICENSE-MIT`, and `crates/ocelli-wasm/LICENSE-APACHE` to the
three repository-root licence files. This gives `wasm-pack` package-local
licence inputs without copying legal text. Extend
`scripts/pin_and_size_check.py --with-size`, which already inspects
`crates/ocelli-wasm/pkg`, to require both dual-licence texts in the generated
package and to compare their bytes with the repository originals. The `wasm`
gate owns this proof because it is the gate that creates and measures that
package.

Anticipated write set: `.claude/commands/parity.md`,
`.agents/skills/parity/SKILL.md`, `scripts/gen_sprint_plan.py`, the three
licence symlinks under `crates/ocelli-wasm`,
`scripts/pin_and_size_check.py`, its focused test file,
`scripts/guards/catalogue.py`, `ci/guard-probe-budget.json`,
`docs/lld/oracle.md`, and `docs/lld/packaging.md`.

## Boundary and tier

- wasm-bindgen: `ocelli-wasm` only, packaging metadata changes only
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | The two operational parity consumers resolve to the 5.8.2 oracle pin | `tools/oracle/tests/pins_test.mjs` |
| package | The release wasm package contains byte-identical MIT and Apache texts | `scripts/tests/test_pin_and_size_check.py` and `scripts/pin_and_size_check.py --with-size` |
| guard mutation | A stale operational parity claim and an absent packaged licence each make their named guard red | `scripts/guards/catalogue.py` through `scripts/guard_probe.py` |

## Parity surface covered

None. This story makes the reference version for the whole surface
unambiguous but implements no Appendix B row.

## Deviations

D-11, already recorded. No new deviation.

## LLD impact

Update `docs/lld/oracle.md` and `docs/lld/packaging.md`.

## Open questions

None.
