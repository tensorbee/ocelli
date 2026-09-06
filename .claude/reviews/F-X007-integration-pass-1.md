# F-X007 review, integration pass 1

**Reviewed**: `b878c9c`, the integrated F-ID commit, against `9df8539`.
**Reviewer**: the integrator, independent of the implementing agent.
**Result**: 1 defect found and fixed by the author on top, then 0 defects and
0 smells on the re-check.

## Defects

### D1, the volume counters reported attempts where the stack counters reported achievements

**Where**: `tools/oracle/run.mjs`, the volume accounting, and `run.json`'s
`boundaries`.
**What**: `reformatsPresented` and `reformatsReadBack` both read 12 while nine
reformats existed in `volumes[].frames` and nine `volume__*.raw` files existed
on disk. Twelve is four subjects times three orientations, the DECLARED count.
**Why it is wrong**: the same `boundaries` object used the same words to mean
achieved for stacks, where `readBack: 89` equals 89 files on disk. So one record
used `readBack` two ways and the volume side claimed three read-backs that
produced nothing. `docs/lld/oracle.md`'s own rule is that covered is not the
same as measured, and the four boundaries exist so a count cannot claim more
than happened.
**Evidence**: `run.json` at `0a5267b` gave `reformatsReadBack: 12`,
`sum(len(v['frames']))` gave 9, and `ls volume__*.raw` gave 9.
**Root cause, stated by the author after the fix and worth keeping**: the volume
boundaries do not run in the order they are listed. `volume-geometry` is the
driver's and runs LAST, after the page has presented every orientation, so a
subject can be refused having already rendered three reformats.
**Fixed at `00db7c7`**, and verified by me: all three counters now read 9,
`reformatsDeclared: 12` publishes the declared total under a name that says so,
the refused subject keeps `reformatsPresented: 3` and `reformatsReadBack: 3` on
its own entry rather than having the fact erased, and the accounting identity
now covers the trio. The author observed the identity red before claiming it,
with `MUTATED_RUN_EXIT=1` and `REVERTED_RUN_EXIT=0`.

**The transferable lesson, and the author wrote it at the site**: the identity
that existed covered `reformatsWritten` alone, which is exactly why the defect
survived. A counter with no identity on it is a counter nothing can contradict.

## Smells

None.

## Verified clean

**The acceptance check, run by me twice.** All 89 stack `rows[].sha256` are
byte-identical to the baseline taken at `e1bcf53` before this story, compared
per row: zero moved, zero missing, zero extra, and the combined digest
`02bfccad111154f2` matches. Checked again after the remediation commit.
`render-params.json` is untouched across the whole change, so
`renderParamsSha256` is mechanical evidence that the stack frames are the same
artefact.

**The prediction was tested, not assumed.** The uniform and non-uniform
synthetic series render bit-identically in all three orientations, which is what
the endpoint-only spacing derivation entails, and `volume-truth.json` asserts it
so the reference improving turns it red.

**Counter semantics now agree across the record.** Stack `readBack: 89` equals
89 raw files. Volume `reformatsReadBack: 9` equals 9 raw files and 9 frame
entries. Both verified against `ls`.

**Gates.** `bin/ocelli.sh oracle` exit 0 on the merged tree, `gate --floor` ALL
GREEN over 24, `gate corpus` pass. That is a full sprint profile over 26 gates,
recorded in the ledger, and the commit carries the trailer.

## Nitpicks

None blocking.

## Carried forward, not defects

- **Six of the nine volume reformats are low-information**, both AXIAL frames at
  100 per cent black and white. The `framePairs` claim is carried by SAGITTAL
  and CORONAL, which cut across slices. F-011's `weak` qualifier must reach
  volume views.
- **F-011 gets no real CT volume reference**, because the corpus's only real CT
  series is the one that is not a volume.
