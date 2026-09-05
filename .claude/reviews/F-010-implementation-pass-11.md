# F-010 review, pass 11

**Reviewed**: the fully staged index on `work/f-010-claude`, 53 paths,
+9522/-41, after ten rounds of remediation
**Result**: 3 defects, 2 smells, 8 nitpicks

Eleven mutations run, ten red and one deliberately green, which is the finding.
No result is a survivor from an earlier pass. But two of the three defects and
both smells are questions passes 9 and 10 asked, applied to places nobody
applied them, and one is round 10's fix breaking its own count in the same
round. The remediation is the least-reviewed code in the tree, and this round
that is measured rather than asserted: **every guard added below was mutated
before the round ended.**

## Defects

### D1, the LLD said eight suites and there were nine, in the row round 10 had just edited

**Where**: `docs/lld/oracle.md`
**What**: round 10 incremented this row from seven to eight while, in the same
round, adding `tests/registration_test.mjs`. The count was stale before the ink
dried. Third occurrence of one off-by-one: pass 8 found it on the `src/` row,
pass 10 on this one, pass 11 on this one again.
**Fixed, and made mechanical.** `registration_test.mjs` now asserts the LLD's
own spelled-out count against the number of suite files on disk. Observed red
by reverting the word, and red again when the eleventh suite landed later in
this same round, which is precisely the recurrence it exists to stop.

### D2, `build-page.mjs` claimed a cross-check that did not exist

**Where**: `tools/oracle/build-page.mjs`
**What**: `CODEC_WASM`'s doc comment said "Exported because `run.mjs` checks
these packages' versions from the same list ... a second list of them free to
disagree with this one would be a pin that pinned the wrong thing." Nothing
imported it. `run.mjs` kept its own four-entry `PINNED_CODECS` literal, which
IS the second list, and nothing compared them.
**Why it is wrong**: a false sentence about a mechanism, in a tree where
`checkPins` and `registration_test.mjs` both genuinely perform this
cross-check and say so in almost the same words. The `export` keyword had no
consumer, so the stated reason for it was false too.
**Fixed by making the sentence true.** `codecPackages()` reduces each subpath
specifier to its package name, and `checkPins` now compares that set against
`PINNED_CODECS` in both directions. Observed red both ways: a codec removed
from `PINNED_CODECS` gives "copies a wasm binary from ... and run.mjs pins no
version for it", and one removed from `CODEC_WASM` gives "pins ... and
build-page.mjs copies nothing from it".

### D3, `--inject constructor` injected nothing, printed "OK: 1 reference frame(s)" and exited 0

**Where**: `tools/oracle/run.mjs`
**What**: `!FAULTS[options.inject]` is a truthiness test on a plain object, so
it resolves through `Object.prototype`. `FAULTS["constructor"]` is `Object`,
truthy, so the refusal did not fire. Every fault helper then fell through to
its no-op branch, the row rendered normally, and the driver printed a green
line naming frames it never wrote.
**Why it is wrong**: pass 10's D2 one file over, unasked. Round 10 fixed the
identical hole in `tests/faults.mjs` and left its sibling. "Reports green
having done nothing" is the shape this whole story exists to refuse.
**Fixed three ways**, because one was not enough:
1. `Object.hasOwn` at both sites.
2. The fault name is now validated in `parseArgs`, where every other argument
   is validated, so it is unit-testable and fails at parse time.
3. **An injected run that reaches the end with nothing wrong is now itself a
   failure.** The fault having failed to fire is the one outcome an injection
   must never report as success, whatever the reason.
**Observed**: `--inject constructor` now exits 1 naming the six real faults and
writes no directory. `constructor`, `toString`, `hasOwnProperty` and
`__proto__` are all refused by test, and reverting to the truthiness lookup
turns that test red.

## Smells

### S1, round 10's own flagship fix shipped with no probe

**Where**: `tools/oracle/run.mjs`, the `--rows` plus canonical `--out` refusal
**What**: `parseArgs` was not exported and no suite called it. Deleting the
entire guard left 104 of 104 tests green and `eslint` silent. This is the guard
protecting the 269-file reference render that F-011 reads and that costs a full
gate run to rebuild, and it is verbatim pass 10's own S1 turned on round 10.
**Fixed**: `parseArgs` and `DEFAULT_OUT` are exported and `tests/args_test.mjs`
covers them in sixteen tests. Deleting the guard now turns four red, dropping
the `resolve` turns one red, and reverting the identity comparison turns two
red.

**And a real weakness was found while writing those tests.** The identity
comparison needs the canonical directory to EXIST, and the gate runs the unit
suites after `prepareOutput` has emptied it, so on a gate run it degrades to a
resolved-string comparison. That is the honest boundary rather than a bug: a
case-only variant and a symlink can only name an existing directory, and if
there is no canonical output there is no render to protect. It is now written
down in `sameDirectory`'s own comment and in the test, and a second test
asserts the identity behaviour against a scratch directory so something covers
it on every run including a gate run.

### S2, `registration_test.mjs` could be made green while the two lists disagreed

**Where**: `tools/oracle/tests/registration_test.mjs`
**What**: the slice ran to the end of the file rather than the end of the
function, and the matcher took any `tests/<name>_test.mjs` string in it,
comments included. `run.mjs` does carry comment references to files under
`tests/`, so this was not hypothetical.
**Evidence**: dropping a suite from `runUnitTests` AND adding one plausible
comment naming it 380 lines later left all four tests green while the gate list
really was short.
**Fixed**: the slice is bounded at the function's closing brace. Demonstrated:
with the bound, the drop-plus-comment goes red. Without it, green. The hole and
its closure were both observed.

## Nitpicks, and what was done

- **A case-only `--out` on a case-insensitive filesystem** named the canonical
  directory and survived resolving, because `realpathSync` does not normalise
  case on macOS. **`sameDirectory` now asks the filesystem** by device and
  inode, which is its own answer to the question.
- **`rowId`'s refusal was the one in `parseManifest` with no line number**,
  against a module header that makes a point of naming the row. **It names the
  line now.**
- **`rowId` accepted `..` segments** and `join(CORPUS_DATA, row.path)` had no
  containment check, beside a module that performs exactly that check for its
  own directory and explains why. Not reachable, since the manifest is tracked
  and digest-checked. **Refused now**, so the one path derivation without the
  check is no longer one.
- **`cache.purgeCache()` ran only on the success path**, which is the path
  least likely to need it. One corpus row takes the other path today.
  **Moved into a `finally`.**
- Three `presented` failure returns omit `attributes`, the `--sprint` and
  `--all` arms are identical, and `CLAUDE.md`'s "Current state" is stale.
  **Left**: the first is cosmetic on a path that already names its boundary,
  and the other two belong to the integrator.

## Verified clean

- **Round 10's changes, item by item.** The unknown-fault refusal and the
  zero-fault refusal both fire and neither can fire spuriously. `--out`
  resolution handles a trailing slash, `./`, `//`, `..`, and a non-existent
  target. Keying the duplicate guard on `rowId` loses nothing: two rows with
  the same path still collide on the same id, and the message now names both
  lines. The nine names in `run.mjs`, the nine in `package.json` and the nine
  on disk agreed exactly.
- **Every filter in the diff was swept** for what happens when it selects
  nothing: `--rows`, `--rows ""`, `--inject`, `runSelfTest(only)`, `entryFor`,
  `claimedRows`, the render-params rules, `EXPECTED` under `--partial`, and
  `check_sidecars.py` against an empty and a missing directory. All refuse or
  report honestly. `MATCH_KEYS` and `APPLY_KEYS` are `Set`s, so no prototype
  hole there.
- **Every count in the tracked prose re-derived from the tree**, including the
  91 rows, the 89 sidecars, the 269 output files, the 87 magnified against 2
  fitted down, the 85 windows against 4 colour, the 16 low-information rows
  against the 18 `syntax/` rows, the 62 `series` tokens, the 16 pinned
  packages, the six that walk, the five with a nested module marker, the ten
  exported subpaths, the nine fixture rows and the five `output_test` cases.
  `reference_mono12`'s worked example holds arithmetically and in the recorded
  output.
- The arithmetic and the DICOM citations were re-derived once more and are
  correct. No arithmetic of the harness's own enters the reference window.
- Boundary, tier and structure unchanged and clean.
- After remediation: **121 unit tests** in ten suites pass, `npx eslint .`
  exits 0, `prose_check.py` is clean over 79 files, and `bin/ocelli.sh gate
  skills prose content oracle` is ALL GREEN over four gates with every frame
  digest unchanged.

## The loop signal

Eleven passes. The findings keep changing and nothing survives, but three
rounds running the blocking finding has been *the previous round's own
remediation*, unprobed. Round 11 took the remedy rather than restating it:
every guard added or changed here was mutated before the round ended, and two
of those mutations found something. The rule to carry forward is the one this
round finally applied to itself: **a remediation is unreviewed code, so mutate
it in the same round you write it.**
