# F-118, provenance classification

**Status**: approved
**Epic ref**: E19.5
**Sprint**: S41
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/24-agent-code-standards.md` section 27.2 R6 says this. The fence is the author's text.

```text
Provenance trailer on every commit
Cheap now; a retrofit across sixty thousand lines is not, and a device pathway may require it.
```

The commit-msg hook writes `Ocelli-Verify` and `Ocelli-Generated-By` from the verify ledger. A hand-written `Ocelli-Verify` line is refused. An agent `Co-Authored-By` trailer for Claude, Codex, GPT or Copilot is refused. The hook is the writer.

## What the specification does not cover

R6 does not say how a reader classifies a message that already exists. It does not say what a missing trailer means on an open commit, and it does not say what a partial trailer means.

Those gaps are local decisions. They are not HLD formulas.

`classifyProvenance(message)` returns one of three words.

- `recorded` when the message has one `Ocelli-Verify` line and one `Ocelli-Generated-By` line, and the verify line contains `profile=`, `gates=`, `corpus=` and `tree=`.
- `missing` when it has neither trailer line and it has no agent co-author line.
- `forged` when an agent co-author line is present, when only one of the two trailer lines is present, when a trailer line is duplicated, or when the verify line lacks one of those four fields.

The function does not write a trailer and does not call the hook. The hook stays the only writer.

## Problem and reproducer

A commit message can carry a hand-written verify line. The hook refuses that at commit time. A message that is already text has no classifier, so a later check can only look at it by eye.

## Scope, API and compatibility

In scope:

- `classifyProvenance` in `packages/core/src/provenance.ts`
- Tests and the published name
- A short note in `docs/lld/public-api.md`

Out of scope:

- Editing `.githooks/commit-msg`
- Writing trailers into old commits
- Amending the S41 open commit
- An agent co-author trailer

The co-author test is case-insensitive on the token Claude, Codex, GPT or Copilot after `Co-Authored-By:`. A `Co-Authored-By` line that names none of those tokens does not by itself forge the message.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| A reader of one commit message | `classifyProvenance` | The full message text | recorded, missing or forged |
| commit-msg hook | unchanged | Still writes from the ledger | Unchanged |
| Verify ledger | unchanged | Still keys on the index | Unchanged |

## State transitions and invariants

Stateless. The message is not rewritten. `missing` is not success. `recorded` does not prove the named tree matches a commit. It proves only that the two lines have the required shape.

## Ownership, commit and recovery

Not applicable. No commit is created.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Both trailers and the four verify fields return `recorded` | unit | Requiring the comparison fields | implementation |
| AC2 | A message with neither trailer returns `missing` | unit | Calling an open commit forged | implementation |
| AC3 | One trailer without the other returns `forged` | unit | Accepting a verify line alone | implementation |
| AC4 | A verify line missing `tree=` returns `forged` | unit | Accepting a partial field list | implementation |
| AC5 | A duplicated trailer line returns `forged` | unit | Counting two verify lines as one | implementation |
| AC6 | An agent co-author line returns `forged` even when both trailers are present | unit | Ignoring the co-author line | implementation |
| AC7 | The published name is exported | `index.test.ts` | Leaving the function private | implementation |

## Approach

One new module. No wasm-bindgen. No pixels. No `unsafe`. The hook is not edited.

## Tiers

| Tier | Behaviour |
|------|-----------|
| A | Not applicable. The classifier does not render. |
| B | Not applicable. The same word is returned. |
| C | Not applicable. The same word is returned. |

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | recorded, missing and forged | `provenance.test.ts` |
| export | published name | `index.test.ts` |

## Parity surface covered

None.

## Deviations

None.

## LLD impact

`docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`.

## Open questions

None. The three-way classification is the approved local decision.
