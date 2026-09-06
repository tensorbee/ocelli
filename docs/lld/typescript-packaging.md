# TypeScript packaging

**F-IDs that contributed:** F-003, F-004, F-005
**Last updated:** 2026-09-06

What `@ocelli/core` and `@ocelli/react` publish, and what proves it.

## The two packages

| Package | What it is | Runtime dependencies |
|---------|-----------|---------------------|
| `@ocelli/core` | The TypeScript shell of HLD section 10 | none |
| `@ocelli/react` | The React binding | `@ocelli/core` at an exact version |

Both are scaffolds. The public API is designed in F-100 and the boundary is
built in F-101.

### What is in the `@ocelli/core` tarball today

Everything under `packages/core/src` compiles into `dist` and is re-exported
from `index.ts`. F-005 added three modules to the four files that were there,
and F-004 then added `capabilities.ts`.

| Module | Holds |
|--------|-------|
| `bulk.ts` | The bulk channel down into linear memory |
| `ring.ts` | The event ring's consumer side |
| `errors.ts` | The error-code mirror, the message table, `decodeRecord` |
| `panic.ts` | `readPanicRecord`, and the panic record's layout |
| `fatal.ts` | `CoreStatus` and the never-returns-to-ok latch |
| `capabilities.ts` | The shell-side probes `wasmSimd128Supported` and `sharedMemoryAvailable`, and the committed SIMD module they validate |

`docs/lld/errors.md` specifies `errors.ts`, `panic.ts` and `fatal.ts`, and
`docs/lld/tier-resolution.md` specifies `capabilities.ts`. What belongs to
this file is that they ship, that they add no runtime dependency, and that
`errors.ts` is where the human text for an error code lives, which is HLD
section 23's "the message is for humans and may change" taken literally.

## `panic.ts` is the second file permitted a view over linear memory

`eslint.config.js` bans building any typed array or `DataView` over anything
ending `.memory.buffer`, and `ALLOWED_TO_DISABLE` turns that off through three
path patterns and not two. The first list is the production allowance,
`bulk.ts` and `panic.ts`. The second is `packages/core/src/*.test.ts`, kept as
a separate list precisely so the production allowance does not read as more
files than the two it grants.

HLD section 17.2 says "outside the two functions that are allowed to do it".
ESLint scopes overrides by file rather than by function, so the allowance is
file-scoped, and the specification already expected two. This repository had
one only because nothing else needed linear memory yet.

`bulk.ts` writes bytes down, between an `alloc` and the `commit` that takes
ownership back. `panic.ts` reads the panic record up, after the instance has
trapped, which is the safest possible instance of the hazard the rule guards:
no wasm code can run, so memory cannot grow between the view's construction and
its last use. The discipline is kept anyway. The view is built inside the
function, used immediately, and neither stored nor returned.

**Widening the list is a design-plan decision** and `.claude/plans/
F-005-design.md` item F is the one that added the second entry. **A third
production file is not granted.** `packages/core/src/ring.ts` will need one
when F-101 gives it a real ring to drain, and that is F-101's plan to argue.

## There is no bundler, and that is a decision rather than an omission

`@ocelli/core` has no runtime dependency and emits ESM with declarations
through `tsc --build`. A bundler over a dependency-free ESM package produces
the same modules with an extra tool in the path. `AGENTS.md`'s structural test
asks whether a construct reduces the cases a reader must consider or increases
the places they must look, and a rollup config increases them.

The story's title says "bundling", and what it actually needs from that word is
that **a consumer's resolver handles the published tarball**. That is a
property of the tarball's `exports` map and its emitted module syntax, not of a
build step in this repository, so the pipeline proves the property directly
instead of adding a tool that would hide it.

Revisit this when the wasm consumption path lands in F-101. `wasm-pack --target
web` emits an ESM module plus a `.wasm` asset, and bundlers treat that asset
specially. If a bundler is needed then, it is added then, with a named reason.

## The `packages` gate

```bash
bin/ocelli.sh gate packages
```

Two things, in order. `vitest run` over `packages/*/src/**/*.test.ts`, then
`scripts/package_check.py`.

`vitest.config.ts` sets `passWithNoTests: false`. Vitest's default is to pass
an empty run, and this project's rule is that a check which could not run must
never read as one that ran and was happy.

## What `scripts/package_check.py` proves

`tsc --build` proves the TypeScript compiles. It proves nothing about what a
consumer receives, and every check here is about the second thing.

| Assertion | The defect it catches |
|-----------|----------------------|
| Every path `main`, `types` and each `exports` condition names is inside the tarball | The package installs cleanly and fails at the consumer's first import. npm does not check this |
| `README.md`, `LICENSE-MIT` and `LICENSE-APACHE` are inside the tarball | A manifest saying `MIT OR Apache-2.0` with no licence text is a claim rather than a grant |
| No `src/`, no `node_modules/`, no `.tsbuildinfo` | Bytes a consumer downloads and never uses. `.tsbuildinfo` additionally embeds absolute paths from the build machine |
| The npm version equals the Rust workspace version | `docs/RELEASE.md` says the crates and the packages version together and that a skew is not supported |
| `@ocelli/react` depends on the exact version `@ocelli/core` publishes | A range resolves to whatever the registry happens to have |
| A project **outside the workspace** installs the real tarballs, imports under `node`, and type-checks under both `bundler` and `node16` | The one that matters. See below |

### Why the consumer install is outside the workspace

**Inside it, the test would prove nothing.** An install within the npm
workspace resolves `@ocelli/core` through the workspace link to `src/` and
never consults the tarball at all. The repository's own path mapping hides
exactly the defect this check exists for.

So the tarballs are built with `npm pack`, installed into a temporary directory
outside the tree, and imported from there.

**Both resolution modes are checked because an `exports` map can satisfy one
and not the other.** `bundler` is what a Vite or webpack consumer uses.
`node16` is what a plain `tsc` consumer uses and it is the stricter of the two.

## `npm publish --dry-run`

The last step, and it publishes nothing. It is the only step that exercises the
registry-facing path at all, including the `publishConfig.access` setting both
packages carry.

**An earlier draft refused to run where npm credentials existed, and that was
removed before it landed.** The theory was defence against a future edit
removing the `--dry-run`. The cost is that any developer logged into npm for an
unrelated project would have had this gate fail on them, and a gate that fails
for reasons nobody can act on is a gate that gets disabled, which `AGENTS.md`
makes a change to `.claude/WORKFLOW.md`. The protection against publishing here
is that this command cannot publish. `/release` owns publishing and this story
does not touch it.

## The absent-core seam

`packages/core/src/index.ts` exports `coreAvailable()`, which returns `false`.
A clean clone has no built wasm core, so that is the honest answer rather than
a failure to start, and the example viewer uses it to render a "core not built"
state.

It stays literally true after F-002, which produces
`crates/ocelli-wasm/pkg` on a machine that ran the build. What changed is that
the function now has a real question to answer. **Answering it belongs to
F-101**, and a test asserts the current answer so that the change is visible in
a diff rather than happening quietly.

`VERSION` is asserted against a literal for the same reason
`ocelli_version()`'s test in `crates/ocelli-wasm` is: comparing the constant
against the `package.json` it was copied from restates it and passes whatever
either says. Comparing the two FILES is `package_check.py`'s job, and it does
that against `Cargo.toml` as well.
