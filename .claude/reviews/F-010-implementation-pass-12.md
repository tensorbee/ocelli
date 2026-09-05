# F-010 review, pass 12

**Reviewed**: the fully staged index on `work/f-010-claude`, 55 paths,
+10076/-41, after eleven rounds of remediation
**Result**: 2 defects, 1 smell, 5 nitpicks

Ten mutations run, eight red and two green, and the two green ones are the
defect. For the fourth round running the blocking finding is the previous
round's own remediation. Round 11 claimed it had applied the remedy to itself,
and two of the guards it added had not been mutated. That claim was the defect
as much as the coverage gap was.

## Defects

### D1, two guards round 11 added were watched by nothing, and the round said otherwise

**Where**: `tools/oracle/src/manifest.mjs`, `rowId`'s relative-segment and
absolute-path refusals, and `parseManifest`'s line-naming catch
**What**: neither was reached by any test, script or gate. Replacing either
with `if (false)` left 121 of 121 green.
**Why it is wrong, twice over**: microscope section 4, a guard that does not go
red was never a guard. And section 3, because pass 11's own summary says "every
guard added below was mutated before the round ended", which is false of these
two. Round 11 justified the `rowId` refusal by pointing at `src/server.mjs`,
"which refuses exactly this for its own directory" -- and that one has four
assertions. Its new sibling had none. `sameDirectory`, added in the same round,
got its own test. These did not.
**Fixed**: four tests. The two refusals, the adjacent shapes that are
legitimate and must still be accepted (`a/.hidden.dcm`, `a/..b.dcm`), and the
line-naming catch through `parseManifest`. All three mutations now go red, at
two, one and one test respectively.

The test also records something the round did not: **the safe-name regex would
not have caught `..` on its own**, because `.` is inside its character class,
so `..____..____etc` is a legal file name. The refusal adds something real.

### D2, the pass-11 record's prose count was one short of the tree it delivered

**Where**: `.claude/reviews/F-010-implementation-pass-11.md`
**What**: "clean over 78 files" where the staged tree has 79.
`.claude/reviews/` is in scope, so the report counts itself, and the count was
taken before the report was staged.
**Why it recurs**: this is pass 4's own D1 against pass 3, diagnosed there as
"a check that answered a question about a file that did not exist yet, and said
so in the language of success". Every pass since has repeated it, each quoting
the count of the tree before its own record existed.
**Fixed**: 79, and the general lesson is recorded here rather than in another
number: **a review record that quotes a count of tracked prose files is
counting a tree that does not include itself yet.** The gate is green either
way, which is why nine passes missed it.

## Smell

### S1, the interpolation guard resolved through `Object.prototype`, one file over from where round 11 fixed exactly that

**Where**: `tools/oracle/page/app.mjs`
**What**: `Enums.InterpolationType[params.interpolation]` on a TypeScript
numeric enum, which is a plain object on `Object.prototype` AND is reverse
mapped. So the guard admitted `constructor`, `toString`, `valueOf`,
`hasOwnProperty`, and `"0"`, `"1"`, `"2"`.
**Why it is worse than round 11's D3**: the neighbouring branch states the rule
this one broke. `camera.mode` is compared to an exact string and cannot be
defeated, and its comment says "A declared parameter the renderer ignores is
worse than an undeclared one, because the sidecar would record it as having
acted". A `render-params.json` saying `"constructor"` passed, `interpolationType`
became the `Object` constructor, and `renderParams.interpolation` would be
copied into all 89 sidecars as though it acted.
**Fixed on both sides of the boundary.** The page now checks the NAME against
`Enums.InterpolationType`'s own non-numeric keys, and the spec side asserts the
exact value the way `camera.mode` is asserted rather than merely `typeof
"string"`.
**Demonstrated**: under the old guard `constructor`, `toString` and `"0"` all
passed. Under the new one only `NEAREST`, `LINEAR` and `FAST_LINEAR` do.

## Nitpicks, and what was done

- **`codecPackages()` mis-derived an unscoped subpath specifier**, taking two
  segments unconditionally. It failed closed, but the refusal would have named
  a package that does not exist. **Fixed** to take one segment when the first
  is not a scope.
- **`--out` naming a subdirectory of the canonical output was accepted**, which
  would nest a partial render inside the reference render. **Refused now**, and
  the message names all five spellings that are refused. The guard goes red
  under mutation.
- **The LLD suite assertion tested presence, not absence**, so a second stale
  copy of the row would have been shadowed by the correct one. **It now matches
  every spelling of the row and requires exactly one.** Observed red by
  appending a second row saying "four".
- **`tests/args_test.mjs` imports the whole driver**, so it is the one suite
  that needs the reference stack installed and it costs about 140 ms. **Left**:
  the alternative is exporting `parseArgs` from a module that does not import
  playwright, which would put the argument parsing somewhere other than the
  file that uses it.
- `CLAUDE.md`'s "Current state" remains for the integrator.

## Verified clean

- **Round 11's changes, attacked one at a time.** `sameDirectory`'s device and
  inode test was traced in both failure directions: a wrong `true` is a
  spurious refusal and safe, a wrong `false` would clobber the render, and the
  fallback cannot produce a wrong `true` because two identical absolute lexical
  paths cannot be two directories. A trailing separator, `..` past the root and
  a bind mount were each considered. `codecPackages` is correct for every
  specifier shape the tree contains and is exercised on every oracle run.
  `rowId`'s refusals reject nothing the committed manifest contains, verified
  by parsing all 91 rows. The injected-run-stays-green check cannot fire on a
  legitimate run, traced through every path where `inject` is set, and was
  observed firing when a fault was made to inject nothing. Importing `run.mjs`
  from a test has no side effects. The word-number table is robust through
  twelve and fails with the right instruction beyond.
- **`cache.purgeCache()` in a `finally` was confirmed harmless by evidence, not
  argument**: it runs after the return value is evaluated and before the
  promise resolves, the result carries only plain values, and the 269-file
  render on disk was produced with the `finally` in place, with determinism
  matched over two passes and the resolved-metadata fixtures still passing.
- **The parts of the diff earlier passes said least about were read**:
  `page/index.html` cannot move a pixel, `server.mjs`'s five content types
  cover everything the build emits and `application/wasm` is right for
  streaming instantiation, `check_sidecars.py`'s `_read` cannot confuse a
  legitimate `0` with an empty value, the extra-sidecar check runs even under
  `--partial`, and `run.mjs` returns only 0, 1 or 2 so no oracle outcome can be
  mistaken for a named skip.
- The runbook's four new rows were re-executed, including row 21's exact count
  of eight failures and row 22's surviving file.
- Every count re-derived from the tree, and the arithmetic and DICOM citations
  re-checked once more.
- After remediation: **127 unit tests** in ten suites pass, `npx eslint .`
  exits 0, `prose_check.py` is clean, and `bin/ocelli.sh gate skills prose
  content oracle` is ALL GREEN over four gates with every frame digest
  unchanged.

## The loop signal

Twelve passes. Nothing survives from an earlier pass and the findings keep
changing, but four rounds running the blocking finding has been the previous
round's own remediation. Round 12 mutated every guard it added, in the same
command that added it, and two of those mutations shaped the fix: the
subdirectory guard and the LLD absence check were both written because the
first version of each went green. That is the discipline working rather than
being asserted, and it is what the loop has been converging on since pass 9.
