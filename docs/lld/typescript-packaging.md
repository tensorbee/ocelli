# TypeScript packaging

**F-IDs that contributed:** F-003, F-004, F-005, F-021, F-X017, F-035, F-100, F-101, F-104, F-050, F-103, F-102, F-X003, F-198, F-217, F-218, F-221, F-228, F-230, F-236, F-237
**Last updated:** 2026-11-08

What `@ocelli/core`, `@ocelli/wasm` and `@ocelli/react` publish, and what proves it.

## The three packages

| Package | What it is | Runtime dependencies |
|---------|-----------|---------------------|
| `@ocelli/core` | The TypeScript shell of HLD section 10 | `dcmjs` at `0.52.2`, peer `@ocelli/wasm` at an exact version |
| `@ocelli/wasm` | The module built from `crates/ocelli-wasm`, both halves | none |
| `@ocelli/react` | The React binding | `@ocelli/core` at an exact version |

`@ocelli/react` publishes the viewport pump, the hook and the host component.
`jsdom` and `@testing-library/react` are workspace devDependencies used by the
component test. Neither is a dependency of the published package. `@ocelli/core`
now also publishes F-021's
bounded DICOMweb transport and one-write response-sink contract, and F-035's
decode-to-render handoff. F-100 publishes `viewport.ts`, which is the
viewport, scene and tool types. F-101 adds `commands.ts` and the copied
ring drain. `coreAvailable()` stays false until a built module is loaded.

## The module package

`@ocelli/wasm` is `packages/wasm`, the published place of the module. The HLD
names no distribution route, and F-237's answer is a package in the `@ocelli`
scope beside the shell. The crate `ocelli-wasm` stays `publish = false`: what
npm publishes is the artefact, not a crate.

The tarball is staged by `packages/wasm/scripts/stage.mjs`, which npm runs as
the prepack lifecycle script on every pack and publish, unless scripts are
ignored, which packs whatever `dist` already holds without a rebuild. It
builds both modules through `bin/ocelli.sh wasm` and
`bin/ocelli.sh wasm-decode`, copies the glue and both `.wasm` files out of
`crates/ocelli-wasm/pkg` and `crates/ocelli-wasm/pkg-decode`, and writes
`dist/build-id.json`. The two out-dirs stay where they are, because the size
budget of `scripts/pin_and_size_check.py` reads them there.

The render module is the package's root entry and the decode module is the
`./decode` subpath. Both keep the file names wasm-pack generates, the decode
files under `dist/decode/`, so the glue's relative fetch of its `.wasm`
needs no edit.

`dist/build-id.json` records the package version, whether the working tree
had unstaged tracked changes when the modules were built, the git SHA of
that HEAD, the git write-tree of the staged index, and the SHA-256 digest
of each module. That is the form `docs/RELEASE.md` item 6
compares to the release HEAD. The wasm build is byte-reproducible for a
given SHA on one toolchain, measured on 2026-11-07: two cold builds of one
tree and a warm rebuild agree on every byte of both modules. A release
compares the recorded digests to a cold build of the tagged source with
`python3 scripts/release_wasm.py HEAD`, a preflight item that runs before
any tag, push or publish, and copies the verified `.tgz`
to `target/release-wasm`, the exact file `npm publish <file>` is then
given. `npm publish <file>` runs no prepack, so the published bytes are
the compared bytes.

`gate packages` packs all three packages and refuses a `@ocelli/wasm`
tarball without both modules and their glue, without the two third-party
files at the tarball root, and with a build id that is absent, stale against
the current git HEAD or the current index, recorded with unstaged tracked
changes, or recording a digest the
tarball's own module bytes do not hash to. Eleven level-1 probes drive the
build id refusals: nine over the parsed id and two over its wiring, one
through `wasm_identity_from_tarball` on a real tarball and one behavioural,
running `main()` with the npm steps stubbed over a stale build id and
refusing a `main()` that does not refuse it. Two more pin the module
surface, one over the smoke text and one over the type check text, and one
drives the notice rule. `check_tarball`'s presence refusal for the module
paths has no probe of its own, standing beside the generic `REQUIRED` check
the way that one does.

`@ocelli/core` declares `@ocelli/wasm` as a peer dependency at the same
version, verified by `check_manifest` against the workspace version. The
shell does not import the module itself: `loadCore(initialise)` leaves the
initialiser to the caller, which is why the module is a peer dependency and
not a runtime one.

The consumer proof installs all three tarballs outside the workspace and
loads the module through the public path: `loadCore` with an initialiser
that reads the installed module's bytes and calls its default export, then
`ocelli_version()` compared against the `VERSION` string the shell exports.
It reads `@ocelli/wasm/decode`'s bytes through `import.meta.resolve` the
same way and initialises it directly, because the decode pool builds its
own instances and loadCore latches once per realm. The proof tells the two
modules apart by the render-only export `apply_commands`, compiled only
with the render-worker feature: the root entry must export it as a
function, `./decode` must not carry it, and the type check reads it the
same way under `bundler` and `node16`, so a staging edit that swaps the two
copies is refused by both halves. The bytes come from
`import.meta.resolve` inside the installed package, never from a
repository path.

### What is in the `@ocelli/core` tarball today

`packages/core/tsconfig.json` is the library project. It compiles the modules
under `packages/core/src` into `dist` and excludes `src/**/*.test.ts`,
`src/**/*.test.tsx` and `src/node-test-modules.d.ts`. `@ocelli/react`
excludes the two test globs. A compiled test is not a library output. Public modules are
re-exported from `index.ts`, including F-021's DICOMweb client, request
options, errors, response kinds, sink contract, and one-write helper, and
F-035's `exportDecodedFrame`, `acceptTransferredFrame` and
`transferredFrameMessage`.

| Module | Holds |
|--------|-------|
| `bulk.ts` | Frame writes, the F-021 one-write DICOMweb response sink, and the F-035 decode-to-render handoff |
| `dicomweb.ts` | Bounded WADO-RS, WADO-URI, and QIDO-RS request construction, validation, authentication, cancellation, and response transfer |
| `ring.ts` | The event ring's consumer side, including `drainRing` over a copied buffer |
| `commands.ts` | Little-endian packing of the eleven stack commands and one submit per frame |
| `generated.ts` | F-104's boundary constants and `StackCommand`, plus the F-050 snapshot magic, version and error codes |
| `snapshot.ts` | F-050's JSON parser and the session snapshot adapter |
| `events.ts` | F-103's catalogue, delivery classes and payload decoders |
| `errors.ts` | The error-code mirror, the message table, `decodeRecord` |
| `ocelli-error.ts` | F-230's `OcelliError` class and its closed string `OcelliErrorCode` union |
| `options.ts` | F-230's per-type option key lists and `refuseUnknownOptions` |
| `panic.ts` | `readPanicRecord`, and the panic record's layout |
| `fatal.ts` | `CoreStatus` and the never-returns-to-ok latch |
| `capabilities.ts` | The shell-side probes `wasmSimd128Supported` and `sharedMemoryAvailable`, and the committed SIMD module they validate |
| `viewport.ts` | F-100's viewport, scene and tool types. No wasm call and no pixels |

`packages/core/tsconfig.test.json` and `packages/react/tsconfig.test.json`
include only the test sources. The core test project also includes
`src/node-test-modules.d.ts` and `src/dcmjs.d.ts`. Test files import `dcmjs`
directly, and that ambient module declaration does not cross a project
reference. Each references its library project, and the
react test project also references `packages/core`. The react test project
sets `jsx` to `react-jsx`, because it extends the base config and the base
does not set it. The root `tsconfig.json` references both test projects, so
`npm run typecheck` still sees every test file, including
`boundary.wasm.test.ts`. Vitest discovers tests by its own globs and does not
read these projects.

The test projects emit under `node_modules/.cache/ocelli-tsc/`. That directory
is ignored, never packed and never linted. Each test project names its own
`tsBuildInfoFile` there, so the two never write one build info file. `dist` stays the library output
whether `build` or `typecheck` ran last.

`build` and `clean` both remove `dist` and `tsconfig.tsbuildinfo`.
`tsc --build` treats a surviving `.tsbuildinfo` as proof that `dist` is
current, so deleting `dist` alone leaves the directory absent and the next
build writes nothing. The two go together. A file planted in `dist` cannot
survive `build`.

`scripts/package_check.py` refuses a packed path whose basename contains
`.test.` or `.spec.`, and a path with a `__tests__` segment. The message
names the package and the path. `latest.js` and `contest.d.ts` are not test
files. The probe is `packages.test-file-in-tarball`.

`docs/lld/errors.md` specifies `errors.ts`, `panic.ts` and `fatal.ts`, and
`docs/lld/tier-resolution.md` specifies `capabilities.ts`. What belongs to
this file is that they ship, that they add no runtime dependency, and that
`errors.ts` is where the human text for an error code lives, which is HLD
section 23's "the message is for humans and may change" taken literally.

`generated.ts` is committed rather than produced at build time, so a clean
clone type-checks without a codegen step. The drift check is
`crates/ocelli-viewport/tests/typegen.rs`, which the existing `test` gate runs:
it emits the file from the Rust table and the real `#[repr(C)]` layouts and
compares it with the committed bytes, naming the first differing line. The
`wasm` gate does not run it and no new gate is added, because `test` is
already in the floor.

## `panic.ts` is the second file permitted a view over linear memory

`eslint.config.js` bans building any standard typed array or `DataView` over a
receiver whose TypeScript type is `WebAssembly.Memory`. The type-aware rule
follows parameters, assignments, call results, getters, fields, for-of
bindings and computed `buffer` access without guessing from identifier names.
It leaves caller-owned and ordinary `ArrayBuffer` views legal.

`ALLOWED_TO_DISABLE` turns both the typed rule and the remaining buffer-alias
syntax rule off through three path patterns and not two. The first list is the
production allowance, `bulk.ts` and `panic.ts`. The second is
`packages/core/src/*.test.ts`, kept separate so the production allowance does
not read as more files than the two it grants.

HLD section 17.2 says "outside the two functions that are allowed to do it".
ESLint scopes overrides by file rather than by function, so the allowance is
file-scoped, and the specification already expected two. This repository had
one only because nothing else needed linear memory yet.

`bulk.ts` writes bytes down, between an `alloc` and the `commit` that takes
ownership back. F-035 keeps the handoff in this file. `exportDecodedFrame`
copies one decoded range out into a fresh `ArrayBuffer` for section 24's
worker transfer, and the view exists only for that copy.
`acceptTransferredFrame` is the render-worker side of the same hop. It uses
`writeFrame`'s order, which is `alloc`, then a view, then one `set`, then the
ticketed sink's commit. The validated target fields travel beside the
handoff, so the sink picks the export exactly as it does for `writeFrame`.
`commit_cached_frame` takes a target carrying `stackIndex`, `stackEpoch` and
`content`, `commit_selected_frame` takes the two stack keys without
`content`, and `commit_frame` takes the generation otherwise. A stale index
or epoch comes back as `OcelliError` code `CommitRefused` with the slot
already released by the session, and a missing selected-frame or
cached-frame export throws `ExportUnavailable` and is released through
`releaseAllocated`. A buffer length that disagrees with `meta.bytes` throws
`LengthMismatch`, and a lone stack key or a `content` without both stack keys
throws `InvalidValue`, each before `alloc`, so a refused call neither grows
linear memory nor copies. `transferredFrameMessage` returns the section 24 frame
message and the transfer list `[buffer]`. It does not call `postMessage` and
it does not build a view. A third production file is still not granted, so
none of these moved out of `bulk.ts`.

`panic.ts` reads the panic record up, after the instance has trapped, which
is the safest possible instance of the hazard the rule guards. No wasm code
can run, so memory cannot grow between the view's construction and its last
use. The discipline is kept anyway. The view is built inside the function,
used immediately, and neither stored nor returned.

**Widening the list is a design-plan decision** and `.claude/plans/
F-005-design.md` item F is the one that added the second entry. **A third
production file is not granted.** F-101 did not ask for one. It drained the
ring by copying bytes out, so `packages/core/src/ring.ts` reads a copied
buffer and never builds a view over linear memory.

## There is no bundler, and that is a decision rather than an omission

`@ocelli/core` depends on `dcmjs` at `0.52.2` and emits ESM with
declarations through `tsc --build`. A bundler is not part of this
pipeline. The consumer resolves `dcmjs`. `AGENTS.md`'s structural test
asks whether a construct reduces the cases a reader must consider or increases
the places they must look, and a rollup config increases them.

The story's title says "bundling", and what it actually needs from that word is
that **a consumer's resolver handles the published tarball**. That is a
property of the tarball's `exports` map and its emitted module syntax, not of a
build step in this repository, so the pipeline proves the property directly
instead of adding a tool that would hide it.

F-101 landed the boundary by copying bytes out of wasm rather than by
importing the wasm asset into this package. The bundler decision stands.
`wasm-pack --target web` still emits an ESM module plus a `.wasm` asset, and
a bundler is added only with a named reason.

`@ocelli/core` and `@ocelli/react` list `NOTICE` in `files`. The example
viewer builds with Vite 7. The root devDependencies pin `vite` at `^7.3.7`
and `@vitejs/plugin-react` at `^5.2.0`. The viewer worker is already
constructed as a module, and `worker.format` is `es`, because Vite 7 will
not emit an iife bundle for a worker that imports the wasm package.

**The viewer's build output depends on whether `crates/ocelli-wasm/pkg`
exists when `vite build` runs.** `vite.config.ts` defines the worker's wasm
URL as a constant `/@fs` path, and Vite 7's Rollup worker sub-build resolves
a constant dynamic import and loads the file. With the pkg present the wasm
glue and its `.wasm` are emitted as their own files. With it absent, which is
every clean clone and the CI frontend job, the config marks that URL external
for the worker build, so the import is left to runtime, and the core reads as
not built because that import fails. That is the behaviour the viewer had
under Vite 8 on `main`. Without it a clean clone fails `npm run build` with
ENOENT on `ocelli_wasm.js`. The URL carries the decoded path, because Rollup
loads it as a literal path when the pkg is present and a `%20` there is ENOENT
for a checkout under a path with a space. The config counts the pkg as built
only when the glue and the `.wasm` both exist. It reads the filesystem once
when it loads, so the pkg must exist before `vite build` starts for the
bundled form.

`scripts/viewer_build_check.py` builds both states, because CI only ever has
the first and a developer with a built pkg only ever has the second. It copies
the viewer into a temporary tree shaped like the repository, under a directory
name with a space so the decoded URL is exercised on every run, builds it once
with no pkg and once with a stand-in pkg, and refuses a failed build, a
pkg-absent build that emits a wasm file or drops the runtime `import()` or the
`fetch()` probe of the pkg URL, and a pkg-present build that emits no
`ocelli_wasm` chunk or no `ocelli_wasm_bg` `.wasm` asset, or still imports the
URL at runtime. The stand-in is a glue module that references its `.wasm`
through `new URL(..., import.meta.url)`, as `wasm-pack --target web` output
does, and a valid module padded past Vite's 4096-byte inline limit. It proves
what Vite emits for a pkg that exists, not that the real pkg instantiates.

## The `packages` gate

```bash
bin/ocelli.sh gate packages
```

In order, `vitest run` over `packages/*/src/**/*.test.ts` and
`packages/*/src/**/*.test.tsx`, then `scripts/package_check.py`, then
`scripts/viewer_build_check.py`, then the unreachable-injection table. The component
test opts into jsdom with a vitest environment comment, so the rest of the
suite keeps the default environment. `npm run dev` starts with no session and
shows the absent state.

`vitest.config.ts` sets `passWithNoTests: false`. Vitest's default is to pass
an empty run, and this project's rule is that a check which could not run must
never read as one that ran and was happy.

## What `scripts/package_check.py` proves

`tsc --build` proves the TypeScript compiles. It proves nothing about what a
consumer receives, and every check here is about the second thing.

| Assertion | The defect it catches |
|-----------|----------------------|
| Every path `main`, `types` and each `exports` condition names is inside the tarball | The package installs cleanly and fails at the consumer's first import. npm does not check this |
| `README.md`, `LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE` are inside the tarball | A manifest saying `MIT OR Apache-2.0` with no licence text is a claim rather than a grant. `NOTICE` names the holder |
| No `src/`, no `node_modules/`, no `.tsbuildinfo` | Bytes a consumer downloads and never uses. `.tsbuildinfo` additionally embeds absolute paths from the build machine |
| No packed path whose basename contains `.test.` or `.spec.`, and no `__tests__` segment | A published tarball that ships a compiled test. The library projects exclude tests, and this refusal is the backstop |
| The npm version equals the Rust workspace version | `docs/RELEASE.md` says the crates and the packages version together and that a skew is not supported |
| `@ocelli/react` depends on the exact version `@ocelli/core` publishes | A range resolves to whatever the registry happens to have |
| A project **outside the workspace** installs the real tarballs, imports under `node`, and type-checks under both `bundler` and `node16` | The one that matters. See below |
| The consumer resolves each unreachable exception at its recorded version. The smoke run imports `@ocelli/core` and requires `dcmjs` under `module.registerHooks`, so both the ESM build and the CommonJS build are resolved. The trace must record `node_modules/dcmjs/build/dcmjs.es.js` and `node_modules/dcmjs/build/dcmjs.js` and must not record a URL under the excepted package. Every other installed `.js`, `.mjs` or `.cjs` file is scanned for the package name as a module specifier in single quotes, double quotes or backticks, including a subpath, a relative `./node_modules/<name>` path, and a relative `../` or `../../` path whose next segment is the package name | An exception that describes this repository's lock rather than the consumer's, a hook that records nothing, or a published file that names the package as a specifier |

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

### Why no override is used

npm's documentation says "Overrides are only considered in the root
`package.json` file for a project." An override written in this repository
changes only this repository's lockfile. A consumer of `@ocelli/core` resolves
dependencies from their own root `package.json`. The recorded answer is a
check that no shipped file loads `adm-zip` on the paths this gate runs.
`gate packages` checks that on every run.

The check has three parts. The consumer lock must resolve the excepted
package at the recorded version. The smoke script imports `@ocelli/core` and
calls `createRequire(import.meta.url)("dcmjs")` under node's
`module.registerHooks` resolve hook, which records ESM and CommonJS
resolutions on the main thread. `module.register` does not see a `require`
from `createRequire`, which is why the hook is `registerHooks`. The trace
must contain the resolved `dcmjs/build/dcmjs.es.js` URL and the resolved
`dcmjs/build/dcmjs.js` URL and must not contain a URL under the excepted
package. An empty or absent trace is a refusal. Every other installed script
is scanned for that package name as a quoted module specifier, in any of the
three quote styles, with or without a subpath, including a relative `../` or
`../../` path. A load whose specifier is built at run time in code the smoke
run does not execute on its main thread (for example a Worker or a child
process, or a function the import never calls) is outside the check. The
root `package.json` has no `engines` field. CI pins node 24, and
`registerHooks` is the node 24 API this check uses. The packages gate also
runs `scripts/tests/test_unreachable_injections.py` after that check.

A consumer's own npm audit also lists `@ocelli/core` and `@ocelli/react` as
high, because they depend on dcmjs.

## `npm publish --dry-run`

The last step, and it publishes nothing. It is the only step that exercises the
registry-facing path at all, including the `publishConfig.access` setting all
three packages carry.

**An earlier draft refused to run where npm credentials existed, and that was
removed before it landed.** The theory was defence against a future edit
removing the `--dry-run`. The cost is that any developer logged into npm for an
unrelated project would have had this gate fail on them, and a gate that fails
for reasons nobody can act on is a gate that gets disabled, which `AGENTS.md`
makes a change to `.claude/WORKFLOW.md`. The protection against publishing here
is that this command cannot publish. `/release` owns publishing and this story
does not touch it.

## The absent-core seam

`packages/core/src/index.ts` exports `coreAvailable()` and `loadCore()`.
`loadCore(initialise)` returns `Promise<true>`. It resolves `true` once the
initialiser resolves, and only then does `coreAvailable()` turn true. A thrown or
rejected initialiser rejects with `OcelliError` code `CoreLoadFailed`, message
"The imaging core did not load.", and `cause` set to the value the initialiser
threw or rejected with. The flag stays false and a later call retries. Concurrent calls share
one pending load, and a call after success does not run the initialiser again.
A clean clone has no built wasm core.
`examples/viewer-react/src/core-worker.ts` awaits `loadCore` inside `try`. Its
`catch` logs the error's `code`, or the raw value when it is not an
`OcelliError`, and posts a status with an empty tier and `available` false, so
the example page stays on "core not built".
A test that imports `coreAvailable` and does not call `loadCore` still expects
false.

`VERSION` is asserted against a literal for the same reason
`ocelli_version()`'s test in `crates/ocelli-wasm` is: comparing the constant
against the `package.json` it was copied from restates it and passes whatever
either says. Comparing the two FILES is `package_check.py`'s job, and it does
that against `Cargo.toml` as well.

## The Content Security Policy an embedder needs

F-228. An embedding page that loads the Ocelli worker and the wasm module
needs exactly four directives, and it needs the same header on every
response, the worker script included:

<!-- ocelli:csp -->
```text
Content-Security-Policy: default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self'
```

`script-src 'self'` loads the page bundle, the module worker's own imports
and the wasm-bindgen glue through `import()`. `'wasm-unsafe-eval'` lets the
worker compile `ocelli_wasm_bg.wasm` and is narrower than `'unsafe-eval'`,
which would also allow `eval`. `connect-src 'self'` covers the glue's fetch
of the module and the worker's probe fetch of the glue. `worker-src 'self'`
is an explicit pin rather than a requirement: the worker is also allowed
through `worker-src`'s fallback to `script-src 'self'`, and the pin keeps a
later widening of `script-src` from widening worker construction with it.
`default-src 'none'` is in the tested header so the probe can prove Ocelli
needs nothing beyond the three named sources. Styles, images and fonts are
the embedding page's own business and outside this claim. A DICOMweb
service on another origin is added to `connect-src`.

The evidence for the every-response rule is E1 of the design plan, measured
in the pinned Chromium: the worker's wasm compile obeys the worker
response's policy, not the page's. A header on the HTML document alone
leaves the worker unrestricted in Chromium, and a header on the worker alone
without `'wasm-unsafe-eval'` breaks the compile. `blob:` and `data:` are
refused in the other direction by the same response model, measured as E2:
a worker constructed from a local scheme inherits its creator's policy, so a
strict page breaks it and loosening the page for it weakens every script on
the page. Ocelli constructs workers from URLs only, and the embedder's
`openDecodePool` factory should too.

The policy exists once, in `examples/viewer-react/src/csp-policy.ts`. Its
test spells out the four directives and forbidden source list independently.
It also checks marked policy blocks in README, this document and the Use page,
with one expected header inside each Markdown text fence or HTML `pre` block.
Two guards
stand beside the document check. The first reads the resolved normal-edge
dependency graph of `crates/ocelli-wasm` through `cargo tree` with `-e
normal --all-features`, and it must not name `ocelli-codec`, by any
spelling of the edge (a rename, a dotted key, a subtable) and through no
intermediate crate. A dev-dependency on the codec links nothing into the
module and stays green. An optional edge that a feature enables, as
`decode-worker` would, goes red. That graph check is the decode gap: the
wasm module decodes nothing today, so the probe measures the worker and
wasm path with a stored sample. When the codec joins the module's normal
dependencies, the test fails until the probe is extended to decode one
frame in the worker under the policy. The second guard scans every shipped
source directory found under `packages/*/src` and the example's `src` for
`createObjectURL`, `blob:`, `data:`, `eval(` and `new Function(`, so
nothing Ocelli ships needs a weaker policy to run.

The probe is `examples/viewer-react/csp-probe.mjs`. It serves the built
example and the built wasm package through Playwright routes on the invented
origins `http://ocelli.test` and `http://dicomweb.test`, so it binds no
port, and it measures the strict header, each directive's minimality, the
worker-src fallback, where the worker's policy comes from, the blob control
and its positive half, the dcmjs document paths and a cross-origin DICOMweb
instance from `corpus/data/syntax`. The pure parts run in `npm run test`
under the `packages` gate. No gate was added.

The `@ocelli/core` tarball carries no wasm module. The module ships as
`@ocelli/wasm`, and an embedder serves it from wherever it hosts
`node_modules/@ocelli/wasm/dist`, or from a `bin/ocelli.sh wasm` build of
`crates/ocelli-wasm/pkg` inside this repository.

### The local folder viewer's page policy

The folder viewer uses `csp::page_policy` on each embedded page, caching its
hashes at first use. The main page, Use and More and their `.html` aliases
carry their own `Content-Security-Policy` header. Inline script and style text,
including the import map, is hashed with SHA-256 and RFC 4648 base64.
The policy permits same-origin script, style, image and connection resources,
and `data:` images for the favicon. The source pages contain no inline handler
or style attribute. The policy permits no eval, blob worker or wasm compilation.
The process decodes frames.
`examples/folder-viewer/csp-probe.mjs` independently checks hashes, blocked
tampering, compressed-frame pixels, the region window and SR save/load.
