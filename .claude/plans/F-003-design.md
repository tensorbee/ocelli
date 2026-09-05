# F-003, TS package scaffold, bundling, npm publish pipeline

**Status**: approved
**Epic ref**: E1.3
**Sprint**: S02
**Estimate**: 2w

## Normative source, transcribed

_Transcriptions below are verbatim except for one normalisation: a prose
semicolon in the source is written as a comma, and an em-dash as a hyphen,
because `scripts/prose_check.py` covers `.claude/plans/` and `docs/hld/` is
exempt. No word is changed. Where the exact bytes matter, the tracked
Markdown under `docs/hld/` wins._

### `docs/hld/12-workspace-and-build.md`, section 15.1, the packages entries

> packages/
>
> core/ \# @ocelli/core (TypeScript shell)
>
> react/ \# @ocelli/react

### `docs/hld/07-concurrency-and-typescript.md`, section 10, verbatim

> Not a compromise - the correct home for this work. Cornerstone has 531
> addEventListener sites, 137 uses of document, and 72 createElement calls,
> and pointer handling is the one workload where crossing into WebAssembly
> makes things measurably worse.
>
> - DOM, pointer, touch and wheel events, canvas lifecycle, ResizeObserver
>
> - The SVG annotation drawing layer - DOM-native, and faster left there
>
> - Tool interaction state machines, the geometry and hit-testing behind them
>   is Rust, in ocelli-geom
>
> - Framework bindings, DICOMweb fetch and authentication
>
> - ONNX and SAM-backed AI tools, and the dcmjs bridge for TID 1500 structured
>   reports

### `docs/hld/03-architecture-and-crates.md`, section 3, verbatim

> The shell is TypeScript on the main thread: DOM and pointer events, the SVG
> annotation layer, tool interaction state, framework bindings, and DICOMweb
> fetch and authentication. The core is Rust running in workers. Between them
> sits a deliberately narrow boundary carrying commands down, bulk bytes down,
> and events up.

### `docs/hld/09-migration.md`, section 12, the integration seam, verbatim

> The mitigation is to keep the **integration seam** identical even though the
> API is not: the new library enables on a plain DOM element and dispatches
> events on it, exactly as cornerstone does. That single constraint is what
> makes incremental replacement possible.

### `docs/sprints/CURRENT_SPRINT.md`, what done means, verbatim

> - F-003 builds the tracked TypeScript workspaces and proves the package and
>   publish pipeline without publishing a release.

## What the specification does not cover

The HLD names the two packages and what belongs in the shell. It says nothing
about how they are built, bundled, packed or published. Everything in
`## Approach` is this plan's decision, and the four that matter are:

1. **Whether a bundler is added.** The story title says "bundling".
2. **What proves a publish pipeline without publishing.**
3. **How a package declares its dependency on a wasm artefact that is not
   committed and may be absent.**
4. **Where the npm version number comes from**, given `/release` is the only
   command permitted to publish and the crates carry the same `0.1.0`.

## Approach

**1. No bundler, and the reason is written down rather than assumed.**

`@ocelli/core` has no runtime dependency and emits ESM with declarations
through `tsc --build` today. A bundler over a dependency-free ESM package
produces the same modules with an extra tool in the path, and `AGENTS.md`'s
structural test asks whether a construct reduces the cases a reader must
consider or increases the places they must look. It increases them.

What the story actually needs from "bundling" is that a consumer's bundler can
resolve the package correctly, and that is a property of the published
tarball's `exports` map and its emitted module syntax, not of a bundle step
here. So this story proves that property mechanically instead of adding a tool
that would hide it.

The decision is recorded in `docs/lld/typescript-packaging.md` so the next
person to ask why there is no rollup config finds the answer without
archaeology. If the wasm consumption path in F-096 turns out to need one, it
is added then with a named reason, which is the `AGENTS.md` rule for a feature
flag applied to a build tool.

**2. A packaging gate, `packages`, added to `bin/ocelli.sh`.**

`scripts/package_check.py` runs `npm pack --dry-run --json` for each
publishable workspace and asserts, per package:

- The tarball contains `dist/` and the two licence files and the readme, and
  contains no `src/`, no `*.tsbuildinfo`, no `*.map` source content it did not
  intend, and no `node_modules`.
- Every path named by `main`, `types` and each `exports` condition exists
  inside the tarball. This is the failure that npm does not catch and that a
  consumer hits on install.
- The declared `dependencies` are resolvable at the declared range, and a
  workspace-internal dependency (`@ocelli/react` depends on `@ocelli/core`)
  names a version that matches what `@ocelli/core` actually publishes.
- `version` matches the Rust workspace version in the root `Cargo.toml`.
  One number across both toolchains, so `/release` has one thing to bump.

**3. A consumer resolution proof, not a claim.**

`npm pack` produces the tarballs, they are installed into a temporary
directory outside the workspace, and a generated consumer imports
`@ocelli/core` and `@ocelli/react` by their public specifiers under
`"moduleResolution": "bundler"` and again under `"node16"`. Both must
type-check and the ESM import must execute under plain `node`. This is what
catches an `exports` map that resolves for the repository's own path mapping
and not for anybody else, which is the defect class this story exists to
prevent and which no amount of `tsc --build` in place will find.

The temporary directory is outside the npm workspace root deliberately. An
install inside it would resolve `@ocelli/core` through the workspace link and
prove nothing about the tarball.

**4. `npm publish --dry-run` as the last step, and nothing beyond it.**

It exercises the registry-facing path, including the `publishConfig.access`
setting both packages already carry, and it publishes nothing. `/release` owns
the real publish and this story does not touch it. `package_check.py` refuses
to run if `NPM_TOKEN` or an `.npmrc` is present in the environment, so the
dry run cannot become a real one by accident on a machine that happens to be
logged in.

**5. The absent-core state stays honest.**

`packages/core/src/index.ts` exports `coreAvailable()` returning `false`
because a clean clone has no built wasm. That stays literally true in this
story. What changes is that `crates/ocelli-wasm/pkg` now exists on a machine
that ran F-002, so the function acquires a real question to answer. Answering
it belongs to F-096, which builds the boundary. This story records the seam in
the LLD and adds a test asserting the current honest answer, so that when
F-096 changes it, the change is deliberate and visible in a diff.

## Boundary and tier

- wasm-bindgen: not touched. This story is TypeScript and Python only.
- Pixels across the boundary: no.
- Render-loop allocation: none. There is no render loop.
- unsafe: none.
- Tier A (WebGPU): n/a. Packaging resolves no tier.
- Tier B (WebGL2): n/a, same reason.
- Tier C (CPU): n/a, same reason.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | `coreAvailable()` returns `false` with no built core, and `VERSION` matches `package.json` | `packages/core/src/index.test.ts` under vitest |
| `conformance` | Each tarball carries every path its `exports` map names, and the versions agree across Cargo and npm | `scripts/package_check.py`, gate `packages` |
| `conformance` | A consumer outside the workspace resolves and imports both packages under `bundler` and `node16` resolution | `scripts/package_check.py`, the temporary consumer |

No `fixture` row. This story computes no pixel and no coordinate, so HLD 27.2
R3 does not apply. Named rather than omitted.

## Parity surface covered

None. Appendix B enumerates viewports, tools, blend modes, VOI functions,
transfer syntaxes, segmentation representations, events and adapters. A
packaging pipeline is on none of those axes.

## Deviations

None.

## LLD impact

A new `docs/lld/typescript-packaging.md`: the two packages, the exports
contract, the no-bundler decision and its reason, the packaging gate and what
each of its assertions catches, and the absent-core seam.

## Decisions taken in the design round

1. **No bundler.** Confirmed in the consolidated design round. The story
   proves the property "bundling" is a proxy for, which is that a consumer's
   resolver handles the published tarball, and it proves it by installing the
   real tarball outside the workspace rather than by adding a tool. The
   decision and its reason go in `docs/lld/typescript-packaging.md`, and
   F-096 revisits it if the wasm consumption path needs one.
2. **`npm publish --dry-run` stays in the `packages` gate.** It is the only
   step that exercises the registry-facing path at all, it publishes nothing,
   and `scripts/package_check.py` refuses to run when `NPM_TOKEN` or an
   `.npmrc` is present so the dry run cannot become a real one by accident.

## Open questions

None. Both were resolved above.
