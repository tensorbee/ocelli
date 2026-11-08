# F-217 fresh-build fix review, pass 1

**Reviewed**: `9f6e1904` on branch `work/f-217-fresh-build`, worktree
`ocelli-wt-f217fix`, read against the sprint head `b5e54203`. The working
tree was clean apart from the untracked `.venv` and `tools/oracle/out` at the
start and the end of the pass, and this file is the only path the pass added.
**Author / reviewer**: fix by a Claude implementer. Reviewed by Claude Opus
5.5 as an independent reviewer that wrote no part of the fix, the
investigation or the plan.
**Result**: 1 defect, 0 smells, 3 nitpicks.

`/microscope F-217 --working`, pass 1: 1 defect, 0 smells, 3 nitpicks.

## Method

Three `git clone`s of the worktree's repository under
`f217rev/` in the session scratchpad, at `9f6e1904` (fix), `b5e54203`
(sprint head) and `7099f0c0` (main), each with its own `npm ci`. The real
pkg was copied from the worktree's `crates/ocelli-wasm/pkg` (wasm 2202907
bytes, glue 138104 bytes). Every mutation ran in the fix clone and was
restored by copying the saved original back. The whole `f217rev` tree was
removed with `shutil.rmtree` at the end. No git command other than `status`,
`show` and `log` ran in the worktree.

## 1. CI's frontend job, step by step, in a fresh clone

Steps from `.github/workflows/ci.yml` job `frontend`, after `npm ci`.

| Step | pkg absent | real pkg present |
|------|-----------|------------------|
| `npm ci` | 0 | (already installed) |
| `bin/ocelli.sh gate advisories` | 0, ALL GREEN | 0, ALL GREEN |
| `bin/ocelli.sh gate lint` | 0, ALL GREEN | 0, ALL GREEN |
| `npm run typecheck` | 0 | 0 |
| `bin/ocelli.sh gate packages` | 0, ALL GREEN | 0, ALL GREEN |

Both `gate packages` runs printed `OK: the example viewer builds with the
wasm pkg absent, keeping the runtime probe, and with it present, emitting the
wasm chunk`. With the pkg absent the real tree's `dist/assets` held only
`core-worker-*.js` and `index-*.js`, and the worker carried
`fetch("/@fs/.../ocelli_wasm.js")` and `import("/@fs/.../ocelli_wasm.js")`.

The regression is real at the sprint head. `npm run build` in the head clone
with no pkg exited 1 with `[vite:worker-import-meta-url] Could not load
.../crates/ocelli-wasm/pkg/ocelli_wasm.js (imported by src/core-worker.ts):
ENOENT`.

## 2. Nothing changed for a developer who builds the wasm first

Head clone and fix clone, both with the same real pkg, `npm run build`.

- File lists are the same: `core-worker-*.js`, `index-*.js`,
  `ocelli_wasm-D-mKcNlf.js`, `ocelli_wasm_bg-URHZQetH.wasm`, `index.html`.
- `ocelli_wasm-*.js` and `ocelli_wasm_bg-*.wasm` are byte identical (same
  SHA-256 and same content-hashed name).
- The worker chunk is byte identical after replacing the clone directory name
  (`fix` or `head`), which is embedded once, in the `/@fs` URL of the fetch
  probe. Its hash differs only because of that path.
- `index-*.js` is byte identical after normalising the worker's hashed
  filename, and `index.html` after normalising both hashed names.

So `rollupOptions: undefined` with the pkg present yields the head's output.

## 3. The stand-in pkg and the mutations

| Case in the fix clone | `viewer_build_check.py` |
|-----------------------|-------------------------|
| unmodified | exit 0, OK |
| `rollupOptions: undefined` always (option removed) | exit 1, `vite build failed with the pkg absent`, ENOENT on `ocelli_wasm.js` |
| `rollupOptions: { external: [wasmUrl] }` always | exit 1, `no ocelli_wasm chunk`, `no ocelli_wasm_bg .wasm asset`, `still imports ... at runtime` |
| condition inverted | exit 1, absent build fails with ENOENT |
| restored | exit 0, OK |

The stand-in claims, executed:

- `STUB_WASM` is 8207 bytes and `WebAssembly.validate` returns true under
  Node 24.
- The padding is load-bearing as the docstring says. An eight-byte module in
  the present tree builds green and emits no `.wasm` asset, and
  `present_problems` reports `no ocelli_wasm_bg .wasm asset`.
- The real glue references its module as `new URL('ocelli_wasm_bg.wasm',
  import.meta.url)` (pkg `ocelli_wasm.js` line 2993), the shape the stub
  copies. The real pkg build produced the same file shape the stub build
  produced.
- Removing the `packages` link from the temporary tree fails the build on
  `packages/core/tsconfig.json`, and removing the copied `tsconfig.base.json`
  fails it on `extends`, so both copies in `make_tree` are needed as the
  comment says.
- The stand-in proves only the emitted shape, which the docstring, the LLD and
  the catalogue limit all say. It does not prove the pkg instantiates.

Catalogue probes: `guard_probe.py --only viewer-build.absent-bundled` and
`--only viewer-build.present-external` both report OK with their control
green. Disabling the glue-chunk and runtime-import refusals in
`present_problems` turns `present-external` red for `nothing in the output
contains 'no ocelli_wasm chunk'`. Disabling the runtime-import refusal in
`absent_problems` turns `absent-bundled` red the same way. Both restored.

## 4. When the existence check runs, and dev mode

`existsSync` runs when the config module is evaluated, once per `vite build`
process, before any build work, so a build cannot carry a decision from an
earlier state. `worker.rollupOptions` is a build option and the dev server's
worker is not bundled by Rollup, so a dev server never consults
`wasmBuilt`.

Dev server (`vite --port 5291`) in the fix clone, started with the pkg moved
aside: `/` 200, `/src/core-worker.ts` transformed with the import left as
`import(__vite__injectQuery(__OCELLI_WASM_URL__, 'import'))`, the page shows
`core not built`. With the pkg moved back while the server ran, the glue and
the `.wasm` were served 200 from `/@fs` with no restart. So no stale decision
is possible in dev mode.

With the real pkg present the page also showed `core not built` in
Playwright's Chromium, after the glue, `?import` and `.wasm` requests all
returned 200. The sprint head's dev server with the same pkg showed the same,
so this is not caused by the fix. Why the core did not start in that browser
was not investigated.

## 5. Catalogue, budget, runbook and the CI floor

- `python3 scripts/guard_census.py` exit 0, `viewer-build` listed with its
  LIMIT text.
- `python3 scripts/guard_census.py --check-runbook` exit 0. The runbook diff
  is the regenerated table with the new rows, which renumbers the rows after
  them.
- `ci/guard-probe-budget.json` `"viewer-build": 11` is accepted by the census.
- `python3 scripts/ci_floor_check.py` exit 0, `all 28 floor gate(s) are
  invoked by CI`.
- `bin/ocelli.sh` runs `viewer_build_check.py` after `package_check.py`,
  whose `npm run build` builds the two library `dist`s the check requires, and
  the LLD's new gate paragraph names the four steps in that order.

## 6. Added prose, executed

- LLD, "Vite 7's Rollup worker sub-build resolves a constant dynamic import
  and loads the file", and "Without it a clean clone fails `npm run build`
  with ENOENT on `ocelli_wasm.js`". Confirmed at the head and by the removal
  mutation.
- LLD, "That is the behaviour the viewer had under Vite 8 on `main`". The
  main clone at `7099f0c0` with no pkg built green under Vite 8.2.2, emitted
  only the worker and index chunks, and its worker carries the `fetch` and the
  `import()` of the `/@fs` URL as template literals. `_calls` accepts
  backticks. Confirmed.
- LLD and config comment, "The config reads the filesystem once when it
  loads". Confirmed by reading and by section 4.
- LLD, the stand-in description and "Vite's 4096-byte inline limit".
  Confirmed in section 3.
- DEVELOPER_SETUP, "The same holds for `npm run build`", "With the pkg it
  bundles the wasm as its own files", "Without it the worker keeps a runtime
  import and reports the core as not built". Confirmed, the last one by
  `vite preview` of the pkg-absent build, which shows `core not built` with
  `CoreLoadFailed` logged from the worker.
- The probe attribution in the LLD, the config comment, the catalogue `spec`
  and the check's docstring. Refuted, see D1.

## Findings

### D1, defect. The prose credits the fetch probe with a result the probe does not produce under Vite's own servers

**Reviewed source**: `docs/lld/typescript-packaging.md` new paragraph,
"so the import is left to runtime and the worker's `fetch` probe reports the
core as not built". The same claim is in `examples/viewer-react/vite.config.ts`
("the worker's probe reports the core as not built"), the `viewer-build`
catalogue `spec` in `scripts/guards/catalogue.py` ("leaves the runtime probe
to report the core as not built") and the `scripts/viewer_build_check.py`
docstring ("the `fetch()` probe in front of it, which is the path that
reports the core as not built").

**Trigger**: build the viewer with no pkg and serve `dist` with `vite
preview`, or run the dev server with no pkg, then request the pkg URL.

**Observed**: `curl` of `/@fs/.../crates/ocelli-wasm/pkg/ocelli_wasm.js` on
the preview server returned `200 text/html` (the SPA fallback), and the dev
server returned `200` with the `index.html` body. The probe in
`core-worker.ts` tests `probed.ok`, so it passes. The worker reaches
`import()` of an HTML response, which fails, and `loadCore` turns that into
`CoreLoadFailed` and the panel shows `core not built`. The browser's network
log of the preview showed the one probe request and nothing from the probe
path that refuses.

**Expected**: the outcome the prose describes holds, the core reads as not
built. The mechanism it names does not. The probe reports only on a server
that answers the missing path with a non-2xx status, which neither server
this repository ships does.

**Contract**: microscope section 3, a claim in prose that is false.

**Consumers**: a reader of the LLD or the config who trusts the probe as the
refusal path, and `absent_problems`, which requires the probe to exist on the
strength of that claim. Requiring it is still a reasonable shape check of the
emitted worker, so the check itself need not change.

**Smallest fix**: say what DEVELOPER_SETUP already says, that the worker
reports the core as not built because the runtime import fails, or that the
probe or the import fails, in the four places. Prefer deleting the probe
attribution to explaining the SPA fallback. The probe's own weakness against
the fallback is pre-existing in `core-worker.ts` and outside this diff.

### N1, nitpick. The existence check keys on the glue only

`wasmBuilt` tests `ocelli_wasm.js`. A pkg directory holding the glue without
`ocelli_wasm_bg.wasm` builds green and emits no `.wasm` asset (observed with
the stand-in tree). `wasm-pack` writes both together, so this needs an
interrupted or hand-edited pkg.

### N2, nitpick. The gate description says "with and without the wasm pkg"

`bin/ocelli.sh`'s `packages` row reads as if the real pkg were built. The
present state is a stand-in. The LLD and the catalogue limit say so, the gate
list does not.

### N3, nitpick. `wasm_url` does not percent-encode, the config does

The config's URL is `new URL(...).pathname`, which percent-encodes a space,
and `viewer_build_check.wasm_url` uses `Path.as_posix()`. A temporary
directory whose path contains a space would make `_calls` miss the URL and
the check refuse a correct build. Not reproduced, the default temporary
directories here and on the CI runner have no space.

## Pre-existing, outside this diff

- The bundled worker still probes the build machine's absolute `/@fs` URL
  before importing its own chunk, as the investigation noted for `main`.
  Unchanged by the fix (section 2).
- The Playwright MCP wrote its snapshots into the canonical checkout's
  `.playwright-mcp/`, which `.gitignore` line 117 ignores and which already
  existed. Nothing was written to this worktree.

## Not exercised

- The core loading successfully in a browser, in dev or from `dist`, with the
  real pkg. It did not start in Playwright's Chromium at either the head or
  the fix.
- The real `wasm-pack` output of this tree. The pkg used was the one already
  in the worktree, built at 19:00 on the day of the commit.
- A Linux runner. Every run here was macOS with Node 24.13.0.
