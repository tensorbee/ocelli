# F-010 review, pass 4

**Reviewed**: the fully staged index on `work/f-010-claude`, after three rounds
of remediation
**Result**: 4 defects, 3 smells, 5 nitpicks

## Defects

### D1, the staged tree failed the `prose` gate

**Where**: `.claude/reviews/F-010-implementation-pass-3.md`, three lines
**What**: three prose semicolons in the `canvasScale` bullet.
`scripts/prose_check.py` covers `.claude/reviews/` with no exemption, and
`prose` is in every gate profile, so `--floor`, `--sprint` and `--all` were all
red on this tree. Pass 3's own claim that `prose_check.py` was clean over 70
files was true of the tree it read and false of the tree it delivered.
**Why it happened, which is the useful part**: the gate was run BEFORE the
review file was written. `AGENTS.md` states the order as write, stage, gate,
record, and this is the same mistake at one remove: a check that answered a
question about a file that did not exist yet, and said so in the language of
success.
**Fixed**: the three semicolons rewritten as full stops.
`python3 scripts/prose_check.py` is clean over 71 files.

### D2, two files argued from a `WORKFLOW.md` sentence this same change deletes

**Where**: `bin/ocelli.sh` and `docs/lld/oracle.md`
**What**: both quoted, present tense, `.claude/WORKFLOW.md` saying "the
exception cannot apply once F-010 moves from pending". The staged
`WORKFLOW.md` no longer contains that sentence, because this change replaces it.
**Why it is wrong**: worse than a stale citation. A reader who checks the
source finds nothing and cannot tell whether the reasoning still holds.
**Fixed**: both now state the reasoning themselves and record that
`WORKFLOW.md`, `verify.md` and `close-sprint.md` are corrected in the same
change. `grep` finds no remaining quotation of the deleted sentence.

### D3, the LLD described a codec pin mechanism the code no longer has

**Where**: `docs/lld/oracle.md`, "They are checked through the `.wasm` subpath
rather than through `./package.json`"
**What**: pass 3's S3 fix deleted the separate codec loop. The four codecs now
go through `installedVersion` like everything else. The paragraph fifteen lines
below described the actual mechanism, so the file contradicted itself.
**Fixed**: the stale sentence removed.

### D4, `--partial` did not suppress `_coverage`, so `--rows real/...` failed with a false message and lost its output

**Where**: `tools/oracle/check_sidecars.py`
**What**: `_coverage` asks that every photometric interpretation and pixel
representation PRESENT IN THE SIDECARS ON DISK be asserted by an `EXPECTED` row
also on disk. On a partial run "on disk" is the subset, so any `--rows`
selection containing no `EXPECTED` row that asserts those fields failed,
including `--rows real/`, which is the most likely thing a developer chasing a
divergence types. And because `report()` routes cross-read failures into
`problems`, the frames they asked for were then discarded.
**Why it is wrong**: `_coverage` is a completeness claim about the expectation
table against the WHOLE corpus, exactly like the check `--partial` already
suppressed. Its failure text was also false in that mode: the table does assert
`photometricInterpretation` on `MONOCHROME2`, on a row that run did not render.
**Evidence**: `--rows syntax/reference_mono12` exited 1 with that message and
left no output.
**Fixed**: `--partial` now suppresses all three completeness claims and nothing
else, and its help text names them. Observed: both
`--rows syntax/reference_mono12` and `--rows real/dx_varepop` now pass and keep
their frames.

## Smells

### S1, `installedVersion`'s name predicate was exercised by nothing

**Where**: `tools/oracle/run.mjs`
**What**: two independent mutations of `manifest.name === name` left a full run
green. The comment's trap, a `dist/esm/package.json` carrying only
`{"type": "module"}`, is real but exists only in the five `@cornerstonejs/*`
trees, and every one of those resolves through the direct route and never
enters the walk. All six packages that do take the walk find their manifest on
the first iteration.
**Why it would bite**: a future edit that weakened the predicate would silently
record a wrong version, and a wrong recorded version is oracle drift, the one
thing the whole pin apparatus exists to refuse.
**Fixed**: the walk is now `versionFromPackageRoot` in `src/pins.mjs`, a pure
function over a start directory and a name, with `tests/pins_test.mjs` building
real package trees including the `type: module` trap, a scoped package, a
neighbour's manifest higher up, and a manifest with the right name and no
version. Three separate mutations of the predicate go red, one of them on five
of the six tests.

### S2, `runRecord.problems` could only ever be `[]`

The file is written only when `problems` is empty, and the two checks that can
still push run after `JSON.stringify`. The code and the LLD both argued that a
field that could say only one thing is one more thing to keep true, and then
shipped one.
**Fixed**: removed, with the reasoning recorded beside the missing `ok` field.

### S3, a run that skipped checks wrote an indistinguishable `run.json`

`--no-metadata-check`, `--no-self-test` and `--no-unit` do not require `--out`,
so a skipped run could fill the canonical output with a record byte-shaped like
a gate run's. The USAGE sentence "a run that skipped a check cannot be recorded
as one that passed it" was defended only by the gate's own invocation.
**Fixed**: `run.json` now carries `checks`, naming which of the four ran, for
the same reason `partial` is in the file.

## Nitpicks, and what was done

- `paths.mjs` was missing from the LLD's layout row. **Added.**
- `docs/lld/oracle.md` claimed `$pins` carries the reasoning "for each" package,
  where it carries six reasons covering groups. **Corrected to per group.**
- `discardOutput` is an exported unguarded `rm -rf` whose safety rests entirely
  on `prepareOutput` having accepted the path first, and its doc comment did
  not say so. **The precondition is now written down where the next caller
  reads it.**
- `docs/runbooks/guard-verification.md` had no row for the new guards. **Four
  added**: the oracle output refusal, the `_show` redaction self test, the
  `--out` refusal, and the six-fault harness.
- `--sprint` and `--all` are now character-identical arms in `bin/ocelli.sh`.
  Left alone and reported to the integrator: the arms are outside this story's
  region and another story is editing that function.

## Verified clean

- **The serious hazard asked about is not present.** `discardOutput` cannot run
  on a directory `prepareOutput` refused. `prepareOutput` is called before
  everything, its refusal throws out of `runOracle` into the top-level catch,
  and `report()`, the only caller of `discardOutput`, is never entered. Both
  paths that skip `prepareOutput` also cannot reach the discard: `--inject` is
  excluded and `--report-unsupported` returns 2 upstream of it. `--help`
  returns before the wipe too. All observed: an operator's directory holding
  `important.md` survives, and so does a `--report-unsupported` target.
- Pass 3's D1 and S1 to S4 all hold, checked against the tree.
- **The eslint `ignores` inside a `files` block works in flat config, in both
  directions**, verified by mutation: `process.env.HOME` appended to
  `page/app.mjs` is `'process' is not defined`, and `document.title` appended
  to `src/output.mjs` is `'document' is not defined`.
- `installedVersion` terminates: the loop is bounded at eight and breaks at the
  filesystem root, and an absent package throws out of `require.resolve` into
  `checkPins`'s catch and is reported as not installed.
- The pin cross-check works in both directions and refuses a package declared
  in both `dependencies` and `devDependencies`. The arithmetic checks out:
  10 plus 4 plus 2 is 16, which is 14 dependencies plus 2 devDependencies.
- The accounting cross-check goes red when the `readBack` counter is disabled,
  and double counting is structurally impossible because every failing return
  precedes the point where `stage.readBack` is set.
- `--partial` was traced statement by statement.
- The `tools/oracle/out/` refusal fires on a real reference PNG in a throwaway
  repository, and `.gitignore` covers the path independently.
- `CLAUDE.md`'s `D-01` to `D-11` matches `DEVIATIONS.md` exactly.
- Twelve mutations run in the pass, ten red as intended, two survived and
  became S1.
- After remediation: 76 unit tests pass, `npx eslint .` exits 0,
  `prose_check.py` is clean over 71 files, and `bin/ocelli.sh gate oracle` is
  ALL GREEN with 91 rows attempted, 89 read back, 2 accounted for, identical
  digests across two passes, and all six faults red at their named boundary.
