# F-010 review, pass 13

**Reviewed**: the fully staged index on `work/f-010-claude`, 56 paths,
+10321/-41, after twelve rounds of remediation
**Result**: 1 defect, 1 smell, 4 nitpicks

Sixteen mutations run. The pass named the cause of a pattern that had run for
five rounds, and it is worth quoting because the remediation below is shaped by
it: **round 12 mutated its guards in the same command that added them, which
proves the guard fired once and leaves nothing watching it afterwards.** Pass
12's D1 fix was four tests and held. Its S1 fix was a hand-run demonstration
and did not.

## Defect

### D1, the subdirectory refusal was the string comparison the line above it says is not enough

**Where**: `tools/oracle/run.mjs`
**What**: round 12 added `options.out.startsWith(DEFAULT_OUT + sep)` directly
beneath a comment explaining that a string comparison misses a case-only
variant and a symlink. The new branch was a raw string prefix test, defeated by
exactly those two spellings applied to the PARENT, and its error message
enumerated five spellings and said all were refused.
**Evidence**: `<out>/sub` refused, `<OUT>/sub` and `<symlink-to-out>/sub`
accepted. On this filesystem `OUT/` resolves, so the accepted run would have
nested a partial render inside the reference render, which is the outcome the
guard's own comment says it prevents.
**Why it is a defect and not a smell**: the message is a false claim in prose,
and a reader of it would stop looking.
**Fixed** the way the reviewer suggested, which collapses the two branches into
one: `isInside` walks the candidate up through its ancestors and asks
`sameDirectory` about each, so the case and symlink handling lives in one place
and covers containment too. The message now says what is true.
**Watched**: four new tests, including the two spellings that were accepted and
a sibling sharing a name prefix, which must NOT be refused. Reverting to a
string prefix turns **five** red. Stopping the walk after one step turns
**two** red.

> **Corrected during the final guard sweep.** This paragraph originally
> said eight and four.
> Neither number could be reproduced, because the command that produced them
> was `node --test tests/`, which fails unconditionally: node treats the
> directory itself as a test and reports one failure before running anything.
> The correct invocation is `node --test tests/*_test.mjs`, and the numbers
> above are measured with it. The mutations themselves are unchanged and both
> still go red.

## Smell

### S1, round 12's own remediation was watched by nothing, and the test beside it watched the wrong thing

**Where**: `tools/oracle/page/app.mjs`, `tools/oracle/tests/params_test.mjs`
**What**: the interpolation guard's LOGIC was correct, verified against the
installed enum. But replacing it with `if (false)` left 127 of 127 green, and
so did deleting the `camera.mode` guard and the unconditional
`VOILUTFunction`. **Several of the refusals in `page/app.mjs` were reached by
nothing**, including the one that stops the previous row's frame being written
under this row's name, which is the quietly-wrong-pixel class this project
names as its dangerous defect.

> **Corrected during the final guard sweep.** This sentence originally
> counted six of eleven.
> The count was taken by hand and is not reproducible against any tree, so it
> is stated without one. What is reproducible is the final state: every
> refusal in `page/app.mjs` is reached by a named fault injector, and the
> mapping is in `docs/lld/oracle.md`. **Pass 13 is the last review pass on
> record.** No pass 14 was written, and the sweep that produced these
> corrections is recorded in `.claude/handoffs/F-010-ready.md` instead.

The test round 12 added was titled "the declared interpolation is a name
cornerstone3D defines" and never imported cornerstone3D. Its first assertion
made the other two unreachable, and the assertion above it already made the
first redundant. Deleting both left 127 green.

**Fixed on both sides.**

*The test now asks cornerstone3D.* It imports `Enums` under node, which works
and costs nothing measurable, and asserts the declared value is one of the
enum's own non-numeric keys. The title is now true, the dead assertions are
gone, and cornerstone3D renaming or dropping `NEAREST` goes red. Declaring
`"constructor"` in the spec turns two red.

*Four new fault injections*, taking the self test from six to ten, so the
refusals inside the boundaries are observed red rather than argued about:

| Fault | Boundary | What it breaks |
|-------|----------|----------------|
| `no-stack` | presented | the row never reaches the viewport |
| `stack-throws` | decoded | `setStack` rejects |
| `bad-interpolation` | presented | an interpolation resolved from `Object.prototype` |
| `bad-camera` | presented | a camera mode the page does not implement |

`no-stack` is the important one. It skips `setStack` rather than throwing
inside it, so the row never reaches the viewport and the identity check is what
catches it, which is the guard that matters. Throwing there would have
exercised the catch instead, and that is `stack-throws`. All four observed red
with the right message, and the full gate now reports ten faults red at their
named boundary.

`bad-interpolation` uses `constructor` specifically, so the injection is the
prototype hole itself rather than a nonsense word.

## Nitpicks, and what was done

- **`codecPackages`'s unscoped branch was unreachable and untested**, and
  taking two segments unconditionally would have named a package that does not
  exist. **`packageNameOf` is now exported and tested for all four specifier
  shapes**, and the mutation goes red.
- **Two plain-object lookups sat twenty lines from a correct `Object.hasOwn`**
  in the same function. A dependency named `constructor`, a legal npm name,
  would have failed OPEN at one of them. **Both now use `Object.hasOwn`.**
- **`minimumWidth` is PS3.3-correct and cornerstone3D's `toLowHighRange` is
  not**: it applies LINEAR's `(w - 1) / 2` to SIGMOID, so a SIGMOID window
  narrower than 1 would be accepted here and produce an inverted range in the
  reference. Unreachable today, because all eighty-five windowed rows resolve
  LINEAR. **Recorded in the LLD**, because the first SIGMOID row added to the
  corpus will meet it, and because it is the reference's divergence rather than
  this harness's.
- **The LLD absence regex matches table rows only**, while the comment said
  "every spelling of the row". **The comment now says what the regex does**,
  and notes that the file carries no prose count for it to miss.
- `CLAUDE.md`'s "Current state" remains for the integrator.

## Verified clean

- **Every round-12 change was attacked.** The interpolation guard's logic is
  right: `Object.keys` yields own enumerable keys only, and the numeric filter
  strips the reverse mappings. The other string enum was checked as asked:
  `VOILUTFunctionType`'s keys and values differ, so copying the same pattern
  there would be wrong, no such copy exists, and a bogus declared function
  fails loudly at `toLowHighRange` before `setProperties` can silently
  downgrade it. `toLowHighRange`'s argument order was read from the bundle and
  matches the call. `resetCamera` touches no VOI or interpolation state, so the
  ordering is safe. A row failing after `setStack` succeeded lands in one of
  three structured returns. The four new manifest tests pin values, not just
  messages.
- **Runbook row 20 was re-executed in a scratch repository** and row 21 gave
  exactly eight failures again.
- **Every prose count was re-derived**, including the two downsampled rows with
  their `parallelScale` arithmetic recomputed from the files, the 85 windowed
  against 4 colour surveyed across all 89 sidecars, and pass 12's own header
  reconciled line for line against the diff it reviewed.
- The oracle gate still has no skip path, `--sprint` and `--all` still select
  identically, and the eslint split is still two-way.
- After remediation: **132 unit tests** in ten suites pass, `npx eslint .`
  exits 0, `prose_check.py` is clean, and `bin/ocelli.sh gate skills prose
  content oracle` is ALL GREEN over four gates, with ten faults red at their
  named boundary and every frame digest unchanged.

## The loop signal

Thirteen passes, and this is the round the pattern should break, because the
cause was finally named rather than the symptom. Every guard added or changed
here is watched by a TEST, not by a mutation run once at the terminal: the
containment walk, the interpolation name check, `packageNameOf`, and the four
new injections which are themselves permanent and run on every gate. The
distinction pass 13 drew is the one to hold: a hand-run mutation proves the
guard fired once, a test keeps proving it.
