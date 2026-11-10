# F-266, the built example viewer loads its core from its own output

**Status**: approved
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
static host, the headless Chromium run of the built dist on a plain static
server reading `core ready`, and the dev server's `fs.allow` decoded so a
checkout path with a space, unicode or a percent sign serves the package.
The no-build-machine-absolute-path property applies to the pkg-PRESENT
emitted output only. The pkg-ABSENT build deliberately keeps the external
encoded /@fs runtime import, which is exempt from that ban.

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
| Built dist worker | import-only load of the rewritten glue chunk in the emitted worker, default wasm init error as the named absence | one `wasmPackage` URL, specifier chosen by Vite `command` AND pkg presence, decoded only for build AND `wasmBuilt`, otherwise the encoded browser URL, external exactly when the build is absent | `core ready`, or the named not-built absence from import failure |
| `vite build` | The define and the worker rollup options | pkg present or absent | Bundled chunks, or an external runtime import |
| Dev server | `server.fs.allow` | A checkout path with a space, unicode or `%` | The package served, or 403 |
| `csp-probe.mjs` | The dist server | F-228's header checks unchanged | Probes the real dist |
| `viewer_build_check.py` | The both-shapes build | Existing both-files condition | Refusal for the bad shapes |

## State transitions and invariants

Not applicable. The story changes build output and dev-server allowance, not
runtime state. The one stateful surface is the worker's load: an import
error or a default wasm init error independently leaves core not built,
and a page reload retries. A refused package never draws a wrong frame.

## Ownership, commit and recovery

Not applicable in the render-loop sense. The dist is an artefact, rebuilt and
never patched in place. `viewer_build_check.py` refuses the bad shapes at
build time, before anything is served.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | `vite build` with the real pkg present emits a dist in which no emitted file contains the build machine's absolute path, and the worker references the emitted glue rather than a repository /@fs reach | A check over the emitted `dist/assets/*` files | A define that only renames the absolute path | implementation |
| AC2 | That dist, served by a plain static server, loads in headless Chromium and the page reads `core ready` | The browser run the recovered draft never made, recorded with Chromium's version | A preview-only check that the HTML fallback conceals | integrated browser |
| AC3 | on plain static HTTP the emitted worker imports the real module and wasm (both 200) and real glue, with default init completing before `core ready`, no redundant fetch | the future AC run's network log on the real dist | a probe that succeeds only under a repository mapping or route interception | integrated browser |
| AC4 | Dev mode under checkout paths with a space, unicode and a percent sign serves the package URL, `?import` and the `.wasm` | F-217 pass 2's four-name matrix re-run | An allowance left percent-encoded | implementation |
| AC5 | A literal `%` checkout serves both the encoded browser URL and the decoded Rollup specifier from the one `wasmPackage` value | The same matrix, the `%` name | One decoded string assumed right for both uses | implementation |
| AC6 | `viewer_build_check.py` refuses a pkg-PRESENT dist whose emitted worker carries the build machine's absolute path, and accepts the fixed dist. The intentional pkg-ABSENT external encoded /@fs import is exempt, while every other existing absent-shape refusal and negative control, wrong worker count, unexpected wasm, missing external import and the rest, remains | The script run against both shapes | A check that reads the config and not the emitted file | implementation |
| AC7 | Reintroducing the old redundant preflight fetch of the absolute package URL before the import, with the default non-ok check that throws, with the pkg present, must fail the PRESENT emission check and fail the real plain-HTTP `core ready` run for the intended reason, an absolute /@fs fetch answering 404 before the bundled import, with no repository or fallback routing, then restoring the green state. The old single unconditional decoded form may be restored alongside, but the valid decoded build specifier itself is not the defect | Observed control in `f266-pass2-repaired-frozen/fresh-preflight-control/results.json` on 2026-11-10, with a physical source block restored then exactly un-restored | A suite that passes on the wrong dist | implementation, reviewer |
| AC8 | the pkg-absent build keeps external import behavior with the named `core not built` failure, and old fetch-shape expectations are dropped deliberately | the existing `viewer_build_check.py` run | a fix that breaks the fresh-clone path | implementation |
| AC9 | The CSP probe passes against the real dist | `examples/viewer-react/csp-probe.mjs` | A dist change that breaks F-228's evidence | implementation |

Unrun ownership. AC2, AC3 and AC9 need the operator's Mac with Chromium, the
host F-228's probe uses. If that host is unavailable the rows stay open with
that owner named and are never passed. Nothing in this filing ran a build, a
browser or a server.

## Approach

1. Import-only loading. The worker has no probe fetch. In every mode the
   module import itself is the availability probe. An import error OR a
   default wasm init error independently leaves core not built, readiness
   requires a successful import AND a successful default init, and the
   initializer awaits both. The negative control for the probe removal is
   AC7, which restores the old preflight fetch and must fail red before
   the fix is restored green, as a future planned check. A page reload
   retries. No synthetic core_ready, no second fetch of the chunk.
2. One source value, two specifier forms, selected by mode AND presence.
   The Vite 7 config `defineConfig(({ command }) => ...)` selects the
   import specifier from the single `wasmPackage` URL object formed in
   config. The existing `wasmBuilt` check tests both the glue and the wasm
   at config load, before the build. Only when `command == 'build'` AND
   `wasmBuilt` is the proven decoded filesystem module specifier used. In
   every other case, serve present, serve absent and build absent, the
   encoded browser URL is used.
3. Serve (present and absent): the browser imports the encoded browser URL,
   `'/@fs' + wasmPackage.pathname`. It is already encoded exactly once by
   `URL`, so a real space encodes `%20` and a literal `%` encodes `%25`,
   and the browser URL must read `%25` for that case, never blind
   decode-reencode. `fs.allow` becomes `fileURLToPath(repoRoot)`, the
   minimal dev filesystem allowance only, not a build resolver and not a
   production HTTP policy. The dev server serves the transformed source,
   not production chunks. The dated F-217 control says the encoded browser
   config served 200 for the pc%t and lit%20sp JS and WASM, while the
   decoded single-string approach failed the pc%t and lit%20sp browser
   cases. A real space encodes as `%20`, not as the literal `%2520sp`.
4. Build present only (`command == 'build'` AND `wasmBuilt`): the Rollup
   specifier is the proven decoded filesystem module specifier
   `'/@fs' + decodeURIComponent(wasmPackage.pathname)`.
   Rollup rewrites the worker import to the emitted hashed glue chunk and
   rewrites the default wasm asset. No runtime URL reconstruction.
5. Build absent: mark THAT EXACT encoded browser URL external. The build
   emits fine with no file, and at runtime the import fails with the named
   not-built error. Import-only holds in all states.
   No guessed relative path and no `new URL` asset magic. The existing
   empty-URL guard stays.
6. Q2 settled: point the existing CSP probe at the genuine unmodified
   emitted dist on plain static HTTP with an unknown-path 404, no
   repository or /@fs routing fallback, no `page.route` fulfillment, and
   no prepended worker bytes. The current probe's interceptions are live
   today, F-228 done does not fix them. Keep the CSP policy, minimality,
   and negative controls. Any instrumented diagnostic negative case is a
   separate explicit scenario and never replaces the genuine positive.
7. Q3 settled: extend the existing `viewer_build_check` plus unit controls
   already in `packages/floor`, in place. No new gate, profile, or
   operator decision. Only the PRESENT output is checked to forbid
   repository or /@fs reachability, including fetch and not only import.
   The ABSENT build intentionally keeps the external encoded /@fs import,
   the named not-built failure and a page reload as the retry, and drops
   the old fetch-shape expectations deliberately, with no unchanged-byte
   claim. Only the intentional absent external encoded /@fs URL is exempt
   from the PRESENT path ban. Every other existing absent-shape refusal
   and negative control remains, and no guard is disabled.
8. Rejected: regex post-processing of dist, a dev-only pkg proxy, a fixed
   port or host, probe-then-import, fake core_ready, and runtime URL
   reconstruction.


## Dependencies, write ownership and resources

Depends on F-217 (done) and F-228 (done, S46). Per newer user authority
superseding the earlier S46-closure timing, isolated S47 implementation
may start after the normal design approval while S46 remains open. No
integration, push or close waiver is granted by this story. Lands in S47
after F-237 packaging and beside F-245: the built front door must load
its own core before any single-viewer distribution claim, and this story
creates no second viewer. F-237 is an independent release packaging step
with one viewer retained.

Writes `examples/viewer-react/vite.config.ts` and
`examples/viewer-react/src/core-worker.ts`, and extends the existing
consumers `scripts/viewer_build_check.py`,
`scripts/tests/test_viewer_build_check.py` and
`examples/viewer-react/csp-probe.mjs` in place. There is no live F-245 writer
today, but before implementation the S47 coordination must confirm future
configuration, worker, bootstrap, and probe path ownership. This story
does not guarantee F-245 will never need those files. No shared file has
two writers once that is confirmed. Browser runs use the one allocated
resource slot. No ledger allocation or status change in this filing.


## Verification and evidence reuse

The scratch-clone builds with and without the pkg, `viewer_build_check.py`,
the CSP probe and the AC2 browser run, each with node and Chromium versions
recorded. AC1 and AC6 are checks over the pkg-PRESENT emitted files,
never over the config, and AC8 keeps the absent external import. Evidence
invalidates on any later change to the define, the loader or the check,
which is what AC6 exists to catch again.

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
Update docs/lld/build-targets.md and docs/lld/typescript-packaging.md to record the PRESENT bundled-chunk rewrite and ABSENT encoded external import. Add entries to docs/lld/README.md contributor index.

## User-facing documentation impact
main README.md documents static hosting of the React example. The native folder viewer's examples/folder-viewer/src/use.html and more.html need no change because its runtime is unaffected. These are feature-local doc impacts, not Done.

## Proposed design points and settled lead inputs

1. Mode-and-presence selection (approved by root design review): a single
   `wasmPackage` URL object formed in config, with the
   `command == 'build'` AND `wasmBuilt` branch choosing the decoded
   Rollup module specifier and every other branch the encoded browser
   form, build absent marking that exact encoded URL external. Approved in `.claude/reviews/F-266-design-pass-2.md`. The mandatory percent re-encode stays, pc%t to pc%25t and
   lit%20sp to lit%2520sp, and `fs.allow` stays a dev filesystem
   allowance only.
2. The CSP probe serves the real dist (settled): genuine unmodified
   emitted output, plain static HTTP, unknown 404, no fallback or route
   interception.
3. The emitted-file check stays where it is (settled): extend the
   existing floor checker and unit controls in place, no new profile.



Historical proof (dated, not acceptance): the raw decoded build specifier
served all four class names while the encoded space failed with ENOENT, and
the unbound worker artifact (SHA
731fa300250e16c630c0a7a50785d548fe959db4b2ed8117170853dc01c0e01e) shows the rewritten
`'./ocelli_wasm-hKcY7Sue.js'` import plus an absolute /@fs fetch. Those are
observed mechanisms, not source-bound new acceptance. All new route, real
package, browser, network, and resource checks are future acceptance, not
performed proof. Reintroduced absolute fetch, misencoded percent, or a
stand-in HTML positive must fail red.

## Approved amendment: emitted-surface inspection and truthful spelling scope

Approved by root independent amendment review pass 3. This amendment extends the
approved five product files by exactly three tracked paths and no others.
The frozen prior catalogue proposal stays unadopted. This appendix
supersedes the earlier `## DRAFT AMENDMENT` appendix in full.

**The three extension paths.** `scripts/guards/catalogue.py`: helpers,
fixtures, a dedicated Probe registration that actually invokes the checker
on the new controls, and the F-217/F-266 normative sentence update.
`ci/guard-probe-budget.json`: only the actual post-repair viewer-build
count. `docs/runbooks/guard-verification.md`: the generated table from the
existing renderer. There is no anonymous extra guard-unit path: the existing
`scripts/tests/test_viewer_build_check.py` is already owned and holds the
permanent file-surface and control tests. The five product paths stay
owned, and this is an explicit extension, not a contradiction.

**All-emitted-file contract.** Every emitted file under `dist/` is read recursively as raw bytes and scanned for three separate byte patterns: the raw UTF-8 root bytes, their once-URL-encoded form, and the standalone `/@fs` loader prefix. These are distinct patterns, not the root concatenated with `/@fs`, and they are checked in every file including binaries, so there is no name-only or size-only exemption. The actual WASM carries a `/Users/` registry-source marker, which matches neither pattern, so foreign absolute paths are not banned. Text decoding is limited to roles the loader treats as text: the worker chunk is decoded for import and URL-spelling checks, and a `.map` is normally JSON text and is checked the same bounded way. No WASM decoding, no blanket non-UTF-8 refusal, and no promise about obfuscated JS. `emitted()` returns names and a mapping from dist-relative keys like `assets/core-worker-a1.js` to raw bytes. The checker resolves the worker through `_workers` basename matching against those keys. `present_problems` and `absent_problems` receive the raw-byte mapping, keep their positional shape and refusal sentences, and the ABSENT build keeps its intentional external encoded `/@fs` import while PRESENT bans all files. Existing controls pass worker strings as the one-entry raw-bytes mapping form.

Both public checkers receive the actual temporary build base, the resolved Path for the tree under the tempfile.TemporaryDirectory prefix, as an explicit argument. Only PRESENT derives the raw UTF-8 root and the actual Node-encoded pathname root, the form Node pathToFileURL yields per the shared _node_url_pathname helper (Node was already needed by build), plus the standalone /@fs marker, and scans every emitted file including binaries for all three patterns. ABSENT keeps its intentional external encoded import and its original absence checks. It neither derives these scan forms nor bans that import. Raw-UTF-8 and percent-encoded root checks stay distinct so a non-worker encoded root is caught without /@fs.

```python
def emitted(viewer: Path) -> tuple[list[str], dict[str, bytes]]:
def absent_problems(names: list[str], raw: dict[str, bytes], url: str, base: Path) -> list[str]:
def present_problems(names: list[str], raw: dict[str, bytes], url: str, base: Path) -> list[str]:
```

**Controls.** Existing healthy-present, present-external, absent-bundled
and absent-healthy probes and their expected reasons and approved absence
semantics stay, worker strings passed as the one-entry `texts` form. Two
independent non-worker negatives with an otherwise healthy worker are
required: one with the absolute reach in the main chunk, one with it in the
glue. A third negative adds another emitted-file class, and a fourth
targets a checkout-root path other than the pkg file. The healthy control
keeps valid worker, glue and WASM names and a hashed import. The dedicated
absolute-fetch negative keeps those valid names too, so only the
absolute-reach refusal causes RED. A countermutant that drops outside-worker
inspection must fail those fixtures. Missing-assets stays its own
expected-refusal case, never a coverage claim. No gate or profile is
disabled or added. The absent build keeps its external encoded import and
its named not-built refusal unchanged.

**Encoding facts, exact.** A literal `%` encodes to `%25` in the browser
URL, a real space to `%20`, a literal filename `%20` to `%2520`, and a
literal `+` stays as `+` per Node's URL handling. The decoded Rollup path is
a separate build-present form. No custom parser and no arbitrary
obfuscation are introduced. The root, raw and encoded scan is bounded as
approved. The claim that a literal `%` survives unchanged in `URL.pathname`
is dropped as unverified.

**Ratchet.** `entry_sites.viewer-build` is recomputed from the repaired
checker's actual discovery, never forced and never reset to an uncovered
state. All unrelated entries, declared constants, `uncovered: 0`,
`sweep_complete` and `wall_clock_seconds` stay byte-identical. The observed
11 sites at HEAD, an earlier partial 10 after retiring the obsolete fetch
branch, and a current actual discovery of 11 after adding the missing-glue-
import refusal and its registered Probe, are check expectations, not
targets. The runbook table regenerates only after the approved catalogue
union. The three extension paths are approved for implementation by root
amendment review pass 3. The original five-file ownership remains in force.


## Evidence appendix

This plan's evidence section is dated 2026-11-10. The final independent clearing review was not performed. The final feature profile and the final corpus check were not performed. No Done, commit or integration claim is made.

| AC | Evidence |
|---|---|
| AC1, AC6 | Ten real Vite present and absent builds over plain, space, unicode, pc%t and lit%20sp, all green. Immutable emitted hash maps retained, temps removed. Config and worker unchanged across passes. |
| AC2, AC3 | Real pkg Chromium 151.0.7922.34 on a plain static host, settled core ready. Worker, glue and WASM answered 200 with JS, application/wasm content types and strict headers on document, worker, glue and wasm, with body hashes matching the emitted dist. No /@fs mapping or route fulfillment. |
| AC7 | `f266-pass2-repaired-frozen/fresh-preflight-control/results.json`: fixed build and checker 0, actual browser run of the old preflight fetch blocked with PRESENT absolute-root refusal plus /@fs 404 plus core not built, exact restore 0. |
| AC9 | `f266-fresh-dist-csp-sensitivity/sensitivity-results.json`: 14 CSP green, removing actual header emission yields a received null policy assertion with exit 1, exact restore 14 green. Current CSP 269a6 differs from tested 692a6 by comments only, identical executable tokens. |
| AC4, AC5 | Actual Node 24.13.0 and Vite 7 over HTTP, six names times both presences. Worker rewritten env module and injected define observed, equal to the Node pathname. Six PRESENT runs got real glue and byte-exact WASM at 200. The 12 collected observations are classified without rewriting the raw matrix. Same physical space root with only the fs.allow expression changed gave ordered 200, 403, 200 for config orig, bad, orig. All 14 children plus the parent group gone. `f266-child-http-reporter-glm/actual-http-matrix/matrix.json` and `run.json`. Functional, not performance. |
| AC8 | Actual absent build healthy with external encoded import guards across five paths. No new absent browser run is claimed. |

Outside-worker production mutant was RED then restored GREEN with the current checker at 33 tests, 0. Exact plus path and valid non-UTF-8 controls were replayed root independently. Four registered refusal probes and two healthy controls at 0, census and runbook at 0.

Classification of the AC4 500 finding. The six PRESENT responses are the
healthy real-package case. The five other ABSENT HTTP 200 responses are HTML
fallback and not the real package. The one ABSENT 500 is an HTTP error and
no-package-evidence label only, not a newly proved product bug. No overall
healthy-all-cases claim is made.
