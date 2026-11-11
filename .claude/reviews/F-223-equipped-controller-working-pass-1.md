# F-223 equipped controller repairs, independent working review pass 1

Reviewer: root Codex, independent of the implementing Codex worker.
Reviewed source: `f561bec17ecc16c6e3baa9f83919a59e768c09d1`, complete
delta against `5e6b3b81c398a557f1394d90df2c94b5e92617fa`.
Result: 0 defects, 0 smells, 0 nitpicks in these bounded repairs.

## Findings and repairs

The first actual equipped attempt failed before timings. Three Codex
controller defects were subsequently isolated, separate from the original
GLM page findings.

1. Selected subsets were symbolic links. The installed directory-upload path
   omitted them, submitting zero files. The controller now creates neutral
   regular copies with `COPYFILE_FICLONE`, which permits ordinary-copy fallback.
   Source bytes, ordering, original full-root submission, deny scanning and
   cleanup are retained. The pure control checks independent storage and exact
   bytes. The browser control reaches both real folder consumers.
2. The Cornerstone capture callback named its parameter `window`, hiding the
   browser global. Renaming it to `request` restores the actual capture caller.
   The permanent regression calls `pixelPrecheck` and the release comparator,
   not just manual page captures. No tolerance or pixel predicate changes.
3. Warm trials retained measured pages when creating their successors.
   `newPage` now awaits closing the preceding page and its automatically owned
   context. The browser and native server remain retained. Real `measureAll`
   orchestration checks three fixture trials per side, an input failure, later
   recovery, at most one live context, and final page/browser cleanup.

The existing global cleanup still handles page-close, navigation and trial
failures. Serial callers do not request concurrent pages from one actor.
Production workload definitions, counts, endpoints and publication policy are
unchanged. A documentation nit calling decision 12 AC12 was corrected.

## Actual controls

The first failed full attempt exits 1 after 150.554 s, with no timings.
Symlink-only controls fail at uploaded 0 versus 1. A separate shadow-only
control fails inside the real capture caller. The old warm-ownership control
fails at the actual browser-context invariant, with the intended input error
and recovery exercised. These are distinct controls, with corrected source
restored between mutations.

The final three-case browser suite passes. The actual controlled 200 CT
precheck passes with informative fraction 0.466796875, maximum difference 1,
no pixels over the existing maximum, and signed informative bias -0.0585774.
Both sides report Chromium 151 and the Apple M4 Max Metal renderer. The final
precheck uses release binary SHA-256
`b32c96cf33a3c719b49150b98040db4ffce8a02f82fe0e8e724be51ddaac2d59`,
which includes root's later byte-store implementation. Its identity is
separate from this instrument-only snapshot. All owned actors close.

Receipts and exact source/mutant identities are under
`/private/tmp/ocelli-s46-codex/f223-open-diagnosis/`. The runtime GLM audit
remains on the earlier db719 snapshot and was not supplied this review.
These controls are correctness diagnostics, not performance observations.
No completed timing matrix, generated publication, feature profile, manual
check or story completion is claimed by this bounded review.
