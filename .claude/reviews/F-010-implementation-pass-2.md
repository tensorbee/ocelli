# F-010 review, pass 2

**Reviewed**: the staged working tree on `work/f-010-claude`, after the pass-1
remediation
**Result**: 4 defects, 5 smells, 6 nitpicks

An independent reviewer took pass 1's findings one at a time, decided fixed,
not fixed or fixed-but for each, and then hunted for defects the remediation
introduced. Seventeen mutations were run against the new code and two survived.

## Pass-1 findings, closed

D1 to D12 and S1 to S16 are all closed except S2 and S6, which are
deliberate and now documented. Each closure was confirmed by mutation or by
execution, not by reading:

- **D1**, `voi.mjs` throws on `max < min` rather than clamping. Deleting the
  throw goes red.
- **D2**, sha1-length, truncated and over-length digests all refused.
  `{64}` to `{1,64}` goes red.
- **D3 and D4**, `apply` replaces wholesale. Re-introducing the deep merge goes
  red on the new test, and the four colour sidecars now carry
  `voi = {source: "none"}` with no inherited rationale.
- **D8**, `WORKFLOW.md`, `verify.md` and `close-sprint.md` describe the
  exception in the past tense, and the two generated adapters match.
- **D9**, `resolveServedPath` withstood seventeen traversal payloads including
  double encoding, backslashes, `%00`, a trailing-separator base and
  `/index.html/../../../x`. Removing the containment check goes red on three
  tests.
- **D12**, an explicit `$OCELLI_PYTHON` short-circuits, matching
  `corpus_tests.py`'s stop semantics.
- **S7**, the new `stale-frame` fault paints the sentinel at `app.mjs:560`,
  attaches the listener at 564 and dispatches at 569, so the sentinel is on the
  canvas before the injected event exists and `render()` is skipped. The
  expectation fragment matches the guard's message verbatim.
- **S13**, `EXPECTED_CORNERSTONE` was checked value by value against
  `corpus_synth.py`'s `case_multiframe` and `case_signed_12in16`, and both
  mutations go red.
- **S12**, `field in expected` isolated as the operative condition: dropping the
  assertion goes red, dropping the assertion AND the condition goes green.
- **S2** (`@cornerstonejs/tools` pinned and never loaded) is mandated by the
  plan and by D-11, and is now stated in `package.json`'s `$pins` and in the
  LLD. Confirmed absent from the built bundle, zero occurrences.
- **S6** (`bin/ocelli.sh` hunks) is unchanged and each edit is a consequence of
  the plan's step 8. Reported to the integrator in the handoff.

## Defects, introduced by the remediation

### D1, "Most rows are larger than the canvas and are magnified into it"

**Where**: `docs/lld/oracle.md:177`
**What**: magnifying something larger than the canvas is incoherent, and the
fact is the wrong way round. `render-params.json` states it correctly.
**Evidence**: 87 of the 89 rendered frames are within 512 in both dimensions,
2 are not.
**Fixed**: the sentence now says eighty-seven are smaller and two are not.

### D2, "a failed run empties `out/`" was false for most failure modes

**Where**: `run.mjs`, the comment on the output block
**What**: the `rm` was inside `report()`. A missing corpus row, a digest
mismatch, a pin that moved, a failed unit suite, a browser that would not
start: every one throws before `report()`, and every one left the previous
run's frames in place beside a red gate.
**Evidence**: a run with `--rows nosuchrow` exited 1 and left `run.json` and a
stale frame on disk.
**Fixed**: `prepareOutput` empties the directory before the first row is read,
so every failure path leaves nothing. Re-observed: the same failing run now
leaves no directory at all.

### D3, "Every one of those is checked by `checkPins`" was false

**Where**: `docs/lld/oracle.md`, against `run.mjs`'s pin lists
**What**: the sentence followed three bullets, one of which was `esbuild`, and
neither `esbuild` nor `@kitware/vtk.js` nor `gl-matrix` was checked. vtk.js is
what cornerstone3D v5 renders through, so it is the one package whose drift
would move the most reference pixels.
**Fixed** in the code rather than in the prose: `@kitware/vtk.js`, `gl-matrix`
and `esbuild` are now in `PINNED`, and `checkPins` additionally cross-checks
its own lists against `package.json`'s declared dependencies in both
directions. `d3-array` and `d3-interpolate` expose no `./package.json` and are
named as deliberately unreachable, so a package can no longer be pinned in one
place and unchecked in the other. Observed red by editing a version in
`package.json`.

### D4, `minimumWidth`'s non-LINEAR constant was pinned by no test

**Where**: `src/voi.mjs`, test in `tests/params_test.mjs`
**What**: `Number.MIN_VALUE` is a correct encoding of PS3.3 C.11.2.1.3's
`w > 0`, and the section numbering is right, but the test asserted only
`< 1` and `> 0`, so any value in that open interval survived. A threshold of
0.5 would silently discard a conformant LINEAR_EXACT window of 0.25.
**Evidence**: changing the constant to 0.5 left 30 of 30 green.
**Fixed**: the test now honours four concrete widths under LINEAR_EXACT, 0.5,
0.25, 0.001 and `Number.MIN_VALUE`, and asserts that zero and negative widths
are refused under every function.

## Smells, introduced by the remediation

### S1, `rm -rf` on an operator-supplied directory

`--rows` now REQUIRES `--out`, so a typed directory name is on the normal
path, and the wipe had no guard. `fs.rm` on a scratch directory removed a file
that was not the harness's.
**Fixed**: `prepareOutput` refuses a directory that exists, is not empty, and
holds no `run.json`. Observed: a directory containing `notes.md` is refused and
survives, and re-running over a real oracle output directory is allowed.

### S2, `_equal`'s single-element unwrap was reached by nothing and relaxed the
broad comparison

Removing both lines left the whole cross-read green. It also applied to the
pydicom loop, where a sidecar reporting `windowCenter: 40` rather than `[40]`
would newly have passed.
**Fixed**: deleted. The fixture writes each value in the shape the pinned
reference returns, and a shape that moved would mean the pin moved.

### S3, the `downsampled` scale ignored Pixel Spacing

`resetCamera` fits by physical extent, not by pixel count, so a non-square-pixel
frame is scaled differently on each axis and neither factor is
`canvasWidth / columns`. The published scale was wrong for every
non-square-pixel row: 5.333 for `syntax/reference_mono12.dcm` where the true
factors are 8 vertical and 4 horizontal.
**Fixed**: `canvasScale` in `src/params.mjs` derives both axes from the
camera's own `parallelScale` and the row and column spacing, and is unit-tested
against the worked case. The independent confirmation is that row's recorded
`blackFraction` of exactly 0.25, which the physical fit predicts and the
pixel-count model does not.

### S4, real rows' values could be printed by the two fixture loops

The module docstring states that a real row reports an attribute by name only,
and `_show` guarded only the broad loop.
**Fixed**: both fixture loops use `_show`.

### S5, two pin lists that could disagree

**Fixed** by D3's cross-check, and the misleading message that named
`package.json` as the source of a pin held in `run.mjs` is corrected.

## Nitpicks

`run.json.ok` could only ever be `true`: **removed**, and the file's existence
is now documented as the claim. `--out ""` passed argument parsing and died
later inside `rm`: **refused** at parse time. The eslint block granted browser
globals to node-only files: **split**, and the two `page.evaluate` callbacks in
`run.mjs` now name `globalThis` rather than `window`, so `no-undef` still
catches a node-side file reaching for a browser global. Left open and reported:
three near-identical spawn helpers, byte-identical `--sprint` and `--all` arms
in `bin/ocelli.sh`, an unbranched `version` in `render-params.json`, the base64
result set held live through `report()`, and `check_sidecars.py` reading the
manifest by column position without a header check.

## Verified clean

- `resolveServedPath` could not be defeated by seventeen payloads, and `%00`
  stays inside the root where `stat` rejects it into the existing 404 path.
- Wholesale replacement loses no key: all 91 manifest rows resolve, the four
  run-level keys are refused, and every resolved parameter set is complete.
- `EXPECTED_CORNERSTONE` is correct in every value against the generator.
- Numbers executed against `run.json`: 16 low-information rows and they are
  exactly the 15 rendered mono16 `syntax/` rows plus `synthetic/ct_unsigned_16.dcm`,
  2 downsampled rows at 879 by 1168 and 590 by 819, 85 `file` and 4 `none` VOI
  sources, 9 fixture rows, boundaries 91/91/90/90/89/2, 62 `series` tokens, and
  the adapter string matching byte for byte.
- 65 unit tests pass, `npx eslint .` exits 0, the sidecar cross-read exits 0,
  `prose_check.py` is clean over 69 files, `bash -n bin/ocelli.sh` is clean.
- Seventeen mutations run, two survived and became D4 and S2 above.
