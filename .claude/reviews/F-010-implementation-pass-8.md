# F-010 review, pass 8

**Reviewed**: the fully staged index on `work/f-010-claude`, 48 paths,
+8650/-41, after seven rounds of remediation
**Result**: 2 defects, 0 smells, 4 nitpicks

Thirteen mutations run, all restored and digest-verified. Both defects are
DICOM section citations, and one was introduced by pass 7's own remediation of
a DICOM section citation.

## Defects

### D1, the round-7 citation fix shipped a new false citation

**Where**: `tools/oracle/page/app.mjs`, and the same sentence in
`.claude/reviews/F-010-implementation-pass-7.md`
**What**: the new comment said `C.7.6.3.1.1's attribute table does not contain
it`. **PS3.3 C.7.6.3.1.1 is "Samples per Pixel"**, a prose attribute
description, not a table. The Image Pixel Module's tables are C.7-11a and
C.7-11c.
**Why it is wrong**: `CLAUDE.md`'s "against the cited specification section, not
against the comment above it", on the block a reader would use to decide
whether an attribute is read from the right module. It also contradicts the
repository's own numbering: `unsupported.json` and `docs/lld/oracle.md`
correctly cite C.7.6.3.1.2 for Photometric Interpretation, which is only
consistent if C.7.6.3.1.1 is Samples per Pixel.
**Evidence**: PS3.3 C.7.6.3.1 lists `.1.1` Samples per Pixel, `.1.2`
Photometric Interpretation, `.1.3` Planar Configuration, `.1.4` Pixel Data.
C.7.6.6 is the Multi-frame Module and Number of Frames is Type 1 in Table
C.7-14, which is the part that was right.
**Fixed**: the comment now reads `// PS3.3 C.7.6.6, Multi-frame Module. Not
Image Pixel.` and stops there. That is the reviewer's own advice and the
microscope's: prefer deleting a wrong sentence to explaining it, because a
remediation that adds three sentences ships three new claims.

### D2, a hand-written fixture cited a section that does not contain the rule

**Where**: `tools/oracle/check_sidecars.py`
**What**: the signed-twelve-in-sixteen fixture cited PS3.3 C.7.6.3.1.4.
**C.7.6.3.1.4 is "Pixel Data"** and its entire text is the ordering rule, left
to right and top to bottom. It says nothing about Bits Allocated, Bits Stored,
High Bit or alignment within a container.
**Why it is wrong**: `CLAUDE.md` requires a fixture to cite its DICOM section,
and the values here are right, so this was a false claim rather than a wrong
number. The microscope's severity table still calls that a defect.
**Fixed**: it now cites PS3.5 8.1.1, the Pixel Cell, with High Bit (0028,0102)
defined in PS3.3 C.7.6.3.

**Scope note carried to the integrator.** The same citation appears in six
already-landed tracked files from F-009 and S01, and most of those pair it with
PS3.5 8.1.1, which is the correct half. Only this instance dropped it.
Correcting the convention repository-wide is a separate decision and is not
made here.

## Nitpicks, and what was done

- `tests/faults.mjs` had the byte-identical chunk-coercion pattern that round 7
  fixed in `run.mjs`, in the file whose output decides whether all six boundary
  guards fired. **Fixed there too**, with the note that every `expect` fragment
  is ASCII so nothing could have been corrupted.
- Both round-7 `structuredClone` calls were hardening that nothing asserted:
  removing either left all 88 tests green. **Two tests added**, and each clone
  now goes red when removed. 90 tests.
- `--report-unsupported`'s usage text listed two of the four things it does on
  the way. **Corrected**: it also runs the unit suites and rebuilds the page,
  and "writes nothing" is now "writes no reference output".
- `manifest.mjs` hardcoded `expected 9 columns` beside a comparison against
  `MANIFEST_COLUMNS.length`. **Now one number.**

## Verified clean

- **Every round-7 change verified.** The re-synced adapter's digest was
  recomputed by hand and matches. The `unsupported_test.mjs` header's new claim
  was measured by replaying the two recorded failures with each conjunct
  removed: all four give the same outcome, so all four are individually
  redundant today, as the header now says. `build-page.mjs`'s "ten subpaths" is
  right: the exports map has eleven keys, `"."` plus ten subpaths, and the
  worker source is reachable through none of them.
- **The unconditional `VOILUTFunction` was verified pixel-neutral by
  measurement, not by assumption.** `resolveVoi` returns bare `{source:"none"}`
  for the four colour rows, so the fallback yields `"LINEAR"`, a valid enum
  value. `StackViewport.setProperties` applies `voiRange` and `VOILUTFunction`
  independently, and with no range the second re-issues `setVOI` with an
  unchanged range, which early-returns. The reviewer reverted the line and
  re-rendered all four affected rows: identical digests in every case.
- **Every DICOM citation in the diff was checked individually** against the
  standard: PS3.10 7.1, PS3.3 C.12.1, C.7.3.1, C.7.6.3, C.7.6.6, C.11.1,
  C.11.2, C.11.2.1.2, C.11.2.1.3.1, C.11.2.1.3.2, C.7.6.2, C.7.6.2.1.1,
  C.7.6.1.1.5, C.7.6.3.1.2, C.7.6.3.1.3, C.7.6.16, C.7.6.16.2.10, PS3.5 8.2.1.
  All correct except the two above. All 25 attribute readings correspond to
  real tags.
- **`dicom-parser` "is not part of the renderer" was checked rather than
  assumed**: `init` passes no `useLegacyMetadataProvider`, so the three loader
  modules that import it at run time are all on the legacy path. That
  independently corroborates `unsupported.json`'s deflated-syntax diagnosis.
- The VOI arithmetic was re-derived from C.11.2.1.2 with the fixtures
  recomputed by hand, and `reference_mono12`'s letterbox re-derived from the
  sidecar rather than from the document, matching the recorded `blackFraction`
  to the digit.
- The new oracle-output refusal was proved load-bearing: pointing its path
  prefix elsewhere lets an 83 KB reference PNG of a real corpus row through the
  DICOM check, the artefact check and the size limit.
- Gate wiring re-traced: `--sprint` and `--all` select identical sets,
  `s01_pre_oracle` has no caller, `skip()` returns 3 from exactly three arms,
  and the oracle arm has no skip path.
- All eleven required commands green, re-run after every mutation.
- After remediation: 90 unit tests pass, `npx eslint .` exits 0,
  `prose_check.py` is clean over 75 files, and `bin/ocelli.sh gate skills prose
  content oracle` is ALL GREEN over four gates, with 91 attempted, 89 read
  back, 2 accounted for, identical digests across two passes and all six faults
  red at their named boundary.

## The loop signal, recorded rather than smoothed over

The pass count is rising and no single finding has survived three passes, which
is progress. But the same CLASS, a false DICOM citation, was the blocking
finding in passes 6, 7 and 8, and in pass 8 the false sentence was introduced by
pass 7's remediation of the same class. The remedy applied here is the one the
reviewer named: correct the citation by naming the section and add no new
explanatory sentence. Both fixes in this round are shorter than what they
replaced.
