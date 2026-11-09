# F-266, the built example viewer loads its core from its own output

**Status**: draft
**Epic ref**: E38.61
**Sprint**: S47
**Estimate**: 1w

Filed at the S46 close from the recovered close-backlog draft, original item 1,
with the scope kept. The estimate is inherited and is provisional,
lead-suggested and not capacity-approved. This draft claims no implementation,
no review cleanliness and no completion.

## Normative source, transcribed

No HLD section covers the built example viewer's host path. A search of
`docs/hld/` for viewer-react and vite finds nothing, and section 15.1's layout
lists `packages/core` and `packages/react` with no `examples/`. The nearest HLD
text is the enforcement pattern:

`docs/hld/12-workspace-and-build.md` 15.3: "Decision D2 is worthless unless it
is enforced. This runs on every pull request."

That pattern, a build property carried by a check rather than by intention, is
what this story applies to the built dist. The property itself is the story
row's, not the HLD's.

The current source at 31323a79, `examples/viewer-react/vite.config.ts`:

```ts
const wasmUrl = `/@fs${decodeURIComponent(wasmPackage.pathname)}`;
```

```ts
define: {
  __OCELLI_WASM_URL__: JSON.stringify(wasmUrl),
},
server: {
  port: 5173,
  fs: {
    allow: [repoRoot.pathname],
  },
```

`examples/viewer-react/src/core-worker.ts`, `openPackage`:

```ts
const probed = await fetch(__OCELLI_WASM_URL__);
if (!probed.ok) {
  throw new Error("wasm package is absent");
}
const imported = (await import(
  /* @vite-ignore */
  __OCELLI_WASM_URL__
)) as {
```

## What the specification does not cover

How the built worker reaches the bundled chunk, what the define holds when the
pkg is present, and how `fs.allow` is expressed. The HLD does not cover the
example viewer's build, so every decision below is this design's. The recorded
caution is F-217 fresh-build review pass 2 N1: the define is one string serving
Rollup and the browser, decoding is right for Rollup and for a space or
unicode, which a browser re-encodes, and it is wrong for a literal `%`, which
the browser leaves as it is. The dev URL and the Rollup specifier may need two
strings.

## Problem and evidence

`docs/sprints/AS_BUILT.md:5685`, F-217 follow-up "Notes for future sessions",
names three open items "outside this fix and belong to a separate story", and
this story is that story. Quoted in part: "The built `dist` hard-codes the
build machine's absolute `/@fs` path, so the built viewer cannot load the core
away from that machine, which was already true on `main`. The viewer page
stays `core not built` in Chromium even with the real pkg, at the sprint head
and with the fix, and why was not investigated. Dev mode answers 403 for the
pkg URL under any checkout path that `URL.pathname` encodes, because
`server.fs.allow` is the percent-encoded `repoRoot.pathname`."

The recovered drafting pass measured the mechanism on the then-current dist:
served with `python3 -m http.server`, the `/@fs/...` probe URL answered 404
while `assets/ocelli_wasm-*.js` answered 200, and `vite preview` answered the
missing path with the HTML fallback at 200 `text/html`, which
`scripts/viewer_build_check.py` also records. The browser symptom was not
reproduced in that pass. Its only browser run was the package-absent preview,
and its handoff records that the real package was never exercised in a
browser. The 404 is the mechanism. The story's acceptance includes the browser
run with the real package, which is row AC2 and nothing recorded before it.

F-228's probe does not catch it. `.claude/plans/F-228-design.md:143`:
`examples/viewer-react/csp-probe.mjs` "maps `/@fs/<absolute path>` to files
under the repository root only", so its dist run never meets the failure.

Source check at 31323a79, read-only: the define, the probe and the encoded
allowance are all present, so the mechanism is still in the tree. No new
measurement was taken for this filing.

## Scope, API and compatibility

In scope: the built viewer's worker reaches the bundled wasm chunk on any
static host, with no build-machine absolute path left in any emitted file, the
headless Chromium run of the built dist on a plain static server reading
`core ready`, and the dev server's `fs.allow` decoded so a checkout path with
a space, unicode or a percent sign serves the package.

Non-goals: no new viewer page, no change to the folder viewer's serving, no
change to the CSP policy text, no CI or gate edit except through the normal
workflow, no publication, and no change to the pkg-absent behavior the F-217
follow-up restored.

Compatibility: with the pkg absent the URL stays external for the worker build,
the import stays a runtime import and the page reads `core not built`. With it
present the glue and the `.wasm` are emitted as their own chunks and the worker
references its own output.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Built dist worker | `openPackage` probe and import of `__OCELLI_WASM_URL__` | One URL for probe and import, or two strings per N1 | `core ready`, or the named absence |
| `vite build` | The define and the worker rollup options | pkg present or absent | Bundled chunks, or an external runtime import |
| Dev server | `server.fs.allow` | A checkout path with a space, unicode or `%` | The package served, or 403 |
| `csp-probe.mjs` | The dist server | F-228's header checks unchanged | Probes the real dist |
| `viewer_build_check.py` | The both-shapes build | Existing both-files condition | Refusal for the bad shapes |

## State transitions and invariants

Not applicable. The story changes build output and dev-server allowance, not
runtime state. The one stateful surface is the worker's load: a failed probe
or import is the named absence and a page reload retries. A refused package
never draws a wrong frame.

## Ownership, commit and recovery

Not applicable in the render-loop sense. The dist is an artefact, rebuilt and
never patched in place. `viewer_build_check.py` refuses the bad shapes at
build time, before anything is served.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | `vite build` with the real pkg present emits a dist in which no emitted file contains the build machine's absolute path or a `/@fs` specifier | A check over the emitted `dist/assets/*` files | A define that only renames the absolute path | implementation |
| AC2 | That dist, served by a plain static server, loads in headless Chromium and the page reads `core ready` | The browser run the recovered draft never made, recorded with Chromium's version | A preview-only check that the HTML fallback conceals | integrated browser |
| AC3 | The worker's probe and import reach the bundled chunk, seen in the network log as a 200 for the chunk URL | The AC2 run's log | A probe that succeeds only under a repository mapping | integrated browser |
| AC4 | Dev mode under checkout paths with a space, unicode and a percent sign serves the package URL, `?import` and the `.wasm` | F-217 pass 2's four-name matrix re-run | An allowance left percent-encoded | implementation |
| AC5 | A literal `%` checkout serves both the Rollup specifier and the browser URL, or the two uses carry two strings per N1 | The same matrix, the `%` name | One decoded string assumed right for both uses | implementation |
| AC6 | `viewer_build_check.py` refuses a dist whose emitted worker carries the absolute path and accepts the fixed dist | The script run against both shapes | A check that reads the config and not the emitted file | implementation |
| AC7 | Negative control. Reintroducing the `/@fs` define turns AC1 and AC2 red | Recorded in the review | A suite that passes on the wrong dist | implementation, reviewer |
| AC8 | The pkg-absent build is unchanged, external import and `core not built` | The existing both-shapes `viewer_build_check.py` run | A fix that breaks the fresh-clone path | implementation |
| AC9 | The CSP probe passes against the real dist | `examples/viewer-react/csp-probe.mjs` | A dist change that breaks F-228's evidence | implementation |

Unrun ownership. AC2, AC3 and AC9 need the operator's Mac with Chromium, the
host F-228's probe uses. If that host is unavailable the rows stay open with
that owner named and are never passed. Nothing in this filing ran a build, a
browser or a server.

## Approach

1. Build the define from the bundled chunk relationship so the emitted worker
   references its own output. The exact shape is decided at design completion
   and written here before approval.
2. Decode `fs.allow` through `fileURLToPath(repoRoot)`, and split the Rollup
   specifier from the browser URL if the `%` case needs it.
3. Extend `viewer_build_check.py` to refuse the absolute path in emitted
   worker files.
4. Point the CSP probe at the real dist so its evidence survives.

Rejected: post-processing the dist with a regex rewrite, which hides the
relationship from the build, serving the pkg from a dev-only proxy, and a
fixed port or host assumption.

## Dependencies, write ownership and resources

Depends on F-217, done, and F-228, in-progress in S46. Both dependencies
must be DONE before implementation starts. If F-228 carries out of S46, this
story follows its actual completion and does not edit its in-flight CSP
probe. Lands in S47 after F-237 packaging and beside
F-245: the built front door must load its own core before any
single-viewer distribution claim, and this story creates no second viewer.
Writes `examples/viewer-react/vite.config.ts`, `examples/viewer-react/src/core-worker.ts`
if the probe shape changes, `scripts/viewer_build_check.py` and
`examples/viewer-react/csp-probe.mjs`. F-245's S47 work owns the viewer page
surface and not the build config, and F-237 owns the release listing and not
the example build, so no shared file has two writers. Browser runs use the one
allocated resource slot.

## Verification and evidence reuse

The scratch-clone builds with and without the pkg, `viewer_build_check.py`,
the CSP probe and the AC2 browser run, each with node and Chromium versions
recorded. AC1's check is over emitted files, never over the config. Evidence
invalidates on any later change to the define, the probe or the check, which
is what AC6 exists to catch again.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Build and loading only
- unsafe: none
- Tier A (WebGPU): unchanged. The loaded core resolves its tier as today
- Tier B (WebGL2): unchanged
- Tier C (CPU): unchanged. The AC2 host resolves its own tier

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | The check refuses the bad dist shape and accepts the fixed one | `scripts/viewer_build_check.py`, AC6 |
| integration | Both build shapes keep the F-217 follow-up behavior | `scripts/viewer_build_check.py`, AC8 |
| browser | The built dist reads core ready on a plain static host | AC2, AC3 |

## Parity surface covered

None. The story is build and loading hygiene for the built front door.

## Deviations

None. A draft names a needed deviation here and the row is written into
`docs/hld/DEVIATIONS.md` only in the change that approves the plan.

## LLD impact

`docs/lld/build-targets.md`, the built viewer's loading contract. Possibly
`docs/lld/typescript-packaging.md` if the define shape changes what is
emitted.

## User-facing documentation impact

`README.md`'s built-viewer section, if it documents static hosting.
`examples/folder-viewer/src/use.html` and `more.html` unchanged.

## Open questions

1. One URL or two strings for the Rollup specifier and the browser, per N1.
   Affects AC4 and AC5. Owner: the design round. Deferral leaves the `%` case
   serving the wrong directory.
2. Whether the CSP probe keeps its repository mapping or serves the real dist.
   Affects AC9. Owner: the design round, with F-228's record.
3. Whether the emitted-file check joins the floor profile. Affects AC6's
   standing. Owner: the operator through the normal workflow. This filing
   proposes and never authorizes a profile change.
