# F-230 review, pass 7

**Reviewed**: staged tree `2de7f4431f22e3e94f7505e89fa4df387e04701f` in
`ocelli-wt-f-230`, branch `work/f-230-xai`. No unstaged tracked changes.
Untracked `tools/oracle/out` and `.claude/reviews/` excluded.
**Author / reviewer**: implementation by an external coding agent, pass-6 remediation of the
test by the sprint lead (Claude). Reviewer is an independent Claude subagent
that did not write the remediation.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 6 S1: no test held the two clauses of the frame-meta carve-out in
`docs/lld/public-api.md` lines 94 to 97. Every non-object, a function
included, must get `InvalidOptions` with `frame meta is not an object` at
`acceptTransferredFrame` and at `writeFrame` with a ticketed sink, and an array
must get the generic `FrameCommitMeta is not an options object`.

Expectations derived from the doc text, not the code. A function has
`typeof` `"function"`, so a guard written as `typeof x !== "object"` sends it
to the specific message. An array has `typeof` `"object"` and is not `null`,
so it passes the specific guard and reaches `refuseUnknownOptions`, whose
`Array.isArray` clause (`options.ts:176`) gives the generic message. Both
plausible wrong behaviours (an array swallowed by the specific guard, a
function let through to the generic one) were introduced at each public entry
point separately and each was rejected by the widened test.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| M1: `bulk.ts:197` adds `\|\| Array.isArray(meta)` (writeFrame, array gets specific) | staged tree, `errors-class.test.ts` | now | red, 1 failed / 10 passed, expected `FrameCommitMeta is not an options object`, received `frame meta is not an object` |
| M2: `bulk.ts:197` becomes `(typeof meta !== "object" && typeof meta !== "function") \|\| meta === null` (writeFrame, function gets generic) | same | now | red, 1 failed / 10 passed, expected `frame meta is not an object`, received the generic message |
| M3: `bulk.ts:397` (acceptTransferredFrame guard, throw on 398) adds `\|\| Array.isArray(target)` | same | now | red, 1 failed / 10 passed, same direction as M1. writeFrame was unmutated, so the failing assertion is the acceptTransferredFrame one |
| M4: `bulk.ts:397` function let through to the generic message | same | now | red, 1 failed / 10 passed, same direction as M2 |
| Restore after each mutation, then `cmp` against a pre-mutation copy and `git diff -- packages/core/src/bulk.ts` | worktree | now | identical, unstaged diff empty |
| `npm run typecheck` | staged tree | now | exit 0 |
| `npm run lint` | staged tree | now | exit 0 |
| `npm run test` | staged tree | now | exit 0, 60 files, 486 tests passed |
| `python3 scripts/prose_check.py` | staged tree | now | exit 0, clean over 844 files |
| `python3 scripts/source_provenance_check.py` | staged tree | now | exit 0, clean over 1674 files |
| `git write-tree` at end | index | now | `2de7f4431f22e3e94f7505e89fa4df387e04701f`, unchanged from start |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass 6 S1 is closed. The loop at `errors-class.test.ts:265` covers `null`,
  `undefined`, `7`, `"meta"`, `true`, `Symbol("meta")` and a function through
  both ticketed `writeFrame` and `acceptTransferredFrame`, and lines 287 to 297
  assert the array case for both.
- The expected strings match the code byte for byte. `frame meta is not an
  object` at `bulk.ts:198` and `bulk.ts:398`, and
  `` `${owner} is not an options object` `` at `options.ts:177` with owner
  `FrameCommitMeta` at both call sites. `expectCode` checks class, `name`,
  `code` and `message` with `toBe`, so a near-miss string would fail.
- `expect(allocs).toBe(0)` holds at line 298, so every refusal, the array
  included, lands before `alloc`, which is the doc's "before any allocation"
  sentence. The `handoff` length is 4 and the buffer is 4 bytes, so the
  `LengthMismatch` branch is not what refuses at acceptTransferredFrame.
- The test now discriminates per entry point, which is what pass 6 asked: M3
  and M4 mutate only `acceptTransferredFrame` and still go red.
- The third guard at `bulk.ts:450` is the internal `frameCommitMeta`, reached
  only after the public guards or through `sessionBulkSink().commitAllocated`,
  which `errors-class.test.ts:245` covers for `null`. It is not part of the
  carve-out sentence.

## Limitations and handoff

- This pass reviewed the pass-6 remediation and the carve-out it covers. It
  did not re-review the rest of the F-230 diff, which earlier passes covered.
- The four mutations were applied with `sed` and restored from a copy. Only
  `errors-class.test.ts` was run under mutation, not the full suite.
- A clean bounded review is not completed acceptance. `/verify` and
  `/complete-feature` remain with the sprint lead.
