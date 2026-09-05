# F-010 review, pass 3

**Reviewed**: the staged working tree on `work/f-010-claude`, after two rounds
of remediation
**Result**: 1 defect, 4 smells, 7 nitpicks

## Defect

### D1, a red gate could still leave a complete, current-looking output directory

**Where**: `tools/oracle/run.mjs`, and the paragraph in `docs/lld/oracle.md`
asserting the opposite
**What**: pass 2 moved the wipe to the front and removed `run.json.ok` on the
grounds that the file's existence is now the claim. Both halves were asserted
absolutely and three routes falsified them.

1. `prepareOutput` ran AFTER `checkPins`, and "a pin that moved" was one of the
   five cases pass 2 named as fixed.
2. The pydicom cross-read runs after the frames are written and threw out of
   `report()`, so not even the "the directory is empty" line was reached.
3. The fault self test did the same.

Routes 2 and 3 are both on the gate path. The resulting `run.json` had
`partial: false`, an empty `problems` array and nothing else saying the run
went red, so F-011 had no in-band way to tell.

**Evidence**: an interpreter stub exiting 1 left five files including
`run.json` beside exit 1, and a version edited in `package.json` after a good
run left the same five beside a red pin gate.

**Fixed**, from both ends:
- `prepareOutput` now runs before the pin check and before everything else,
  so every early abort leaves nothing.
- The cross-read and the self test now RECORD a problem rather than throwing,
  and there is one failure path, which calls `discardOutput` before it prints.
- Re-observed: the same failing cross-read now leaves no directory at all.

The invariant is now one sentence and it is stated as such in the code and in
the LLD: **the output directory holds one complete run that passed every
boundary, or it holds nothing.**

## Smells

### S1, `prepareOutput`'s `rm -rf` guard was exercised by nothing

Not exported, no test. Weakening the condition to `existing.length > 999999`
left 65 of 65 tests green while the run silently deleted a directory holding
`notes.md`.
**Fixed**: the output lifecycle moved to `src/output.mjs` and
`tests/output_test.mjs` covers all five cases, including the one that matters.
Both the guard condition and the `ENOENT` branch go red under mutation.

### S2, the bare `catch { return; }` treated "`--out` is a file" as "does not exist"

`readdir` on a file throws `ENOTDIR`, which was swallowed as "nothing there
yet". The run then rendered the whole corpus and died at `mkdir` with an
unexplained `EEXIST`. That is the shape of failure `parseArgs`'s own comment
refuses, in the function added to enforce the discipline.
**Fixed**: only `ENOENT` returns. Everything else is refused, naming the code.
Covered by `tests/output_test.mjs`.

### S3, `PINNED_IN_MANIFEST_ONLY`'s justification was disproved thirty lines above it

The comment said `d3-array` and `d3-interpolate` were "NOT reachable through
`require`". The narrow claim was true, the operative one was not: `checkPins`
already reached four codec packages by walking up from a resolved path, and the
same walk reaches both d3 packages.
**Fixed**: one `installedVersion` helper that tries `require("<name>/package.json")`
and falls back to resolving the main entry and walking up to the `package.json`
whose `name` matches. The exemption list is gone, the separate codec loop is
gone, and all **sixteen** declared dependencies now have their installed
version checked. Confirmed: `run.json` records sixteen packages.

### S4, `_show`'s redaction was a guard nobody has watched fail

`_show` runs only on a mismatch, and no gate run ever produces one. Deleting
the `startswith("real/")` branch left the cross-read green, and the regression
that leaks a real corpus value into gate output would have been invisible to
every check in the repository.
**Fixed**: `check_sidecars.py --self-test` exercises `_show` for four value
types on a real path and on two non-real paths, and `_equal` for the cases that
matter (absence is not zero, a shorter list is not a prefix, order is not
ignored, a scalar is not a one-element list). The oracle gate runs it, and it
doubles as the interpreter probe so it costs no extra resolution. Both
mutations go red.

## Nitpicks, and what was done

- A run selecting only unsupported rows exits 0 having written no frames. Left
  as correct, and an **accounting cross-check** was added on top:
  `readBack + unsupported === applicable`, which catches a counter that stopped
  counting.
- `CLAUDE.md` said `D-01` to `D-07` while `DEVIATIONS.md` carries D-01 to D-11,
  and its command table still described the sprint gate's bootstrap policy.
  **Both corrected.**
- `checkPins` merged `dependencies` and `devDependencies`, so a package in both
  could pass on the wrong one. **A package in both is now refused.**
- The eslint split was one-directional: page files still received `process` and
  `Buffer`. **The node block now excludes `tools/oracle/page/**`**, so
  `no-undef` catches a driver file reaching for `document` and a page file
  reaching for `process`.
- The `s01_pre_oracle` tombstone read as `gates_cmd`'s doc comment. **Separated,
  and `gates_cmd` given a doc comment of its own.**
- `docs/lld/oracle.md` said `package.json` holds "the pins, and nothing else",
  and its layout table was one file behind. **Both corrected.**
- Pass 2's framing of S4 as a leak was wrong: both fixture tables are keyed only
  on synthetic paths, so `_show` cannot withhold there today. The change is
  still right, because a real row added to either table later would otherwise
  print its values, and the self test is what makes that a checked property
  rather than a hope.

Also found while fixing the above: teaching the cross-read about `--rows`. A
partial run legitimately produces fewer sidecars than the manifest has rows,
and the completeness half of the cross-read failed every partial run.
`--partial` now suppresses that half and nothing else.

## Verified clean

- Pass 2's four defects and five smells were checked against the tree rather
  than against the report, and all nine hold.
- **The `canvasScale` physics was verified independently and is correct.** VTK's
  `parallelScale` is half the viewport height in world units, and `PixelSpacing[0]`
  is the row spacing per PS3.3 C.7.6.2.1.1. Confirmed against four rows'
  recorded `parallelScale`. `reference_mono12` is 24 by 32 mm, height-fit, half
  16. `dx_varepop` is 179.872 mm wide, width-fit, half 89.936. `us_cmb_crc` is
  819, half 409.5. `ct_cmb_mml` is 360, half 180. The predicted letterbox fraction was then
  compared with the independently counted `blackFraction` for **all 89 rows and
  no row disagrees**. `reference_mono12` predicts 0.250000 and records
  0.250000. A pixel-count model would predict at least 0.3333 for that row and
  at least 0.7 for `mr_nonsquare_spacing`, so the observation refutes it.
- `Number.MIN_VALUE` under `width >= minimumWidth(fn)` is exactly `w > 0` for
  doubles, which is C.11.2.1.3.1 and C.11.2.1.3.2, and LINEAR keeps `>= 1`,
  which is C.11.2.1.2. Three separate mutations of the constant go red.
- `EXPECTED_CORNERSTONE` goes red on a wrong value AND on a wrong shape, which
  is the case the removed unwrap used to hide.
- `d3-array` and `d3-interpolate` genuinely expose no `./package.json`, and the
  fallback walk reaches both.
- `globalThis.__oracle` works end to end: the page assigns `window.__oracle`,
  the driver waits on `window.__oracleLoaded` as a browser-evaluated string,
  and a real single-row run on SwiftShader writes all four files.
- Twelve mutations run in the pass, ten red as intended, two survived and
  became S1 and S4.
- After remediation: 70 unit tests pass, `npx eslint .` exits 0, the cross-read
  and its self test both exit 0, `prose_check.py` is clean over 70 files,
  `bash -n bin/ocelli.sh` is clean, and `bin/ocelli.sh gate oracle` is ALL
  GREEN with 91 rows attempted, 89 read back, 2 accounted for, identical
  digests across two passes, and all six faults red at their named boundary.
