# F-010 review, pass 10

**Reviewed**: the fully staged index on `work/f-010-claude`, 50 paths,
+9197/-41, after nine rounds of remediation
**Result**: 3 defects, 2 smells, 3 nitpicks

Eleven mutations run, nine red and two survived. Round 9's own work held: the
new entry-point guard withstood every attack the reviewer constructed, the
write-block `try/catch` was traced end to end, and all four corrected DICOM
citations resolve against the standard's own text. Every finding below is new,
and two of the three defects are guards no earlier pass had exercised at all.

## Defects

### D1, the LLD said seven test suites and there were eight

**Where**: `docs/lld/oracle.md`
**What**: round 9 added `tests/paths_test.mjs` and updated the sibling `src/`
row and not this one.
**Why it matters beyond the number**: this is pass 8's own defect one row down
and one round later. The table is the thing a reader consults to find out what
is there.
**Fixed**, and then made mechanical: see S2 below, which turns the suite list
into something asserted rather than maintained.

### D2, the fault self test reported "every one caught" having run nothing, and exited 0

**Where**: `tools/oracle/tests/faults.mjs`
**What**: `runSelfTest(only)` skips every fault that does not match the filter.
An unmatched name left `problems` empty and `observed` empty, so `ok` was true
and the runner printed a green line naming zero faults.
**Why it is wrong**: the sprint's named defect one level down, inside the file
built to refuse it. `src/faults.mjs` opens with "Every new guard is observed red
before it is claimed", and this path claimed six guards observed red having
observed none. `run.mjs` refuses both of its own filters when they select
nothing, and `--report-unsupported` exits 2 precisely so a mode answering a
different question cannot read as a pass. This filter had neither.
**Not gate-reachable**: the gate calls `runSelfTest()` with no argument. It bit
the operator debugging one fault by name, which is the documented use.
**Evidence**: `node tests/faults.mjs nonexistent-fault-name` printed
`OK: 0 injected fault(s), every one caught` and exited 0.
**Fixed** twice over: an unknown fault name is refused with the list of real
ones, and a run that observed no fault at all is a failure whatever the reason.
Re-observed: the same command now names the six real faults and exits 1.

### D3, the guard stopping `--rows` from writing into the canonical output compared raw strings

**Where**: `tools/oracle/run.mjs`
**What**: `options.out === DEFAULT_OUT` with `DEFAULT_OUT` absolute and `--out`
taken verbatim. So `--out tools/oracle/out` from the repository root, `--out
out` from `tools/oracle`, or a trailing slash all compared unequal, the guard
did not fire, `prepareOutput` recognised the real directory by its `run.json`
and emptied it, and a one-row run replaced the full corpus render that F-011
reads.
**Why it is wrong**: the usage text prints `(default tools/oracle/out)`, which
is exactly the relative spelling an operator would retype, and the comment
above the guard stated an effect the guard did not have. It is also the same
class as pass 9's D1: two paths compared as strings rather than resolved.
**Fixed**: `--out` is resolved at parse time, so the comparison is between two
absolute paths. Observed: both the relative spelling from `tools/oracle` and
the relative spelling from the repository root are now refused, the message
says both spellings are the same directory, and the canonical output survived
the test intact at 269 files.

## Smells

### S1, both refusals round 9 added were untested and both survived mutation

**Where**: `tools/oracle/src/unsupported.mjs`
**What**: the empty `errorContains` refusal and the empty row string refusal
had no test. Removing either left all twelve tests green. Pass 9 claimed both
and shipped neither with a probe.
**Why it matters**: `errorContains: ""` is contained by every string, so it
silently retires the fourth conjunct of `entryFor` while looking like a
complete entry, which is the exact failure being guarded.
**Fixed**: two tests, and the first of them also demonstrates that an empty
fragment really would match an unrelated error, so the refusal is shown to be
necessary rather than asserted to be. Both mutations now go red.

### S2, the suite list existed twice with no cross-check, in the file that cross-checks its other duplicated list

**Where**: `tools/oracle/run.mjs`'s `runUnitTests` against
`tools/oracle/package.json`'s `test` script
**What**: both enumerated the same filenames by hand and nothing compared them.
A suite added to one and not the other is one `npm test` runs and the gate does
not, or the reverse, silently. `checkPins`, 250 lines above in the same file,
does exactly this cross-check for exactly this reason.
**Fixed**: `tests/registration_test.mjs` takes the DISK as the authority and
asserts both lists against it, which also catches the case neither list could
ever catch, a suite file registered nowhere. Observed red three ways: a suite
dropped from `package.json`, a suite dropped from `run.mjs`, and a new
unregistered suite file appearing.

## Nitpicks, and what was done

- `isEntryPoint`'s catch names one of its three causes. **All three named.**
- `rowId` maps `a/b.dcm` and `a__b.dcm` to one output name, and the duplicate
  guard keyed on the path while its message was about the output name.
  **The guard now keys on the output name**, which is what would collide, and
  a test covers the two-paths-one-id case. Reverting the key goes red.
- `--sprint` and `--all` are byte-identical arms in `bin/ocelli.sh`. **Left and
  handed to the integrator**, as in the last four passes.

## Verified clean

- **`isEntryPoint` could not be broken.** A `data:` URL, an `https:` URL and an
  unparseable string all return false through the catch. A relative `argv[1]`
  returns true, which is the route `tests/faults.mjs` uses. A directory returns
  false, an absent `argv[1]` returns false, and `..` in the path returns true.
  `realpathSync` throws nowhere reachable. Reverting to the old idiom turns two
  tests red and reproduces the original symptom exactly, empty stdout and exit
  0 where exit 1 was required.
- **All three call sites work standalone**, including `build-page.mjs` and
  `tests/faults.mjs` run directly through a symlink whose name contains a
  space.
- **The write-block `try/catch` behaves as claimed**, traced by making
  `writeRow` throw after two files were written: exit 1, the error named, and
  the output directory gone entirely.
- **The four corrected citations resolve against the standard's own text.**
  C.11.2.1.2.1 opens with "If VOI LUT Function (0028,1056) is absent or has a
  Value of LINEAR". Table C.7-10 has Pixel Spacing as Type 1, and the three
  corpus rows without it are exactly the two IODs that exclude the Image Plane
  Module and the enhanced CT row that carries it in a shared functional group,
  confirmed with pydicom. Table C.7-9 has all three lossy attributes as Type 3
  and the section says so in its own words. PS3.5 8.1.1's current text is "High
  Bit shall be one less than Bits Stored", with the note that it formerly was
  not restricted, so the comment's framing is the standard's.
- **The whole citation set swept again**, every `PS3.x`, `C.x` and `Table C.x`
  string in the diff. All correct.
- **The committed `unsupported.json` is unaffected by the new refusals**, and
  `readUnsupported` on the real file is asserted by two tests.
- **Every executable number in the tracked prose re-run against the real
  output**, including the boundary counts, the sixteen pinned packages, the
  two downsampled rows enumerated rather than counted, the sixteen
  low-information rows enumerated from the eighteen `syntax/` rows, the
  eleven-key exports map, and `reference_mono12`'s 0.25 letterbox matching its
  recorded `blackFraction` exactly.
- Guard-verification probes 20 to 23 re-observed, including probe 20's control:
  the same PNG staged at a non-oracle path passes, which is what makes the path
  prefix the mechanism rather than the image.
- The S01 pre-oracle removal is complete: no reference to `s01_pre_oracle` or
  `profile` remains, `skip()` is still live for its three real users, and the
  oracle arm exits 1 rather than 3.
- Code read for correctness rather than for claims, across `page/app.mjs`,
  `server.mjs`, `manifest.mjs` and `runSelfTest`. Boundary, tier and structure
  unchanged and clean.
- All eleven required commands green, re-run after every mutation.
- After remediation: 104 unit tests pass, `npx eslint .` exits 0,
  `prose_check.py` is clean over 77 files, and `bin/ocelli.sh gate skills prose
  content oracle` is ALL GREEN over four gates with every frame digest
  unchanged.

## The loop signal

Ten passes, and the findings changed again, which is the healthy shape. None of
the three defects is a survivor, and none of round 9's remediations was wrong.
D2 and D3 were both found by RUNNING the code rather than reading it, D2 by
typing a wrong fault name and D3 by asking whether the path comparison fixed in
round 9 had a sibling elsewhere. That generalisation is the technique to carry
forward: a defect found in one place is a question to ask everywhere.
