# F-217 fresh-build fix review, pass 2

**Reviewed**: range `3f9787dc..3bd40a80` on branch `work/f-217-fresh-build`,
worktree `ocelli-wt-f217fix`, one commit (`3bd40a80`, "fresh build fix review
pass 1, D1 and N1 to N3"). The working tree held only the untracked `.venv`
and `tools/oracle/out` at the start and the end of the pass, and this file is
the only path the pass added.
**Author / reviewer**: fix by a Claude implementer. Reviewed by Claude Opus
5.5 as an independent reviewer that wrote no part of the fix or of pass 1.
**Result**: 0 defects, 0 smells, 3 nitpicks.

`/microscope F-217 --working`, pass 2: 0 defects, 0 smells, 3 nitpicks.

## Acceptance challenged

- Pass 1 D1, the four probe sentences. Replayed against `vite preview` of a
  pkg-absent build in a browser and against both servers with `curl`.
- Pass 1 N1, both pkg files required. A glue-only pkg built with the fix
  config and with the `3f9787dc` config.
- Pass 1 N2, the gate description. Read from `bin/ocelli.sh gate --list`.
- Pass 1 N3 as done, the decoded `/@fs` URL. Built and served under five
  directory names that `URL.pathname` encodes, with the fix config and the
  `3f9787dc` config, in build mode with the pkg present and absent and in dev
  mode. The independently derived expectation: Rollup needs a literal
  filesystem path, and a browser needs a string that its URL parser turns
  into the right request.
- Ordinary paths unchanged. Fix and `3f9787dc` built with the same real pkg.
- The four new unit tests. Mutated the code each one guards.

## Evidence

All runs are executed now, on macOS with Node 24.13.0 and Vite 7, in fresh
`git clone`s of the worktree's repository with their own `npm ci` and
`npm run build`: `f217rev2/fix` at `3bd40a80`, `f217rev2/base` at
`3f9787dc`, and `f217rev2sp/sp ace/repo` at `3bd40a80`. The real pkg was
copied from the worktree (wasm 2202907 bytes). Special-character names were
made by renaming the spaced parent directory. Every mutation was in a clone
and restored by copying the saved file back, with `git status` clean after.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate packages`, no pkg | `fix` clone | now | 0, ALL GREEN, 4 new tests OK |
| `bin/ocelli.sh gate packages`, real pkg | `fix` clone | now | 0, ALL GREEN |
| `bin/ocelli.sh gate packages`, real pkg and no pkg | `sp ace` clone | now | 0 both, ALL GREEN |
| `vite build`, real pkg and no pkg | `fix` and `base` clones | now | identical file lists, see Verified clean |
| `vite build`, `sp ace` and `ünï cødé` and `pc%t` and `lit%20sp` | `sp ace` clone renamed, fix config | now | all 0, present emits `ocelli_wasm-D-mKcNlf.js` and `ocelli_wasm_bg-URHZQetH.wasm`, absent emits only the two chunks |
| `vite build`, real pkg, `sp ace` | same clone, `3f9787dc` config | now | 1, `Could not load .../sp%20ace/... ENOENT` |
| `vite build`, `ha#sh` and `q?m` | same clone, both configs | now | 1 for both configs, Vite refuses the root itself |
| dev server, GET the define as a browser resolves it, plus `?import` and the `.wasm` | four names, both configs | now | 403 for every name and both configs, see Pre-existing |
| same, with `server.fs.allow` changed to `fileURLToPath(repoRoot)` in the clone | four names, both configs | now | see N1 |
| `vite preview` of the no-pkg build, Playwright Chromium | `fix` clone | now | `core not built`, `CoreLoadFailed` from the worker, fetch 200, import 404 |
| `curl` of the missing pkg URL, dev and preview | `fix` clone, no pkg | now | 200 `text/html` for the bare URL and `?import` on both |
| unit tests under mutations M1 to M4 | `fix` clone | now | see Verified clean |
| `viewer_build_check.py` with the config reverted to the encoded URL | `sp ace` clone | now | 1, absent and present both refused |
| `viewer_build_check.py` with the `.wasm` existence term removed | `fix` clone | now | 0, see N3 |
| `guard_census.py`, `--check-runbook` | `fix` clone and worktree | now | 0 and 0 in both, 0 watched by nothing |
| `guard_probe.py --only viewer-build.absent-bundled`, `--only viewer-build.present-external` | `fix` clone | now | OK, control green |
| `ci_floor_check.py` | `fix` clone | now | 0, all 28 floor gates invoked |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the decoded URL is wrong for a browser when the path holds a `%`

**Where**: `examples/viewer-react/vite.config.ts` line 21, and its comment
"A browser encodes the space again when it requests the URL".

The define is one string that serves Rollup and the browser. Decoding is right
for Rollup and for a space or unicode, which the browser re-encodes. It is
wrong for a literal `%`, which the browser leaves as it is. With
`server.fs.allow` corrected in the clone so the dev server serves `/@fs` at
all, the fix config gave `pc%t` 404 and `lit%20sp` 200 `text/html` (the
`%20` decoded to a different directory), where the `3f9787dc` config served
both as 200 `text/javascript` with the `.wasm` as `application/wasm`. `sp ace`
and `ünï cødé` were 200 under both. Not observable in the shipped tree,
because the dev server already refuses every such path (see Pre-existing),
and build mode was correct for all four names. A checkout under a directory
name holding `%` is rare. If `fs.allow` is ever fixed, the comment's claim
should be limited to the characters a browser re-encodes, or the two uses
should get two strings.

### N2, the prefix test checks the constant, not its use

**Where**: `scripts/tests/test_viewer_build_check.py`,
`test_the_gate_builds_under_a_path_with_a_space`.

M4 changed `main` to pass `prefix="viewer-build-"` and left `TREE_PREFIX`
alone. All four tests stayed green. The gate's end-to-end check then builds
under a path with no space, and the encoded-config regression it is meant to
catch would pass. Changing the constant itself (M2) does go red.

### N3, nothing pins the glue-and-wasm condition

**Where**: `examples/viewer-react/vite.config.ts` lines 33 to 35.

The behaviour is right. A glue-only pkg built green in the pkg-absent form
(worker `core-worker-BHZQZD_6.js`, byte identical to the no-pkg build, runtime
`import()` kept), where the `3f9787dc` config emitted a glue chunk and no
`.wasm`. Removing the `.wasm` term left `viewer_build_check.py` and the unit
tests green, because the check never builds a glue-only tree. Pass 1 rated the
gap a nitpick and that stands.

## Verified clean

- **Ordinary paths unchanged.** `fix` and `base` with the real pkg, and with
  none. Same file lists, and with the pkg the same
  `ocelli_wasm-D-mKcNlf.js` and `ocelli_wasm_bg-URHZQetH.wasm` names pass 1
  recorded. Every file is byte identical after replacing the clone directory
  name in the worker and the hashed worker name in `index-*.js`. The decoded
  and encoded URLs are the same string when nothing needs encoding.
- **Space, unicode and `%` in build mode.** Present: bundled glue and `.wasm`,
  worker imports `./ocelli_wasm-D-mKcNlf.js`. Absent: no wasm file, worker
  keeps `fetch` and `import` of the plain decoded path. The `3f9787dc` config
  under `sp ace` fails the present build with ENOENT on `sp%20ace`, which is
  the implementer's finding reproduced. `#` and `?` fail in Vite before the
  config matters, under both configs.
- **Dev mode.** No regression for any name tested, because the dev server
  refuses `/@fs` for all of them under both configs. With `fs.allow` corrected
  the fix config works for space and unicode. For `%`, see N1.
- **The new tests fail when the code is wrong.** M1, `wasm_url` percent
  encodes: `test_a_space_stays_a_space` and
  `test_an_encoded_url_in_the_worker_is_refused` red. M2, `TREE_PREFIX`
  without a space: the prefix test red. M3, `_calls` matching any URL: the
  encoded-URL test red. Restored: 4 OK. The config reverted to the encoded
  URL turns `viewer_build_check.py` red under the spaced prefix, for both the
  absent and the present build. The expected strings in the tests are plain
  paths, consistent with their docstring. The tests run in `gate packages`,
  where the `&&` chain makes a failure red.
- **D1, the four sentences.** LLD "the core reads as not built because that
  import fails", the config comment, the catalogue `spec` "leaves the import
  to runtime, where it fails and the core reads as not built", and the
  docstring. In the preview the fetch got 200 and the `import()` got 404, the
  worker logged `CoreLoadFailed` and the header read `core not built`. The
  docstring's "dev and preview servers answer the missing path with the HTML
  fallback and a 200, so the probe passes" holds for the `fetch`, whose
  request both servers answered 200 `text/html`. The catalogue `refuses`
  text still names the fetch probe as part of the emitted shape, which is
  what the check tests.
- **N1 and N2 of pass 1.** Closed as described in N3 above and by
  `gate --list`, which now reads "the viewer build with the wasm pkg absent
  and with a stand-in pkg".
- **The LLD additions.** "a `%20` there is ENOENT for a checkout under a path
  with a space" reproduced. "under a directory name with a space so the
  decoded URL is exercised on every run" is true, with the N2 limit on what
  pins it.
- **README, use.html, more.html.** README lines 163 to 170 on `core not built`
  stay true. Nothing in the three describes the URL encoding or the gate.
- **Census, runbook, floor and probes.** All exit 0 as listed.

## Pre-existing, outside this diff

- `server.fs.allow: [repoRoot.pathname]` is percent-encoded, so the dev
  server answers 403 Restricted for the pkg URL, `?import` and the `.wasm`
  under any checkout path that `pathname` encodes (space, unicode, `%`),
  under both configs. The same root also covers `packages/core/dist`, so the
  viewer's dev mode under such a path is likely unusable beyond the core.
  That last part was not measured.
- Vite refuses a root containing `#`, and fails to load its own config under
  a root containing `?`.

## Limitations and handoff

- One browser run, Playwright Chromium against the pkg-absent preview. The
  dev-mode results are `curl` requests built from the define with Node's
  WHATWG `URL`, which is what a browser does, not a browser run.
- The core loading with the real pkg in a browser was not exercised, as in
  pass 1.
- macOS only, no Linux runner.
- The scratch clones are removed with `shutil.rmtree` at the end of the pass.
- The three nitpicks do not block. This is a clean bounded pass on the
  changed material, not sprint readiness, which needs `gate --sprint` and the
  whole-sprint review.
