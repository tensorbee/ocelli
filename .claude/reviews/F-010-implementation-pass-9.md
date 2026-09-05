# F-010 review, pass 9

**Reviewed**: the fully staged index on `work/f-010-claude`, 49 paths,
+8810/-41, after eight rounds of remediation
**Result**: 4 defects, 1 smell, 3 nitpicks

Fourteen mutations run, thirteen red and one green by construction. Round 8's
two citation fixes are both correct and shipped no new false sentence, which is
the first round in four where that is true. But three older citations survived
three passes that each declared the citation set "checked individually" while
checking a subset, and the fourth defect is the most serious finding of the
whole review.

## Defects

### D1, the driver ran nothing and exited 0 under a repository path containing a space, and the gate reported green

**Where**: `tools/oracle/run.mjs`, `tools/oracle/tests/faults.mjs` and
`tools/oracle/build-page.mjs`, all three carrying
``if (import.meta.url === `file://${process.argv[1]}`)``
**What**: `import.meta.url` percent-encodes and resolves symlinks.
`process.argv[1]` does neither. So any repository path containing a space, a
`#`, a `%` or a non-ASCII byte, or reached through a symlink, makes the two
unequal. The main block never runs, node exits 0, and `bin/ocelli.sh` reports
the oracle gate **passed** having rendered nothing.
**Why it is wrong**: this is verbatim the sprint's named defect for this story,
one level up. Not a page that starts and does nothing, but a harness that does
not start and says nothing. It defeats all four boundary assertions, the pin
check, the determinism pass and the fault self test at once, and
`tests/faults.mjs` carried the same bug so the self test could not have caught
it either.
**Evidence**, reproduced independently before fixing, through a symlink named
with a space and pointing at the real worktree:

```
$ node "wt space/tools/oracle/run.mjs" --help
exit=0                              # no usage text at all
$ node <plain path>/tools/oracle/run.mjs --help
bin/ocelli.sh oracle [options]      # the plain path works
```

**Fixed**: `isEntryPoint(import.meta.url)` in `src/paths.mjs`, comparing
`realpathSync(process.argv[1])` with
`realpathSync(fileURLToPath(import.meta.url))`. `fileURLToPath` does the
decoding, `realpathSync` does the symlink and `..` resolution on both sides, so
the two are compared as the same kind of thing. All three call sites now use
it. Re-observed: the same symlinked path with a space now prints the usage
text.

**And it is now a regression test rather than a fixed bug.**
`tests/paths_test.mjs` builds a real symlink whose name contains a space and
runs the real driver through it, requiring it to print usage and to REFUSE an
unknown argument with exit 1. It also asserts that all three modules with a
main block use the shared guard, because a fix applied to one of three copies
is how this class comes back. Reverting the guard to the old idiom turns two
tests red.

### D2, `PS3.3 C.11.2.1.3` cited as the source of a rule it does not contain

**Where**: `tools/oracle/src/voi.mjs` twice, and `tests/params_test.mjs`
**What**: C.11.2.1.3, "VOI LUT Function", is entirely about a **present**
value. It says nothing about the attribute being absent.
**Why it is wrong**: the same class as pass 8's D2, values right and cited
section not carrying the rule, on the module whose arithmetic `CLAUDE.md`
singles out. The rule lives in C.11.2.1.2.1, "If VOI LUT Function (0028,1056)
is absent or has a Value of LINEAR", and in Table C.11-2's own description.
**Fixed**: all three sites now cite C.11.2.1.2.1. The half about
LINEAR_EXACT's and SIGMOID's width constraint was already correct via
C.11.2.1.3.1 and C.11.2.1.3.2 and is now cited that way explicitly.

### D3, "PS3.3 C.7.6.2 makes Pixel Spacing optional", where it is Type 1

**Where**: `tools/oracle/src/params.mjs` and `tests/params_test.mjs`
**What**: Pixel Spacing (0028,0030) is **Type 1** in Table C.7-10. C.7.6.2 does
the opposite of making it optional.
**Why the real reason matters**: the comment is the only explanation a later
reader gets for the `?? 1` fallback. Exactly three corpus rows have no
top-level Pixel Spacing, and none of them is "the module made it optional": the
DX and US rows are IODs that do not include the Image Plane Module at all, and
the enhanced CT row carries it in a shared functional group. The reference
resolves a value for all three, so the fallback is reached by the unit test and
by nothing else.
**Fixed**: both sites now say that, and say which rows and why.

### D4, "C.7.6.1.1.5 then requires the instance to declare that it is lossy", where both attributes are Type 3

**Where**: `tools/oracle/check_sidecars.py`
**What**: Lossy Image Compression (0028,2110) and Lossy Image Compression
Method (0028,2114) are both **Type 3** in Table C.7-9. The section says so
itself: "defined as Type 3 for backward compatibility with existing IODs".
**Why it is wrong**: this comment was the stated authority for two hand-written
fixture values. The values are right because `corpus_synth.py` writes them, not
because the standard requires them, and the fixture claimed otherwise.
**Fixed**: it now says both are Type 3 and that the generator writing them is
the claim being checked. The first half was corrected in the same edit: PS3.5
Table 8.2.1-1 permits YBR_FULL_422 **or** RGB for JPEG Baseline at three
samples per pixel, so the rewrite is the encoder's choice rather than the
standard's requirement.

## Smell

### S1, "the directory holds one complete run, or it holds nothing" was falsifiable

**Where**: `tools/oracle/src/output.mjs`, `docs/lld/oracle.md`, `run.mjs`
**What**: `report()` wrapped the pydicom cross-read and the fault self test so
their failures reach the single discard path. It did not wrap the write block.
`assertFrameIntegrity` is designed to throw, and a throw mid-loop went straight
past the discard, leaving a partial run with no `run.json` that the next run's
`prepareOutput` then refuses as foreign, a second unrelated-looking failure
needing a manual removal.
**Evidence**: a probe against the real `output.mjs` wrote row one, threw on row
two, and left three files and no `run.json`, which a later `prepareOutput`
refused.
**Fixed**: the write block is now wrapped like the two checks below it, so
every failure reaches the one discard path and the sentence is true again.

## Nitpicks, and what was done

- The signed-twelve-in-sixteen fixture cited PS3.5 8.1.1 as authority for an
  encoding that **the current edition of that same section retires**: it now
  requires High Bit to be one less than Bits Stored. **Said so**, because that
  is precisely why the corpus carries the row.
- `validateRecord` accepted `errorContains: ""`, which every string contains,
  silently retiring the fourth conjunct while looking like a complete entry.
  **Refused now**, along with an empty string inside `rows`.
- `C.7.6.2.1.1` is cited for `PixelSpacing[0]` being the row spacing, and that
  section defines the geometry without saying which array index is which. The
  ordering is in Table C.7-10's description. **Left**: the repository's own
  `dicom-expert` skill uses the same citation for the same fact, so this is a
  convention question rather than a local slip, and changing it here would put
  this file out of step with everything else.

## Verified clean

- **Round 8's two fixes are correct in every word.** `C.7.6.6` is the
  Multi-frame Module and Number of Frames is Type 1 in Table C.7-14, and the
  tag appears nowhere in Table C.7-11a, C.7-11b or C.7-11c. PS3.5 8.1.1 defines
  the Pixel Cell and says High Bit is where Bits Stored sits within Bits
  Allocated, and High Bit is in C.7.6.3 via Table C.7-11c, which itself says
  "See PS3.5 for further explanation", so the split the comment draws is the
  standard's own.
- **Every remaining DICOM citation in the diff was swept exhaustively**, not
  sampled, and all are correct: PS3.10 7.1, PS3.3 C.12.1, C.7.3.1, C.7.6.3,
  C.7.6.6, C.11.1, C.11.2, C.11.2.1.2, C.11.2.1.3.1, C.11.2.1.3.2, C.7.6.2,
  C.7.6.1.1.5 (the section is the right place for both tags, only the word
  "requires" was wrong), C.7.6.3.1.2, C.7.6.3.1.3, C.7.6.16, C.7.6.16.2.10,
  PS3.5 8.1.1, PS3.5 8.2.1, PS3.5 A.5.
- **`unsupported.json`'s YBR_FULL_422 diagnosis is right down to the byte
  order.** C.7.6.3.1.2 carries the exact formula the entry quotes, with `* 2`
  rather than `* SamplesPerPixel`, and "Two Y values shall be stored followed
  by one CB and one CR value". 12 by 20 by 2 is 480 against 720.
- **`fullRange`'s derivation is the standard's own Note.** C.11.2.1.2.1 says a
  centre of `(x1+x2+1)/2` and a width of `(x2-x1+1)` selects exactly the range
  x1 to x2, which is what the function computes. The three hand-computed
  fixtures were recomputed independently.
- **Every number in the tracked prose executed against the real run.** 91 rows,
  89 sidecars, the four boundary counts, 16 pinned packages, 87 magnified and
  exactly two fitted down, 85 file windows and 4 colour with both fallback
  branches test-only, 84 top-level VOI origins and 1 functional-groups and 4
  absent, 16 low-information rows, 62 `series` tokens, and
  `reference_mono12`'s 0.25 letterbox matching its recorded `blackFraction`.
- **The pins machinery measured from the installed tree**: six on the walk,
  each finding its manifest at the second step, five name-less `type: module`
  manifests none of which enters the walk, eleven exports-map keys.
- Page correctness re-checked by hand: `uniformValue` starts at index 4,
  `frameStatistics` cannot double-count, `readBack` copies the whole buffer,
  `sha256Hex` slices by offset and length, the sentinel is painted before the
  render promise, a new imageId per row plus a cache purge closes the stale
  pixel path, and the current image id is compared against the row's.
- Boundary, tier and structure unchanged and clean. All eleven required
  commands green.
- After remediation: 97 unit tests pass, `npx eslint .` exits 0,
  `prose_check.py` is clean over 76 files, and `bin/ocelli.sh gate skills prose
  content oracle` is ALL GREEN over four gates with every frame digest
  unchanged.

## The loop signal

Nine passes. The citation class was blocking for a fourth round, but with a
difference the reviewer named: none of the three citation defects in this pass
was introduced by a previous remediation. They had been in the tree since the
code was written, and three passes had declared the set checked while checking
a subset. Pass 9 swept every `PS3.x` string in the diff and resolved each once
against the standard's own section index, which is the mechanical remedy rather
than a behavioural one, and that is what found them. D1 was found by a
different route entirely, and is the finding that mattered most.
